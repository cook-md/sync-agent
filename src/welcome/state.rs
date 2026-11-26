// State management for the welcome screen
use crate::api::CookApi;
use crate::auth::AuthManager;
use crate::config::{self, settings::Settings};
use crate::error::Result;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub enum LoginStatus {
    NotStarted,
    InProgress,
    Success { email: String },
    Error(String),
}

#[derive(Clone)]
pub struct WelcomeState {
    // Authentication state
    pub login_status: Arc<Mutex<LoginStatus>>,
    pub user_email: Option<String>,
    pub login_error: Option<String>,
    pub is_logging_in: bool,
    /// User chose to sign in later from the tray menu
    pub login_skipped: bool,
    /// Id of the latest browser login attempt; results from older
    /// (abandoned) attempts are ignored
    login_attempt: Arc<AtomicU64>,

    // Directory selection state
    pub recipes_dir: Option<PathBuf>,
    pub directory_error: Option<String>,

    // Preferences state
    #[allow(dead_code)]
    pub preferences_expanded: bool,
    pub auto_start: bool,
    pub auto_update: bool,

    // UI state
    pub should_close: bool,
}

impl Default for WelcomeState {
    fn default() -> Self {
        Self {
            login_status: Arc::new(Mutex::new(LoginStatus::NotStarted)),
            user_email: None,
            login_error: None,
            is_logging_in: false,
            login_skipped: false,
            login_attempt: Arc::new(AtomicU64::new(0)),
            recipes_dir: None,
            directory_error: None,
            preferences_expanded: false, // Hidden until Steps 1 & 2 complete (per Figma)
            auto_start: true,            // Default to enabled
            auto_update: true,           // Default to enabled
            should_close: false,
        }
    }
}

impl WelcomeState {
    pub fn is_logged_in(&self) -> bool {
        if let Ok(status) = self.login_status.lock() {
            matches!(*status, LoginStatus::Success { .. })
        } else {
            false
        }
    }

    pub fn update_from_login_status(&mut self) {
        if let Ok(status) = self.login_status.lock() {
            match &*status {
                LoginStatus::NotStarted => {
                    self.is_logging_in = false;
                    self.user_email = None;
                    self.login_error = None;
                }
                LoginStatus::InProgress => {
                    self.is_logging_in = true;
                    self.login_error = None;
                }
                LoginStatus::Success { email } => {
                    self.is_logging_in = false;
                    self.user_email = Some(email.clone());
                    self.login_error = None;
                }
                LoginStatus::Error(err) => {
                    self.is_logging_in = false;
                    self.user_email = None;
                    self.login_error = Some(err.clone());
                }
            }
        }
    }

    /// Check if all required setup steps are complete
    pub fn is_setup_complete(&self) -> bool {
        // A pending login does not count: the browser callback is served by
        // this process, so closing the window would abandon the sign-in.
        self.is_step1_complete() && self.recipes_dir.is_some()
    }

    /// Defer sign-in to the tray menu
    pub fn skip_login(&mut self) {
        self.login_skipped = true;
    }

    /// Check if the user can proceed (setup complete)
    pub fn can_proceed(&self) -> bool {
        self.is_setup_complete()
    }

    /// Start a new login attempt and mark it in progress. Returns its id.
    pub fn begin_login_attempt(&mut self) -> u64 {
        // Signing in after "Skip for now" makes the step required again, so
        // the window can't close while the browser callback is pending.
        self.login_skipped = false;
        let id = self.login_attempt.fetch_add(1, Ordering::SeqCst) + 1;
        if let Ok(mut status) = self.login_status.lock() {
            *status = LoginStatus::InProgress;
        }
        id
    }

    /// Record the outcome of a login attempt. A success from any attempt is
    /// kept (its session is already saved); a failure only counts if it comes
    /// from the latest attempt, since an older one was superseded by
    /// "Open browser again".
    pub fn finish_login_attempt(&self, id: u64, result: LoginStatus) {
        if let Ok(mut status) = self.login_status.lock() {
            if matches!(*status, LoginStatus::Success { .. }) {
                return;
            }
            let is_latest = self.login_attempt.load(Ordering::SeqCst) == id;
            if is_latest || matches!(result, LoginStatus::Success { .. }) {
                *status = result;
            }
        }
    }

    /// Initiate browser login (called when user clicks login button)
    pub fn start_browser_login(&mut self) {
        let id = self.begin_login_attempt();
        self.is_logging_in = true;
        self.login_error = None;
        let state = self.clone();

        // Spawn login in background thread
        std::thread::spawn(move || {
            let result = match tokio::runtime::Runtime::new() {
                Ok(rt) => match rt.block_on(perform_browser_login()) {
                    Ok(email) => LoginStatus::Success { email },
                    Err(e) => LoginStatus::Error(e.to_string()),
                },
                Err(e) => LoginStatus::Error(e.to_string()),
            };
            state.finish_login_attempt(id, result);
        });
    }

    /// Set the recipes directory
    pub fn set_recipes_dir(&mut self, dir: PathBuf) {
        self.recipes_dir = Some(dir);
        self.directory_error = None;
    }

    /// Set a directory error
    pub fn set_directory_error(&mut self, error: String) {
        self.directory_error = Some(error);
    }

    /// Toggle preferences panel
    #[allow(dead_code)]
    pub fn toggle_preferences(&mut self) {
        self.preferences_expanded = !self.preferences_expanded;
    }

    /// Request to close the window and start the app
    pub fn request_close(&mut self) {
        if self.can_proceed() {
            self.should_close = true;
        }
    }

    /// Check if step 1 (authentication) is complete
    pub fn is_step1_complete(&self) -> bool {
        self.is_logged_in() || self.login_skipped
    }

    /// Check if step 1 is in progress
    pub fn is_step1_in_progress(&self) -> bool {
        self.is_logging_in && !self.is_logged_in()
    }

    /// Check if step 2 (directory) is complete
    pub fn is_step2_complete(&self) -> bool {
        self.recipes_dir.is_some()
    }
}

/// Perform browser-based login
async fn perform_browser_login() -> Result<String> {
    let config = config::Config::new()?;
    let api_endpoint = Settings::get_api_endpoint();
    let api = CookApi::new(api_endpoint)?;
    let auth = AuthManager::new(config.paths(), Arc::new(api))?;

    // Perform browser-based login
    auth.browser_login().await?;

    // Get session to extract email
    if let Some(session) = auth.get_session() {
        Ok(session.email.unwrap_or(session.user_id))
    } else {
        Err(crate::error::SyncError::Other(
            "Login succeeded but no session found".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_state() {
        let state = WelcomeState::default();
        assert!(!state.is_logged_in());
        assert!(!state.is_setup_complete());
        assert!(!state.can_proceed());
        assert!(state.auto_start);
        assert!(state.auto_update); // Defaults to true
    }

    #[test]
    fn test_login_flow() {
        let mut state = WelcomeState::default();

        // Simulate login in progress
        if let Ok(mut status) = state.login_status.lock() {
            *status = LoginStatus::InProgress;
        }
        state.update_from_login_status();
        assert!(state.is_logging_in);
        assert!(state.is_step1_in_progress());

        // Simulate login success
        if let Ok(mut status) = state.login_status.lock() {
            *status = LoginStatus::Success {
                email: "user@example.com".to_string(),
            };
        }
        state.update_from_login_status();
        assert!(state.is_logged_in());
        assert!(!state.is_logging_in);
        assert_eq!(state.user_email, Some("user@example.com".to_string()));
        assert!(state.is_step1_complete());
    }

    #[test]
    fn test_setup_completion() {
        let mut state = WelcomeState::default();

        // Not complete with just login
        if let Ok(mut status) = state.login_status.lock() {
            *status = LoginStatus::Success {
                email: "user@example.com".to_string(),
            };
        }
        state.update_from_login_status();
        assert!(!state.is_setup_complete());

        // Complete with both login and directory
        state.set_recipes_dir(PathBuf::from("/tmp/recipes"));
        assert!(state.is_setup_complete());
        assert!(state.can_proceed());
    }

    #[test]
    fn test_login_error() {
        let mut state = WelcomeState::default();

        // Simulate login error
        if let Ok(mut status) = state.login_status.lock() {
            *status = LoginStatus::Error("Connection failed".to_string());
        }
        state.update_from_login_status();

        assert!(!state.is_logged_in());
        assert!(!state.is_logging_in);
        assert_eq!(state.login_error, Some("Connection failed".to_string()));
    }

    #[test]
    fn test_login_in_progress_does_not_complete_setup() {
        // Regression: "Get Started" used to be enabled while the browser
        // sign-in was still pending, and closing the window killed the
        // callback listener so the sign-in could never finish.
        let mut state = WelcomeState::default();
        state.set_recipes_dir(PathBuf::from("/tmp/recipes"));
        if let Ok(mut status) = state.login_status.lock() {
            *status = LoginStatus::InProgress;
        }
        state.update_from_login_status();

        assert!(state.is_step1_in_progress());
        assert!(!state.is_step1_complete());
        assert!(!state.can_proceed());
    }

    #[test]
    fn test_skip_login_allows_proceeding() {
        let mut state = WelcomeState::default();
        state.set_recipes_dir(PathBuf::from("/tmp/recipes"));
        assert!(!state.can_proceed());

        state.skip_login();
        assert!(state.is_step1_complete());
        assert!(state.can_proceed());
    }

    #[test]
    fn test_stale_login_attempt_result_is_ignored() {
        let mut state = WelcomeState::default();
        let first = state.begin_login_attempt();
        let second = state.begin_login_attempt();

        // The abandoned first attempt times out after the retry started
        state.finish_login_attempt(first, LoginStatus::Error("Authentication timeout".into()));
        assert!(matches!(
            *state.login_status.lock().unwrap(),
            LoginStatus::InProgress
        ));

        state.finish_login_attempt(
            second,
            LoginStatus::Success {
                email: "user@example.com".into(),
            },
        );
        assert!(state.is_logged_in());
    }

    #[test]
    fn test_login_after_skip_blocks_proceeding_until_done() {
        let mut state = WelcomeState::default();
        state.set_recipes_dir(PathBuf::from("/tmp/recipes"));
        state.skip_login();
        assert!(state.can_proceed());

        // Changing their mind and signing in must not leave "Get Started"
        // enabled while the browser callback is pending.
        state.begin_login_attempt();
        state.update_from_login_status();
        assert!(!state.can_proceed());
    }

    #[test]
    fn test_late_success_from_earlier_attempt_is_kept() {
        // User clicked "Open browser again", but finished the sign-in in the
        // first tab: its token is saved, so the welcome screen must say so.
        let mut state = WelcomeState::default();
        let first = state.begin_login_attempt();
        let second = state.begin_login_attempt();

        state.finish_login_attempt(
            first,
            LoginStatus::Success {
                email: "user@example.com".into(),
            },
        );
        assert!(state.is_logged_in());

        // The retry later times out; that must not undo the success
        state.finish_login_attempt(second, LoginStatus::Error("Authentication timeout".into()));
        assert!(state.is_logged_in());
    }

    #[test]
    fn test_preferences_toggle() {
        let mut state = WelcomeState::default();

        assert!(!state.preferences_expanded); // Defaults to collapsed per Figma
        state.toggle_preferences();
        assert!(state.preferences_expanded);
        state.toggle_preferences();
        assert!(!state.preferences_expanded);
    }
}
