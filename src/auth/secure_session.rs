use super::jwt::JwtToken;
use crate::error::Result;
use log::{error, info};
use serde::{Deserialize, Serialize};

mod keyring_store;
use keyring_store::{KeyringStore, SystemKeyring};

#[cfg(test)]
use keyring_store::MockKeyring;

#[cfg(test)]
#[path = "secure_session_test.rs"]
mod secure_session_test;

const SERVICE_NAME: &str = "cook.md-sync-agent";
const JWT_KEY: &str = "jwt_token";
const USER_ID_KEY: &str = "user_id";
const EMAIL_KEY: &str = "user_email";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecureSession {
    pub jwt: String,
    pub user_id: String,
    pub email: Option<String>,
}

impl SecureSession {
    pub fn new(jwt_string: String) -> Result<Self> {
        let jwt = JwtToken::from_string(jwt_string.clone())?;

        Ok(SecureSession {
            jwt: jwt_string,
            user_id: jwt.user_id(),
            email: jwt.claims.email.clone(),
        })
    }

    pub fn load() -> Result<Option<Self>> {
        info!("Loading session from keyring");
        let result = Self::load_with_store(&SystemKeyring);
        match &result {
            Ok(Some(_)) => info!("Session loaded successfully from keyring"),
            Ok(None) => info!("No session found in keyring"),
            Err(e) => error!("Failed to load session from keyring: {e}"),
        }
        result
    }

    fn load_with_store(store: &dyn KeyringStore) -> Result<Option<Self>> {
        // Try to load JWT from keyring
        let jwt = match store.get_password(SERVICE_NAME, JWT_KEY)? {
            Some(jwt) => jwt,
            None => return Ok(None),
        };

        // Validate JWT is not expired
        let jwt_token = match JwtToken::from_string(jwt.clone()) {
            Ok(token) => token,
            Err(_) => {
                // Invalid JWT, clean up
                let _ = Self::delete_with_store(store);
                return Ok(None);
            }
        };

        if jwt_token.is_expired() {
            // Clean up expired token
            let _ = Self::delete_with_store(store);
            return Ok(None);
        }

        // Load user_id
        let user_id = store
            .get_password(SERVICE_NAME, USER_ID_KEY)?
            .unwrap_or_else(|| jwt_token.user_id());

        // Load email (optional)
        let email = store.get_password(SERVICE_NAME, EMAIL_KEY)?;

        Ok(Some(SecureSession {
            jwt,
            user_id,
            email,
        }))
    }

    pub fn save(&self) -> Result<()> {
        info!("Saving session to keyring");
        let result = self.save_with_store(&SystemKeyring);
        match &result {
            Ok(_) => info!("Session saved successfully to keyring"),
            Err(e) => error!("Failed to save session to keyring: {e}"),
        }
        result
    }

    fn save_with_store(&self, store: &dyn KeyringStore) -> Result<()> {
        // Save JWT
        store.set_password(SERVICE_NAME, JWT_KEY, &self.jwt)?;

        // Save user_id
        store.set_password(SERVICE_NAME, USER_ID_KEY, &self.user_id)?;

        // Save email if present
        if let Some(email) = &self.email {
            store.set_password(SERVICE_NAME, EMAIL_KEY, email)?;
        }

        Ok(())
    }

    pub fn delete() -> Result<()> {
        Self::delete_with_store(&SystemKeyring)
    }

    fn delete_with_store(store: &dyn KeyringStore) -> Result<()> {
        // Delete JWT
        store.delete_password(SERVICE_NAME, JWT_KEY)?;

        // Delete user_id
        store.delete_password(SERVICE_NAME, USER_ID_KEY)?;

        // Delete email
        store.delete_password(SERVICE_NAME, EMAIL_KEY)?;

        Ok(())
    }

    pub fn jwt_token(&self) -> Result<JwtToken> {
        JwtToken::from_string(self.jwt.clone())
    }
}

// Test helpers
#[cfg(test)]
impl SecureSession {
    pub fn load_with_mock(mock: &MockKeyring) -> Result<Option<Self>> {
        Self::load_with_store(mock)
    }

    pub fn save_with_mock(&self, mock: &MockKeyring) -> Result<()> {
        self.save_with_store(mock)
    }

    pub fn delete_with_mock(mock: &MockKeyring) -> Result<()> {
        Self::delete_with_store(mock)
    }
}
