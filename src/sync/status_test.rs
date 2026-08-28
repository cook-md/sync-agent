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
        assert_eq!(format!("{}", SyncStatus::NeedsPlan), "Needs a plan");
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
    fn test_sync_state_set_needs_plan() {
        let mut state = SyncState::default();
        let message = "Sync needs a Cook Basic or Pro plan — your files are untouched.".to_string();

        state.set_needs_plan(message.clone());

        assert_eq!(state.status, SyncStatus::NeedsPlan);
        assert_eq!(state.error_message, Some(message));
    }

    #[test]
    fn test_sync_state_needs_plan_recovers_to_idle_on_success() {
        let mut state = SyncState::default();
        state.set_needs_plan("Sync needs a Cook Basic or Pro plan".to_string());

        // A later successful sync should clear NeedsPlan without a restart,
        // the same way it clears a plain Error.
        state.set_idle();

        assert_eq!(state.status, SyncStatus::Idle);
        assert!(state.error_message.is_none());
    }

    /// Regression test for the real poll cycle: `run_async` calls
    /// `on_status_changed(Syncing)` (-> `set_syncing()`) at the *start* of
    /// every poll, before the sync attempt resolves. A notification gate
    /// based on comparing `state.status` to `NeedsPlan` is therefore always
    /// evaluated right after a `set_syncing()` write, so it sees `Syncing`,
    /// not `NeedsPlan`, and incorrectly decides to notify on every single
    /// poll while unpaid — not just on the first one. This proved the bug
    /// (see PR discussion): with the old status-comparison decision, the
    /// second assertion below fails.
    ///
    /// `should_notify_needs_plan()` / `mark_needs_plan_notified()` fix this
    /// by tracking the notification itself, which `set_syncing()` does not
    /// touch — only `set_idle()` (a genuine exit from needs-plan) resets it.
    #[test]
    fn test_needs_plan_notification_survives_syncing_transitions() {
        let mut state = SyncState::default();
        let msg = "Sync needs a Cook Basic or Pro plan — your files are untouched.".to_string();

        // Poll 1: run_async writes Syncing, then the sync fails with 402.
        state.set_syncing();
        let decision_1 = state.should_notify_needs_plan();
        state.set_needs_plan(msg.clone());
        if decision_1 {
            state.mark_needs_plan_notified();
        }
        assert!(decision_1, "must notify on the first payment-required poll");

        // Poll 2: still unpaid. run_async writes Syncing again *before* the
        // next 402 — this used to overwrite the NeedsPlan status a
        // status-comparison decision relied on.
        state.set_syncing();
        let decision_2 = state.should_notify_needs_plan();
        state.set_needs_plan(msg.clone());
        if decision_2 {
            state.mark_needs_plan_notified();
        }
        assert!(
            !decision_2,
            "must not notify again on a later poll while still unpaid"
        );

        // The user subscribes: the next sync succeeds -> Idle, which is the
        // only thing that resets the notified flag.
        state.set_syncing();
        state.set_idle();

        // A later episode (e.g. the subscription lapses again) must notify
        // once more, proving the flag was genuinely reset and not just
        // permanently latched.
        state.set_syncing();
        let decision_3 = state.should_notify_needs_plan();
        state.set_needs_plan(msg.clone());
        if decision_3 {
            state.mark_needs_plan_notified();
        }
        assert!(
            decision_3,
            "must notify again after a genuine recovery and a new payment-required episode"
        );
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

        state.status = SyncStatus::NeedsPlan;
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
            SyncStatus::NeedsPlan,
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
