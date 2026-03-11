use crate::error::{Result, SyncError};
use log::error;
use notify_rust::Notification;

#[cfg(target_os = "macos")]
use std::sync::Once;

#[cfg(target_os = "macos")]
static MACOS_APP_INIT: Once = Once::new();

#[cfg(target_os = "macos")]
fn ensure_macos_application() {
    MACOS_APP_INIT.call_once(|| {
        // Must set the bundle identifier before sending any notification on macOS.
        // Without this, mac-notification-sys tries to look up an app called "use_default"
        // which triggers a macOS "Choose Application" dialog.
        if let Err(e) = notify_rust::set_application("com.cook.sync-agent") {
            error!("Failed to set macOS notification application: {}", e);
        }
    });
}

/// Show a simple notification with title and message
pub fn show_notification(title: &str, message: &str) -> Result<()> {
    #[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
    {
        #[cfg(target_os = "macos")]
        ensure_macos_application();

        let mut notification = Notification::new();
        notification
            .summary(title)
            .body(message)
            .appname("Cook Sync");

        notification.show().map_err(|e| {
            error!("Failed to show notification: {}", e);
            SyncError::Platform(format!("Failed to show notification: {}", e))
        })?;

        Ok(())
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        error!("Notifications not supported on this platform");
        Err(SyncError::Platform(
            "Notifications not supported on this platform".to_string(),
        ))
    }
}

/// Show an update notification with a custom action
#[allow(dead_code)]
#[cfg(target_os = "macos")]
pub fn show_update_dialog(version: &str, notes: &str) -> Result<bool> {
    // For interactive dialogs on macOS, we still use osascript
    // because notify-rust doesn't support buttons/actions well on macOS
    let script = format!(
        r#"display dialog "Cook Sync {} is available.\n\n{}\n\nNote: Installation may require administrator privileges." buttons {{"Later", "Install"}} default button "Install" with title "Cook Sync Update""#,
        version, notes
    );

    let output = std::process::Command::new("osascript")
        .args(["-e", &script])
        .output()
        .map_err(|e| SyncError::Platform(format!("Failed to show dialog: {}", e)))?;

    Ok(output.status.success() && String::from_utf8_lossy(&output.stdout).contains("Install"))
}

#[allow(dead_code)]
#[cfg(target_os = "windows")]
pub fn show_update_dialog(version: &str, notes: &str) -> Result<bool> {
    use std::ptr::null_mut;
    use winapi::um::winuser::{MessageBoxW, IDYES, MB_ICONINFORMATION, MB_YESNO};

    let title = format!("Cook Sync Update - v{}", version);
    let message = format!(
        "Cook Sync {} is available.\n\n{}\n\nNote: Installation will require administrator privileges.\n\nWould you like to install it now?",
        version, notes
    );

    let title_wide: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
    let message_wide: Vec<u16> = message.encode_utf16().chain(std::iter::once(0)).collect();

    unsafe {
        let result = MessageBoxW(
            null_mut(),
            message_wide.as_ptr(),
            title_wide.as_ptr(),
            MB_YESNO | MB_ICONINFORMATION,
        );

        Ok(result == IDYES)
    }
}

#[allow(dead_code)]
#[cfg(target_os = "linux")]
pub fn show_update_dialog(version: &str, notes: &str) -> Result<bool> {
    // First try to show a notification
    let _ = show_notification(
        "Cook Sync Update Available",
        &format!(
            "Version {} is ready to install. Check your Downloads folder.",
            version
        ),
    );

    // For GUI dialog with privilege warning
    // Use clean_appimage_env to avoid library conflicts with bundled libs
    if let Ok(output) = crate::platform::linux::desktop_integration::clean_appimage_env("zenity")
        .args([
            "--question",
            "--title=Cook Sync Update",
            "--text",
            &format!(
                "Cook Sync {} is available.\n\n{}\n\nNote: Installation will require administrator privileges.\n\nWould you like to install it now?",
                version, notes
            ),
        ])
        .output()
    {
        Ok(output.status.success())
    } else {
        // Fallback: just show notification and return false
        Ok(false)
    }
}
