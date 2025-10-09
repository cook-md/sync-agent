use crate::error::{Result, SyncError};
use base64::{engine::general_purpose, Engine as _};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

#[cfg(test)]
#[path = "jwt_test.rs"]
mod jwt_test;

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum UserId {
    Integer(i64),
    String(String),
}

impl UserId {
    fn as_string(&self) -> String {
        match self {
            UserId::Integer(id) => id.to_string(),
            UserId::String(id) => id.clone(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub uid: UserId, // User ID (can be integer or string for compatibility)
    pub exp: i64,    // Expiration time
    #[serde(default)]
    pub iat: i64, // Issued at (optional for backward compatibility)
    pub email: Option<String>,
}

pub struct JwtToken {
    #[allow(dead_code)]
    pub token: String,
    pub claims: Claims,
}

impl JwtToken {
    pub fn from_string(token: String) -> Result<Self> {
        // Extract claims without verification for now (similar to desktop app)
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() != 3 {
            return Err(SyncError::Jwt(jsonwebtoken::errors::Error::from(
                jsonwebtoken::errors::ErrorKind::InvalidToken,
            )));
        }

        let decoded = general_purpose::STANDARD_NO_PAD
            .decode(parts[1])
            .map_err(|_| SyncError::Other("Failed to decode JWT".to_string()))?;

        let claims: Claims = serde_json::from_slice(&decoded)?;

        Ok(JwtToken { token, claims })
    }

    pub fn is_expired(&self) -> bool {
        let now = Utc::now().timestamp();
        self.claims.exp <= now
    }

    pub fn expires_in(&self) -> Duration {
        let exp_time = DateTime::<Utc>::from_timestamp(self.claims.exp, 0).unwrap_or_else(Utc::now);
        exp_time - Utc::now()
    }

    pub fn should_refresh(&self) -> bool {
        // Refresh if less than 1 hour remaining
        self.expires_in() < Duration::hours(1)
    }

    pub fn user_id(&self) -> String {
        self.claims.uid.as_string()
    }
}
