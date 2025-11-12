#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(target_os = "windows")]
pub mod windows;

use crate::error::Result;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc;
use std::sync::Arc;

pub trait PlatformIntegration {
    fn enable_auto_start(&self, app_name: &str, app_path: &str) -> Result<()>;
    fn disable_auto_start(&self, app_name: &str) -> Result<()>;
    #[allow(dead_code)]
    fn is_auto_start_enabled(&self, app_name: &str) -> Result<bool>;
    fn is_dark_mode(&self) -> bool;

    /// Watch for theme changes and send notifications through the provided channel
    /// Returns a ThemeWatcher that can be used to stop the watcher
    fn watch_theme_changes(&self, _shutdown_signal: Arc<AtomicBool>) -> Option<ThemeWatcher> {
        None
    }

    /// Check if desktop integration is installed (Linux AppImage only)
    /// Returns true if desktop entry and icons are installed
    fn is_desktop_integration_installed(&self) -> Result<bool> {
        Ok(false) // Default: not installed
    }

    /// Install desktop integration (Linux AppImage only)
    /// Creates desktop entry and installs application icons
    fn install_desktop_integration(&self) -> Result<()> {
        Ok(()) // Default no-op for non-Linux platforms
    }

    /// Uninstall desktop integration (Linux AppImage only)
    /// Removes desktop entry and application icons
    fn uninstall_desktop_integration(&self) -> Result<()> {
        Ok(()) // Default no-op for non-Linux platforms
    }
}

pub struct ThemeWatcher {
    pub receiver: mpsc::Receiver<ThemeChange>,
    #[allow(dead_code)]
    pub handle: std::thread::JoinHandle<()>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ThemeChange {
    Light,
    Dark,
}

#[cfg(target_os = "macos")]
pub fn get_platform() -> Box<dyn PlatformIntegration> {
    Box::new(macos::MacOSIntegration)
}

#[cfg(target_os = "linux")]
pub fn get_platform() -> Box<dyn PlatformIntegration> {
    Box::new(linux::LinuxIntegration)
}

#[cfg(target_os = "windows")]
pub fn get_platform() -> Box<dyn PlatformIntegration> {
    Box::new(windows::WindowsIntegration)
}
