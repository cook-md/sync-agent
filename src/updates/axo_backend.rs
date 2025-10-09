use anyhow::Result;
use axoupdater::AxoUpdater;
use log::{info, warn};

pub struct AxoUpdaterBackend;

impl AxoUpdaterBackend {
    pub fn new() -> Result<Self> {
        Ok(Self)
    }

    pub async fn check(&self) -> Result<bool> {
        info!("Checking for updates using axoupdater...");

        match AxoUpdater::new_for("cook-sync")
            .load_receipt()
            .and_then(|updater| updater.is_update_needed_sync())
        {
            Ok(true) => {
                info!("Update available");
                Ok(true)
            }
            Ok(false) => {
                info!("No updates available");
                Ok(false)
            }
            Err(e) => {
                warn!("Failed to check for updates: {}", e);
                Ok(false)
            }
        }
    }

    pub async fn install(&self) -> Result<()> {
        info!("Installing update using axoupdater...");

        match AxoUpdater::new_for("cook-sync")
            .load_receipt()
            .and_then(|updater| updater.run_sync())
        {
            Ok(Some(_result)) => {
                info!("Update installed successfully");
                Ok(())
            }
            Ok(None) => {
                info!("Already up to date");
                Ok(())
            }
            Err(e) => {
                anyhow::bail!("Failed to install update: {}", e)
            }
        }
    }
}
