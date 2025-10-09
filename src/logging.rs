use crate::error::Result;
use env_logger::{Builder, Env, Target};
use log::LevelFilter;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::Mutex;

static LOG_FILE: Mutex<Option<std::fs::File>> = Mutex::new(None);

// Maximum log file size before rotation (10 MB)
const MAX_LOG_SIZE: u64 = 10 * 1024 * 1024;
// Maximum number of rotated log files to keep
const MAX_ROTATED_LOGS: usize = 5;

/// Rotate log files if the current log exceeds the maximum size
fn rotate_logs_if_needed(log_file_path: &Path) -> Result<()> {
    // Check if log file exists and its size
    if let Ok(metadata) = fs::metadata(log_file_path) {
        if metadata.len() > MAX_LOG_SIZE {
            // Perform rotation
            let log_dir = log_file_path.parent().unwrap_or_else(|| Path::new("."));
            let log_name = log_file_path.file_name().unwrap_or_default();

            // Shift existing rotated logs
            for i in (1..MAX_ROTATED_LOGS).rev() {
                let old_path = log_dir.join(format!("{}.{}", log_name.to_string_lossy(), i));
                let new_path = log_dir.join(format!("{}.{}", log_name.to_string_lossy(), i + 1));
                if old_path.exists() {
                    fs::rename(&old_path, &new_path).ok();
                }
            }

            // Rotate current log to .1
            let rotated_path = log_dir.join(format!("{}.1", log_name.to_string_lossy()));
            fs::rename(log_file_path, rotated_path)?;

            // Remove oldest log if it exists
            let oldest_path = log_dir.join(format!(
                "{}.{}",
                log_name.to_string_lossy(),
                MAX_ROTATED_LOGS + 1
            ));
            if oldest_path.exists() {
                fs::remove_file(oldest_path).ok();
            }
        }
    }

    Ok(())
}

pub fn init_logging(log_file_path: &Path) -> Result<()> {
    // Rotate logs if needed before opening
    rotate_logs_if_needed(log_file_path)?;

    // Open or create the log file
    let log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_file_path)?;

    // Store the file handle for use by the logger
    *LOG_FILE.lock().unwrap() = Some(log_file);

    // Initialize env_logger with custom output
    let mut builder = Builder::from_env(Env::default().default_filter_or("info"));

    // Configure to write to both stderr and our log file
    builder.target(Target::Stderr);
    builder.format(move |buf, record| {
        // Format the log message
        let formatted = format!(
            "[{}] {} {}",
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
            record.level(),
            record.args()
        );

        // Write to stderr (console)
        writeln!(buf, "{}", &formatted)?;

        // Also write to log file if available
        if let Ok(mut guard) = LOG_FILE.lock() {
            if let Some(ref mut file) = *guard {
                writeln!(file, "{}", &formatted).ok();
                file.flush().ok();
            }
        }

        Ok(())
    });

    // Set the log level from environment or default
    if let Ok(rust_log) = std::env::var("RUST_LOG") {
        builder.parse_filters(&rust_log);
    } else {
        builder.filter_level(LevelFilter::Info);
    }

    builder.init();

    Ok(())
}
