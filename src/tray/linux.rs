// Linux-specific tray implementation using ksni (StatusNotifierItem)
// This works directly with GNOME's AppIndicator extension via D-Bus

use crate::auth::AuthManager;
use crate::config::Config;
use crate::error::{Result, SyncError};
use crate::sync::{SyncManager, SyncStatus};
use log::{debug, error, info, warn};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tokio::runtime::Handle;

// Event messages sent from menu items to the main handler
#[derive(Debug, Clone)]
pub enum TrayEvent {
    Quit,
    ToggleSync,
    SetFolder,
    OpenFolder,
    OpenWeb,
    CheckUpdates,
    About,
    ToggleAutoStart,
    LoginLogout,
}

// Shared state between the tray and the application
struct TrayState {
    sync_manager: Arc<SyncManager>,
    auth_manager: Arc<AuthManager>,
    config: Arc<Config>,
    runtime_handle: Handle,

    // Tray state
    status: Arc<Mutex<SyncStatus>>,
    status_text: Arc<Mutex<String>>,
    folder_path: Arc<Mutex<Option<String>>>,
    user_email: Arc<Mutex<Option<String>>>,
    is_logged_in: Arc<Mutex<bool>>,
    auto_start_enabled: Arc<Mutex<bool>>,
    sync_paused: Arc<Mutex<bool>>,

    // Icon state for dark mode
    icon_name: Arc<Mutex<String>>,
    shutdown_signal: Arc<AtomicBool>,
}

impl TrayState {
    fn new(
        sync_manager: Arc<SyncManager>,
        auth_manager: Arc<AuthManager>,
        config: Arc<Config>,
        runtime_handle: Handle,
        auto_start_enabled: bool,
    ) -> Self {
        let icon_name = match dark_light::detect() {
            Ok(dark_light::Mode::Dark) => "cook-sync-light",
            _ => "cook-sync-dark",
        };

        Self {
            sync_manager,
            auth_manager,
            config,
            runtime_handle,
            status: Arc::new(Mutex::new(SyncStatus::Starting)),
            status_text: Arc::new(Mutex::new("Starting".to_string())),
            folder_path: Arc::new(Mutex::new(None)),
            user_email: Arc::new(Mutex::new(None)),
            is_logged_in: Arc::new(Mutex::new(false)),
            auto_start_enabled: Arc::new(Mutex::new(auto_start_enabled)),
            sync_paused: Arc::new(Mutex::new(false)),
            icon_name: Arc::new(Mutex::new(icon_name.to_string())),
            shutdown_signal: Arc::new(AtomicBool::new(false)),
        }
    }

    fn handle_event(&self, event: TrayEvent) {
        match event {
            TrayEvent::Quit => {
                info!("Quit requested from tray menu");
                self.shutdown_signal.store(true, Ordering::Relaxed);
                // Exit the application
                std::process::exit(0);
            }
            TrayEvent::ToggleSync => {
                let state = self.sync_manager.state();
                let is_paused = state.lock().unwrap().status == SyncStatus::Paused;
                if is_paused {
                    self.sync_manager.resume();
                    *self.sync_paused.lock().unwrap() = false;
                } else {
                    self.sync_manager.pause();
                    *self.sync_paused.lock().unwrap() = true;
                }
                // Menu will auto-update on next click
            }
            TrayEvent::SetFolder => {
                let config_clone = Arc::clone(&self.config);
                let sync_manager_clone = Arc::clone(&self.sync_manager);
                let runtime_handle_clone = self.runtime_handle.clone();
                let folder_path_clone = Arc::clone(&self.folder_path);

                std::thread::spawn(move || {
                    let folder = rfd::FileDialog::new()
                        .set_title("Select Recipes Folder")
                        .pick_folder();

                    if let Some(path) = folder {
                        info!("Setting recipes folder to: {:?}", path);
                        let path_clone = path.clone();

                        if let Err(e) = config_clone.update_settings(|s| {
                            s.recipes_dir = Some(path_clone);
                        }) {
                            error!("Failed to update recipes directory: {}", e);
                            return;
                        }

                        *folder_path_clone.lock().unwrap() = Some(path.display().to_string());

                        // Start sync with new folder
                        runtime_handle_clone.block_on(async {
                            if let Err(e) = sync_manager_clone.start().await {
                                error!("Failed to start sync: {}", e);
                            }
                        });
                    }
                });
            }
            TrayEvent::OpenFolder => {
                let settings = self.config.settings();
                let settings_lock = settings.lock().unwrap();
                if let Some(recipes_dir) = &settings_lock.recipes_dir {
                    if let Err(e) = open::that(recipes_dir) {
                        error!("Failed to open recipes folder: {}", e);
                    }
                } else {
                    warn!("No recipes folder configured");
                }
            }
            TrayEvent::OpenWeb => {
                if let Err(e) = open::that("https://cook.md") {
                    error!("Failed to open cook.md: {}", e);
                }
            }
            TrayEvent::CheckUpdates => {
                info!("Manual update check requested");

                // Show immediate feedback
                let _ =
                    crate::notifications::show_notification("Cook Sync", "Checking for updates...");

                let config_clone = Arc::clone(&self.config);
                let runtime_handle = self.runtime_handle.clone();

                std::thread::spawn(move || {
                    runtime_handle.block_on(async {
                        // Get auto_update setting
                        let auto_update = config_clone.settings().lock().unwrap().auto_update;

                        // Check for updates
                        match crate::updater::check_for_updates(auto_update).await {
                            Ok(Some(version)) => {
                                if auto_update {
                                    let _ = crate::notifications::show_notification(
                                        "Cook Sync Update",
                                        &format!(
                                            "Updated to version {}. Restart to apply.",
                                            version
                                        ),
                                    );
                                } else {
                                    let _ = crate::notifications::show_notification(
                                        "Cook Sync Update Available",
                                        &format!(
                                            "Version {} is available. Enable auto-update to install.",
                                            version
                                        ),
                                    );
                                }
                            }
                            Ok(None) => {
                                let _ = crate::notifications::show_notification(
                                    "Cook Sync",
                                    "You're running the latest version.",
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
                });
            }
            TrayEvent::About => {
                let log_file_path = self.config.paths().log_file.clone();
                super::about::show_about_dialog(&log_file_path);
            }
            TrayEvent::ToggleAutoStart => {
                let platform = crate::platform::get_platform();
                let current_state = *self.auto_start_enabled.lock().unwrap();
                let new_state = !current_state;

                let app_path = std::env::current_exe()
                    .unwrap_or_else(|_| std::path::PathBuf::from("cook-sync"));

                let result = if new_state {
                    platform.enable_auto_start("cook-sync", &app_path.to_string_lossy())
                } else {
                    platform.disable_auto_start("cook-sync")
                };

                match result {
                    Ok(_) => {
                        *self.auto_start_enabled.lock().unwrap() = new_state;
                        info!(
                            "Auto-start {}",
                            if new_state { "enabled" } else { "disabled" }
                        );
                    }
                    Err(e) => {
                        error!("Failed to toggle auto-start: {}", e);
                    }
                }
            }
            TrayEvent::LoginLogout => {
                let is_logged_in = *self.is_logged_in.lock().unwrap();
                let auth_manager = Arc::clone(&self.auth_manager);
                let sync_manager = Arc::clone(&self.sync_manager);
                let config = Arc::clone(&self.config);
                let runtime_handle = self.runtime_handle.clone();
                let user_email_clone = Arc::clone(&self.user_email);
                let is_logged_in_clone = Arc::clone(&self.is_logged_in);

                std::thread::spawn(move || {
                    if is_logged_in {
                        // Logout (not async)
                        info!("Logout requested");
                        if let Err(e) = auth_manager.logout() {
                            error!("Failed to logout: {}", e);
                        } else {
                            *user_email_clone.lock().unwrap() = None;
                            *is_logged_in_clone.lock().unwrap() = false;
                            info!("Logged out successfully");

                            // Stop sync manager
                            runtime_handle.block_on(async {
                                if let Err(e) = sync_manager.stop().await {
                                    error!("Failed to stop sync manager: {}", e);
                                }
                            });
                        }
                    } else {
                        // Login (async)
                        info!("Login requested");
                        runtime_handle.block_on(async {
                            match auth_manager.browser_login().await {
                                Ok(()) => {
                                    info!("Login completed successfully");

                                    // Get session email
                                    if let Some(session) = auth_manager.get_session() {
                                        *user_email_clone.lock().unwrap() = session.email.clone();
                                        *is_logged_in_clone.lock().unwrap() = true;
                                        if let Some(ref email) = session.email {
                                            info!("Logged in as: {}", email);
                                        }

                                        // Start sync manager if recipes folder is configured
                                        if config.settings().lock().unwrap().recipes_dir.is_some() {
                                            if let Err(e) = sync_manager.start().await {
                                                error!("Failed to start sync manager: {}", e);
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    error!("Login failed: {}", e);
                                }
                            }
                        });
                    }

                    // Menu will auto-update on next click
                });
            }
        }
    }
}

// The actual tray implementation that ksni displays
pub struct CookSyncTray {
    state: Arc<TrayState>,
}

impl CookSyncTray {
    fn new(state: Arc<TrayState>) -> Self {
        Self { state }
    }
}

impl ksni::Tray for CookSyncTray {
    fn id(&self) -> String {
        "cook-sync".to_string()
    }

    fn title(&self) -> String {
        "Cook Sync".to_string()
    }

    fn icon_name(&self) -> String {
        self.state.icon_name.lock().unwrap().clone()
    }

    // Use icon_pixmap for embedded icon if icon_name doesn't work
    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        // Try to load icon from file
        load_icon_pixmap(&self.state.icon_name.lock().unwrap()).unwrap_or_else(|e| {
            warn!("Failed to load icon pixmap: {}", e);
            vec![]
        })
    }

    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        let state = &self.state;
        let status_text = state.status_text.lock().unwrap().clone();
        let folder = state.folder_path.lock().unwrap().clone();
        let user_email = state.user_email.lock().unwrap().clone();
        let is_logged_in = *state.is_logged_in.lock().unwrap();
        let auto_start = *state.auto_start_enabled.lock().unwrap();
        let sync_paused = *state.sync_paused.lock().unwrap();

        vec![
            // Status (disabled, just for display)
            ksni::menu::StandardItem {
                label: format!("Status: {}", status_text),
                enabled: false,
                ..Default::default()
            }
            .into(),
            // Sync toggle
            ksni::menu::StandardItem {
                label: if sync_paused {
                    "Resume Sync"
                } else {
                    "Pause Sync"
                }
                .to_string(),
                activate: Box::new(move |this: &mut Self| {
                    this.state.handle_event(TrayEvent::ToggleSync);
                }),
                ..Default::default()
            }
            .into(),
            ksni::menu::MenuItem::Separator,
            // User info (disabled, just for display)
            ksni::menu::StandardItem {
                label: if is_logged_in {
                    user_email.unwrap_or_else(|| "Logged in".to_string())
                } else {
                    "Not logged in".to_string()
                },
                enabled: false,
                ..Default::default()
            }
            .into(),
            // Login/Logout
            ksni::menu::StandardItem {
                label: if is_logged_in { "Logout" } else { "Login" }.to_string(),
                activate: Box::new(move |this: &mut Self| {
                    this.state.handle_event(TrayEvent::LoginLogout);
                }),
                ..Default::default()
            }
            .into(),
            ksni::menu::MenuItem::Separator,
            // Folder info (disabled, just for display)
            ksni::menu::StandardItem {
                label: folder.unwrap_or_else(|| "Not configured".to_string()),
                enabled: false,
                ..Default::default()
            }
            .into(),
            // Set folder
            ksni::menu::StandardItem {
                label: "Set recipes folder...".to_string(),
                activate: Box::new(move |this: &mut Self| {
                    this.state.handle_event(TrayEvent::SetFolder);
                }),
                ..Default::default()
            }
            .into(),
            // Open folder
            ksni::menu::StandardItem {
                label: "Open recipes folder".to_string(),
                activate: Box::new(move |this: &mut Self| {
                    this.state.handle_event(TrayEvent::OpenFolder);
                }),
                ..Default::default()
            }
            .into(),
            ksni::menu::MenuItem::Separator,
            // Open web
            ksni::menu::StandardItem {
                label: "Open cook.md".to_string(),
                activate: Box::new(move |this: &mut Self| {
                    this.state.handle_event(TrayEvent::OpenWeb);
                }),
                ..Default::default()
            }
            .into(),
            // Auto-start checkbox
            ksni::menu::CheckmarkItem {
                label: "Start on system startup".to_string(),
                checked: auto_start,
                activate: Box::new(move |this: &mut Self| {
                    this.state.handle_event(TrayEvent::ToggleAutoStart);
                }),
                ..Default::default()
            }
            .into(),
            // Check for updates
            ksni::menu::StandardItem {
                label: "Check for updates...".to_string(),
                activate: Box::new(move |this: &mut Self| {
                    this.state.handle_event(TrayEvent::CheckUpdates);
                }),
                ..Default::default()
            }
            .into(),
            // About
            ksni::menu::StandardItem {
                label: "About Cook Sync".to_string(),
                activate: Box::new(move |this: &mut Self| {
                    this.state.handle_event(TrayEvent::About);
                }),
                ..Default::default()
            }
            .into(),
            ksni::menu::MenuItem::Separator,
            // Quit
            ksni::menu::StandardItem {
                label: "Quit".to_string(),
                activate: Box::new(move |this: &mut Self| {
                    this.state.handle_event(TrayEvent::Quit);
                }),
                ..Default::default()
            }
            .into(),
        ]
    }
}

// Public interface that matches the existing SystemTray API
pub struct SystemTray {
    state: Arc<TrayState>,
}

impl SystemTray {
    pub fn new(
        sync_manager: Arc<SyncManager>,
        auth_manager: Arc<AuthManager>,
        config: Arc<Config>,
        runtime_handle: Handle,
    ) -> Result<Self> {
        debug!("Creating Linux tray with ksni");

        // Get auto-start status
        let platform = crate::platform::get_platform();
        let auto_start_enabled = platform.is_auto_start_enabled("cook-sync").unwrap_or(false);

        let state = Arc::new(TrayState::new(
            sync_manager,
            auth_manager,
            config,
            runtime_handle,
            auto_start_enabled,
        ));

        Ok(SystemTray { state })
    }

    pub fn run(self) -> Result<()> {
        info!("Starting ksni tray service");

        let tray = CookSyncTray::new(Arc::clone(&self.state));

        // Spawn the tray service (this is async but doesn't block)
        let service = ksni::TrayService::new(tray);
        service.spawn();

        info!("ksni tray service started");

        // Note: ksni automatically calls menu() when the user clicks the tray icon,
        // so the menu will always show current state without manual updates

        // Start background status updater
        self.start_status_updater();

        // Start theme watcher
        self.start_theme_watcher();

        // Block forever (the tray runs in background via D-Bus)
        loop {
            if self.state.shutdown_signal.load(Ordering::Relaxed) {
                info!("Shutdown signal received, exiting tray loop");
                break;
            }
            std::thread::sleep(std::time::Duration::from_secs(1));
        }

        Ok(())
    }

    fn start_status_updater(&self) {
        let sync_manager = Arc::clone(&self.state.sync_manager);
        let status_arc = Arc::clone(&self.state.status);
        let status_text_arc = Arc::clone(&self.state.status_text);
        let shutdown_signal = Arc::clone(&self.state.shutdown_signal);

        std::thread::spawn(move || {
            while !shutdown_signal.load(Ordering::Relaxed) {
                let state = sync_manager.state();
                let status = state.lock().unwrap().status;
                let mut current_status = status_arc.lock().unwrap();

                if *current_status != status {
                    *current_status = status;

                    let text = match status {
                        SyncStatus::Starting => "Starting",
                        SyncStatus::Idle => "Idle",
                        SyncStatus::Syncing => "Syncing",
                        SyncStatus::Paused => "Paused",
                        SyncStatus::Offline => "Offline",
                        SyncStatus::Error => "Error",
                    };

                    *status_text_arc.lock().unwrap() = text.to_string();
                }

                std::thread::sleep(std::time::Duration::from_secs(2));
            }
        });
    }

    fn start_theme_watcher(&self) {
        let icon_name_arc = Arc::clone(&self.state.icon_name);
        let shutdown_signal = Arc::clone(&self.state.shutdown_signal);

        std::thread::spawn(move || {
            let mut last_mode = dark_light::detect().ok();

            while !shutdown_signal.load(Ordering::Relaxed) {
                let current_mode = dark_light::detect().ok();

                if current_mode != last_mode {
                    last_mode = current_mode;

                    let icon = match current_mode {
                        Some(dark_light::Mode::Dark) => "cook-sync-light",
                        _ => "cook-sync-dark",
                    };

                    *icon_name_arc.lock().unwrap() = icon.to_string();
                    info!("Theme changed, updating icon to: {}", icon);
                }

                std::thread::sleep(std::time::Duration::from_secs(5));
            }
        });
    }
}

// Helper function to load icon as pixmap
fn load_icon_pixmap(icon_name: &str) -> Result<Vec<ksni::Icon>> {
    // Use dedicated tray icons (monochrome)
    // For dark mode, use white icons; for light mode, use black icons
    let icon_sizes = if icon_name.contains("light") {
        // Dark mode: use white tray icons
        vec![
            "icon_whitetray_32.png",
            "icon_whitetray_24.png",
            "icon_whitetray_48.png",
        ]
    } else {
        // Light mode: use black tray icons
        vec![
            "icon_blacktray_32.png",
            "icon_blacktray_24.png",
            "icon_blacktray_48.png",
        ]
    };

    // Try each icon size in order of preference
    for icon_file in &icon_sizes {
        // Search for icon in standard locations
        let possible_paths = vec![
            format!("{}/assets/{}", env!("CARGO_MANIFEST_DIR"), icon_file),
            format!("/usr/share/cook-sync/{}", icon_file),
            format!("/usr/local/share/cook-sync/{}", icon_file),
        ];

        // Add AppImage paths if running from AppImage
        let mut all_paths = possible_paths;
        if let Ok(appdir) = std::env::var("APPDIR") {
            all_paths.insert(0, format!("{}/usr/lib/cook-sync/{}", appdir, icon_file));
            all_paths.insert(0, format!("{}/usr/share/cook-sync/{}", appdir, icon_file));
        }

        for path in &all_paths {
            if let Ok(icon_data) = std::fs::read(path) {
                debug!("Loaded tray icon from: {}", path);

                // Load PNG and convert to RGBA
                if let Ok(img) = image::load_from_memory(&icon_data) {
                    let rgba = img.to_rgba8();
                    let (width, height) = rgba.dimensions();

                    return Ok(vec![ksni::Icon {
                        width: width as i32,
                        height: height as i32,
                        data: rgba.into_raw(),
                    }]);
                }
            }
        }
    }

    Err(SyncError::Tray(format!(
        "Failed to load tray icon for '{}' from any path",
        icon_name
    )))
}
