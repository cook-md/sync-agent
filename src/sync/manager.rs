use super::status::{SyncState, SyncStatus};
use crate::auth::AuthManager;
use crate::config::Config;
use crate::error::{Result, SyncError};
use cooklang_sync_client::extract_uid_from_jwt;
use log::{debug, error, info};
use std::path::Path;
use std::sync::{Arc, Mutex};
use tokio::sync::watch;
use tokio::task::JoinHandle;
use tokio::time::{interval, Duration};

pub struct SyncManager {
    auth: Arc<AuthManager>,
    config: Arc<Config>,
    state: Arc<Mutex<SyncState>>,
    shutdown_tx: Arc<Mutex<Option<watch::Sender<bool>>>>,
    sync_task: Arc<Mutex<Option<JoinHandle<()>>>>,
}

impl SyncManager {
    pub fn new(auth: Arc<AuthManager>, config: Arc<Config>) -> Self {
        SyncManager {
            auth,
            config,
            state: Arc::new(Mutex::new(SyncState::default())),
            shutdown_tx: Arc::new(Mutex::new(None)),
            sync_task: Arc::new(Mutex::new(None)),
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

        // Create shutdown channel
        let (shutdown_tx, mut shutdown_rx) = watch::channel(false);
        *self.shutdown_tx.lock().unwrap() = Some(shutdown_tx);

        // Start sync loop
        let state = Arc::clone(&self.state);
        let auth = Arc::clone(&self.auth);
        let config = Arc::clone(&self.config);
        let recipes_dir = recipes_dir.unwrap();
        let sync_task_clone = Arc::clone(&self.sync_task);

        let handle = tokio::spawn(async move {
            let interval_secs = config.settings().lock().unwrap().sync_interval_secs;
            let mut interval = interval(Duration::from_secs(interval_secs));

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        // Check if we should sync
                        let should_sync = {
                            let st = state.lock().unwrap();
                            st.status != SyncStatus::Paused && auth.is_authenticated()
                        };

                        if !should_sync {
                            continue;
                        }

                        // Perform sync with cancellation support
                        state.lock().unwrap().set_syncing();

                        // Create a cancellable sync task
                        let sync_future = perform_sync(&auth, &config, &recipes_dir);

                        // Run sync with timeout/cancellation
                        tokio::select! {
                            result = sync_future => {
                                match result {
                                    Ok(stats) => {
                                        let mut st = state.lock().unwrap();
                                        st.set_idle();
                                        st.items_synced = stats.synced;
                                        st.items_pending = stats.pending;
                                        debug!("Sync completed: {} synced, {} pending", stats.synced, stats.pending);
                                    }
                                    Err(e) => {
                                        error!("Sync failed: {e}");
                                        let mut st = state.lock().unwrap();
                                        match e {
                                            SyncError::Network(_) => st.set_offline(),
                                            SyncError::AuthenticationRequired => {
                                                st.set_error("Authentication required".to_string());
                                                // Clear session
                                                let _ = auth.logout();
                                            }
                                            _ => st.set_error(e.to_string()),
                                        }
                                    }
                                }
                            }
                            _ = shutdown_rx.changed() => {
                                if *shutdown_rx.borrow() {
                                    info!("Sync cancelled due to shutdown");
                                    break;
                                }
                            }
                        }
                    }
                    _ = shutdown_rx.changed() => {
                        if *shutdown_rx.borrow() {
                            info!("Sync manager shutting down");
                            break;
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
        if self.state.lock().unwrap().status == SyncStatus::Paused {
            self.state.lock().unwrap().status = SyncStatus::Idle;
        }
    }

    pub async fn stop(&self) -> Result<()> {
        info!("Stopping sync manager");

        // Send shutdown signal
        if let Some(tx) = self.shutdown_tx.lock().unwrap().as_ref() {
            let _ = tx.send(true);
        }

        // Abort the sync task if it's running
        let handle = self.sync_task.lock().unwrap().take();
        if let Some(handle) = handle {
            info!("Aborting sync task");
            handle.abort();
            // Wait for it to finish (will return immediately if aborted)
            let _ = handle.await;
            info!("Sync task aborted");
        }

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

struct SyncStats {
    synced: usize,
    pending: usize,
}

async fn perform_sync(
    auth: &AuthManager,
    config: &Config,
    recipes_dir: &Path,
) -> Result<SyncStats> {
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

    // Run sync once (we handle the loop ourselves for better control)
    cooklang_sync_client::run_async(
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

    Ok(SyncStats {
        synced: 0,
        pending: 0,
    })
}
