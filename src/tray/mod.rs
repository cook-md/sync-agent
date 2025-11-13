// Platform-specific tray implementations
// Linux uses ksni (StatusNotifierItem via D-Bus, no GTK event loop required)
// macOS/Windows use tray-icon (requires winit event loop)

pub mod about;

// Platform-specific menu module
#[cfg(not(target_os = "linux"))]
pub mod menu;

// Linux-specific tray implementation using ksni
#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "linux")]
pub use linux::SystemTray;

// macOS/Windows tray implementation using tray-icon
#[cfg(not(target_os = "linux"))]
mod tray_icon_impl;

#[cfg(not(target_os = "linux"))]
pub use tray_icon_impl::SystemTray;
