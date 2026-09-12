//! Ownership of the single background sync task.
//!
//! The sync client stores file paths relative to the configured recipes
//! folder in a registry shared through `sync.db`. Two clients running at the
//! same time against that registry with different base paths re-interpret
//! each other's records and ping-pong files into ever deeper
//! `Recipes/Recipes/...` paths (see issue #104). `SyncTaskSlot` makes that
//! impossible: a new task is only spawned after the previous one has been
//! cancelled and fully joined.

use cooklang_sync_client::SyncContext;
use log::{info, warn};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio::time::{timeout, Duration};

/// How long `replace`/`shutdown` wait for a cancelled task to wind down
/// before aborting it. A cancelled client finishes its in-flight request
/// first; those are bounded by the client's 60s request timeout.
const JOIN_TIMEOUT: Duration = Duration::from_secs(120);

struct RunningTask {
    context: Arc<SyncContext>,
    handle: JoinHandle<()>,
}

/// Holds at most one running sync task.
///
/// All lifecycle operations are serialised through one async mutex, so a
/// folder change, a logout and a login racing each other can never leave two
/// clients alive at once.
pub struct SyncTaskSlot {
    inner: Mutex<Option<RunningTask>>,
}

impl SyncTaskSlot {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(None),
        }
    }

    /// Cancels and fully joins the current task (if any), then spawns the
    /// replacement. `spawn` is only invoked once the previous task is gone,
    /// so any preparation it does (like resetting the registry) can never
    /// race with the old task. If `spawn` fails the slot stays empty.
    pub async fn replace<F, E>(&self, context: Arc<SyncContext>, spawn: F) -> Result<(), E>
    where
        F: FnOnce() -> Result<JoinHandle<()>, E>,
    {
        let mut slot = self.inner.lock().await;
        if let Some(task) = slot.take() {
            shutdown_task(task).await;
        }
        let handle = spawn()?;
        *slot = Some(RunningTask { context, handle });
        Ok(())
    }

    /// Cancels the current task and waits until it has fully finished,
    /// aborting it if it does not wind down within `JOIN_TIMEOUT`.
    pub async fn shutdown(&self) {
        let mut slot = self.inner.lock().await;
        if let Some(task) = slot.take() {
            shutdown_task(task).await;
        }
    }

    /// Cancels the current task and waits up to `grace` for it to finish.
    /// Returns `true` if no task is left running. A task that needs longer
    /// stays in the slot so a later `replace`/`shutdown` still joins it
    /// instead of letting it run alongside a new one.
    pub async fn stop(&self, grace: Duration) -> bool {
        let mut slot = self.inner.lock().await;
        let Some(task) = slot.take() else {
            return true;
        };

        task.context.cancel();
        let RunningTask { context, handle } = task;
        let mut handle = handle;
        match timeout(grace, &mut handle).await {
            Ok(Ok(())) => {
                info!("Sync task completed gracefully");
                true
            }
            Ok(Err(e)) => {
                warn!("Sync task panicked: {e:?}");
                true
            }
            Err(_) => {
                warn!("Sync task did not complete within {grace:?}, cancellation signal sent");
                *slot = Some(RunningTask { context, handle });
                false
            }
        }
    }
}

impl Default for SyncTaskSlot {
    fn default() -> Self {
        Self::new()
    }
}

async fn shutdown_task(task: RunningTask) {
    task.context.cancel();
    let mut handle = task.handle;
    match timeout(JOIN_TIMEOUT, &mut handle).await {
        Ok(Ok(())) => info!("Previous sync task finished"),
        Ok(Err(e)) => warn!("Previous sync task panicked: {e:?}"),
        Err(_) => {
            warn!("Sync task ignored cancellation for {JOIN_TIMEOUT:?}, aborting it");
            handle.abort();
            // Wait for the abort to take effect so the task's resources
            // (registry connection, file handles) are released before the
            // caller proceeds.
            let _ = handle.await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cooklang_sync_client::SyncContext;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use tokio::time::{sleep, Duration};

    /// Sets a flag when dropped, so a test can observe that an aborted task's
    /// future was actually torn down.
    struct SetOnDrop(Arc<AtomicBool>);

    impl Drop for SetOnDrop {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }

    /// Spawns a task that honours cancellation but, like a client mid
    /// download, needs `tail` more time to wind down afterwards.
    fn slow_to_stop(
        context: &Arc<SyncContext>,
        tail: Duration,
        done: Arc<AtomicBool>,
    ) -> tokio::task::JoinHandle<()> {
        let token = context.token();
        tokio::spawn(async move {
            token.cancelled().await;
            sleep(tail).await;
            done.store(true, Ordering::SeqCst);
        })
    }

    #[tokio::test(start_paused = true)]
    async fn replace_joins_previous_task_before_spawning_next() {
        let slot = SyncTaskSlot::new();
        let first_done = Arc::new(AtomicBool::new(false));

        let ctx1 = SyncContext::new();
        let done = Arc::clone(&first_done);
        slot.replace(Arc::clone(&ctx1), || {
            Ok::<_, ()>(slow_to_stop(&ctx1, Duration::from_secs(5), done))
        })
        .await
        .unwrap();

        // The bug: the old task outlived the 1s grace period and overlapped
        // with the new one. The new task must only be spawned once the old
        // one has completely finished.
        let overlapped = Arc::new(AtomicBool::new(false));
        let ctx2 = SyncContext::new();
        let observed = Arc::clone(&overlapped);
        let done = Arc::clone(&first_done);
        slot.replace(ctx2, move || {
            observed.store(!done.load(Ordering::SeqCst), Ordering::SeqCst);
            Ok::<_, ()>(tokio::spawn(async {}))
        })
        .await
        .unwrap();

        assert!(
            !overlapped.load(Ordering::SeqCst),
            "new sync task was spawned while the previous one was still running"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn stop_keeps_unfinished_task_so_later_replace_still_joins_it() {
        let slot = SyncTaskSlot::new();
        let first_done = Arc::new(AtomicBool::new(false));

        let ctx1 = SyncContext::new();
        let done = Arc::clone(&first_done);
        slot.replace(Arc::clone(&ctx1), || {
            Ok::<_, ()>(slow_to_stop(&ctx1, Duration::from_secs(3), done))
        })
        .await
        .unwrap();

        let finished = slot.stop(Duration::from_secs(1)).await;
        assert!(
            !finished,
            "task needs 3s to wind down, stop() only waited 1s"
        );
        assert!(!first_done.load(Ordering::SeqCst));

        let overlapped = Arc::new(AtomicBool::new(false));
        let observed = Arc::clone(&overlapped);
        let done = Arc::clone(&first_done);
        slot.replace(SyncContext::new(), move || {
            observed.store(!done.load(Ordering::SeqCst), Ordering::SeqCst);
            Ok::<_, ()>(tokio::spawn(async {}))
        })
        .await
        .unwrap();

        assert!(!overlapped.load(Ordering::SeqCst));
    }

    #[tokio::test(start_paused = true)]
    async fn stop_reports_finished_when_task_winds_down_within_grace() {
        let slot = SyncTaskSlot::new();
        let done = Arc::new(AtomicBool::new(false));

        let ctx = SyncContext::new();
        let done_clone = Arc::clone(&done);
        slot.replace(Arc::clone(&ctx), || {
            Ok::<_, ()>(slow_to_stop(&ctx, Duration::from_millis(100), done_clone))
        })
        .await
        .unwrap();

        assert!(slot.stop(Duration::from_secs(1)).await);
        assert!(done.load(Ordering::SeqCst));
    }

    #[tokio::test(start_paused = true)]
    async fn shutdown_aborts_task_that_ignores_cancellation() {
        let slot = SyncTaskSlot::new();
        let dropped = Arc::new(AtomicBool::new(false));

        let guard_flag = Arc::clone(&dropped);
        slot.replace(SyncContext::new(), move || {
            Ok::<_, ()>(tokio::spawn(async move {
                let _guard = SetOnDrop(guard_flag);
                std::future::pending::<()>().await;
            }))
        })
        .await
        .unwrap();

        slot.shutdown().await;

        assert!(
            dropped.load(Ordering::SeqCst),
            "a task that never finishes must be aborted, not waited on forever"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn replace_propagates_spawn_error_and_leaves_slot_empty() {
        let slot = SyncTaskSlot::new();
        let first_done = Arc::new(AtomicBool::new(false));

        let ctx1 = SyncContext::new();
        let done = Arc::clone(&first_done);
        slot.replace(Arc::clone(&ctx1), || {
            Ok::<_, ()>(slow_to_stop(&ctx1, Duration::from_secs(1), done))
        })
        .await
        .unwrap();

        // Preparation (e.g. resetting the registry) runs after the previous
        // task is gone; if it fails nothing new is spawned.
        let done = Arc::clone(&first_done);
        let result = slot
            .replace(SyncContext::new(), move || {
                assert!(done.load(Ordering::SeqCst), "old task must be joined first");
                Err::<tokio::task::JoinHandle<()>, &str>("registry reset failed")
            })
            .await;
        assert_eq!(result, Err("registry reset failed"));

        // Nothing to stop: the slot is empty.
        assert!(slot.stop(Duration::from_millis(1)).await);
    }

    #[tokio::test]
    async fn stop_and_shutdown_are_noops_without_a_task() {
        let slot = SyncTaskSlot::new();
        assert!(slot.stop(Duration::from_millis(10)).await);
        slot.shutdown().await;
    }
}
