// tray-icon implementation for macOS and Windows
// Note: about and menu modules are declared in mod.rs

use super::menu::TrayMenu;
use crate::auth::AuthManager;
use crate::config::Config;
use crate::error::{Result, SyncError};
use crate::platform::{ThemeChange, ThemeWatcher};
use crate::sync::{SyncManager, SyncStatus};
use log::{debug, error, info, trace};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use tokio::runtime::Handle;
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};
use winit::event_loop::{ControlFlow, EventLoop, EventLoopProxy};

pub enum TrayEvent {
    Quit,
    ToggleSync,
    SetFolder,
    OpenFolder,
    OpenWeb,
    CheckUpdates,
    About,
    ToggleAutoStart,
    UpdateStatus,
    LoginLogout,
    ThemeChanged(ThemeChange),
}

/// Shared state for the system tray
struct TrayState {
    tray_icon: Arc<Mutex<TrayIcon>>,
    last_status: Arc<Mutex<SyncStatus>>,
    shutdown_signal: Arc<AtomicBool>,
    theme_watcher: Arc<Mutex<Option<ThemeWatcher>>>,
}

pub struct SystemTray {
    state: Arc<TrayState>,
    menu: TrayMenu,
    event_loop: EventLoop<TrayEvent>,
    sync_manager: Arc<SyncManager>,
    auth_manager: Arc<AuthManager>,
    config: Arc<Config>,
    runtime_handle: Handle,
}

impl SystemTray {
    pub fn new(
        sync_manager: Arc<SyncManager>,
        auth_manager: Arc<AuthManager>,
        config: Arc<Config>,
        runtime_handle: Handle,
    ) -> Result<Self> {
        debug!("SystemTray::new() called");

        // Initialize GTK on Linux before creating any GTK widgets
        #[cfg(target_os = "linux")]
        {
            debug!("Initializing GTK for Linux tray...");
            gtk::init().map_err(|_| SyncError::Tray("Failed to initialize GTK".to_string()))?;
            debug!("GTK initialized successfully");
        }

        // Load icon
        debug!("Loading tray icon...");
        let icon = load_icon()?;
        debug!("Tray icon loaded successfully");

        // Create event loop with platform-specific attributes
        debug!("Creating event loop...");
        let mut event_loop_builder = EventLoop::<TrayEvent>::with_user_event();

        // On macOS, configure as background app
        #[cfg(target_os = "macos")]
        {
            use winit::platform::macos::EventLoopBuilderExtMacOS;
            debug!("Configuring macOS background app policy...");
            event_loop_builder
                .with_activation_policy(winit::platform::macos::ActivationPolicy::Accessory);
        }

        debug!("Building event loop...");
        let event_loop = event_loop_builder
            .build()
            .map_err(|e| SyncError::Tray(format!("Failed to create event loop: {e}")))?;
        debug!("Event loop created successfully");

        // Get actual system state for auto-start
        debug!("Checking auto-start status...");
        let platform = crate::platform::get_platform();
        let auto_start_enabled = platform.is_auto_start_enabled("cook-sync").unwrap_or(false);
        debug!("Auto-start enabled: {}", auto_start_enabled);

        // Create menu with actual auto-start state
        debug!("Creating tray menu...");
        let menu = TrayMenu::new(auto_start_enabled);
        debug!("Tray menu created successfully");

        // Create tray icon
        debug!("Building tray icon...");
        let tray_icon = TrayIconBuilder::new()
            .with_menu(Box::new(menu.menu.clone()))
            .with_tooltip("Cook Sync")
            .with_icon(icon)
            .build()
            .map_err(|e| SyncError::Tray(format!("Failed to create tray icon: {e}")))?;
        debug!("Tray icon created successfully");

        #[allow(clippy::arc_with_non_send_sync)]
        let state = Arc::new(TrayState {
            tray_icon: Arc::new(Mutex::new(tray_icon)),
            last_status: Arc::new(Mutex::new(SyncStatus::Starting)),
            shutdown_signal: Arc::new(AtomicBool::new(false)),
            theme_watcher: Arc::new(Mutex::new(None)),
        });

        Ok(SystemTray {
            state,
            menu,
            event_loop,
            sync_manager,
            auth_manager,
            config,
            runtime_handle,
        })
    }

    pub fn run(self) -> Result<()> {
        let event_loop_proxy = self.event_loop.create_proxy();

        // Start theme watcher if available
        self.start_theme_watcher(event_loop_proxy.clone());

        // Trigger immediate status update so user sees their login/folder status right away
        event_loop_proxy.send_event(TrayEvent::UpdateStatus).ok();

        // Start status update timer
        self.start_status_updater(event_loop_proxy.clone());

        // Handle menu events
        let menu = self.menu;
        let sync_manager = Arc::clone(&self.sync_manager);
        let auth_manager = Arc::clone(&self.auth_manager);
        let config = Arc::clone(&self.config);
        let state = Arc::clone(&self.state);
        let runtime_handle = self.runtime_handle.clone();

        #[allow(deprecated)]
        let _ = self.event_loop.run(move |event, event_loop| {
            event_loop.set_control_flow(ControlFlow::Wait);

            if let winit::event::Event::UserEvent(tray_event) = event {
                match tray_event {
                    TrayEvent::Quit => {
                        info!("Quit requested, shutting down gracefully");

                        // Signal shutdown to all background threads
                        state.shutdown_signal.store(true, Ordering::Relaxed);

                        // Clean shutdown of theme watcher if running
                        if let Some(watcher) = state.theme_watcher.lock().unwrap().take() {
                            debug!("Stopping theme watcher...");
                            // The watcher thread will exit when it sees the shutdown signal
                            // We don't join here to avoid blocking the UI thread
                            drop(watcher);
                        }

                        event_loop.exit();
                    }
                    TrayEvent::ToggleSync => {
                        let state = sync_manager.state();
                        let is_paused = state.lock().unwrap().status == SyncStatus::Paused;
                        if is_paused {
                            sync_manager.resume();
                        } else {
                            sync_manager.pause();
                        }
                        menu.update_sync_toggle(!is_paused);

                        // Trigger immediate status update to refresh menu
                        event_loop_proxy.send_event(TrayEvent::UpdateStatus).ok();
                    }
                    TrayEvent::SetFolder => {
                        // Use native file dialog to select folder
                        // Spawn in a separate thread to avoid blocking the event loop
                        let config_clone = Arc::clone(&config);
                        let sync_manager_clone = Arc::clone(&sync_manager);
                        let event_proxy_clone = event_loop_proxy.clone();
                        let runtime_handle_clone = runtime_handle.clone();

                        std::thread::spawn(move || {
                            let folder = rfd::FileDialog::new()
                                .set_title("Select Recipes Folder")
                                .pick_folder();

                            if let Some(path) = folder {
                                info!("Setting recipes folder to: {path:?}");
                                let path_clone = path.clone();

                                // Update configuration
                                if let Err(e) = config_clone.update_settings(|s| {
                                    s.recipes_dir = Some(path_clone);
                                }) {
                                    error!("Failed to update recipes directory: {e}");
                                } else {
                                    // Restart sync manager with new folder
                                    let was_running = sync_manager_clone.is_running();

                                    // Use the Tokio runtime handle to spawn the async task
                                    runtime_handle_clone.spawn(async move {
                                        if was_running {
                                            if let Err(e) = sync_manager_clone.stop().await {
                                                error!("Failed to stop sync manager: {e}");
                                            }
                                        }

                                        if let Err(e) = sync_manager_clone.start().await {
                                            error!("Failed to start sync manager: {e}");
                                        }

                                        // Trigger status update after sync manager starts
                                        event_proxy_clone.send_event(TrayEvent::UpdateStatus).ok();
                                    });

                                    info!("Recipes folder set successfully");
                                }
                            }
                        });
                    }
                    TrayEvent::OpenFolder => {
                        if let Some(dir) = config.settings().lock().unwrap().recipes_dir.clone() {
                            let _ = open::that(dir);
                        }
                    }
                    TrayEvent::OpenWeb => {
                        let _ = open::that("https://cook.md");
                    }
                    TrayEvent::CheckUpdates => {
                        info!("Manual update check requested");

                        // Show immediate feedback
                        let _ = crate::notifications::show_notification(
                            "Cook Sync",
                            "Checking for updates...",
                        );

                        // Spawn async task to check for updates
                        let config_clone = Arc::clone(&config);
                        runtime_handle.clone().spawn(async move {
                            // Get auto_update setting
                            let auto_update = config_clone
                                .settings()
                                .lock()
                                .unwrap()
                                .auto_update;

                            // Check for updates
                            match crate::updater::check_for_updates(auto_update).await {
                                Ok(Some(version)) => {
                                    if auto_update {
                                        #[cfg(target_os = "macos")]
                                        let message = format!(
                                            "Update to version {} has been downloaded. Please drag Cook Sync to Applications to complete installation.",
                                            version
                                        );

                                        #[cfg(not(target_os = "macos"))]
                                        let message = format!(
                                            "Update to version {} has been downloaded and will be installed on next restart.",
                                            version
                                        );

                                        let _ = crate::notifications::show_notification(
                                            "Cook Sync Update",
                                            &message,
                                        );
                                    } else {
                                        let _ = crate::notifications::show_notification(
                                            "Cook Sync Update Available",
                                            &format!(
                                                "Version {} is available. Enable auto-update in settings to install automatically.",
                                                version
                                            ),
                                        );
                                    }
                                }
                                Ok(None) => {
                                    let _ = crate::notifications::show_notification(
                                        "Cook Sync",
                                        "You are running the latest version.",
                                    );
                                }
                                Err(e) => {
                                    error!("Update check failed: {}", e);
                                    let _ = crate::notifications::show_notification(
                                        "Cook Sync Update Check Failed",
                                        &format!("Failed to check for updates: {}", e),
                                    );
                                }
                            }
                        });
                    }
                    TrayEvent::About => {
                        info!("About requested");
                        let log_file_path = config.paths().log_file.clone();
                        super::about::show_about_dialog(&log_file_path);
                    }
                    TrayEvent::ToggleAutoStart => {
                        let enabled = menu.auto_start.is_checked();
                        let _ = config.update_settings(|s| s.auto_start = enabled);

                        // Update system auto-start
                        use crate::platform;
                        let platform = platform::get_platform();
                        let app_path = std::env::current_exe()
                            .unwrap_or_else(|_| std::path::PathBuf::from("cook-sync"));

                        let result = if enabled {
                            platform.enable_auto_start("cook-sync", &app_path.to_string_lossy())
                        } else {
                            platform.disable_auto_start("cook-sync")
                        };

                        if let Err(e) = result {
                            error!("Failed to update auto-start: {e}");
                            // Show error notification to user
                            let _ = crate::notifications::show_notification(
                                "Cook Sync Auto-start",
                                &format!(
                                    "Failed to {} auto-start: {}",
                                    if enabled { "enable" } else { "disable" },
                                    e
                                ),
                            );
                            // Revert checkbox state since it failed
                            menu.auto_start.set_checked(!enabled);
                            // Revert config as well
                            let _ = config.update_settings(|s| s.auto_start = !enabled);
                        } else {
                            info!(
                                "Auto-start {}",
                                if enabled { "enabled" } else { "disabled" }
                            );
                        }
                    }
                    TrayEvent::UpdateStatus => {
                        // Update menu based on current state
                        let sync_state_mutex = sync_manager.state();
                        let sync_state = sync_state_mutex.lock().unwrap();

                        // Check for configuration issues
                        let has_auth = auth_manager.get_session().is_some();
                        let folder_path = config.settings().lock().unwrap().recipes_dir.clone();
                        let has_folder = folder_path.is_some();

                        // Determine status and error message
                        let (display_status, error_msg) = if !has_auth && !has_folder {
                            (SyncStatus::Error, Some("Not logged in, no folder selected"))
                        } else if !has_auth {
                            (SyncStatus::Error, Some("Not logged in"))
                        } else if !has_folder {
                            (SyncStatus::Error, Some("No folder selected"))
                        } else {
                            (sync_state.status, sync_state.error_message.as_deref())
                        };

                        menu.update_status(display_status, error_msg);

                        if let Some(session) = auth_manager.get_session() {
                            menu.update_user(session.email.as_deref());
                        } else {
                            menu.update_user(None);
                        }

                        // Update folder display
                        menu.update_folder(folder_path.as_deref());

                        // Update tray icon if status changed
                        let mut last_status_guard = state.last_status.lock().unwrap();
                        let status_changed = *last_status_guard != display_status;

                        if status_changed {
                            trace!(
                                "Status changed from {:?} to {:?}",
                                *last_status_guard,
                                display_status
                            );
                            if let Ok(new_icon) = load_icon_with_status(display_status) {
                                let _ = state.tray_icon.lock().unwrap().set_icon(Some(new_icon));
                            }
                            *last_status_guard = display_status;
                        }
                    }
                    TrayEvent::ThemeChanged(theme) => {
                        debug!("Received theme change event: {:?}", theme);

                        // Get current status to update icon with new theme
                        let current_status = {
                            let state = sync_manager.state();
                            let state_guard = state.lock().unwrap();
                            state_guard.status
                        };

                        // Determine if we're in dark mode based on the theme change
                        let is_dark = matches!(theme, crate::platform::ThemeChange::Dark);

                        // Load new icon for the theme
                        match load_icon_for_theme(current_status, is_dark) {
                            Ok(new_icon) => {
                                if let Err(e) =
                                    state.tray_icon.lock().unwrap().set_icon(Some(new_icon))
                                {
                                    error!("Failed to set new icon: {:?}", e);
                                } else {
                                    debug!("Successfully updated tray icon for theme: {:?}", theme);
                                }
                            }
                            Err(e) => {
                                error!("Failed to load icon for theme {:?}: {:?}", theme, e);
                            }
                        }
                    }
                    TrayEvent::LoginLogout => {
                        let has_auth = auth_manager.get_session().is_some();

                        if has_auth {
                            // Logout
                            info!("Logout requested");
                            if let Err(e) = auth_manager.logout() {
                                error!("Failed to logout: {e}");
                            }

                            // Stop sync manager
                            let sync_manager_clone = Arc::clone(&sync_manager);
                            let event_proxy_clone = event_loop_proxy.clone();
                            // Use the Tokio runtime handle to spawn the async task
                            runtime_handle.clone().spawn(async move {
                                if let Err(e) = sync_manager_clone.stop().await {
                                    error!("Failed to stop sync manager: {e}");
                                }

                                // Trigger immediate status update after logout completes
                                event_proxy_clone.send_event(TrayEvent::UpdateStatus).ok();
                            });
                        } else {
                            // Login
                            info!("Login requested");
                            let auth_manager_clone = Arc::clone(&auth_manager);
                            let sync_manager_clone = Arc::clone(&sync_manager);
                            let config_clone = Arc::clone(&config);
                            let event_proxy_clone = event_loop_proxy.clone();
                            // Use the Tokio runtime handle to spawn the async task
                            runtime_handle.clone().spawn(async move {
                                match auth_manager_clone.browser_login().await {
                                    Ok(()) => {
                                        info!("Login completed successfully");

                                        // Start sync manager if recipes folder is configured
                                        if config_clone
                                            .settings()
                                            .lock()
                                            .unwrap()
                                            .recipes_dir
                                            .is_some()
                                        {
                                            if let Err(e) = sync_manager_clone.start().await {
                                                error!(
                                                    "Failed to start sync manager after login: {e}"
                                                );
                                            }
                                        }

                                        // Trigger immediate status update after login completes
                                        event_proxy_clone.send_event(TrayEvent::UpdateStatus).ok();
                                    }
                                    Err(e) => error!("Failed to login: {e}"),
                                }
                            });
                        }
                    }
                }
            }

            // Handle menu events
            if let Ok(event) = tray_icon::menu::MenuEvent::receiver().try_recv() {
                if event.id == menu.get_menu_id(&menu.quit) {
                    event_loop_proxy.send_event(TrayEvent::Quit).ok();
                } else if event.id == menu.get_menu_id(&menu.sync_toggle) {
                    event_loop_proxy.send_event(TrayEvent::ToggleSync).ok();
                } else if event.id == menu.get_menu_id(&menu.set_folder) {
                    event_loop_proxy.send_event(TrayEvent::SetFolder).ok();
                } else if event.id == menu.get_menu_id(&menu.open_folder) {
                    event_loop_proxy.send_event(TrayEvent::OpenFolder).ok();
                } else if event.id == menu.get_menu_id(&menu.open_web) {
                    event_loop_proxy.send_event(TrayEvent::OpenWeb).ok();
                } else if event.id == menu.get_menu_id(&menu.check_updates) {
                    event_loop_proxy.send_event(TrayEvent::CheckUpdates).ok();
                } else if event.id == menu.get_menu_id(&menu.about) {
                    event_loop_proxy.send_event(TrayEvent::About).ok();
                } else if event.id == menu.get_check_menu_id(&menu.auto_start) {
                    event_loop_proxy.send_event(TrayEvent::ToggleAutoStart).ok();
                } else if event.id == menu.get_menu_id(&menu.login_logout) {
                    event_loop_proxy.send_event(TrayEvent::LoginLogout).ok();
                }
            }
        });

        Ok(())
    }

    fn start_theme_watcher(&self, proxy: EventLoopProxy<TrayEvent>) {
        // Start OS-specific theme change watcher if available
        let platform = crate::platform::get_platform();
        let shutdown_signal = Arc::clone(&self.state.shutdown_signal);

        if let Some(mut watcher) = platform.watch_theme_changes(shutdown_signal) {
            // Take the receiver, leaving the handle in the watcher
            let receiver = std::mem::replace(
                &mut watcher.receiver,
                mpsc::channel().1, // Replace with dummy receiver
            );

            // Store the watcher handle so we can join it on shutdown
            *self.state.theme_watcher.lock().unwrap() = Some(watcher);

            std::thread::spawn(move || {
                while let Ok(theme) = receiver.recv() {
                    debug!("Detected system theme change: {:?}", theme);
                    if proxy.send_event(TrayEvent::ThemeChanged(theme)).is_err() {
                        debug!("Event loop closed, stopping theme watcher receiver");
                        break;
                    }
                }
                debug!("Theme watcher receiver thread exiting");
            });
            info!("Started OS-specific theme change watcher");
        } else {
            info!("OS-specific theme change notifications not available");
        }
    }

    fn start_status_updater(&self, proxy: EventLoopProxy<TrayEvent>) {
        // Update status periodically (no need to check theme here anymore)
        std::thread::spawn(move || loop {
            std::thread::sleep(std::time::Duration::from_secs(10));
            let _ = proxy.send_event(TrayEvent::UpdateStatus);
        });
    }
}

/// Detects if running inside Flatpak sandbox
fn is_flatpak() -> bool {
    std::path::Path::new("/.flatpak-info").exists()
}

/// Finds installed icon in standard locations
fn find_installed_icon(icon_name: &str) -> Option<std::path::PathBuf> {
    let possible_paths = [
        format!("/usr/share/cook-sync/{}", icon_name),
        format!("/usr/local/share/cook-sync/{}", icon_name),
        "/app/share/icons/hicolor/16x16/apps/cook-sync.png".to_string(), // Flatpak path
        "/usr/share/icons/hicolor/16x16/apps/cook-sync.png".to_string(),
        "/usr/share/icons/hicolor/22x22/apps/cook-sync.png".to_string(),
        "/usr/share/icons/hicolor/32x32/apps/cook-sync.png".to_string(),
    ];

    possible_paths
        .iter()
        .map(std::path::PathBuf::from)
        .find(|p| p.exists())
}

/// Adds a red error indicator dot to the icon
fn add_error_dot(rgba_img: &mut image::RgbaImage, width: u32, height: u32) {
    // Draw status dot (proportional to icon size, slightly bigger)
    let dot_size = (width.min(height) / 4).max(8);
    let dot_offset = dot_size + 1;

    for dy in 0..dot_size {
        for dx in 0..dot_size {
            let x = width - dot_offset + dx;
            let y = height - dot_offset + dy;

            // Make it a circle
            let center = dot_size as f32 / 2.0;
            let dist = ((dx as f32 - center).powi(2) + (dy as f32 - center).powi(2)).sqrt();

            if dist <= center {
                if let Some(pixel) = rgba_img.get_pixel_mut_checked(x, y) {
                    pixel[0] = 255; // R
                    pixel[1] = 0; // G
                    pixel[2] = 0; // B
                    pixel[3] = 255; // A
                }
            }
        }
    }
}

fn load_icon() -> Result<Icon> {
    load_icon_with_status(SyncStatus::Starting)
}

fn load_icon_with_status(status: SyncStatus) -> Result<Icon> {
    // Detect if we're in dark mode
    let platform = crate::platform::get_platform();
    let is_dark_mode = platform.is_dark_mode();
    load_icon_for_theme(status, is_dark_mode)
}

fn load_icon_for_theme(status: SyncStatus, is_dark_mode: bool) -> Result<Icon> {
    // Choose icon based on theme
    // In dark mode, we need light/white icon to be visible
    // In light mode, we need dark/black icon to be visible
    let icon_name = if is_dark_mode {
        "icon_white.png"
    } else {
        "icon_black.png"
    };

    debug!("Dark mode: {}, using icon: {}", is_dark_mode, icon_name);

    // Check for Flatpak environment first
    if is_flatpak() {
        debug!("Running in Flatpak sandbox");
        if let Some(installed_icon) = find_installed_icon(icon_name) {
            debug!("Found installed icon in Flatpak: {:?}", installed_icon);
            // Try to load the Flatpak-specific icon first
            if let Ok(icon_data) = std::fs::read(&installed_icon) {
                if let Ok(img) = image::load_from_memory(&icon_data) {
                    let mut rgba_img = img.to_rgba8();
                    let (width, height) = rgba_img.dimensions();

                    // Add error indicator if needed
                    if status == SyncStatus::Error {
                        add_error_dot(&mut rgba_img, width, height);
                    }

                    let rgba_data = rgba_img.into_raw();
                    if let Ok(icon) = Icon::from_rgba(rgba_data, width, height) {
                        return Ok(icon);
                    }
                }
            }
        }
    }

    // Build list of icon search paths
    let mut icon_paths = vec![
        // Development path (relative to binary)
        format!("{}/assets/{}", env!("CARGO_MANIFEST_DIR"), icon_name),
        // macOS installed path
        format!("/Applications/Cook Sync.app/Contents/Resources/{icon_name}"),
        // Linux system installed paths (cargo-packager uses lib, but traditional installs use share)
        format!("/usr/local/lib/cook-sync/{icon_name}"),
        format!("/usr/lib/cook-sync/{icon_name}"),
        format!("/usr/local/share/cook-sync/{icon_name}"),
        format!("/usr/share/cook-sync/{icon_name}"),
    ];

    // AppImage specific paths (check APPDIR environment variable)
    #[cfg(target_os = "linux")]
    if let Ok(appdir) = std::env::var("APPDIR") {
        // cargo-packager puts resources in usr/lib not usr/share
        icon_paths.insert(0, format!("{}/usr/lib/cook-sync/{}", appdir, icon_name));
        icon_paths.insert(0, format!("{}/usr/share/cook-sync/{}", appdir, icon_name));
        // Also try with different size tray icons
        for size in &[16, 22, 24, 32] {
            let base_name = icon_name.trim_end_matches(".png");
            icon_paths.insert(
                0,
                format!(
                    "{}/usr/lib/cook-sync/{}_tray_{}.png",
                    appdir, base_name, size
                ),
            );
            icon_paths.insert(
                0,
                format!(
                    "{}/usr/share/cook-sync/{}_tray_{}.png",
                    appdir, base_name, size
                ),
            );
        }
    }

    // Add paths relative to executable (for AppImage and other portable installs)
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            // AppImage: binary is in usr/bin, icons can be in usr/lib or usr/share
            icon_paths.push(
                exe_dir
                    .join("../lib/cook-sync")
                    .join(icon_name)
                    .to_string_lossy()
                    .to_string(),
            );
            icon_paths.push(
                exe_dir
                    .join("../share/cook-sync")
                    .join(icon_name)
                    .to_string_lossy()
                    .to_string(),
            );
            // User local installation
            icon_paths.push(
                exe_dir
                    .parent()
                    .map(|p| p.join("lib/cook-sync").join(icon_name))
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default(),
            );
            icon_paths.push(
                exe_dir
                    .parent()
                    .map(|p| p.join("share/cook-sync").join(icon_name))
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default(),
            );
            // Icons next to binary (fallback)
            icon_paths.push(exe_dir.join(icon_name).to_string_lossy().to_string());
        }
    }

    // Also check XDG data dirs for user installations
    if let Ok(data_home) = std::env::var("XDG_DATA_HOME") {
        icon_paths.push(format!("{}/cook-sync/{}", data_home, icon_name));
    } else if let Ok(home) = std::env::var("HOME") {
        icon_paths.push(format!("{}/.local/share/cook-sync/{}", home, icon_name));
    }

    debug!(
        "Searching for tray icon '{}' in {} locations",
        icon_name,
        icon_paths.len()
    );

    for path in &icon_paths {
        trace!("Checking icon path: {}", path);
        if let Ok(icon_data) = std::fs::read(path) {
            debug!("Successfully loaded tray icon from: {}", path);
            // Load the PNG image
            if let Ok(img) = image::load_from_memory(&icon_data) {
                let mut rgba_img = img.to_rgba8();
                let (width, height) = rgba_img.dimensions();

                // Only add red dot for error status
                if status == SyncStatus::Error {
                    add_error_dot(&mut rgba_img, width, height);
                }

                let rgba_data = rgba_img.into_raw();
                return Icon::from_rgba(rgba_data, width, height)
                    .map_err(|e| SyncError::Tray(format!("Failed to create icon: {e}")));
            }
        }
    }

    error!(
        "Failed to load tray icon '{}' from any of {} paths",
        icon_name,
        icon_paths.len()
    );
    error!("Searched paths:");
    for (i, path) in icon_paths.iter().enumerate() {
        error!("  [{}] {}", i + 1, path);
    }

    // Fallback: try embedded icon data
    debug!("Attempting to use embedded fallback icon...");

    // Use embedded icon as fallback (32x32)
    let embedded_icon_data = if icon_name.contains("white") {
        // For dark theme, use white icon if available
        include_bytes!("../../assets/icon_whitetray_32.png").as_slice()
    } else {
        // For light theme, use black icon
        include_bytes!("../../assets/icon_blacktray_32.png").as_slice()
    };

    if let Ok(img) = image::load_from_memory(embedded_icon_data) {
        let mut rgba_img = img.to_rgba8();
        let (width, height) = rgba_img.dimensions();

        if status == SyncStatus::Error {
            add_error_dot(&mut rgba_img, width, height);
        }

        let rgba_data = rgba_img.into_raw();
        if let Ok(icon) = Icon::from_rgba(rgba_data, width, height) {
            info!(
                "Using embedded fallback tray icon ({})",
                if icon_name.contains("white") {
                    "white"
                } else {
                    "black"
                }
            );
            return Ok(icon);
        }
    }

    error!("Failed to load embedded fallback icon");
    Err(SyncError::Tray(format!(
        "Failed to load tray icon '{}' from any of {} expected paths and embedded fallback failed. Check logs for searched paths.",
        icon_name,
        icon_paths.len()
    )))
}
