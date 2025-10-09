use crate::error::Result;
use log::{debug, error};
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

/// Real keyring implementation using the system keyring
pub struct SystemKeyring;

impl KeyringStore for SystemKeyring {
    fn get_password(&self, service: &str, key: &str) -> Result<Option<String>> {
        debug!("Getting password from keyring: service={service}, key={key}");
        let entry = keyring::Entry::new(service, key)?;
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
                error!("Error getting password from keyring: {e}");
                Err(e.into())
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
