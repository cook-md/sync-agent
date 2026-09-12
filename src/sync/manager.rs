use super::status::{SyncState, SyncStatus};
use super::status_listener::SyncManagerListener;
use super::task_slot::SyncTaskSlot;
use crate::auth::AuthManager;
use crate::config::Config;
use crate::error::{Result, SyncError};
use cooklang_sync_client::{extract_uid_from_jwt, SyncContext};
use log::{debug, error, info, warn};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tokio::time::{interval, Duration};

/// How long `stop()` waits for the sync task before giving up. Used on quit,
/// where the process exits shortly afterwards anyway. Anything that starts a
/// new task goes through `SyncTaskSlot`, which always joins fully.
const STOP_GRACE: Duration = Duration::from_millis(1000);

pub struct SyncManager {
    auth: Arc<AuthManager>,
    config: Arc<Config>,
    state: Arc<Mutex<SyncState>>,
    /// The single background sync task. Never two at once: the sync client
    /// keys its registry by path relative to the recipes folder, and two
    /// clients with different folders sharing that registry corrupt the
    /// namespace (issue #104).
    tasks: SyncTaskSlot,
    retry_policy: RetryPolicy,
}

#[derive(Clone)]
struct RetryPolicy {
    max_retries: usize,
    base_delay: Duration,
    max_delay: Duration,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 5,
            base_delay: Duration::from_secs(5),
            max_delay: Duration::from_secs(300),
        }
    }
}

impl RetryPolicy {
    fn calculate_delay(&self, attempt: usize) -> Duration {
        let delay_secs = self.base_delay.as_secs() * 2_u64.pow(attempt as u32);
        let delay = Duration::from_secs(delay_secs);
        std::cmp::min(delay, self.max_delay)
    }
}

impl SyncManager {
    pub fn new(auth: Arc<AuthManager>, config: Arc<Config>) -> Self {
        SyncManager {
            auth,
            config,
            state: Arc::new(Mutex::new(SyncState::default())),
            tasks: SyncTaskSlot::new(),
            retry_policy: RetryPolicy::default(),
        }
    }

    pub fn state(&self) -> Arc<Mutex<SyncState>> {
        Arc::clone(&self.state)
    }

    /// Starts the background sync task for the configured recipes folder.
    /// If a task is already running it is cancelled and fully joined first,
    /// so callers never end up with two clients on the same registry.
    pub async fn start(&self) -> Result<()> {
        // Check authentication
        if !self.auth.is_authenticated() {
            return Err(SyncError::AuthenticationRequired);
        }

        // Check if recipes directory is set
        let recipes_dir = self.config.settings().lock().unwrap().recipes_dir.clone();
        if recipes_dir.is_none() {
            return Err(SyncError::InvalidConfiguration(
                "Recipes directory not configured".to_string(),
            ));
        }

        // Create sync context with listener for status callbacks
        let sync_context = SyncContext::new();
        let listener = Arc::new(SyncManagerListener::new(Arc::clone(&self.state)));
        sync_context.set_listener(listener);

        // Start sync loop
        let state = Arc::clone(&self.state);
        let auth = Arc::clone(&self.auth);
        let config = Arc::clone(&self.config);
        let recipes_dir = recipes_dir.unwrap();
        let retry_policy = self.retry_policy.clone();

        // Get cancellation token from context
        let token = sync_context.token();
        let task_context = Arc::clone(&sync_context);
        let db_path = config.paths().database_file.clone();
        let registry_dir = recipes_dir.clone();

        let sync_loop = async move {
            let interval_secs = config.settings().lock().unwrap().sync_interval_secs;
            let mut interval = interval(Duration::from_secs(interval_secs));
            let mut last_success = std::time::Instant::now();
            let mut consecutive_failures = 0;

            // Run first sync immediately instead of waiting for the interval
            let mut first_sync = true;

            loop {
                // Check cancellation before each iteration
                if token.is_cancelled() {
                    info!("Sync loop cancelled");
                    break;
                }

                // For first sync, skip the interval tick and run immediately
                if !first_sync {
                    tokio::select! {
                        _ = interval.tick() => {},
                        _ = token.cancelled() => {
                            info!("Sync manager shutting down");
                            break;
                        }
                    }
                }
                first_sync = false;

                // Check if we should sync
                let should_sync = {
                    let st = state.lock().unwrap();
                    st.status != SyncStatus::Paused && auth.is_authenticated()
                };

                if !should_sync {
                    continue;
                }

                // Reset consecutive failures if enough time has passed since last success
                // This handles the case where system woke from sleep or network recovered
                let time_since_success = std::time::Instant::now().duration_since(last_success);
                if time_since_success > retry_policy.max_delay * 2 && consecutive_failures > 0 {
                    info!("Resetting retry counter after extended idle period ({:?} since last success)", time_since_success);
                    consecutive_failures = 0;
                    // Clear error state to allow retry
                    state.lock().unwrap().clear_error();
                }

                // Retry loop for sync attempts
                let mut retry_attempt = 0;
                loop {
                    // Check cancellation before retry
                    if token.is_cancelled() {
                        info!("Sync cancelled during retry");
                        break;
                    }

                    // Perform sync with cancellation support
                    let sync_result = perform_sync_with_context(
                        &auth,
                        &config,
                        &recipes_dir,
                        Arc::clone(&sync_context),
                    )
                    .await;

                    match sync_result {
                        Ok(()) => {
                            debug!("Sync completed successfully");
                            // Success - reset counters and update last success time
                            last_success = std::time::Instant::now();
                            consecutive_failures = 0;
                            break;
                        }
                        Err(e) => {
                            error!("Sync failed: {e}");

                            // Check if error is retriable
                            let is_retriable =
                                matches!(e, SyncError::Network(_) | SyncError::Other(_));

                            if !is_retriable {
                                // Non-retriable error - update state and break
                                let mut notify_needs_plan = false;
                                {
                                    let mut st = state.lock().unwrap();
                                    match e {
                                        SyncError::AuthenticationRequired => {
                                            st.set_error("Authentication required".to_string());
                                            // Clear session
                                            let _ = auth.logout();
                                        }
                                        SyncError::PaymentRequired => {
                                            // Only fire the notification once per needs-plan
                                            // episode. This must be tracked by a dedicated flag
                                            // rather than comparing `st.status` to `NeedsPlan`:
                                            // run_async calls on_status_changed(Syncing) (->
                                            // set_syncing()) at the start of every poll, which
                                            // would overwrite NeedsPlan before this check runs
                                            // and make it re-fire on every poll.
                                            notify_needs_plan = st.should_notify_needs_plan();
                                            st.set_needs_plan(
                                                super::error_display::humanize_error(
                                                    &e.to_string(),
                                                ),
                                            );
                                            if notify_needs_plan {
                                                st.mark_needs_plan_notified();
                                            }
                                        }
                                        _ => st.set_error(super::error_display::humanize_error(
                                            &e.to_string(),
                                        )),
                                    }
                                }

                                if notify_needs_plan {
                                    let _ = crate::notifications::show_notification(
                                        "Cook Sync",
                                        "Sync needs a Cook Basic or Pro plan — your files are untouched.",
                                    );
                                }

                                consecutive_failures += 1;
                                break;
                            }

                            // Check if we should retry
                            if retry_attempt >= retry_policy.max_retries {
                                error!("Sync failed after {} retries: {}", retry_attempt, e);
                                state.lock().unwrap().set_error(format!(
                                    "Sync failed after {} retries",
                                    retry_attempt
                                ));
                                consecutive_failures += 1;
                                break;
                            }

                            // Calculate backoff delay
                            let delay = retry_policy.calculate_delay(retry_attempt);
                            warn!(
                                "Sync failed (attempt {}/{}), retrying in {:?}: {}",
                                retry_attempt + 1,
                                retry_policy.max_retries,
                                delay,
                                e
                            );

                            retry_attempt += 1;

                            // Wait with cancellation check
                            tokio::select! {
                                _ = tokio::time::sleep(delay) => {},
                                _ = token.cancelled() => {
                                    info!("Retry cancelled during backoff");
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        };

        // The closure runs only after any previous task has been fully
        // joined, so the registry is never touched while a client uses it.
        self.tasks
            .replace(task_context, || {
                prepare_registry_for_dir(&db_path, &registry_dir)?;
                Ok::<_, SyncError>(tokio::spawn(sync_loop))
            })
            .await?;

        Ok(())
    }

    /// Switches syncing to a different recipes folder: saves the setting and
    /// restarts the sync task. The restart stops and fully joins the running
    /// client before the new one starts, and `start()` resets the local
    /// registry when it was built for a different folder.
    pub async fn change_recipes_dir(&self, new_dir: PathBuf) -> Result<()> {
        info!("Changing recipes folder to {}", new_dir.display());

        self.config.update_settings(|s| {
            s.recipes_dir = Some(new_dir);
        })?;

        if !self.auth.is_authenticated() {
            info!("Recipes folder saved; sync will start after login");
            return Ok(());
        }

        self.start().await
    }

    pub fn pause(&self) {
        self.state.lock().unwrap().status = SyncStatus::Paused;
    }

    pub fn resume(&self) {
        let mut state = self.state.lock().unwrap();
        if state.status == SyncStatus::Paused || state.status == SyncStatus::Error {
            state.status = SyncStatus::Idle;
            state.error_message = None;
        }
    }

    /// Cancels the sync task and waits briefly for it to finish. Meant for
    /// shutdown and logout; a task that needs longer keeps running in the
    /// background and is joined by the next `start()`.
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping sync manager");
        self.tasks.stop(STOP_GRACE).await;
        self.state.lock().unwrap().status = SyncStatus::Idle;
        info!("Sync manager stopped");
        Ok(())
    }
}

fn same_directory(a: &Path, b: &Path) -> bool {
    if a == b {
        return true;
    }
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

/// Sidecar file recording which recipes folder the registry was built for.
fn registry_dir_marker(db_path: &Path) -> PathBuf {
    let db_name = db_path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    db_path.with_file_name(format!("{db_name}.recipes-dir"))
}

/// Makes sure the local registry belongs to `recipes_dir`.
///
/// The sync client stores paths relative to the recipes folder. A registry
/// built for another folder would be re-interpreted against the new one:
/// every file looks new (uploaded under a spurious prefix) and every record
/// looks deleted (tombstoned on cook.md). Resetting instead makes the new
/// folder behave like a fresh device: cook.md is downloaded into it and
/// local files not yet on cook.md are uploaded.
///
/// A registry without a marker (created by an older version) is trusted
/// and the marker is written for it.
fn prepare_registry_for_dir(db_path: &Path, recipes_dir: &Path) -> Result<()> {
    let marker = registry_dir_marker(db_path);

    match std::fs::read_to_string(&marker) {
        Ok(previous) => {
            let previous = PathBuf::from(previous.trim());
            if !same_directory(&previous, recipes_dir) {
                info!(
                    "Recipes folder changed from {} to {}, resetting local sync registry",
                    previous.display(),
                    recipes_dir.display()
                );
                remove_registry_files(db_path)?;
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e.into()),
    }

    std::fs::write(&marker, recipes_dir.display().to_string())?;
    Ok(())
}

/// Deletes the sync client's registry database together with any SQLite
/// sidecar files, so the next client start begins from an empty registry.
pub(crate) fn remove_registry_files(db_path: &Path) -> std::io::Result<()> {
    let db_name = db_path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();

    for suffix in ["", "-journal", "-wal", "-shm"] {
        let path = db_path.with_file_name(format!("{db_name}{suffix}"));
        match std::fs::remove_file(&path) {
            Ok(()) => debug!("Removed {}", path.display()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e),
        }
    }
    Ok(())
}

async fn perform_sync_with_context(
    auth: &AuthManager,
    config: &Config,
    recipes_dir: &Path,
    context: Arc<cooklang_sync_client::SyncContext>,
) -> Result<()> {
    // Get current session
    let session = auth
        .get_session()
        .ok_or(SyncError::AuthenticationRequired)?;

    let namespace_id = extract_uid_from_jwt(&session.jwt);

    // Get config settings
    let sync_endpoint = crate::config::settings::Settings::get_sync_endpoint();

    // Get the db path
    let db_path = config.paths().database_file.clone();

    // Perform full sync (upload and download)
    info!("Starting sync for directory: {}", recipes_dir.display());

    let recipes_dir_str = recipes_dir.to_string_lossy().to_string();
    let db_path_str = db_path.to_string_lossy().to_string();

    cooklang_sync_client::run_async(
        context,
        &recipes_dir_str,
        &db_path_str,
        &sync_endpoint,
        &session.jwt,
        namespace_id,
        false, // both download and upload
    )
    .await
    .map_err(|e| match e {
        cooklang_sync_client::errors::SyncError::Unauthorized => SyncError::AuthenticationRequired,
        cooklang_sync_client::errors::SyncError::PaymentRequired => SyncError::PaymentRequired,
        cooklang_sync_client::errors::SyncError::ConnectionInitError(err) => {
            SyncError::Other(format!("Connection error: {err}"))
        }
        _ => SyncError::Other(format!("Sync failed: {e:?}")),
    })?;

    info!("Sync completed successfully");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retry_policy_delay_calculation() {
        let policy = RetryPolicy::default();

        // Test exponential backoff
        assert_eq!(policy.calculate_delay(0), Duration::from_secs(5));
        assert_eq!(policy.calculate_delay(1), Duration::from_secs(10));
        assert_eq!(policy.calculate_delay(2), Duration::from_secs(20));
        assert_eq!(policy.calculate_delay(3), Duration::from_secs(40));

        // Test max delay cap
        assert!(policy.calculate_delay(10) <= policy.max_delay);
    }

    #[test]
    fn prepare_registry_keeps_db_and_records_dir_when_no_marker_exists() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("sync.db");
        std::fs::write(&db, b"records").unwrap();
        let recipes = dir.path().join("Recipes");
        std::fs::create_dir(&recipes).unwrap();

        prepare_registry_for_dir(&db, &recipes).unwrap();

        assert!(
            db.exists(),
            "an existing registry without a marker is trusted"
        );
        assert_eq!(
            std::fs::read_to_string(registry_dir_marker(&db)).unwrap(),
            recipes.display().to_string()
        );
    }

    #[test]
    fn prepare_registry_keeps_db_when_marker_matches_dir() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("sync.db");
        std::fs::write(&db, b"records").unwrap();
        let recipes = dir.path().join("Recipes");
        std::fs::create_dir(&recipes).unwrap();
        std::fs::write(registry_dir_marker(&db), recipes.display().to_string()).unwrap();

        prepare_registry_for_dir(&db, &recipes).unwrap();

        assert!(db.exists());
    }

    #[test]
    fn prepare_registry_resets_db_when_marker_points_to_other_dir() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("sync.db");
        std::fs::write(&db, b"records").unwrap();
        std::fs::write(dir.path().join("sync.db-wal"), b"wal").unwrap();
        let old = dir.path().join("CookRecipes").join("Recipes");
        let new = dir.path().join("CookRecipes");
        std::fs::create_dir_all(&old).unwrap();
        std::fs::write(registry_dir_marker(&db), old.display().to_string()).unwrap();

        prepare_registry_for_dir(&db, &new).unwrap();

        assert!(
            !db.exists(),
            "registry built for another folder must be reset"
        );
        assert!(!dir.path().join("sync.db-wal").exists());
        assert_eq!(
            std::fs::read_to_string(registry_dir_marker(&db)).unwrap(),
            new.display().to_string()
        );
    }

    #[test]
    fn remove_registry_files_deletes_db_and_sqlite_sidecars() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("sync.db");
        for name in ["sync.db", "sync.db-journal", "sync.db-wal", "sync.db-shm"] {
            std::fs::write(dir.path().join(name), b"x").unwrap();
        }
        // Unrelated files in the data dir must survive.
        std::fs::write(dir.path().join("updates.json"), b"{}").unwrap();

        remove_registry_files(&db).unwrap();

        for name in ["sync.db", "sync.db-journal", "sync.db-wal", "sync.db-shm"] {
            assert!(!dir.path().join(name).exists(), "{name} should be removed");
        }
        assert!(dir.path().join("updates.json").exists());
    }

    #[test]
    fn remove_registry_files_is_ok_when_nothing_exists() {
        let dir = tempfile::tempdir().unwrap();
        remove_registry_files(&dir.path().join("sync.db")).unwrap();
    }

    #[tokio::test]
    async fn test_cancellation_token_hierarchy() {
        let context = SyncContext::new();
        let parent_token = context.token();
        let child_token = parent_token.child_token();

        // Cancel parent
        context.cancel();

        // Both should be cancelled
        assert!(parent_token.is_cancelled());
        assert!(child_token.is_cancelled());
    }
}
