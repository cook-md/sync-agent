use crate::api::CookApi;
use crate::auth::AuthManager;
use crate::config::Config;
use crate::error::Result;
use crate::sync::SyncManager;
use crate::tray::SystemTray;
// Update manager is available for manual checks via UpdateManager::new()
// Auto-update checking can be re-implemented if needed
use log::info;
use std::fs;
use std::sync::Arc;

pub struct Daemon {
    config: Arc<Config>,
    auth_manager: Arc<AuthManager>,
    sync_manager: Arc<SyncManager>,
}

impl Daemon {
    pub async fn new() -> Result<Self> {
        // Initialize configuration
        let config = Arc::new(Config::new()?);

        // Create API client
        let api_endpoint = crate::config::settings::Settings::get_api_endpoint();
        let api = Arc::new(CookApi::new(api_endpoint)?);
        // Initialize auth manager
        let auth_manager = Arc::new(AuthManager::new(config.paths(), api)?);

        // Initialize sync manager
        let sync_manager = Arc::new(SyncManager::new(
            Arc::clone(&auth_manager),
            Arc::clone(&config),
        ));

        Ok(Daemon {
            config,
            auth_manager,
            sync_manager,
        })
    }

    pub async fn run(&self) -> Result<()> {
        info!("Starting Cook Sync daemon");

        // Write PID file
        self.write_pid_file()?;

        // Start token refresh if authenticated
        if self.auth_manager.is_authenticated() {
            self.auth_manager.start_token_refresh().await;
        }

        // Start sync manager if configured and authenticated
        if self.config.settings().lock().unwrap().recipes_dir.is_some()
            && self.auth_manager.is_authenticated()
        {
            self.sync_manager.start().await?;
        }

        // Check for updates in background if enabled
        let config_clone = Arc::clone(&self.config);
        tokio::spawn(async move {
            // Wait a bit before checking to avoid slowing down startup
            tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;

            let auto_update = config_clone.settings().lock().unwrap().auto_update;
            if auto_update {
                info!("Checking for updates in background (auto-update enabled)");
                match crate::updater::check_for_updates(true).await {
                    Ok(Some(version)) => {
                        info!(
                            "Update to version {} downloaded, will install on restart",
                            version
                        );
                        let _ = crate::notifications::show_notification(
                            "Cook Sync Update",
                            &format!(
                                "Update to version {} has been downloaded and will be installed on next restart.",
                                version
                            ),
                        );
                    }
                    Ok(None) => {
                        info!("No updates available");
                    }
                    Err(e) => {
                        log::warn!("Background update check failed: {}", e);
                    }
                }
            } else {
                info!("Auto-update disabled, skipping background update check");
            }
        });

        // Setup signal handler for graceful shutdown
        let _sync_manager_clone = Arc::clone(&self.sync_manager);
        tokio::spawn(async move {
            #[cfg(unix)]
            {
                use tokio::signal::unix::{signal, SignalKind};
                let mut sigterm = signal(SignalKind::terminate()).unwrap();
                sigterm.recv().await;
                info!("Received SIGTERM, stopping sync manager");
                let _ = _sync_manager_clone.stop().await;
            }
        });

        // Sync auto-start state with system
        let platform = crate::platform::get_platform();
        let config_auto_start = self.config.settings().lock().unwrap().auto_start;
        let system_auto_start = platform.is_auto_start_enabled("cook-sync").unwrap_or(false);

        if config_auto_start && !system_auto_start {
            // Config says enabled but system doesn't have it - install it
            let app_path = std::env::current_exe()?;
            platform.enable_auto_start("cook-sync", &app_path.to_string_lossy())?;
            info!("Auto-start enabled by default: registered with system");
        } else if !config_auto_start && system_auto_start {
            // Config says disabled but system has it - unregister it
            platform.disable_auto_start("cook-sync")?;
            info!("Auto-start disabled: unregistered from system");
        }

        // Create and run system tray
        info!("Initializing system tray...");

        // Get the current Tokio runtime handle
        let runtime_handle = tokio::runtime::Handle::current();

        let tray = match SystemTray::new(
            Arc::clone(&self.sync_manager),
            Arc::clone(&self.auth_manager),
            Arc::clone(&self.config),
            runtime_handle,
        ) {
            Ok(tray) => {
                info!("System tray created successfully");
                tray
            }
            Err(e) => {
                log::error!("Failed to create system tray: {}", e);

                #[cfg(windows)]
                log::error!(
                    "Windows: Ensure no other instance is running. Check Task Manager for cook-sync processes."
                );

                #[cfg(target_os = "linux")]
                {
                    log::error!("Linux: System tray icon failed to initialize.");
                    log::error!("This may be due to:");
                    log::error!("  1. Missing libappindicator3 library");
                    log::error!("     Ubuntu/Debian: sudo apt-get install libappindicator3-1");
                    log::error!("  2. GNOME desktop environment without AppIndicator extension");
                    log::error!(
                        "     Install: sudo apt-get install gnome-shell-extension-appindicator"
                    );
                    log::error!(
                        "     Enable: gnome-extensions enable ubuntu-appindicators@ubuntu.com"
                    );
                    log::error!(
                        "  3. Desktop environment: {}",
                        std::env::var("XDG_CURRENT_DESKTOP")
                            .unwrap_or_else(|_| "Unknown".to_string())
                    );
                    log::error!(
                        "  4. Session type: {}",
                        std::env::var("XDG_SESSION_TYPE").unwrap_or_else(|_| "Unknown".to_string())
                    );
                }

                return Err(e);
            }
        };

        // Run the tray (this blocks)
        info!("Starting system tray event loop...");
        if let Err(e) = tray.run() {
            log::error!("System tray event loop failed: {}", e);
            return Err(e);
        }

        // Stop sync manager before cleanup
        self.sync_manager.stop().await?;

        // Cleanup
        self.cleanup()?;

        Ok(())
    }

    fn write_pid_file(&self) -> Result<()> {
        let pid = std::process::id();
        let pid_file = &self.config.paths().pid_file;
        fs::write(pid_file, pid.to_string())?;
        Ok(())
    }

    fn cleanup(&self) -> Result<()> {
        // Remove PID file
        let pid_file = &self.config.paths().pid_file;
        if pid_file.exists() {
            fs::remove_file(pid_file)?;
        }
        Ok(())
    }
}

pub fn is_already_running(config: &Config) -> bool {
    let pid_file = &config.paths().pid_file;

    if let Ok(pid_str) = fs::read_to_string(pid_file) {
        if let Ok(pid) = pid_str.trim().parse::<u32>() {
            // Check if process is still running
            #[cfg(unix)]
            {
                let running = unsafe { libc::kill(pid as i32, 0) == 0 };
                if !running {
                    log::warn!("Found stale PID file for process {}, cleaning up", pid);
                    let _ = fs::remove_file(pid_file);
                    return false;
                }
                running
            }

            #[cfg(windows)]
            {
                use winapi::um::handleapi::CloseHandle;
                use winapi::um::processthreadsapi::OpenProcess;
                use winapi::um::winnt::PROCESS_QUERY_INFORMATION;

                unsafe {
                    let handle = OpenProcess(PROCESS_QUERY_INFORMATION, 0, pid);
                    if !handle.is_null() {
                        CloseHandle(handle);
                        true
                    } else {
                        log::warn!("Found stale PID file for process {}, cleaning up", pid);
                        let _ = fs::remove_file(pid_file);
                        false
                    }
                }
            }

            #[cfg(not(any(unix, windows)))]
            {
                false
            }
        } else {
            log::warn!(
                "Invalid PID file content: '{}', cleaning up",
                pid_str.trim()
            );
            let _ = fs::remove_file(pid_file);
            false
        }
    } else {
        false
    }
}
