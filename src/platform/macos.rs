use super::{PlatformIntegration, ThemeChange, ThemeWatcher};
use crate::error::{Result, SyncError};
use auto_launch::AutoLaunchBuilder;
use log::{debug, error, info};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::Duration;

pub struct MacOSIntegration;

impl PlatformIntegration for MacOSIntegration {
    fn enable_auto_start(&self, app_name: &str, app_path: &str) -> Result<()> {
        let auto = AutoLaunchBuilder::new()
            .set_app_name(app_name)
            .set_app_path(app_path)
            .set_args(&["daemon"])
            .set_use_launch_agent(true) // Use LaunchAgent on macOS
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
            .set_use_launch_agent(true)
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
            .set_use_launch_agent(true)
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
            debug!("Starting macOS theme watcher (5-second polling)");

            let mut last_is_dark = check_is_dark_mode();

            while !shutdown_signal.load(Ordering::Relaxed) {
                // Sleep for 5 seconds between checks
                for _ in 0..50 {
                    if shutdown_signal.load(Ordering::Relaxed) {
                        debug!("Theme watcher received shutdown signal");
                        return;
                    }
                    thread::sleep(Duration::from_millis(100));
                }

                // Check for theme changes
                let current_is_dark = check_is_dark_mode();
                if current_is_dark != last_is_dark {
                    let theme = if current_is_dark {
                        ThemeChange::Dark
                    } else {
                        ThemeChange::Light
                    };

                    debug!("macOS theme changed to: {:?}", theme);

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

// Helper function to check current dark mode state
fn check_is_dark_mode() -> bool {
    // Run the defaults command
    let output = std::process::Command::new("defaults")
        .args(["read", "-g", "AppleInterfaceStyle"])
        .output();

    match output {
        Ok(result) => {
            // Check if command succeeded (exit code 0)
            if result.status.success() {
                // Parse the output - should be "Dark" for dark mode
                let theme = String::from_utf8_lossy(&result.stdout);
                theme.trim().eq_ignore_ascii_case("dark")
            } else {
                // Command failed - key doesn't exist, means light mode
                false
            }
        }
        Err(_) => {
            // Failed to run command - assume light mode
            false
        }
    }
}
