use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncStatus {
    Starting,
    Syncing,
    Idle,
    Paused,
    Error,
    Offline,
}

impl fmt::Display for SyncStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SyncStatus::Starting => write!(f, "Starting"),
            SyncStatus::Syncing => write!(f, "Syncing"),
            SyncStatus::Idle => write!(f, "Up to date"),
            SyncStatus::Paused => write!(f, "Paused"),
            SyncStatus::Error => write!(f, "Error"),
            SyncStatus::Offline => write!(f, "Offline"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SyncState {
    pub status: SyncStatus,
    pub last_sync: Option<chrono::DateTime<chrono::Utc>>,
    pub error_message: Option<String>,
    #[allow(dead_code)]
    pub items_synced: usize,
    #[allow(dead_code)]
    pub items_pending: usize,
}

impl Default for SyncState {
    fn default() -> Self {
        Self {
            status: SyncStatus::Starting,
            last_sync: None,
            error_message: None,
            items_synced: 0,
            items_pending: 0,
        }
    }
}

impl SyncState {
    pub fn set_syncing(&mut self) {
        self.status = SyncStatus::Syncing;
        self.error_message = None;
    }

    pub fn set_idle(&mut self) {
        self.status = SyncStatus::Idle;
        self.last_sync = Some(chrono::Utc::now());
        self.error_message = None;
    }

    pub fn set_error(&mut self, message: String) {
        self.status = SyncStatus::Error;
        self.error_message = Some(message);
    }

    pub fn clear_error(&mut self) {
        if self.status == SyncStatus::Error {
            self.status = SyncStatus::Idle;
            self.error_message = None;
        }
    }

    #[allow(dead_code)]
    pub fn set_offline(&mut self) {
        self.status = SyncStatus::Offline;
        self.error_message = Some("No internet connection".to_string());
    }

    #[allow(dead_code)]
    pub fn is_active(&self) -> bool {
        matches!(self.status, SyncStatus::Syncing | SyncStatus::Idle)
    }
}
