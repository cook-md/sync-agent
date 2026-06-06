use std::time::{Duration, Instant};

use log::error;
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;

use crate::error::{Result, SyncError};

#[cfg(test)]
#[path = "device_flow_test.rs"]
mod device_flow_test;

const GRANT_TYPE: &str = "urn:ietf:params:oauth:grant-type:device_code";

/// Response from `POST /oauth/device/code`.
#[derive(Debug, Clone, Deserialize)]
pub struct DeviceCodeResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub verification_uri_complete: String,
    pub expires_in: u64,
    pub interval: u64,
}

#[derive(Debug, Serialize)]
struct DeviceCodeRequest<'a> {
    client_name: &'a str,
}

#[derive(Debug, Serialize)]
struct TokenRequest<'a> {
    grant_type: &'a str,
    device_code: &'a str,
}

#[derive(Debug, Deserialize)]
struct TokenSuccess {
    access_token: String,
}

#[derive(Debug, Deserialize)]
struct TokenError {
    error: String,
}

/// One step of the RFC 8628 token-polling state machine. Pure and
/// network-free so it can be unit-tested.
#[derive(Debug, PartialEq, Eq)]
pub enum PollOutcome {
    /// Got the JWT.
    Token(String),
    /// Keep polling at the current interval.
    Pending,
    /// Increase the polling interval by 5 seconds.
    SlowDown,
    /// User denied authorization (terminal).
    Denied,
    /// Device code expired (terminal).
    Expired,
    /// Unexpected/unparseable response (terminal).
    Bad(String),
}

/// Interpret a `/oauth/device/token` HTTP response body. `is_success` is the
/// HTTP 2xx status; `body` is the raw response text.
pub fn interpret_token_response(is_success: bool, body: &str) -> PollOutcome {
    if is_success {
        return match serde_json::from_str::<TokenSuccess>(body) {
            Ok(ok) => PollOutcome::Token(ok.access_token),
            Err(e) => PollOutcome::Bad(format!("unparseable success body: {e}")),
        };
    }

    let err: TokenError = match serde_json::from_str(body) {
        Ok(e) => e,
        Err(e) => return PollOutcome::Bad(format!("unparseable error body: {e}")),
    };

    match err.error.as_str() {
        "authorization_pending" => PollOutcome::Pending,
        "slow_down" => PollOutcome::SlowDown,
        "access_denied" => PollOutcome::Denied,
        "expired_token" => PollOutcome::Expired,
        other => PollOutcome::Bad(format!("unexpected error code: {other}")),
    }
}

/// Builds the `client_name` sent to cook.md; identifies the device on the
/// approval screen. E.g. `"Cook Sync 0.6.0 (linux/server)"`.
pub fn client_name() -> String {
    format!(
        "Cook Sync {} ({}/{})",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS,
        server_host_label()
    )
}

/// Returns "docker" if `/.dockerenv` exists, else "server".
pub fn server_host_label() -> &'static str {
    if std::path::Path::new("/.dockerenv").exists() {
        "docker"
    } else {
        "server"
    }
}
