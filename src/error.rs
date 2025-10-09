use thiserror::Error;

#[derive(Error, Debug)]
pub enum SyncError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("JWT error: {0}")]
    Jwt(#[from] jsonwebtoken::errors::Error),

    #[error("Sync client error: {0}")]
    #[allow(dead_code)]
    SyncClient(String),

    #[error("Authentication required")]
    AuthenticationRequired,

    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),

    #[error("Tray error: {0}")]
    Tray(String),

    #[error("Update error: {0}")]
    Update(String),

    #[error("Platform error: {0}")]
    Platform(String),

    #[error("Keyring error: {0}")]
    Keyring(#[from] keyring::Error),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, SyncError>;
