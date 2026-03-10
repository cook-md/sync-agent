use crate::error::Result;
use log::debug;
#[cfg(not(target_os = "linux"))]
use log::error;
#[cfg(test)]
use std::collections::HashMap;
#[cfg(test)]
use std::sync::Mutex;

/// Trait for keyring operations - allows mocking in tests
pub trait KeyringStore: Send + Sync {
    fn get_password(&self, service: &str, key: &str) -> Result<Option<String>>;
    fn set_password(&self, service: &str, key: &str, password: &str) -> Result<()>;
    fn delete_password(&self, service: &str, key: &str) -> Result<()>;
}

/// Real keyring implementation using the system keyring (macOS/Windows only)
#[cfg(not(target_os = "linux"))]
pub struct SystemKeyring;

#[cfg(not(target_os = "linux"))]
impl KeyringStore for SystemKeyring {
    fn get_password(&self, service: &str, key: &str) -> Result<Option<String>> {
        debug!("Getting password from keyring: service={service}, key={key}");
        let entry = match keyring::Entry::new(service, key) {
            Ok(entry) => entry,
            Err(e) => {
                error!("Keyring backend unavailable (service={service}, key={key}): {e}");
                return Ok(None);
            }
        };
        match entry.get_password() {
            Ok(password) => {
                debug!("Password found in keyring for key: {key}");
                Ok(Some(password))
            }
            Err(keyring::Error::NoEntry) => {
                debug!("No entry found in keyring for key: {key}");
                Ok(None)
            }
            Err(e) => {
                error!("Failed to read keyring (service={service}, key={key}): {e}");
                Ok(None)
            }
        }
    }

    fn set_password(&self, service: &str, key: &str, password: &str) -> Result<()> {
        debug!("Setting password in keyring: service={service}, key={key}");
        let entry = keyring::Entry::new(service, key)?;
        match entry.set_password(password) {
            Ok(_) => {
                debug!("Password set successfully in keyring for key: {key}");
                Ok(())
            }
            Err(e) => {
                error!("Error setting password in keyring: {e}");
                Err(e.into())
            }
        }
    }

    fn delete_password(&self, service: &str, key: &str) -> Result<()> {
        debug!("Deleting password from keyring: service={service}, key={key}");
        let entry = keyring::Entry::new(service, key)?;
        match entry.delete_credential() {
            Ok(_) => {
                debug!("Password deleted successfully from keyring for key: {key}");
                Ok(())
            }
            Err(keyring::Error::NoEntry) => {
                debug!("No entry to delete in keyring for key: {key}");
                Ok(())
            }
            Err(e) => {
                error!("Error deleting password from keyring: {e}");
                Err(e.into())
            }
        }
    }
}

/// File-based session storage for Linux (avoids keyring/Secret Service password prompts)
#[cfg(target_os = "linux")]
pub struct FileStore {
    store_path: std::path::PathBuf,
}

#[cfg(target_os = "linux")]
impl FileStore {
    pub fn new() -> Self {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
            .join("cook-sync");

        // Ensure directory exists
        let _ = std::fs::create_dir_all(&config_dir);

        Self {
            store_path: config_dir.join("session-store.json"),
        }
    }

    fn read_store(&self) -> std::collections::HashMap<String, String> {
        match std::fs::read_to_string(&self.store_path) {
            Ok(contents) => serde_json::from_str(&contents).unwrap_or_default(),
            Err(_) => std::collections::HashMap::new(),
        }
    }

    fn write_store(&self, store: &std::collections::HashMap<String, String>) -> Result<()> {
        let contents = serde_json::to_string(store)?;
        std::fs::write(&self.store_path, &contents)?;

        // Set file permissions to 600 (owner read/write only)
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o600);
        std::fs::set_permissions(&self.store_path, perms)?;

        Ok(())
    }

    fn make_key(service: &str, key: &str) -> String {
        format!("{service}:{key}")
    }
}

#[cfg(target_os = "linux")]
impl KeyringStore for FileStore {
    fn get_password(&self, service: &str, key: &str) -> Result<Option<String>> {
        let store_key = Self::make_key(service, key);
        debug!("Getting password from file store: {store_key}");
        let store = self.read_store();
        match store.get(&store_key) {
            Some(value) => {
                debug!("Password found in file store for key: {store_key}");
                Ok(Some(value.clone()))
            }
            None => {
                debug!("No entry found in file store for key: {store_key}");
                Ok(None)
            }
        }
    }

    fn set_password(&self, service: &str, key: &str, password: &str) -> Result<()> {
        let store_key = Self::make_key(service, key);
        debug!("Setting password in file store: {store_key}");
        let mut store = self.read_store();
        store.insert(store_key, password.to_string());
        self.write_store(&store)?;
        debug!("Password set successfully in file store");
        Ok(())
    }

    fn delete_password(&self, service: &str, key: &str) -> Result<()> {
        let store_key = Self::make_key(service, key);
        debug!("Deleting password from file store: {store_key}");
        let mut store = self.read_store();
        store.remove(&store_key);
        if store.is_empty() {
            // Remove the file entirely if no more entries
            let _ = std::fs::remove_file(&self.store_path);
        } else {
            self.write_store(&store)?;
        }
        debug!("Password deleted from file store");
        Ok(())
    }
}

/// Returns the appropriate store for the current platform
#[cfg(not(target_os = "linux"))]
pub fn default_store() -> SystemKeyring {
    SystemKeyring
}

#[cfg(target_os = "linux")]
pub fn default_store() -> FileStore {
    FileStore::new()
}

/// Mock keyring for testing - stores passwords in memory
#[cfg(test)]
pub struct MockKeyring {
    store: Mutex<HashMap<(String, String), String>>,
}

#[cfg(test)]
impl MockKeyring {
    pub fn new() -> Self {
        Self::default()
    }
}

#[cfg(test)]
impl Default for MockKeyring {
    fn default() -> Self {
        Self {
            store: Mutex::new(HashMap::new()),
        }
    }
}

#[cfg(test)]
impl KeyringStore for MockKeyring {
    fn get_password(&self, service: &str, key: &str) -> Result<Option<String>> {
        let store = self.store.lock().unwrap();
        Ok(store.get(&(service.to_string(), key.to_string())).cloned())
    }

    fn set_password(&self, service: &str, key: &str, password: &str) -> Result<()> {
        let mut store = self.store.lock().unwrap();
        store.insert((service.to_string(), key.to_string()), password.to_string());
        Ok(())
    }

    fn delete_password(&self, service: &str, key: &str) -> Result<()> {
        let mut store = self.store.lock().unwrap();
        store.remove(&(service.to_string(), key.to_string()));
        Ok(())
    }
}
