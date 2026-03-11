use log::error;
#[cfg(target_os = "linux")]
use log::{info, warn};

pub fn show_about_dialog(log_file_path: &std::path::Path) {
    let version = env!("CARGO_PKG_VERSION");
    let log_path_str = log_file_path.to_string_lossy();

    // Check if log file exists and has content
    let log_status = if log_file_path.exists() {
        match std::fs::metadata(log_file_path) {
            Ok(metadata) if metadata.len() > 0 => "Active",
            _ => "Empty",
        }
    } else {
        "Not created"
    };

    let message = format!(
        "Cook Sync v{version}\n\n\
        © 2025 Cooklang\n\n\
        Log status: {log_status}\n\
        Log file: {log_path_str}"
    );

    let open_logs_message = format!(
        "{message}\n\n\
        Would you like to open the logs folder?"
    );

    // On Linux, rfd::MessageDialog requires GTK to be initialized, but the
    // Linux tray uses ksni (D-Bus based, no GTK). Use zenity/kdialog instead.
    #[cfg(target_os = "linux")]
    {
        if show_linux_dialog(&open_logs_message, log_file_path) {
            return;
        }
        // Fallback: show a notification with the about info
        warn!("No dialog tool available (zenity/kdialog), falling back to notification");
        let _ = crate::notifications::show_notification("About Cook Sync", &message);
    }

    #[cfg(not(target_os = "linux"))]
    {
        use rfd::MessageDialog;

        let dialog = MessageDialog::new()
            .set_title("About Cook Sync")
            .set_description(&open_logs_message)
            .set_buttons(rfd::MessageButtons::YesNo);

        let result = dialog.show();

        if result == rfd::MessageDialogResult::Yes {
            open_logs_dir(log_file_path);
        }
    }
}

fn open_logs_dir(log_file_path: &std::path::Path) {
    if let Err(e) = open::that(log_file_path.parent().unwrap_or(log_file_path)) {
        error!("Failed to open logs directory: {e}");
    }
}

#[cfg(target_os = "linux")]
fn show_linux_dialog(message: &str, log_file_path: &std::path::Path) -> bool {
    // Try zenity first (GNOME/GTK desktops)
    // Use clean_appimage_env to avoid library conflicts with bundled libs
    if let Ok(output) = crate::platform::linux::desktop_integration::clean_appimage_env("zenity")
        .args([
            "--question",
            "--title=About Cook Sync",
            &format!("--text={message}"),
            "--no-markup",
            "--ok-label=Open Logs",
            "--cancel-label=Close",
        ])
        .output()
    {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stderr.is_empty() {
            warn!("zenity stderr: {}", stderr.trim());
        }
        // zenity returns 0 for OK/Yes, 1 for Cancel/No, 5 for timeout, -1/other for errors
        match output.status.code() {
            Some(0) => {
                info!("About dialog shown via zenity (user clicked Open Logs)");
                open_logs_dir(log_file_path);
                return true;
            }
            Some(1) => {
                info!("About dialog shown via zenity (user clicked Close)");
                return true;
            }
            Some(code) => {
                warn!(
                    "zenity exited with unexpected code {}, trying fallback",
                    code
                );
                // Don't return true — fall through to try kdialog or notification
            }
            None => {
                warn!("zenity terminated by signal, trying fallback");
            }
        }
    }

    // Try kdialog (KDE desktops)
    if let Ok(output) = crate::platform::linux::desktop_integration::clean_appimage_env("kdialog")
        .args([
            "--title",
            "About Cook Sync",
            "--yesno",
            message,
            "--yes-label",
            "Open Logs",
            "--no-label",
            "Close",
        ])
        .output()
    {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stderr.is_empty() {
            warn!("kdialog stderr: {}", stderr.trim());
        }
        match output.status.code() {
            Some(0) => {
                info!("About dialog shown via kdialog (user clicked Open Logs)");
                open_logs_dir(log_file_path);
                return true;
            }
            Some(1) => {
                info!("About dialog shown via kdialog (user clicked Close)");
                return true;
            }
            Some(code) => {
                warn!(
                    "kdialog exited with unexpected code {}, trying fallback",
                    code
                );
            }
            None => {
                warn!("kdialog terminated by signal");
            }
        }
    }

    false
}
