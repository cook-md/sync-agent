// Library module for cook-sync
// This allows modules to be used in tests and other binaries

// Allow dead code for platform-specific implementations
// Each platform (Linux/macOS/Windows) has exclusive code paths
#![allow(dead_code)]

pub mod api;
pub mod appimage_integration;
pub mod auth;
pub mod config;
pub mod daemon;
pub mod error;
pub mod logging;
pub mod notifications;
pub mod platform;
pub mod sentry_integration;
pub mod sync;
pub mod tray;
pub mod updates;

#[cfg(target_os = "windows")]
pub mod windows_integration;

pub use error::{Result, SyncError};
