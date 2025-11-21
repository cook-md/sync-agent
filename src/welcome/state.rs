// State management for the welcome screen
use crate::api::CookApi;
use crate::auth::AuthManager;
use crate::config::{self, settings::Settings};
use crate::error::Result;
use std::path::PathBuf;
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

    // Directory selection state
    pub recipes_dir: Option<PathBuf>,
    pub directory_error: Option<String>,

    // Preferences state
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
            recipes_dir: None,
            directory_error: None,
            preferences_expanded: true, // Default to expanded
            auto_start: true,  // Default to enabled
            auto_update: true, // Default to enabled
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
        // Accept both logged in OR login in progress
        (self.is_logged_in() || self.is_logging_in) && self.recipes_dir.is_some()
    }

    /// Check if the user can proceed (setup complete)
    pub fn can_proceed(&self) -> bool {
        self.is_setup_complete()
    }

    /// Initiate browser login (called when user clicks login button)
    pub fn start_browser_login(&mut self) {
        let login_status = self.login_status.clone();

        // Spawn login in background thread
        std::thread::spawn(move || {
            // Create tokio runtime for async login
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                // Set status to in progress
                if let Ok(mut status) = login_status.lock() {
                    *status = LoginStatus::InProgress;
                }

                // Perform the actual login
                match perform_browser_login().await {
                    Ok(email) => {
                        if let Ok(mut status) = login_status.lock() {
                            *status = LoginStatus::Success { email };
                        }
                    }
                    Err(e) => {
                        if let Ok(mut status) = login_status.lock() {
                            *status = LoginStatus::Error(e.to_string());
                        }
                    }
                }
            });
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
        self.is_logged_in() || self.is_logging_in
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
        assert!(state.auto_update);
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
    fn test_preferences_toggle() {
        let mut state = WelcomeState::default();

        assert!(state.preferences_expanded); // Now defaults to expanded
        state.toggle_preferences();
        assert!(!state.preferences_expanded);
        state.toggle_preferences();
        assert!(state.preferences_expanded);
    }
}
