use super::{PlatformIntegration, ThemeChange, ThemeWatcher};
use crate::error::{Result, SyncError};
use auto_launch::AutoLaunchBuilder;
use log::{debug, error, info};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::Duration;

pub struct LinuxIntegration;

// Desktop integration module for AppImage
pub mod desktop_integration {
    use super::*;

    /// Check if we're running from an AppImage
    pub fn is_running_from_appimage() -> bool {
        // Check APPIMAGE environment variable (set by AppImage runtime)
        if std::env::var("APPIMAGE").is_ok() {
            return true;
        }

        // Check if exe path looks like AppImage mount
        if let Ok(exe_path) = std::env::current_exe() {
            let path_str = exe_path.to_string_lossy();
            if path_str.contains(".AppImage") || path_str.contains("/tmp/.mount_") {
                return true;
            }
        }

        false
    }

    /// Create a Command with the AppImage's LD_LIBRARY_PATH cleared so that
    /// system tools (zenity, kdialog, etc.) load their own libraries instead of
    /// the bundled ones, which can cause symbol-lookup crashes.
    pub fn clean_appimage_env(program: &str) -> std::process::Command {
        let mut cmd = std::process::Command::new(program);
        if let Ok(orig) = std::env::var("LD_LIBRARY_PATH_ORIG") {
            cmd.env("LD_LIBRARY_PATH", orig);
        } else {
            cmd.env_remove("LD_LIBRARY_PATH");
        }
        cmd
    }

    /// Check if the AppImage is in a transient location (Downloads, Desktop, /tmp)
    fn is_transient_location(appimage_path: &Path) -> bool {
        let path_str = appimage_path.to_string_lossy();

        // /tmp/.mount_* is the AppImage runtime mount point, not a transient location
        if path_str.starts_with("/tmp/.mount_") {
            return false;
        }

        // Check /tmp/
        if path_str.starts_with("/tmp/") {
            return true;
        }

        // Check ~/Downloads/ and ~/Desktop/
        if let Some(home) = dirs::home_dir() {
            let downloads = home.join("Downloads");
            let desktop = home.join("Desktop");

            if appimage_path.starts_with(&downloads) || appimage_path.starts_with(&desktop) {
                return true;
            }
        }

        false
    }

    /// Get the path to the relocation-declined marker file
    fn relocation_declined_path() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("cook-sync").join("relocation-declined"))
    }

    /// Check if the user has previously declined relocation
    fn relocation_declined() -> bool {
        relocation_declined_path()
            .map(|p| p.exists())
            .unwrap_or(false)
    }

    /// Create the marker file indicating the user declined relocation
    fn mark_relocation_declined() {
        if let Some(path) = relocation_declined_path() {
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let _ = fs::write(&path, "declined");
        }
    }

    /// Show a zenity dialog asking the user if they want to relocate the AppImage
    fn show_relocation_dialog() -> bool {
        let mut cmd = clean_appimage_env("zenity");
        match cmd
            .args([
                "--question",
                "--title=Move Cook Sync?",
                "--text=Cook Sync is running from a temporary location.\n\nMove to ~/Applications/ for permanent installation?",
                "--ok-label=Yes, move it",
                "--cancel-label=No thanks",
                "--no-markup",
            ])
            .status()
        {
            Ok(status) => status.success(),
            Err(e) => {
                debug!("zenity not found or failed to run: {}", e);
                false
            }
        }
    }

    /// Move the AppImage from source to ~/Applications/cook-sync.AppImage
    fn move_appimage(source: &Path) -> Result<PathBuf> {
        let target_dir = dirs::home_dir()
            .ok_or_else(|| SyncError::Platform("Cannot determine home directory".into()))?
            .join("Applications");

        fs::create_dir_all(&target_dir)
            .map_err(|e| SyncError::Platform(format!("Failed to create ~/Applications/: {}", e)))?;

        let target = target_dir.join("cook-sync.AppImage");

        // Try atomic rename first (works on same filesystem)
        if fs::rename(source, &target).is_ok() {
            info!("Moved AppImage to {} (rename)", target.display());
            return Ok(target);
        }

        // Fallback: copy + remove (cross-filesystem)
        fs::copy(source, &target).map_err(|e| {
            SyncError::Platform(format!(
                "Failed to copy AppImage to {}: {}",
                target.display(),
                e
            ))
        })?;
        // Delete original (best effort — copy succeeded so the new location works)
        if let Err(e) = fs::remove_file(source) {
            error!(
                "AppImage copied to {} but failed to remove original {}: {}",
                target.display(),
                source.display(),
                e
            );
        }
        info!("Moved AppImage to {} (copy+remove)", target.display());

        Ok(target)
    }

    /// Check if the AppImage is in a transient location and offer to relocate it.
    /// If the user accepts, moves the file and re-execs from the new location.
    pub fn check_and_relocate() -> Result<()> {
        // Get AppImage path; return Ok if not running from AppImage
        let appimage_path = match get_appimage_path() {
            Ok(p) => p,
            Err(_) => return Ok(()),
        };

        if !is_transient_location(&appimage_path) {
            return Ok(());
        }

        if relocation_declined() {
            return Ok(());
        }

        if !show_relocation_dialog() {
            mark_relocation_declined();
            return Ok(());
        }

        let new_path = match move_appimage(&appimage_path) {
            Ok(p) => p,
            Err(e) => {
                error!("Failed to move AppImage: {}", e);
                return Ok(());
            }
        };

        // Re-exec from the new location with the same arguments
        info!("Re-executing from {}", new_path.display());
        let args: Vec<String> = std::env::args().skip(1).collect();
        let err = std::os::unix::process::CommandExt::exec(
            std::process::Command::new(&new_path).args(&args),
        );
        error!("Failed to re-exec from new location: {}", err);
        Ok(())
    }

    /// Get the AppImage path from environment or current exe
    pub fn get_appimage_path() -> Result<PathBuf> {
        // First try the APPIMAGE environment variable
        if let Ok(appimage) = std::env::var("APPIMAGE") {
            return Ok(PathBuf::from(appimage));
        }

        // Fallback: check if current exe is in an AppImage mount
        let exe_path = std::env::current_exe()?;
        let path_str = exe_path.to_string_lossy();

        if path_str.contains(".AppImage") || path_str.contains("/tmp/.mount_") {
            return Ok(exe_path);
        }

        Err(SyncError::Platform(
            "Not running from AppImage. Desktop integration only works with AppImage builds."
                .into(),
        ))
    }

    /// Extract the desktop file from the mounted AppImage
    pub fn extract_desktop_file() -> Result<String> {
        let exe_path = std::env::current_exe()?;

        // When AppImage is mounted, the binary is typically at:
        // /tmp/.mount_XXXXXX/usr/bin/cook-sync
        // Desktop file should be at: /tmp/.mount_XXXXXX/usr/share/applications/cook-sync.desktop
        let mount_root = exe_path
            .parent() // /usr/bin
            .and_then(|p| p.parent()) // /usr
            .and_then(|p| p.parent()) // mount root
            .ok_or_else(|| SyncError::Platform("Cannot find AppImage mount point".into()))?;

        let desktop_file = mount_root.join("usr/share/applications/cook-sync.desktop");

        debug!("Looking for desktop file at: {}", desktop_file.display());

        fs::read_to_string(&desktop_file).map_err(|e| {
            SyncError::Platform(format!(
                "Failed to read desktop file from AppImage: {}. Path: {}",
                e,
                desktop_file.display()
            ))
        })
    }

    /// Extract an icon from the mounted AppImage
    pub fn extract_icon(size: &str) -> Result<Vec<u8>> {
        let exe_path = std::env::current_exe()?;

        let mount_root = exe_path
            .parent()
            .and_then(|p| p.parent())
            .and_then(|p| p.parent())
            .ok_or_else(|| SyncError::Platform("Cannot find AppImage mount point".into()))?;

        // Try multiple possible icon locations
        let icon_paths = [
            mount_root.join(format!(
                "usr/share/icons/hicolor/{}/apps/cook-sync.png",
                size
            )),
            mount_root.join(format!("usr/share/pixmaps/cook-sync-{}.png", size)),
            // Fallback to looking for icon files in the binary directory
            mount_root.join(format!(
                "usr/bin/icon-{}.png",
                size.split('x').next().unwrap()
            )),
        ];

        for icon_path in &icon_paths {
            if icon_path.exists() {
                debug!("Found icon at: {}", icon_path.display());
                return fs::read(icon_path)
                    .map_err(|e| SyncError::Platform(format!("Failed to read icon file: {}", e)));
            }
        }

        Err(SyncError::Platform(format!(
            "Icon not found for size: {}",
            size
        )))
    }

    /// Update the Exec line in the desktop file with the absolute AppImage path
    pub fn update_desktop_exec(content: &str, appimage_path: &Path) -> Result<String> {
        let lines: Vec<String> = content
            .lines()
            .map(|line| {
                if line.starts_with("Exec=") {
                    // Extract any arguments after the command
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() > 1 {
                        // Has arguments (e.g., "Exec=cook-sync start")
                        let args = parts[1..].join(" ");
                        format!("Exec={} {}", appimage_path.display(), args)
                    } else {
                        // No arguments
                        format!("Exec={}", appimage_path.display())
                    }
                } else {
                    line.to_string()
                }
            })
            .collect();

        Ok(lines.join("\n"))
    }

    /// Install icons at multiple resolutions
    pub fn install_icons() -> Result<()> {
        let icons_dir = dirs::data_local_dir()
            .ok_or_else(|| SyncError::Platform("Cannot find local data directory".into()))?
            .join("icons/hicolor");

        // Icon sizes to install
        let sizes = ["16x16", "32x32", "48x48", "128x128", "256x256"];

        for size in &sizes {
            let target_dir = icons_dir.join(format!("{}/apps", size));
            fs::create_dir_all(&target_dir)?;

            // Try to extract icon from AppImage
            if let Ok(icon_data) = extract_icon(size) {
                let icon_path = target_dir.join("cook-sync.png");
                fs::write(&icon_path, icon_data)?;
                debug!("Installed icon: {}", icon_path.display());
            } else {
                debug!("Icon not found for size: {}", size);
            }
        }

        Ok(())
    }

    /// Update the desktop database (optional, best effort)
    pub fn update_desktop_database(apps_dir: &Path) -> Result<()> {
        match clean_appimage_env("update-desktop-database")
            .arg(apps_dir)
            .status()
        {
            Ok(_) => {
                debug!("Desktop database updated");
                Ok(())
            }
            Err(e) => {
                // Not a critical error, just log it
                debug!("Could not update desktop database: {}", e);
                Ok(())
            }
        }
    }

    /// Install desktop integration
    pub fn install() -> Result<()> {
        info!("Installing desktop integration for Cook Sync AppImage");

        // 1. Verify we're in an AppImage
        let appimage_path = get_appimage_path()?;
        info!("AppImage path: {}", appimage_path.display());

        // 2. Extract desktop file
        let desktop_content = extract_desktop_file()?;

        // 3. Update Exec line with absolute path
        let updated_content = update_desktop_exec(&desktop_content, &appimage_path)?;

        // 4. Ensure applications directory exists
        let apps_dir = dirs::data_local_dir()
            .ok_or_else(|| SyncError::Platform("Cannot find local data directory".into()))?
            .join("applications");
        fs::create_dir_all(&apps_dir)?;

        // 5. Write desktop file
        let desktop_file_path = apps_dir.join("cook-sync.desktop");
        fs::write(&desktop_file_path, updated_content)?;
        info!("Desktop file installed to: {}", desktop_file_path.display());

        // 6. Install icons
        install_icons()?;

        // 7. Update desktop database
        update_desktop_database(&apps_dir)?;

        println!("✓ Desktop integration installed successfully!");
        println!("  Cook Sync can now be launched from your application menu.");

        Ok(())
    }

    /// Uninstall desktop integration
    pub fn uninstall() -> Result<()> {
        info!("Uninstalling desktop integration for Cook Sync");

        let mut removed_items = Vec::new();

        if let Some(data_dir) = dirs::data_local_dir() {
            // 1. Remove desktop file
            let desktop_file = data_dir.join("applications/cook-sync.desktop");
            if desktop_file.exists() {
                fs::remove_file(&desktop_file)?;
                removed_items.push("Desktop entry");
                info!("Removed desktop file: {}", desktop_file.display());
            }

            // 2. Remove icons
            let icons_dir = data_dir.join("icons/hicolor");
            let sizes = ["16x16", "32x32", "48x48", "128x128", "256x256"];

            let mut removed_icons = false;
            for size in &sizes {
                let icon_path = icons_dir.join(format!("{}/apps/cook-sync.png", size));
                if icon_path.exists() {
                    fs::remove_file(&icon_path)?;
                    debug!("Removed icon: {}", icon_path.display());
                    removed_icons = true;
                }
            }

            if removed_icons {
                removed_items.push("Application icons");
            }

            // 3. Update desktop database
            let apps_dir = data_dir.join("applications");
            update_desktop_database(&apps_dir)?;
        }

        if removed_items.is_empty() {
            println!("No desktop integration found to remove.");
        } else {
            println!("✓ Desktop integration removed successfully!");
            for item in removed_items {
                println!("  - Removed {}", item);
            }
        }

        Ok(())
    }

    /// Check if desktop integration is installed
    pub fn is_installed() -> Result<bool> {
        if let Some(data_dir) = dirs::data_local_dir() {
            let desktop_file = data_dir.join("applications/cook-sync.desktop");
            Ok(desktop_file.exists())
        } else {
            Ok(false)
        }
    }
}

impl PlatformIntegration for LinuxIntegration {
    fn enable_auto_start(&self, app_name: &str, app_path: &str) -> Result<()> {
        // When running from AppImage, use the actual AppImage path instead of the
        // temporary mount point. The APPIMAGE env var contains the real .AppImage file path.
        let actual_path = if desktop_integration::is_running_from_appimage() {
            if let Ok(appimage_path) = std::env::var("APPIMAGE") {
                debug!("Using AppImage path for auto-start: {}", appimage_path);
                appimage_path
            } else {
                debug!("Running from AppImage but APPIMAGE env var not set, using provided path");
                app_path.to_string()
            }
        } else {
            app_path.to_string()
        };

        let auto = AutoLaunchBuilder::new()
            .set_app_name(app_name)
            .set_app_path(&actual_path)
            .set_args(&["daemon"])
            .build()
            .map_err(|e| SyncError::Platform(format!("Failed to create auto-launch: {e}")))?;

        auto.enable()
            .map_err(|e| SyncError::Platform(format!("Failed to enable auto-start: {e}")))?;

        info!(
            "Auto-start enabled for {} with path: {}",
            app_name, actual_path
        );
        Ok(())
    }

    fn disable_auto_start(&self, app_name: &str) -> Result<()> {
        // Use AppImage path if running from AppImage
        let actual_path = if desktop_integration::is_running_from_appimage() {
            std::env::var("APPIMAGE").unwrap_or_else(|_| {
                std::env::current_exe()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string()
            })
        } else {
            std::env::current_exe()?.to_string_lossy().to_string()
        };

        let auto = AutoLaunchBuilder::new()
            .set_app_name(app_name)
            .set_app_path(&actual_path)
            .set_args(&["daemon"])
            .build()
            .map_err(|e| SyncError::Platform(format!("Failed to create auto-launch: {e}")))?;

        auto.disable()
            .map_err(|e| SyncError::Platform(format!("Failed to disable auto-start: {e}")))?;

        info!("Auto-start disabled for {}", app_name);
        Ok(())
    }

    fn is_auto_start_enabled(&self, app_name: &str) -> Result<bool> {
        // Use AppImage path if running from AppImage
        let actual_path = if desktop_integration::is_running_from_appimage() {
            std::env::var("APPIMAGE").unwrap_or_else(|_| {
                std::env::current_exe()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string()
            })
        } else {
            std::env::current_exe()?.to_string_lossy().to_string()
        };

        let auto = AutoLaunchBuilder::new()
            .set_app_name(app_name)
            .set_app_path(&actual_path)
            .set_args(&["daemon"])
            .build()
            .map_err(|e| SyncError::Platform(format!("Failed to create auto-launch: {e}")))?;

        auto.is_enabled()
            .map_err(|e| SyncError::Platform(format!("Failed to check auto-start status: {e}")))
    }

    fn is_dark_mode(&self) -> bool {
        matches!(dark_light::detect(), Ok(dark_light::Mode::Dark))
    }

    fn is_desktop_integration_installed(&self) -> Result<bool> {
        desktop_integration::is_installed()
    }

    fn install_desktop_integration(&self) -> Result<()> {
        desktop_integration::install()
    }

    fn uninstall_desktop_integration(&self) -> Result<()> {
        desktop_integration::uninstall()
    }

    fn watch_theme_changes(&self, shutdown_signal: Arc<AtomicBool>) -> Option<ThemeWatcher> {
        let (tx, rx) = mpsc::channel();

        let handle = thread::spawn(move || {
            debug!("Starting Linux theme watcher (5-second polling)");

            let mut last_is_dark = matches!(dark_light::detect(), Ok(dark_light::Mode::Dark));

            // Try to detect the desktop environment
            let desktop = std::env::var("XDG_CURRENT_DESKTOP")
                .or_else(|_| std::env::var("DESKTOP_SESSION"))
                .unwrap_or_default()
                .to_lowercase();

            debug!("Detected desktop environment: {}", desktop);

            while !shutdown_signal.load(Ordering::Relaxed) {
                // Sleep for 5 seconds between checks
                for _ in 0..50 {
                    if shutdown_signal.load(Ordering::Relaxed) {
                        debug!("Theme watcher received shutdown signal");
                        return;
                    }
                    thread::sleep(Duration::from_millis(100));
                }

                // Check theme using appropriate method for the desktop environment
                let current_is_dark = if desktop.contains("gnome") || desktop.contains("ubuntu") {
                    // GNOME/Ubuntu: Check gsettings
                    check_gnome_dark_mode()
                } else if desktop.contains("kde") || desktop.contains("plasma") {
                    // KDE Plasma: Check config file
                    check_kde_dark_mode()
                } else {
                    // Fallback to dark-light crate detection
                    matches!(dark_light::detect(), Ok(dark_light::Mode::Dark))
                };

                if current_is_dark != last_is_dark {
                    let theme = if current_is_dark {
                        ThemeChange::Dark
                    } else {
                        ThemeChange::Light
                    };

                    debug!("Linux theme changed to: {:?}", theme);

                    if let Err(e) = tx.send(theme) {
                        error!("Failed to send theme change notification: {}", e);
                        break;
                    }

                    last_is_dark = current_is_dark;
                }
            }

            debug!("Theme watcher shutting down gracefully");
        });

        Some(ThemeWatcher {
            receiver: rx,
            handle,
        })
    }
}

fn check_gnome_dark_mode() -> bool {
    // Try to get GNOME color scheme setting
    if let Ok(output) = desktop_integration::clean_appimage_env("gsettings")
        .args(["get", "org.gnome.desktop.interface", "color-scheme"])
        .output()
    {
        let scheme = String::from_utf8_lossy(&output.stdout);
        return scheme.contains("dark") || scheme.contains("prefer-dark");
    }

    // Fallback: Check GTK theme name
    if let Ok(output) = desktop_integration::clean_appimage_env("gsettings")
        .args(["get", "org.gnome.desktop.interface", "gtk-theme"])
        .output()
    {
        let theme = String::from_utf8_lossy(&output.stdout).to_lowercase();
        return theme.contains("dark");
    }

    // Final fallback
    matches!(dark_light::detect(), Ok(dark_light::Mode::Dark))
}

fn check_kde_dark_mode() -> bool {
    // Check KDE color scheme configuration
    if let Some(home) = dirs::home_dir() {
        let config_path = home.join(".config/kdeglobals");
        if let Ok(content) = fs::read_to_string(&config_path) {
            // Look for dark color scheme indicators in the config
            for line in content.lines() {
                if line.starts_with("ColorScheme=") {
                    let scheme = line.replace("ColorScheme=", "").to_lowercase();
                    return scheme.contains("dark");
                }
            }
        }
    }

    // Fallback to dark-light crate
    matches!(dark_light::detect(), Ok(dark_light::Mode::Dark))
}
