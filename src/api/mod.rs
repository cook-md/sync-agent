pub mod endpoints;

use crate::error::{Result, SyncError};
use endpoints::*;
use log::debug;
use reqwest::{Client, StatusCode};
use std::time::Duration;

pub struct CookApi {
    client: Client,
    base_url: String,
}

impl CookApi {
    pub fn new(base_url: String) -> Result<Self> {
        let client = Client::builder().timeout(Duration::from_secs(30)).build()?;

        Ok(CookApi { client, base_url })
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub async fn refresh_token(&self, current_token: &str) -> Result<String> {
        let url = format!("{}/sessions/renew", self.base_url);

        debug!("Refreshing token at: {url}");

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {current_token}"))
            .json(&RefreshTokenRequest {
                token: current_token.to_string(),
            })
            .send()
            .await?;

        match response.status() {
            StatusCode::OK => {
                let data: RefreshTokenResponse = response.json().await?;
                Ok(data.token)
            }
            StatusCode::UNAUTHORIZED => Err(SyncError::AuthenticationRequired),
            status => {
                let error = response.json::<ErrorResponse>().await.ok();
                Err(SyncError::Other(format!(
                    "Token refresh failed with status {}: {}",
                    status,
                    error.and_then(|e| e.message).unwrap_or_default()
                )))
            }
        }
    }
}
