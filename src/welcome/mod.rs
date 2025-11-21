// Welcome screen module - shows first-time user welcome UI
mod app;
mod components;
mod state;
mod style;

pub use app::show_welcome_screen;
pub use app::WelcomeResult;
// Re-export WelcomeApp for tests
#[cfg(test)]
pub use app::WelcomeApp;

#[cfg(test)]
#[path = "mod_test.rs"]
mod mod_test;
