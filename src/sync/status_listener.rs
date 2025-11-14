use super::status::SyncState;
use cooklang_sync_client::{SyncStatus as ClientSyncStatus, SyncStatusListener};
use log::{debug, error, info, warn};
use std::sync::{Arc, Mutex};

/// Listener that receives status updates from the sync client and updates our internal state
pub struct SyncManagerListener {
    state: Arc<Mutex<SyncState>>,
}

impl SyncManagerListener {
    pub fn new(state: Arc<Mutex<SyncState>>) -> Self {
        Self { state }
    }
}

impl SyncStatusListener for SyncManagerListener {
    fn on_status_changed(&self, status: ClientSyncStatus) {
        let mut state = self.state.lock().unwrap();

        match status {
            ClientSyncStatus::Idle => {
                debug!("Sync status: Idle");
                state.set_idle();
            }
            ClientSyncStatus::Syncing => {
                info!("Sync status: Syncing");
                state.set_syncing();
            }
            ClientSyncStatus::Indexing => {
                debug!("Sync status: Indexing");
                state.set_syncing();
            }
            ClientSyncStatus::Downloading => {
                debug!("Sync status: Downloading");
                state.set_syncing();
            }
            ClientSyncStatus::Uploading => {
                debug!("Sync status: Uploading");
                state.set_syncing();
            }
            ClientSyncStatus::Error { message } => {
                error!("Sync status: Error - {}", message);
                state.set_error(message);
            }
        }
    }

    fn on_complete(&self, success: bool, message: Option<String>) {
        let mut state = self.state.lock().unwrap();

        if success {
            info!("Sync completed successfully");
            state.set_idle();
        } else {
            let error_msg = message.unwrap_or_else(|| "Sync failed".to_string());
            warn!("Sync failed: {}", error_msg);
            state.set_error(error_msg);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::status::SyncStatus;
    use super::*;

    #[test]
    fn test_status_transitions() {
        let state = Arc::new(Mutex::new(SyncState::default()));
        let listener = SyncManagerListener::new(Arc::clone(&state));

        // Test syncing transition
        listener.on_status_changed(ClientSyncStatus::Syncing);
        assert_eq!(state.lock().unwrap().status, SyncStatus::Syncing);

        // Test idle transition
        listener.on_status_changed(ClientSyncStatus::Idle);
        assert_eq!(state.lock().unwrap().status, SyncStatus::Idle);

        // Test error transition
        listener.on_status_changed(ClientSyncStatus::Error {
            message: "test error".to_string(),
        });
        assert_eq!(state.lock().unwrap().status, SyncStatus::Error);
        assert_eq!(
            state.lock().unwrap().error_message,
            Some("test error".to_string())
        );
    }

    #[test]
    fn test_completion_callbacks() {
        let state = Arc::new(Mutex::new(SyncState::default()));
        let listener = SyncManagerListener::new(Arc::clone(&state));

        // Test successful completion
        listener.on_complete(true, None);
        assert_eq!(state.lock().unwrap().status, SyncStatus::Idle);
        assert!(state.lock().unwrap().last_sync.is_some());

        // Test failed completion
        listener.on_complete(false, Some("sync failed".to_string()));
        assert_eq!(state.lock().unwrap().status, SyncStatus::Error);
        assert_eq!(
            state.lock().unwrap().error_message,
            Some("sync failed".to_string())
        );
    }
}
