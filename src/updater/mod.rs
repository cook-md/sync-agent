// Auto-update module using cargo-packager-updater
// This module handles checking for and installing updates from GitHub Releases

use crate::error::{Result, SyncError};
use cargo_packager_updater::{check_update, semver::Version, url::Url, Config};
use log::{error, info, warn};

// Public key for signature verification (embedded at compile time)
// This key is generated using: cargo packager signer generate
// To set this at build time, use: CARGO_PACKAGER_PUBLIC_KEY="your_key_here"
const PUBLIC_KEY: &str = env!("CARGO_PACKAGER_PUBLIC_KEY");

// GitHub Releases manifest URL
// The manifest.json file contains version information and download URLs for all platforms
const MANIFEST_URL: &str =
    "https://github.com/cook-md/sync-agent/releases/latest/download/manifest.json";

/// Check for updates and optionally install them
///
/// # Arguments
/// * `auto_install` - If true, automatically download and install updates
///
/// # Returns
/// * `Ok(Some(version))` - Update is available (and installed if auto_install=true)
/// * `Ok(None)` - No update available
/// * `Err(_)` - Error occurred during update check
pub async fn check_for_updates(auto_install: bool) -> Result<Option<String>> {
    let current_version = env!("CARGO_PKG_VERSION")
        .parse::<Version>()
        .map_err(|e| SyncError::Other(format!("Invalid version: {}", e)))?;

    info!(
        "Checking for updates (current version: {})",
        current_version
    );

    let config = Config {
        endpoints: vec![Url::parse(MANIFEST_URL)
            .map_err(|e| SyncError::Other(format!("Invalid manifest URL: {}", e)))?],
        pubkey: PUBLIC_KEY.to_string(),
        ..Default::default()
    };

    // Run the blocking update check in a separate thread to avoid runtime conflicts
    let current_version_clone = current_version.clone();
    let update_result =
        tokio::task::spawn_blocking(move || check_update(current_version_clone, config))
            .await
            .map_err(|e| SyncError::Other(format!("Update check task failed: {}", e)))?;

    match update_result {
        Ok(Some(update)) => {
            let version_string = update.version.to_string();
            info!(
                "Update available: {} -> {}",
                current_version, version_string
            );

            if auto_install {
                info!("Auto-install enabled, downloading and installing update...");

                // Use download_and_install() on all platforms:
                // - macOS: replaces .app bundle atomically (expects tar.gz of .app)
                // - Linux: replaces AppImage binary in-place
                // - Windows: launches NSIS installer
                let install_result =
                    tokio::task::spawn_blocking(move || update.download_and_install())
                        .await
                        .map_err(|e| SyncError::Other(format!("Install task failed: {}", e)))?;

                match install_result {
                    Ok(()) => {
                        info!("Update downloaded and installed successfully");
                        Ok(Some(version_string))
                    }
                    Err(e) => {
                        error!("Failed to download/install update: {}", e);
                        Err(SyncError::Update(format!("Update failed: {}", e)))
                    }
                }
            } else {
                info!("Update available but auto-install disabled");
                Ok(Some(version_string))
            }
        }
        Ok(None) => {
            info!("No updates available");
            Ok(None)
        }
        Err(e) => {
            warn!("Update check failed: {}", e);
            Err(SyncError::Update(format!("Update check failed: {}", e)))
        }
    }
}

/// Restart the application after a successful update.
/// This function does not return on success.
pub fn restart_app() -> ! {
    info!("Restarting application after update...");

    #[cfg(target_os = "macos")]
    {
        // Navigate from binary inside .app/Contents/MacOS/cook-sync up to .app bundle
        let bundle_path = std::env::current_exe().ok().and_then(|p| {
            p.parent() // MacOS/
                .and_then(|p| p.parent()) // Contents/
                .and_then(|p| p.parent()) // .app
                .map(|p| p.to_path_buf())
        });

        if let Some(bundle) = &bundle_path {
            info!("Relaunching app bundle: {:?}", bundle);
            let _ = std::process::Command::new("open")
                .arg("-n")
                .arg(bundle)
                .arg("--args")
                .arg("start")
                .spawn();
        } else {
            error!("Could not determine app bundle path for restart");
        }
        std::process::exit(0);
    }

    #[cfg(target_os = "linux")]
    {
        use std::os::unix::process::CommandExt;
        // On Linux AppImage, $APPIMAGE points to the AppImage file.
        // After update, the binary has been replaced in-place, so $APPIMAGE is correct.
        let exe = std::env::var("APPIMAGE")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| std::env::current_exe().unwrap_or_default());

        info!("Restarting via exec: {:?}", exe);
        let err = std::process::Command::new(&exe).args(["start"]).exec();
        // exec() only returns on error
        error!("Failed to restart: {}", err);
        std::process::exit(1);
    }

    #[cfg(target_os = "windows")]
    {
        // On Windows, the NSIS installer typically handles restart.
        // As a fallback, spawn new process and exit.
        let exe = std::env::current_exe().unwrap_or_default();
        info!("Restarting via spawn: {:?}", exe);
        let _ = std::process::Command::new(&exe).args(["start"]).spawn();
        std::process::exit(0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_parsing() {
        let version = env!("CARGO_PKG_VERSION").parse::<Version>();
        assert!(version.is_ok(), "Current version should be valid semver");
    }

    #[test]
    fn test_manifest_url_valid() {
        let url = Url::parse(MANIFEST_URL);
        assert!(url.is_ok(), "Manifest URL should be valid");
    }
}
