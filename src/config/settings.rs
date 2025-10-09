use crate::config::constants;
use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub recipes_dir: Option<PathBuf>,
    pub sync_interval_secs: u64,
    pub auto_start: bool,
    pub auto_update: bool,
    pub show_notifications: bool,
    #[serde(default)]
    pub update_settings: UpdateSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateSettings {
    pub check_interval_hours: u32,
    pub auto_download: bool,
    pub auto_install: bool,
    pub show_release_notes: bool,
    pub skip_versions: Vec<String>,
}

impl Default for UpdateSettings {
    fn default() -> Self {
        Self {
            check_interval_hours: 24,
            auto_download: true,
            auto_install: false, // Require user confirmation by default
            show_release_notes: true,
            skip_versions: Vec::new(),
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            recipes_dir: None,
            sync_interval_secs: 12,
            auto_start: true,
            auto_update: true,
            show_notifications: true,
            update_settings: UpdateSettings::default(),
        }
    }
}

impl Settings {
    pub fn load(path: &PathBuf) -> Result<Self> {
        let settings = if path.exists() {
            let content = std::fs::read_to_string(path)?;
            serde_json::from_str(&content)?
        } else {
            Self::default()
        };

        Ok(settings)
    }

    /// Get the API endpoint, with support for environment variable override
    pub fn get_api_endpoint() -> String {
        // Allow environment variable override for development/testing
        if let Ok(api_endpoint) = std::env::var("COOK_API_ENDPOINT") {
            return api_endpoint.trim_end_matches('/').to_string();
        }

        // Legacy environment variable support
        if let Ok(base_endpoint) = std::env::var("COOK_ENDPOINT") {
            return format!("{}/api", base_endpoint.trim_end_matches('/'));
        }

        constants::endpoints::API.to_string()
    }

    /// Get the sync endpoint, with support for environment variable override
    pub fn get_sync_endpoint() -> String {
        // Allow environment variable override for development/testing
        if let Ok(sync_endpoint) = std::env::var("COOK_SYNC_ENDPOINT") {
            return sync_endpoint.trim_end_matches('/').to_string();
        }

        // Legacy environment variable support
        // Note: In production, the sync server is the same as the API endpoint
        // Only in local development do we use a separate sync server on port 8000
        if let Ok(base_endpoint) = std::env::var("COOK_ENDPOINT") {
            let base = base_endpoint.trim_end_matches('/');
            if base.contains("localhost") || base.contains("127.0.0.1") {
                // For local development, use the separate sync server
                return "http://127.0.0.1:8000".to_string();
            } else {
                // For production/staging, sync server is at the API endpoint
                return format!("{base}/api");
            }
        }

        constants::endpoints::SYNC.to_string()
    }

    pub fn save(&self, path: &PathBuf) -> Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn validate(&self) -> Result<()> {
        if let Some(ref dir) = self.recipes_dir {
            if !dir.exists() {
                return Err(crate::error::SyncError::InvalidConfiguration(format!(
                    "Recipes directory does not exist: {}",
                    dir.display()
                )));
            }
        }

        if self.sync_interval_secs < 5 {
            return Err(crate::error::SyncError::InvalidConfiguration(
                "Sync interval must be at least 5 seconds".to_string(),
            ));
        }

        Ok(())
    }
}
