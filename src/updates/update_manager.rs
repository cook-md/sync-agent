use anyhow::Result;
use log::info;

use super::axo_backend::AxoUpdaterBackend;
#[cfg(target_os = "macos")]
use super::sparkle_backend::SparkleBackend;
#[cfg(target_os = "linux")]
use super::appimage_backend::AppImageBackend;

pub enum UpdateBackend {
    AxoUpdater(AxoUpdaterBackend),
    #[cfg(target_os = "macos")]
    Sparkle(SparkleBackend),
    #[cfg(target_os = "linux")]
    AppImageUpdate(AppImageBackend),
}

pub struct UpdateManager {
    backend: UpdateBackend,
}

impl UpdateManager {
    pub fn new() -> Result<Self> {
        let backend = Self::detect_backend()?;
        Ok(Self { backend })
    }

    fn detect_backend() -> Result<UpdateBackend> {
        // Linux: Check if running as AppImage
        #[cfg(target_os = "linux")]
        if std::env::var("APPIMAGE").is_ok() {
            info!("Detected AppImage, using AppImageUpdate backend");
            return Ok(UpdateBackend::AppImageUpdate(
                AppImageBackend::new()?
            ));
        }

        // macOS: Check if running from .app bundle
        #[cfg(target_os = "macos")]
        if let Ok(exe) = std::env::current_exe() {
            if exe.to_string_lossy().contains(".app/Contents/MacOS/") {
                info!("Detected .app bundle, attempting to use Sparkle backend");

                // Try to initialize Sparkle, but fall back to axoupdater if it's not available
                match SparkleBackend::new() {
                    Ok(backend) => {
                        // Check if Sparkle was actually initialized (not just a stub)
                        // We can tell by trying to check - if it returns false with a warning,
                        // it means Sparkle framework isn't available
                        info!("Sparkle backend initialized successfully");
                        return Ok(UpdateBackend::Sparkle(backend));
                    }
                    Err(e) => {
                        info!("Sparkle not available ({}), falling back to axoupdater", e);
                        // Fall through to use axoupdater
                    }
                }
            }
        }

        // Default: axoupdater (shell, PowerShell, MSI, Homebrew)
        info!("Using axoupdater backend");
        Ok(UpdateBackend::AxoUpdater(
            AxoUpdaterBackend::new()?
        ))
    }

    pub async fn check_for_updates(&self) -> Result<bool> {
        match &self.backend {
            UpdateBackend::AxoUpdater(backend) => backend.check().await,
            #[cfg(target_os = "macos")]
            UpdateBackend::Sparkle(backend) => backend.check().await,
            #[cfg(target_os = "linux")]
            UpdateBackend::AppImageUpdate(backend) => backend.check().await,
        }
    }

    pub async fn install_update(&self) -> Result<()> {
        match &self.backend {
            UpdateBackend::AxoUpdater(backend) => backend.install().await,
            #[cfg(target_os = "macos")]
            UpdateBackend::Sparkle(backend) => backend.install().await,
            #[cfg(target_os = "linux")]
            UpdateBackend::AppImageUpdate(backend) => backend.install().await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_update_manager_creation() {
        // Should not panic
        let _manager = UpdateManager::new();
    }
}
