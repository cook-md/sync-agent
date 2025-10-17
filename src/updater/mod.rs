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
    "https://github.com/cooklang/sync-agent/releases/latest/download/manifest.json";

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

    match check_update(current_version.clone(), config) {
        Ok(Some(update)) => {
            info!(
                "Update available: {} -> {}",
                current_version, update.version
            );

            if auto_install {
                info!("Auto-install enabled, downloading and installing update...");

                match update.download_and_install() {
                    Ok(()) => {
                        info!("Update downloaded and installed successfully");
                        // Note: The updater will restart the application automatically
                        Ok(Some(update.version.to_string()))
                    }
                    Err(e) => {
                        error!("Failed to download/install update: {}", e);
                        Err(SyncError::Update(format!("Update failed: {}", e)))
                    }
                }
            } else {
                info!("Update available but auto-install disabled");
                Ok(Some(update.version.to_string()))
            }
        }
        Ok(None) => {
            info!("No updates available");
            Ok(None)
        }
        Err(e) => {
            warn!("Update check failed: {}", e);
            // Don't fail the app if update check fails, just log it
            Ok(None)
        }
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
