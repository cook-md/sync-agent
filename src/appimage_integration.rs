#[cfg(target_os = "linux")]
use crate::error::{Result, SyncError};
#[cfg(target_os = "linux")]
use log::{debug, info, warn};
#[cfg(target_os = "linux")]
use std::fs;
#[cfg(target_os = "linux")]
use std::path::{Path, PathBuf};

/// Check if we're running from an AppImage
#[cfg(target_os = "linux")]
pub fn is_running_from_appimage() -> bool {
    std::env::var("APPIMAGE").is_ok()
}

/// Get the AppImage path if running from AppImage
#[cfg(target_os = "linux")]
pub fn get_appimage_path() -> Option<PathBuf> {
    std::env::var("APPIMAGE").ok().map(PathBuf::from)
}

/// Check if the AppImage is already integrated (desktop entry exists)
#[cfg(target_os = "linux")]
pub fn is_already_integrated() -> bool {
    let desktop_file_path = get_desktop_file_path();
    desktop_file_path.exists()
}

/// Get the path where the desktop entry should be installed
#[cfg(target_os = "linux")]
fn get_desktop_file_path() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("~/.local/share"))
        .join("applications")
        .join("cook-sync.desktop")
}

/// Get the path where the icon should be installed
#[cfg(target_os = "linux")]
fn get_icon_path() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("~/.local/share"))
        .join("icons")
        .join("hicolor")
        .join("256x256")
        .join("apps")
        .join("cook-sync.png")
}

/// Extract icon from AppImage
#[cfg(target_os = "linux")]
fn extract_icon(_appimage_path: &Path) -> Result<Vec<u8>> {
    // When running from AppImage, APPDIR points to the mounted filesystem
    // Try to read icon from the mounted AppImage first
    if let Ok(appdir) = std::env::var("APPDIR") {
        let appdir_path = PathBuf::from(appdir);

        // Try multiple icon locations
        let icon_paths = vec![
            appdir_path.join("usr/share/icons/hicolor/256x256/apps/cook-sync.png"),
            appdir_path.join("usr/share/cook-sync/icon_black.png"),
            appdir_path.join(".DirIcon"),
        ];

        for icon_path in icon_paths {
            if icon_path.exists() {
                if let Ok(icon_data) = fs::read(&icon_path) {
                    debug!("Found icon at: {}", icon_path.display());
                    return Ok(icon_data);
                }
            }
        }
    }

    // Fallback: Try to find icon using the current executable path
    // This works when the AppImage is mounted
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let icon_paths = vec![
                exe_dir.join("../share/icons/hicolor/256x256/apps/cook-sync.png"),
                exe_dir.join("../share/cook-sync/icon_black.png"),
            ];

            for icon_path in icon_paths {
                if icon_path.exists() {
                    if let Ok(icon_data) = fs::read(&icon_path) {
                        debug!("Found icon at: {}", icon_path.display());
                        return Ok(icon_data);
                    }
                }
            }
        }
    }

    Err(SyncError::Other(
        "Failed to find icon in AppImage".to_string(),
    ))
}

/// Install the desktop entry and icon
#[cfg(target_os = "linux")]
pub fn integrate_appimage() -> Result<()> {
    let appimage_path = get_appimage_path()
        .ok_or_else(|| SyncError::Other("Not running from AppImage".to_string()))?;

    info!("Integrating AppImage into system menu...");

    // Create directories
    let desktop_file_path = get_desktop_file_path();
    let icon_path = get_icon_path();

    if let Some(parent) = desktop_file_path.parent() {
        fs::create_dir_all(parent)?;
    }

    if let Some(parent) = icon_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Extract and install icon
    match extract_icon(&appimage_path) {
        Ok(icon_data) => {
            fs::write(&icon_path, icon_data)?;
            info!("Icon installed to: {}", icon_path.display());
        }
        Err(e) => {
            warn!("Failed to extract icon: {}, will use system default", e);
        }
    }

    // Create desktop entry
    let desktop_entry = format!(
        r#"[Desktop Entry]
Name=Cook Sync
Comment=Sync recipes with Cook.md
Exec={} start
Icon=cook-sync
Terminal=false
Type=Application
Categories=Utility;FileTools;
StartupNotify=false
X-GNOME-Autostart-enabled=true
X-AppImage-Version={}
X-AppImage-Path={}
"#,
        appimage_path.display(),
        env!("CARGO_PKG_VERSION"),
        appimage_path.display()
    );

    fs::write(&desktop_file_path, desktop_entry)?;
    info!(
        "Desktop entry installed to: {}",
        desktop_file_path.display()
    );

    // Update desktop database
    std::process::Command::new("update-desktop-database")
        .arg(desktop_file_path.parent().unwrap())
        .output()
        .ok();

    // Update icon cache
    std::process::Command::new("gtk-update-icon-cache")
        .arg(
            icon_path
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .parent()
                .unwrap(),
        )
        .output()
        .ok();

    info!("AppImage integrated successfully!");
    info!("Cook Sync will now appear in your applications menu");

    Ok(())
}

/// Uninstall the desktop entry and icon
#[cfg(target_os = "linux")]
pub fn unintegrate_appimage() -> Result<()> {
    info!("Removing AppImage integration...");

    let desktop_file_path = get_desktop_file_path();
    let icon_path = get_icon_path();

    if desktop_file_path.exists() {
        fs::remove_file(&desktop_file_path)?;
        debug!("Removed desktop entry: {}", desktop_file_path.display());
    }

    if icon_path.exists() {
        fs::remove_file(&icon_path)?;
        debug!("Removed icon: {}", icon_path.display());
    }

    // Update desktop database
    if let Some(parent) = desktop_file_path.parent() {
        std::process::Command::new("update-desktop-database")
            .arg(parent)
            .output()
            .ok();
    }

    info!("AppImage integration removed");

    Ok(())
}

/// Show a notification offering to integrate the AppImage
#[cfg(target_os = "linux")]
pub fn offer_integration() -> Result<()> {
    if !is_running_from_appimage() {
        return Ok(());
    }

    if is_already_integrated() {
        debug!("AppImage already integrated, skipping offer");
        return Ok(());
    }

    info!("First run from AppImage detected");

    // Show notification with option to integrate
    let notification_result = crate::notifications::show_notification(
        "Cook Sync",
        "Click here to add Cook Sync to your applications menu",
    );

    // For now, just integrate automatically
    // In the future, we could make this interactive
    integrate_appimage()?;

    if notification_result.is_ok() {
        crate::notifications::show_notification(
            "Cook Sync",
            "Cook Sync has been added to your applications menu",
        )?;
    }

    Ok(())
}
