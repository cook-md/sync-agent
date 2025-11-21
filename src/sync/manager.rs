use super::status::{SyncState, SyncStatus};
use super::status_listener::SyncManagerListener;
use crate::auth::AuthManager;
use crate::config::Config;
use crate::error::{Result, SyncError};
use cooklang_sync_client::{extract_uid_from_jwt, SyncContext};
use log::{debug, error, info, warn};
use std::path::Path;
use std::sync::{Arc, Mutex};
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tokio::time::{interval, Duration};
use tokio_util::sync::CancellationToken;

pub struct SyncManager {
    auth: Arc<AuthManager>,
    config: Arc<Config>,
    state: Arc<Mutex<SyncState>>,
    sync_context: Arc<RwLock<Option<Arc<SyncContext>>>>,
    sync_task: Arc<Mutex<Option<JoinHandle<()>>>>,
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
            sync_context: Arc::new(RwLock::new(None)),
            sync_task: Arc::new(Mutex::new(None)),
            retry_policy: RetryPolicy::default(),
        }
    }

    pub fn state(&self) -> Arc<Mutex<SyncState>> {
        Arc::clone(&self.state)
    }

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

        // Create sync context with listener
        let sync_context = SyncContext::new();
        let listener = Arc::new(SyncManagerListener::new(Arc::clone(&self.state)));
        sync_context.set_listener(listener);

        // Store context
        *self.sync_context.write().await = Some(Arc::clone(&sync_context));

        // Start sync loop
        let state = Arc::clone(&self.state);
        let auth = Arc::clone(&self.auth);
        let config = Arc::clone(&self.config);
        let recipes_dir = recipes_dir.unwrap();
        let sync_task_clone = Arc::clone(&self.sync_task);
        let retry_policy = self.retry_policy.clone();

        // Get cancellation token from context
        let token = sync_context.token();

        let handle = tokio::spawn(async move {
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
                        token.child_token(),
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
                                let mut st = state.lock().unwrap();
                                match e {
                                    SyncError::AuthenticationRequired => {
                                        st.set_error("Authentication required".to_string());
                                        // Clear session
                                        let _ = auth.logout();
                                    }
                                    _ => st.set_error(e.to_string()),
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

            // Clear the task handle when done
            *sync_task_clone.lock().unwrap() = None;
        });

        // Store the task handle
        *self.sync_task.lock().unwrap() = Some(handle);

        Ok(())
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

    pub async fn stop(&self) -> Result<()> {
        info!("Stopping sync manager");

        // Cancel via context
        if let Some(ctx) = self.sync_context.read().await.as_ref() {
            ctx.cancel();
            debug!("Cancellation signal sent");
        }

        // Wait for sync task with timeout
        let handle = self.sync_task.lock().unwrap().take();
        if let Some(handle) = handle {
            info!("Waiting for sync task to complete");

            // Give it 30 seconds to finish gracefully
            let timeout = Duration::from_secs(30);
            match tokio::time::timeout(timeout, handle).await {
                Ok(Ok(())) => info!("Sync task completed gracefully"),
                Ok(Err(e)) => warn!("Sync task panicked: {:?}", e),
                Err(_) => {
                    warn!("Sync task did not complete within {:?}", timeout);
                }
            }
        }

        // Clear context
        *self.sync_context.write().await = None;

        self.state.lock().unwrap().status = SyncStatus::Idle;
        info!("Sync manager stopped");
        Ok(())
    }

    pub fn is_running(&self) -> bool {
        let state = self.state.lock().unwrap();
        matches!(
            state.status,
            SyncStatus::Syncing | SyncStatus::Idle | SyncStatus::Offline
        )
    }
}

async fn perform_sync_with_context(
    auth: &AuthManager,
    config: &Config,
    recipes_dir: &Path,
    token: CancellationToken,
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

    // Run sync with the new API
    cooklang_sync_client::run_async(
        token,
        None, // listener is already set on the context
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
