mod api;
mod appimage_integration;
mod auth;
mod config;
mod daemon;
mod error;
mod logging;
mod notifications;
mod platform;
mod sentry_integration;
mod sync;
mod tray;
mod updates;

#[cfg(target_os = "windows")]
mod windows_integration;

use clap::{Parser, Subcommand};
use error::Result;
#[allow(unused_imports)]
use log::{error, info};
use std::sync::Arc;

#[derive(Parser)]
#[command(name = "cook-sync")]
#[command(about = "Cook.md Sync Agent for syncing recipes with Cook.md")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the sync agent daemon
    Start,

    /// Start in daemon mode (used by auto-start)
    Daemon,

    /// Stop the running sync agent
    Stop,

    /// Show sync status
    Status,

    /// Open browser for login
    Login,

    /// Logout and clear session
    Logout,

    /// Configure sync settings
    Config {
        /// Set recipes directory
        #[arg(long)]
        recipes_dir: Option<String>,

        /// Enable/disable auto-start
        #[arg(long)]
        auto_start: Option<bool>,

        /// Enable/disable auto-update
        #[arg(long)]
        auto_update: Option<bool>,

        /// Show current configuration
        #[arg(long, short)]
        show: bool,
    },

    /// Check for updates
    Update,

    /// Install desktop integration (AppImage only)
    Install,

    /// Uninstall desktop integration (AppImage only)
    Uninstall,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize configuration to get log file path
    let config = config::Config::new()?;
    let log_file_path = config.paths().log_file.clone();

    // Initialize logger with file output
    logging::init_logging(&log_file_path)?;

    // Initialize Sentry for error tracking
    sentry_integration::init_sentry();

    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Start) => start_daemon().await,
        Some(Commands::Daemon) => run_daemon().await,
        Some(Commands::Stop) => stop_daemon(),
        Some(Commands::Status) => show_status().await,
        Some(Commands::Login) => login().await,
        Some(Commands::Logout) => logout().await,
        Some(Commands::Config {
            recipes_dir,
            auto_start,
            auto_update,
            show,
        }) => configure(recipes_dir, auto_start, auto_update, show).await,
        Some(Commands::Update) => check_update().await,
        Some(Commands::Install) => install_integration(),
        Some(Commands::Uninstall) => uninstall_integration(),
        None => {
            // If no command specified, start the daemon
            start_daemon().await
        }
    }
}

async fn start_daemon() -> Result<()> {
    let config = config::Config::new()?;

    if daemon::is_already_running(&config) {
        println!("Cook Sync is already running");
        return Ok(());
    }

    println!("Starting Cook Sync...");

    // Fork and run in background
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;

        let exe = std::env::current_exe()?;

        // On macOS, we need to maintain GUI access for the system tray
        // Use launchctl to start the daemon properly
        let mut cmd = Command::new(exe);
        cmd.arg("daemon");

        // Set environment to ensure GUI access
        cmd.env("COOK_SYNC_DAEMON", "1");

        // Detach from parent but keep GUI access
        #[cfg(unix)]
        {
            cmd.stdin(std::process::Stdio::null());
        }

        match cmd.spawn() {
            Ok(_) => {
                println!("Cook Sync started successfully");
                println!("The system tray icon should appear in your menu bar.");
                Ok(())
            }
            Err(e) => {
                error!("Failed to start daemon: {e}");
                Err(error::SyncError::Other(format!(
                    "Failed to start daemon: {e}"
                )))
            }
        }
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        use std::process::Command;

        // Start the daemon subprocess
        let exe = std::env::current_exe()?;
        let mut cmd = Command::new(exe);
        cmd.arg("daemon");

        // Detach from parent, but keep display server access for tray icon
        // Only redirect stdin to avoid blocking on input
        cmd.stdin(std::process::Stdio::null());

        // Start the process detached
        match cmd.spawn() {
            Ok(_) => {
                println!("Cook Sync started successfully");
                println!("The system tray icon should appear in your status bar.");
                Ok(())
            }
            Err(e) => {
                error!("Failed to start daemon: {e}");
                Err(error::SyncError::Other(format!(
                    "Failed to start daemon: {e}"
                )))
            }
        }
    }

    #[cfg(windows)]
    {
        // On Windows, use similar approach
        use std::process::Command;

        let exe = std::env::current_exe()?;
        let mut cmd = Command::new(exe);
        cmd.arg("daemon");

        // Detach from parent
        cmd.stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());

        match cmd.spawn() {
            Ok(_) => {
                println!("Cook Sync started successfully");
                Ok(())
            }
            Err(e) => {
                error!("Failed to start daemon: {e}");
                Err(error::SyncError::Other(format!(
                    "Failed to start daemon: {e}"
                )))
            }
        }
    }
}

async fn run_daemon() -> Result<()> {
    // On Linux and macOS, we avoid traditional fork-based daemonization
    // because system tray apps need to maintain access to the display server
    // (X11/Wayland on Linux, WindowServer on macOS)
    #[cfg(unix)]
    {
        let config = config::Config::new()?;
        let pid_file = config.paths().pid_file.clone();

        // Write PID file
        std::fs::write(&pid_file, std::process::id().to_string())?;
        info!("Cook Sync daemon started (PID: {})", std::process::id());
    }

    // Check if running from AppImage and offer integration on first run
    #[cfg(target_os = "linux")]
    {
        if let Err(e) = appimage_integration::offer_integration() {
            // Don't fail daemon start if integration fails
            error!("Failed to offer AppImage integration: {e}");
        }
    }

    // Offer Windows integration on first run
    #[cfg(target_os = "windows")]
    {
        if let Err(e) = windows_integration::offer_integration() {
            // Don't fail daemon start if integration fails
            error!("Failed to offer Windows integration: {e}");
        }
    }

    // Now run the actual daemon
    let daemon = daemon::Daemon::new().await?;
    daemon.run().await
}

fn stop_daemon() -> Result<()> {
    let config = config::Config::new()?;

    if !daemon::is_already_running(&config) {
        println!("Cook Sync is not running");
        return Ok(());
    }

    // Read PID and send termination signal
    let pid_str = std::fs::read_to_string(&config.paths().pid_file)?;
    let _pid: u32 = pid_str
        .trim()
        .parse()
        .map_err(|_| error::SyncError::Other("Invalid PID file".to_string()))?;

    #[cfg(unix)]
    {
        unsafe {
            // First try SIGTERM for graceful shutdown
            if libc::kill(_pid as i32, libc::SIGTERM) == 0 {
                // Wait a bit to see if it shuts down
                std::thread::sleep(std::time::Duration::from_millis(500));

                // Check if still running
                if libc::kill(_pid as i32, 0) == 0 {
                    // Still running, force kill
                    info!("Process still running, sending SIGKILL");
                    libc::kill(_pid as i32, libc::SIGKILL);
                }

                println!("Cook Sync stopped");
                // Clean up PID file
                std::fs::remove_file(&config.paths().pid_file).ok();
            } else {
                error!("Failed to stop Cook Sync");
            }
        }
    }

    #[cfg(windows)]
    {
        println!("Stopping Cook Sync on Windows not yet implemented");
    }

    Ok(())
}

async fn show_status() -> Result<()> {
    let config = config::Config::new()?;

    if !daemon::is_already_running(&config) {
        println!("Cook Sync is not running");
        return Ok(());
    }

    println!("Cook Sync is running");

    // Load and display current settings
    let settings = config.settings();
    let settings = settings.lock().unwrap();

    println!("Configuration:");
    println!(
        "  Recipes directory: {}",
        settings
            .recipes_dir
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "Not configured".to_string())
    );
    println!("  Auto-start: {}", settings.auto_start);
    println!("  Auto-update: {}", settings.auto_update);
    println!("  Sync interval: {} seconds", settings.sync_interval_secs);

    // Check authentication status
    let api = api::CookApi::new(config::settings::Settings::get_api_endpoint())?;
    let auth = auth::AuthManager::new(config.paths(), Arc::new(api))?;

    if auth.is_authenticated() {
        if let Some(session) = auth.get_session() {
            println!(
                "\nAuthenticated as: {}",
                session.email.unwrap_or(session.user_id)
            );
        }
    } else {
        println!("\nNot authenticated. Run 'cook-sync login' to authenticate.");
    }

    Ok(())
}

async fn login() -> Result<()> {
    println!("Opening browser for login...");

    let config = config::Config::new()?;
    let api_endpoint = config::settings::Settings::get_api_endpoint();
    println!("Using API endpoint: {}", api_endpoint);
    let api = api::CookApi::new(api_endpoint)?;
    let auth = auth::AuthManager::new(config.paths(), Arc::new(api))?;

    // Perform browser-based login
    match auth.browser_login().await {
        Ok(()) => {
            println!("Successfully authenticated!");
            if let Some(session) = auth.get_session() {
                println!("Logged in as: {}", session.email.unwrap_or(session.user_id));
            }
        }
        Err(e) => {
            error!("Authentication failed: {e}");
            return Err(e);
        }
    }

    Ok(())
}

async fn logout() -> Result<()> {
    let config = config::Config::new()?;
    let api = api::CookApi::new(config::settings::Settings::get_api_endpoint())?;
    let auth = auth::AuthManager::new(config.paths(), Arc::new(api))?;

    // Clear the auth session first
    auth.logout()?;

    // If daemon is running, we need to inform it to stop syncing
    // For now, the simplest approach is to restart the daemon
    // In the future, we should implement IPC to notify the running daemon
    if daemon::is_already_running(&config) {
        println!("Stopping sync agent to apply logout...");
        stop_daemon()?;

        // Give it a moment to properly shutdown
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        println!("Restarting sync agent without authentication...");
        start_daemon().await?;
    }

    println!("Logged out successfully");
    Ok(())
}

async fn configure(
    recipes_dir: Option<String>,
    auto_start: Option<bool>,
    auto_update: Option<bool>,
    show: bool,
) -> Result<()> {
    let config = config::Config::new()?;

    if show {
        let settings_mutex = config.settings();
        let settings = settings_mutex.lock().unwrap();
        println!("Current configuration:");
        println!(
            "  Recipes directory: {}",
            settings
                .recipes_dir
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "Not configured".to_string())
        );
        println!("  Auto-start: {}", settings.auto_start);
        println!("  Auto-update: {}", settings.auto_update);
        println!(
            "  API endpoint: {}",
            config::settings::Settings::get_api_endpoint()
        );
        println!(
            "  Sync endpoint: {}",
            config::settings::Settings::get_sync_endpoint()
        );
        return Ok(());
    }

    let mut changed = false;

    if let Some(dir) = recipes_dir {
        let path = std::path::PathBuf::from(&dir);
        if !path.exists() {
            return Err(error::SyncError::InvalidConfiguration(format!(
                "Directory does not exist: {dir}"
            )));
        }
        config.update_settings(|s| {
            s.recipes_dir = Some(path);
        })?;
        println!("Recipes directory set to: {dir}");
        changed = true;
    }

    if let Some(enabled) = auto_start {
        config.update_settings(|s| {
            s.auto_start = enabled;
        })?;

        // Update platform auto-start
        let platform = platform::get_platform();
        let exe_path = std::env::current_exe()?
            .to_str()
            .ok_or_else(|| error::SyncError::Other("Invalid executable path".to_string()))?
            .to_string();

        if enabled {
            platform.enable_auto_start("cook-sync", &exe_path)?;
            println!("Auto-start enabled");
        } else {
            platform.disable_auto_start("cook-sync")?;
            println!("Auto-start disabled");
        }
        changed = true;
    }

    if let Some(enabled) = auto_update {
        config.update_settings(|s| {
            s.auto_update = enabled;
        })?;
        println!(
            "Auto-update {}",
            if enabled { "enabled" } else { "disabled" }
        );
        changed = true;
    }

    if !changed {
        println!("No changes made. Use --show to see current configuration.");
    }

    Ok(())
}

async fn check_update() -> Result<()> {
    println!("Checking for updates...");

    let update_manager = updates::UpdateManager::new().map_err(|e| {
        error::SyncError::Update(format!("Failed to initialize update manager: {}", e))
    })?;

    match update_manager.check_for_updates().await {
        Ok(true) => {
            println!("Update available! Starting installation...");
            update_manager.install_update().await.map_err(|e| {
                error::SyncError::Update(format!("Failed to install update: {}", e))
            })?;
            println!("Update installation completed successfully.");
        }
        Ok(false) => {
            println!("You are running the latest version.");
        }
        Err(e) => {
            error!("Failed to check for updates: {}", e);
            return Err(error::SyncError::Update(format!(
                "Failed to check for updates: {}",
                e
            )));
        }
    }

    Ok(())
}

fn install_integration() -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        if !appimage_integration::is_running_from_appimage() {
            println!("This command is only available when running from an AppImage");
            println!("For system-wide installation, use the .deb or .rpm package instead");
            return Ok(());
        }

        if appimage_integration::is_already_integrated() {
            println!("Cook Sync is already integrated into your applications menu");
            return Ok(());
        }

        println!("Installing Cook Sync into your applications menu...");
        appimage_integration::integrate_appimage()?;
        println!("✓ Cook Sync has been added to your applications menu");
        println!("You can now find it by searching for 'Cook Sync' in your launcher");
        Ok(())
    }

    #[cfg(target_os = "windows")]
    {
        if windows_integration::is_already_integrated() {
            println!("Cook Sync is already integrated into your Start Menu");
            return Ok(());
        }

        println!("Installing Cook Sync shortcuts...");
        println!("Do you want to create a Desktop shortcut? (y/N): ");

        let mut input = String::new();
        std::io::stdin().read_line(&mut input).ok();

        if input.trim().to_lowercase() == "y" {
            windows_integration::integrate_windows_with_desktop()?;
            println!("✓ Cook Sync has been added to your Start Menu and Desktop");
        } else {
            windows_integration::integrate_windows()?;
            println!("✓ Cook Sync has been added to your Start Menu");
        }

        println!("You can now find it in your Start Menu");
        Ok(())
    }

    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        println!("Desktop integration is not available on this platform");
        Ok(())
    }
}

fn uninstall_integration() -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        if !appimage_integration::is_running_from_appimage() {
            println!("This command is only available when running from an AppImage");
            return Ok(());
        }

        if !appimage_integration::is_already_integrated() {
            println!("Cook Sync is not currently integrated");
            return Ok(());
        }

        println!("Removing Cook Sync from your applications menu...");
        appimage_integration::unintegrate_appimage()?;
        println!("✓ Cook Sync has been removed from your applications menu");
        Ok(())
    }

    #[cfg(target_os = "windows")]
    {
        if !windows_integration::is_already_integrated() {
            println!("Cook Sync is not currently integrated");
            return Ok(());
        }

        println!("Removing Cook Sync shortcuts...");
        windows_integration::unintegrate_windows()?;
        println!("✓ Cook Sync has been removed from your Start Menu and Desktop");
        Ok(())
    }

    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        println!("Desktop integration is not available on this platform");
        Ok(())
    }
}
