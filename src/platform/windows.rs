use super::{PlatformIntegration, ThemeChange, ThemeWatcher};
use crate::error::{Result, SyncError};
use auto_launch::AutoLaunchBuilder;
use log::{debug, error, info};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::Duration;
use winapi::shared::minwindef::{DWORD, HKEY};
use winapi::um::winnt::{KEY_NOTIFY, REG_NOTIFY_CHANGE_LAST_SET};
use winapi::um::winreg::RegNotifyChangeKeyValue;
use winreg::enums::*;
use winreg::RegKey;

pub struct WindowsIntegration;

impl PlatformIntegration for WindowsIntegration {
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
            debug!("Starting Windows theme watcher with registry monitoring");

            let mut last_is_dark = matches!(dark_light::detect(), Ok(dark_light::Mode::Dark));

            // Path to Windows theme settings in registry
            let theme_path = r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize";

            while !shutdown_signal.load(Ordering::Relaxed) {
                // Open the registry key
                let hkcu = RegKey::predef(HKEY_CURRENT_USER);

                if let Ok(key) = hkcu.open_subkey_with_flags(theme_path, KEY_READ | KEY_NOTIFY) {
                    // Get the raw handle for notification
                    let handle = key.raw_handle() as HKEY;

                    // Set up registry change notification
                    unsafe {
                        let result = RegNotifyChangeKeyValue(
                            handle,
                            0, // Don't watch subtree
                            REG_NOTIFY_CHANGE_LAST_SET as DWORD,
                            std::ptr::null_mut(), // No event, synchronous
                            0,                    // Synchronous
                        );

                        if result == 0 {
                            // Notification set up successfully
                            // Now check if the theme has actually changed
                            if let Ok(apps_use_light) =
                                key.get_value::<DWORD, _>("AppsUseLightTheme")
                            {
                                let is_dark = apps_use_light == 0;

                                if is_dark != last_is_dark {
                                    let theme = if is_dark {
                                        ThemeChange::Dark
                                    } else {
                                        ThemeChange::Light
                                    };

                                    debug!("Windows theme changed to: {:?}", theme);

                                    if let Err(e) = tx.send(theme) {
                                        error!("Failed to send theme change notification: {}", e);
                                        break;
                                    }

                                    last_is_dark = is_dark;
                                }
                            }
                        }
                    }
                } else {
                    // If we can't open the key, fall back to polling
                    let current_is_dark =
                        matches!(dark_light::detect(), Ok(dark_light::Mode::Dark));

                    if current_is_dark != last_is_dark {
                        let theme = if current_is_dark {
                            ThemeChange::Dark
                        } else {
                            ThemeChange::Light
                        };

                        debug!("Windows theme changed to: {:?}", theme);

                        if let Err(e) = tx.send(theme) {
                            error!("Failed to send theme change notification: {}", e);
                            break;
                        }

                        last_is_dark = current_is_dark;
                    }
                }

                // Check for shutdown periodically
                for _ in 0..10 {
                    if shutdown_signal.load(Ordering::Relaxed) {
                        debug!("Theme watcher received shutdown signal");
                        return;
                    }
                    thread::sleep(Duration::from_millis(100));
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
