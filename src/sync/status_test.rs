#[cfg(test)]
mod tests {
    use super::super::status::{SyncState, SyncStatus};

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
    fn test_sync_state_default() {
        let state = SyncState::default();
        assert_eq!(state.status, SyncStatus::Starting);
        assert!(state.last_sync.is_none());
        assert!(state.error_message.is_none());
        assert_eq!(state.items_synced, 0);
        assert_eq!(state.items_pending, 0);
    }

    #[test]
    fn test_sync_state_set_syncing() {
        let mut state = SyncState {
            error_message: Some("Previous error".to_string()),
            ..Default::default()
        };

        state.set_syncing();

        assert_eq!(state.status, SyncStatus::Syncing);
        assert!(state.error_message.is_none()); // Should clear error
    }

    #[test]
    fn test_sync_state_set_idle() {
        let mut state = SyncState {
            error_message: Some("Previous error".to_string()),
            ..Default::default()
        };

        state.set_idle();

        assert_eq!(state.status, SyncStatus::Idle);
        assert!(state.last_sync.is_some()); // Should set last_sync time
        assert!(state.error_message.is_none()); // Should clear error
    }

    #[test]
    fn test_sync_state_set_error() {
        let mut state = SyncState::default();
        let error_msg = "Test error message".to_string();

        state.set_error(error_msg.clone());

        assert_eq!(state.status, SyncStatus::Error);
        assert_eq!(state.error_message, Some(error_msg));
    }

    #[test]
    fn test_sync_state_set_offline() {
        let mut state = SyncState::default();

        state.set_offline();

        assert_eq!(state.status, SyncStatus::Offline);
        assert_eq!(
            state.error_message,
            Some("No internet connection".to_string())
        );
    }

    #[test]
    fn test_sync_state_is_active() {
        let mut state = SyncState {
            status: SyncStatus::Starting,
            ..Default::default()
        };

        // Test all statuses
        assert!(!state.is_active());

        state.status = SyncStatus::Syncing;
        assert!(state.is_active());

        state.status = SyncStatus::Idle;
        assert!(state.is_active());

        state.status = SyncStatus::Paused;
        assert!(!state.is_active());

        state.status = SyncStatus::Error;
        assert!(!state.is_active());

        state.status = SyncStatus::Offline;
        assert!(!state.is_active());
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
    fn test_sync_status_serialization() {
        use serde_json;

        let status = SyncStatus::Syncing;
        let json = serde_json::to_string(&status).unwrap();
        let deserialized: SyncStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(status, deserialized);

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
}
