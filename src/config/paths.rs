use crate::error::{Result, SyncError};
use dirs;
use std::path::PathBuf;

#[derive(Clone)]
pub struct AppPaths {
    #[allow(dead_code)]
    pub config_dir: PathBuf,
    #[allow(dead_code)]
    pub data_dir: PathBuf,
    pub settings_file: PathBuf,
    #[allow(dead_code)]
    pub session_file: PathBuf,
    #[allow(dead_code)]
    pub profile_file: PathBuf,
    pub database_file: PathBuf,
    #[allow(dead_code)]
    pub updates_file: PathBuf,
    pub log_file: PathBuf,
    pub pid_file: PathBuf,
}

impl AppPaths {
    pub fn new() -> Result<Self> {
        let app_name = "cook-sync";

        let config_dir = dirs::config_dir()
            .ok_or_else(|| {
                SyncError::InvalidConfiguration("Could not determine config directory".to_string())
            })?
            .join(app_name);

        let data_dir = dirs::data_local_dir()
            .ok_or_else(|| {
                SyncError::InvalidConfiguration("Could not determine data directory".to_string())
            })?
            .join(app_name);

        let cache_dir = dirs::cache_dir()
            .ok_or_else(|| {
                SyncError::InvalidConfiguration("Could not determine cache directory".to_string())
            })?
            .join(app_name);

        // Create directories if they don't exist
        std::fs::create_dir_all(&config_dir)?;
        std::fs::create_dir_all(&data_dir)?;
        std::fs::create_dir_all(&cache_dir)?;

        Ok(AppPaths {
            settings_file: config_dir.join("settings.json"),
            session_file: config_dir.join("session.json"),
            profile_file: config_dir.join("profile.json"),
            database_file: data_dir.join("sync.db"),
            updates_file: data_dir.join("updates.json"),
            log_file: cache_dir.join("cook-sync.log"),
            pid_file: cache_dir.join("cook-sync.pid"),
            config_dir,
            data_dir,
        })
    }

    #[allow(dead_code)]
    pub fn recipes_dir(&self) -> Option<PathBuf> {
        dirs::document_dir().map(|d| d.join("CookRecipes"))
    }
}
