pub mod constants;
pub mod paths;
pub mod settings;

use crate::error::Result;
use std::sync::{Arc, Mutex};

pub use paths::AppPaths;
pub use settings::Settings;

pub struct Config {
    paths: AppPaths,
    settings: Arc<Mutex<Settings>>,
}

impl Config {
    pub fn new() -> Result<Self> {
        let paths = AppPaths::new()?;
        let settings = Settings::load(&paths.settings_file)?;

        Ok(Config {
            paths,
            settings: Arc::new(Mutex::new(settings)),
        })
    }

    pub fn paths(&self) -> Arc<AppPaths> {
        Arc::new(self.paths.clone())
    }

    pub fn settings(&self) -> Arc<Mutex<Settings>> {
        Arc::clone(&self.settings)
    }

    pub fn update_settings<F>(&self, updater: F) -> Result<()>
    where
        F: FnOnce(&mut Settings),
    {
        let mut settings = self.settings.lock().unwrap();
        updater(&mut settings);
        settings.save(&self.paths.settings_file)?;
        Ok(())
    }
}
