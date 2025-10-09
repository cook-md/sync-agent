#[cfg(target_os = "linux")]
use anyhow::{Context, Result};
#[cfg(target_os = "linux")]
use log::{info, warn};
#[cfg(target_os = "linux")]
use std::path::PathBuf;
#[cfg(target_os = "linux")]
use std::process::Command;

#[cfg(target_os = "linux")]
pub struct AppImageBackend {
    appimage_path: PathBuf,
}

#[cfg(target_os = "linux")]
impl AppImageBackend {
    pub fn new() -> Result<Self> {
        let appimage_path = std::env::var("APPIMAGE")
            .context("APPIMAGE environment variable not set")?;

        let path = PathBuf::from(appimage_path);

        if !path.exists() {
            anyhow::bail!("AppImage path does not exist: {:?}", path);
        }

        Ok(Self { appimage_path: path })
    }

    pub async fn check(&self) -> Result<bool> {
        info!("Checking for updates using AppImageUpdate...");

        // Use appimageupdatetool to check for updates
        let output = Command::new("appimageupdatetool")
            .arg("--check-for-update")
            .arg(&self.appimage_path)
            .output()
            .context("Failed to run appimageupdatetool. Is it installed?")?;

        // Exit code 1 = update available
        // Exit code 0 = no update available
        let update_available = output.status.code() == Some(1);

        if update_available {
            info!("AppImage update available");
        } else {
            info!("No AppImage updates available");
        }

        Ok(update_available)
    }

    pub async fn install(&self) -> Result<()> {
        info!("Starting AppImage update...");

        // Run appimageupdatetool to update
        let status = Command::new("appimageupdatetool")
            .arg(&self.appimage_path)
            .status()
            .context("Failed to run appimageupdatetool. Is it installed?")?;

        if !status.success() {
            anyhow::bail!("AppImage update failed");
        }

        info!("AppImage updated successfully. Restart required.");

        // AppImage update replaces the file, restart needed
        // Signal to user or auto-restart

        Ok(())
    }
}

// Stub implementation for non-Linux platforms
#[allow(dead_code)]
#[cfg(not(target_os = "linux"))]
pub struct AppImageBackend;

#[allow(dead_code)]
#[cfg(not(target_os = "linux"))]
impl AppImageBackend {
    pub fn new() -> anyhow::Result<Self> {
        anyhow::bail!("AppImage backend is only available on Linux")
    }

    pub async fn check(&self) -> anyhow::Result<bool> {
        Ok(false)
    }

    pub async fn install(&self) -> anyhow::Result<()> {
        Ok(())
    }
}
