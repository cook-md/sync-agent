use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct RefreshTokenRequest {
    pub token: String,
}

#[derive(Debug, Deserialize)]
pub struct RefreshTokenResponse {
    pub token: String,
}

#[derive(Debug, Deserialize)]
pub struct ErrorResponse {
    #[allow(dead_code)]
    pub error: String,
    pub message: Option<String>,
}
