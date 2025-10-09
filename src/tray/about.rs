use log::error;
use rfd::MessageDialog;

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

    // Create the dialog content with options
    let message = format!(
        "Cook Sync v{version}\n\n\
        © 2025 Cooklang\n\n\
        Log status: {log_status}\n\
        Log file: {log_path_str}\n\n\
        Would you like to open the logs folder?"
    );

    // Use rfd for cross-platform native dialogs with Yes/No buttons
    let dialog = MessageDialog::new()
        .set_title("About Cook Sync")
        .set_description(&message)
        .set_buttons(rfd::MessageButtons::YesNo);

    let result = dialog.show();

    // If user clicked "Yes", open the log directory
    if result == rfd::MessageDialogResult::Yes {
        if let Err(e) = open::that(log_file_path.parent().unwrap_or(log_file_path)) {
            error!("Failed to open logs directory: {e}");
        }
    }
}
