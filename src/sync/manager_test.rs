#[cfg(test)]
mod tests {
    use super::super::status::{SyncState, SyncStatus};
    use crate::error::SyncError;

    #[tokio::test]
    async fn test_sync_manager_states() {
        // Test state transitions without actual auth/config
        let mut state = SyncState::default();
        assert_eq!(state.status, SyncStatus::Starting);

        // Test set_syncing
        state.set_syncing();
        assert_eq!(state.status, SyncStatus::Syncing);
        assert!(state.error_message.is_none());

        // Test set_idle
        state.set_idle();
        assert_eq!(state.status, SyncStatus::Idle);
        assert!(state.last_sync.is_some());
        assert!(state.error_message.is_none());

        // Test set_error
        state.set_error("Test error".to_string());
        assert_eq!(state.status, SyncStatus::Error);
        assert_eq!(state.error_message, Some("Test error".to_string()));

        // Test set_offline
        state.set_offline();
        assert_eq!(state.status, SyncStatus::Offline);
        assert_eq!(
            state.error_message,
            Some("No internet connection".to_string())
        );
    }

    #[tokio::test]
    async fn test_sync_state_is_active() {
        let mut state = SyncState {
            status: SyncStatus::Starting,
            ..Default::default()
        };

        // Starting - not active
        assert!(!state.is_active());

        // Syncing - active
        state.status = SyncStatus::Syncing;
        assert!(state.is_active());

        // Idle - active
        state.status = SyncStatus::Idle;
        assert!(state.is_active());

        // Paused - not active
        state.status = SyncStatus::Paused;
        assert!(!state.is_active());

        // Error - not active
        state.status = SyncStatus::Error;
        assert!(!state.is_active());

        // Offline - not active
        state.status = SyncStatus::Offline;
        assert!(!state.is_active());
    }

    #[tokio::test]
    async fn test_sync_stats_tracking() {
        let mut state = SyncState::default();

        // Initial stats
        assert_eq!(state.items_synced, 0);
        assert_eq!(state.items_pending, 0);

        // Update stats
        state.items_synced = 10;
        state.items_pending = 5;

        assert_eq!(state.items_synced, 10);
        assert_eq!(state.items_pending, 5);
    }

    #[test]
    fn test_sync_status_display() {
        assert_eq!(format!("{}", SyncStatus::Starting), "Starting");
        assert_eq!(format!("{}", SyncStatus::Syncing), "Syncing");
        assert_eq!(format!("{}", SyncStatus::Idle), "Up to date");
        assert_eq!(format!("{}", SyncStatus::Paused), "Paused");
        assert_eq!(format!("{}", SyncStatus::Error), "Error");
        assert_eq!(format!("{}", SyncStatus::Offline), "Offline");
    }

    #[test]
    fn test_sync_status_equality() {
        assert_eq!(SyncStatus::Idle, SyncStatus::Idle);
        assert_ne!(SyncStatus::Idle, SyncStatus::Syncing);
        assert_ne!(SyncStatus::Paused, SyncStatus::Error);
    }

    #[test]
    fn test_sync_status_serialization() {
        use serde_json;

        // Test all variants
        for status in [
            SyncStatus::Starting,
            SyncStatus::Syncing,
            SyncStatus::Idle,
            SyncStatus::Paused,
            SyncStatus::Error,
            SyncStatus::Offline,
        ] {
            let json = serde_json::to_string(&status).unwrap();
            let deserialized: SyncStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(status, deserialized);
        }
    }

    #[test]
    fn test_sync_state_transitions_preserve_data() {
        let mut state = SyncState {
            items_synced: 10,
            items_pending: 5,
            ..Default::default()
        };

        // Transition to syncing should preserve counts
        state.set_syncing();
        assert_eq!(state.items_synced, 10);
        assert_eq!(state.items_pending, 5);

        // Transition to idle should preserve counts
        state.set_idle();
        assert_eq!(state.items_synced, 10);
        assert_eq!(state.items_pending, 5);

        // Transition to error should preserve counts
        state.set_error("Error".to_string());
        assert_eq!(state.items_synced, 10);
        assert_eq!(state.items_pending, 5);

        // Transition to offline should preserve counts
        state.set_offline();
        assert_eq!(state.items_synced, 10);
        assert_eq!(state.items_pending, 5);
    }

    #[test]
    fn test_sync_state_last_sync_time() {
        let mut state = SyncState::default();
        assert!(state.last_sync.is_none());

        // Only set_idle should update last_sync
        state.set_syncing();
        assert!(state.last_sync.is_none());

        state.set_idle();
        let first_sync_time = state.last_sync;
        assert!(first_sync_time.is_some());

        // Sleep a tiny bit to ensure different timestamp
        std::thread::sleep(std::time::Duration::from_millis(10));

        state.set_idle();
        let second_sync_time = state.last_sync;
        assert!(second_sync_time.is_some());
        assert!(second_sync_time > first_sync_time);
    }

    // Test error conditions without requiring actual system resources
    #[test]
    fn test_sync_error_types() {
        // Test different error types
        let auth_error = SyncError::AuthenticationRequired;
        assert!(format!("{}", auth_error).contains("Authentication"));

        let config_error = SyncError::InvalidConfiguration("Test".to_string());
        assert!(format!("{}", config_error).contains("Invalid"));

        // Network error uses reqwest::Error, so we test other errors instead
        let other_error = SyncError::Other("Connection failed".to_string());
        assert!(format!("{}", other_error).contains("failed"));
    }

    #[test]
    fn test_sync_state_clone() {
        let mut state = SyncState::default();
        state.set_idle();
        state.items_synced = 10;
        state.items_pending = 5;

        let cloned = state.clone();

        assert_eq!(cloned.status, state.status);
        assert_eq!(cloned.last_sync, state.last_sync);
        assert_eq!(cloned.error_message, state.error_message);
        assert_eq!(cloned.items_synced, state.items_synced);
        assert_eq!(cloned.items_pending, state.items_pending);
    }

    #[test]
    fn test_clear_error() {
        let mut state = SyncState::default();

        // Set an error state
        state.set_error("Sync failed after 5 retries".to_string());
        assert_eq!(state.status, SyncStatus::Error);
        assert_eq!(state.error_message, Some("Sync failed after 5 retries".to_string()));

        // Clear the error
        state.clear_error();
        assert_eq!(state.status, SyncStatus::Idle);
        assert_eq!(state.error_message, None);

        // Clearing error when not in error state should not change anything
        state.set_syncing();
        state.clear_error();
        assert_eq!(state.status, SyncStatus::Syncing);
    }
}
