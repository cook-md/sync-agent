use super::{PlatformIntegration, ThemeChange, ThemeWatcher};
use crate::error::{Result, SyncError};
use auto_launch::AutoLaunchBuilder;
use log::{debug, error, info};
use std::fs;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::Duration;

pub struct LinuxIntegration;

impl PlatformIntegration for LinuxIntegration {
    fn enable_auto_start(&self, app_name: &str, app_path: &str) -> Result<()> {
        let auto = AutoLaunchBuilder::new()
            .set_app_name(app_name)
            .set_app_path(app_path)
            .set_args(&["daemon"])
            .build()
            .map_err(|e| SyncError::Platform(format!("Failed to create auto-launch: {e}")))?;

        auto.enable()
            .map_err(|e| SyncError::Platform(format!("Failed to enable auto-start: {e}")))?;

        info!("Auto-start enabled for {}", app_name);
        Ok(())
    }

    fn disable_auto_start(&self, app_name: &str) -> Result<()> {
        let app_path = std::env::current_exe()?;
        let auto = AutoLaunchBuilder::new()
            .set_app_name(app_name)
            .set_app_path(app_path.to_str().unwrap())
            .build()
            .map_err(|e| SyncError::Platform(format!("Failed to create auto-launch: {e}")))?;

        auto.disable()
            .map_err(|e| SyncError::Platform(format!("Failed to disable auto-start: {e}")))?;

        info!("Auto-start disabled for {}", app_name);
        Ok(())
    }

    fn is_auto_start_enabled(&self, app_name: &str) -> Result<bool> {
        let app_path = std::env::current_exe()?;
        let auto = AutoLaunchBuilder::new()
            .set_app_name(app_name)
            .set_app_path(app_path.to_str().unwrap())
            .build()
            .map_err(|e| SyncError::Platform(format!("Failed to create auto-launch: {e}")))?;

        auto.is_enabled()
            .map_err(|e| SyncError::Platform(format!("Failed to check auto-start status: {e}")))
    }

    fn is_dark_mode(&self) -> bool {
        matches!(dark_light::detect(), Ok(dark_light::Mode::Dark))
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
    if let Ok(output) = std::process::Command::new("gsettings")
        .args(["get", "org.gnome.desktop.interface", "color-scheme"])
        .output()
    {
        let scheme = String::from_utf8_lossy(&output.stdout);
        return scheme.contains("dark") || scheme.contains("prefer-dark");
    }

    // Fallback: Check GTK theme name
    if let Ok(output) = std::process::Command::new("gsettings")
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
