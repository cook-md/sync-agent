use crate::error::{Result, SyncError};
use log::{debug, info, warn};
use std::path::{Path, PathBuf};

/// Check if first-run integration has been completed
pub fn is_already_integrated() -> bool {
    get_start_menu_shortcut_path().exists()
}

/// Get the path where the Start Menu shortcut should be created
fn get_start_menu_shortcut_path() -> PathBuf {
    if let Ok(appdata) = std::env::var("APPDATA") {
        PathBuf::from(appdata)
            .join("Microsoft")
            .join("Windows")
            .join("Start Menu")
            .join("Programs")
            .join("Cook Sync.lnk")
    } else {
        PathBuf::from("Cook Sync.lnk")
    }
}

/// Get the path where a Desktop shortcut would be created
fn get_desktop_shortcut_path() -> PathBuf {
    if let Ok(userprofile) = std::env::var("USERPROFILE") {
        PathBuf::from(userprofile)
            .join("Desktop")
            .join("Cook Sync.lnk")
    } else {
        PathBuf::from("Cook Sync.lnk")
    }
}

/// Create a Windows shortcut using PowerShell
fn create_shortcut(shortcut_path: &Path, target_path: &Path, description: &str) -> Result<()> {
    let shortcut_path_str = shortcut_path
        .to_str()
        .ok_or_else(|| SyncError::Other("Invalid shortcut path".to_string()))?;

    let target_path_str = target_path
        .to_str()
        .ok_or_else(|| SyncError::Other("Invalid target path".to_string()))?;

    // PowerShell script to create shortcut
    let ps_script = format!(
        r#"$WshShell = New-Object -ComObject WScript.Shell; $Shortcut = $WshShell.CreateShortcut('{}'); $Shortcut.TargetPath = '{}'; $Shortcut.Arguments = 'start'; $Shortcut.Description = '{}'; $Shortcut.WorkingDirectory = '{}'; $Shortcut.Save()"#,
        shortcut_path_str,
        target_path_str,
        description,
        target_path.parent().and_then(|p| p.to_str()).unwrap_or("")
    );

    let output = std::process::Command::new("powershell")
        .args(["-NoProfile", "-Command", &ps_script])
        .output()
        .map_err(|e| SyncError::Platform(format!("Failed to run PowerShell: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(SyncError::Platform(format!(
            "Failed to create shortcut: {}",
            stderr
        )));
    }

    Ok(())
}

/// Install Windows integration (Start Menu shortcut)
pub fn integrate_windows() -> Result<()> {
    let exe_path = std::env::current_exe()
        .map_err(|e| SyncError::Other(format!("Failed to get executable path: {}", e)))?;

    info!("Installing Windows integration...");

    // Create Start Menu shortcut
    let start_menu_path = get_start_menu_shortcut_path();

    if let Some(parent) = start_menu_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    create_shortcut(
        &start_menu_path,
        &exe_path,
        "Cook Sync - Sync recipes with Cook.md",
    )?;

    info!("Start Menu shortcut created: {}", start_menu_path.display());

    Ok(())
}

/// Install with optional Desktop shortcut
pub fn integrate_windows_with_desktop() -> Result<()> {
    // First create Start Menu shortcut
    integrate_windows()?;

    let exe_path = std::env::current_exe()
        .map_err(|e| SyncError::Other(format!("Failed to get executable path: {}", e)))?;

    // Create Desktop shortcut
    let desktop_path = get_desktop_shortcut_path();

    if let Some(parent) = desktop_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    create_shortcut(
        &desktop_path,
        &exe_path,
        "Cook Sync - Sync recipes with Cook.md",
    )?;

    info!("Desktop shortcut created: {}", desktop_path.display());

    Ok(())
}

/// Remove Windows integration
pub fn unintegrate_windows() -> Result<()> {
    info!("Removing Windows integration...");

    let start_menu_path = get_start_menu_shortcut_path();
    let desktop_path = get_desktop_shortcut_path();

    if start_menu_path.exists() {
        std::fs::remove_file(&start_menu_path)?;
        debug!("Removed Start Menu shortcut: {}", start_menu_path.display());
    }

    if desktop_path.exists() {
        std::fs::remove_file(&desktop_path)?;
        debug!("Removed Desktop shortcut: {}", desktop_path.display());
    }

    info!("Windows integration removed");

    Ok(())
}

/// Show a notification and offer integration
/// Note: On Windows, the MSI installer handles shortcuts.
/// This function is mainly for portable/non-MSI installations.
pub fn offer_integration() -> Result<()> {
    // Check if installed via MSI (registry key exists)
    if is_installed_via_msi() {
        debug!("Installed via MSI, shortcuts already created by installer");
        return Ok(());
    }

    if is_already_integrated() {
        debug!("Windows integration already complete, skipping offer");
        return Ok(());
    }

    info!("First run detected on Windows (portable mode)");

    // Show notification
    let notification_result = crate::notifications::show_notification(
        "Cook Sync",
        "Click here to add Cook Sync to your Start Menu",
    );

    // Auto-integrate (with Start Menu only, not Desktop)
    integrate_windows()?;

    if notification_result.is_ok() {
        crate::notifications::show_notification(
            "Cook Sync",
            "Cook Sync has been added to your Start Menu",
        )?;
    }

    Ok(())
}

/// Check if Cook Sync was installed via MSI installer
fn is_installed_via_msi() -> bool {
    #[cfg(target_os = "windows")]
    {
        use winreg::enums::*;
        use winreg::RegKey;

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        if let Ok(cook_sync_key) = hkcu.open_subkey("Software\\Cook.md\\CookSync") {
            if let Ok(installed) = cook_sync_key.get_value::<u32, _>("installed") {
                return installed == 1;
            }
        }
    }
    false
}
