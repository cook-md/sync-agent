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
    NeedsPlan,
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
            SyncStatus::NeedsPlan => write!(f, "Needs a plan"),
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
    /// Whether the one-shot "needs a plan" desktop notification has already
    /// fired for the *current* needs-plan episode. A new poll cycle writes
    /// `Syncing` (via `set_syncing`) before the sync attempt resolves, so
    /// this flag — not the current `status` — is what dedupes the
    /// notification across repeated 402s. It is cleared only by
    /// `set_idle`, i.e. a genuine successful sync / exit from the
    /// needs-plan condition.
    pub(crate) notified_needs_plan: bool,
}

impl Default for SyncState {
    fn default() -> Self {
        Self {
            status: SyncStatus::Starting,
            last_sync: None,
            error_message: None,
            items_synced: 0,
            items_pending: 0,
            notified_needs_plan: false,
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
        self.notified_needs_plan = false;
    }

    pub fn set_error(&mut self, message: String) {
        self.status = SyncStatus::Error;
        self.error_message = Some(message);
    }

    pub fn set_needs_plan(&mut self, message: String) {
        self.status = SyncStatus::NeedsPlan;
        self.error_message = Some(message);
    }

    /// Whether the one-shot "needs a plan" notification should fire right
    /// now. Survives the transient `Syncing` status a new poll writes
    /// before the sync attempt resolves, so it does not re-fire on every
    /// retry/poll while still unpaid.
    pub fn should_notify_needs_plan(&self) -> bool {
        !self.notified_needs_plan
    }

    /// Marks the "needs a plan" notification as shown for the current
    /// needs-plan episode. Only `set_idle` clears this.
    pub fn mark_needs_plan_notified(&mut self) {
        self.notified_needs_plan = true;
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
