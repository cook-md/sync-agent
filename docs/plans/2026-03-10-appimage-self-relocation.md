# AppImage Self-Relocation Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** On first AppImage launch from a transient location (~/Downloads, /tmp, ~/Desktop), offer to move the AppImage to ~/Applications/ and re-exec from there.

**Architecture:** New `check_and_relocate_appimage()` function in the existing `desktop_integration` module of `src/platform/linux.rs`, called from `main.rs` before the existing `check_and_auto_install()`. Uses zenity for the GUI prompt, `std::fs::rename`/copy+delete for the move, and `std::os::unix::process::CommandExt::exec()` for re-launch.

**Tech Stack:** Rust, zenity (system dependency, optional), std::process::Command

---

### Task 1: Add `is_transient_location()` helper

**Files:**
- Modify: `src/platform/linux.rs:15-286` (inside `desktop_integration` module)

**Step 1: Write the function**

Add after `is_running_from_appimage()` (after line 34), inside the `desktop_integration` module:

```rust
    /// Check if the given path is in a transient location where AppImages shouldn't live
    fn is_transient_location(appimage_path: &Path) -> bool {
        let path_str = appimage_path.to_string_lossy();

        // /tmp/ but NOT /tmp/.mount_* (AppImage runtime mount point)
        if path_str.starts_with("/tmp/") && !path_str.starts_with("/tmp/.mount_") {
            return true;
        }

        // Check against home-relative transient dirs
        if let Some(home) = dirs::home_dir() {
            let transient_dirs = [
                home.join("Downloads"),
                home.join("Desktop"),
            ];

            for dir in &transient_dirs {
                if appimage_path.starts_with(dir) {
                    return true;
                }
            }
        }

        false
    }
```

**Step 2: Verify it compiles**

Run: `cargo build 2>&1 | head -5`
Expected: compiles (warning about dead code is OK)

**Step 3: Commit**

```bash
git add src/platform/linux.rs
git commit -m "feat: add transient location detection for AppImage relocation"
```

---

### Task 2: Add `relocation_declined()` and `mark_relocation_declined()` helpers

**Files:**
- Modify: `src/platform/linux.rs:15-286` (inside `desktop_integration` module)

**Step 1: Write the functions**

Add after `is_transient_location()`, inside the `desktop_integration` module:

```rust
    /// Path to the marker file that records the user declined relocation
    fn relocation_declined_path() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("cook-sync").join("relocation-declined"))
    }

    /// Check if user previously declined relocation
    fn relocation_declined() -> bool {
        relocation_declined_path()
            .map(|p| p.exists())
            .unwrap_or(false)
    }

    /// Record that the user declined relocation
    fn mark_relocation_declined() {
        if let Some(path) = relocation_declined_path() {
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let _ = fs::write(&path, "declined");
        }
    }
```

**Step 2: Verify it compiles**

Run: `cargo build 2>&1 | head -5`
Expected: compiles

**Step 3: Commit**

```bash
git add src/platform/linux.rs
git commit -m "feat: add relocation decline persistence helpers"
```

---

### Task 3: Add `show_relocation_dialog()` using zenity

**Files:**
- Modify: `src/platform/linux.rs:15-286` (inside `desktop_integration` module)

**Step 1: Write the function**

Add after `mark_relocation_declined()`, inside the `desktop_integration` module:

```rust
    /// Show a zenity dialog asking the user if they want to move the AppImage.
    /// Returns true if user accepted, false if declined or zenity unavailable.
    fn show_relocation_dialog() -> bool {
        let result = std::process::Command::new("zenity")
            .args([
                "--question",
                "--title=Move Cook Sync?",
                "--text=Cook Sync is running from your Downloads folder.\n\nMove to ~/Applications/ for permanent installation?",
                "--ok-label=Yes, move it",
                "--cancel-label=No thanks",
                "--no-markup",
            ])
            .output();

        match result {
            Ok(output) => output.status.success(),
            Err(e) => {
                debug!("zenity not available, skipping relocation prompt: {}", e);
                false
            }
        }
    }
```

**Step 2: Verify it compiles**

Run: `cargo build 2>&1 | head -5`
Expected: compiles

**Step 3: Commit**

```bash
git add src/platform/linux.rs
git commit -m "feat: add zenity relocation dialog"
```

---

### Task 4: Add `move_appimage()` for file relocation

**Files:**
- Modify: `src/platform/linux.rs:15-286` (inside `desktop_integration` module)

**Step 1: Write the function**

Add after `show_relocation_dialog()`, inside the `desktop_integration` module:

```rust
    /// Move the AppImage to ~/Applications/cook-sync.AppImage.
    /// Tries rename first (fast, same filesystem), falls back to copy+delete.
    fn move_appimage(source: &Path) -> Result<PathBuf> {
        let target_dir = dirs::home_dir()
            .ok_or_else(|| SyncError::Platform("Cannot determine home directory".into()))?
            .join("Applications");

        fs::create_dir_all(&target_dir).map_err(|e| {
            SyncError::Platform(format!(
                "Cannot create ~/Applications/: {}",
                e
            ))
        })?;

        let target = target_dir.join("cook-sync.AppImage");

        // Try rename first (atomic, same filesystem)
        if fs::rename(source, &target).is_ok() {
            info!("Moved AppImage: {} -> {}", source.display(), target.display());
            return Ok(target);
        }

        // Cross-filesystem: copy then delete original
        fs::copy(source, &target).map_err(|e| {
            SyncError::Platform(format!(
                "Failed to copy AppImage to {}: {}",
                target.display(),
                e
            ))
        })?;

        // Delete original (best effort — copy succeeded so the new location works)
        if let Err(e) = fs::remove_file(source) {
            error!(
                "AppImage copied to {} but failed to remove original {}: {}",
                target.display(),
                source.display(),
                e
            );
        }

        info!("Moved AppImage: {} -> {} (copy+delete)", source.display(), target.display());
        Ok(target)
    }
```

**Step 2: Verify it compiles**

Run: `cargo build 2>&1 | head -5`
Expected: compiles

**Step 3: Commit**

```bash
git add src/platform/linux.rs
git commit -m "feat: add AppImage file relocation with rename/copy fallback"
```

---

### Task 5: Add the main `check_and_relocate()` orchestrator

**Files:**
- Modify: `src/platform/linux.rs:15-286` (inside `desktop_integration` module, add as `pub fn`)

**Step 1: Write the function**

Add after `move_appimage()`, inside the `desktop_integration` module:

```rust
    /// Check if the AppImage should be relocated from a transient location.
    /// If yes, moves it to ~/Applications/ and re-execs from the new path.
    /// This function only returns if relocation was skipped or declined.
    /// On successful relocation, the current process is replaced via exec().
    pub fn check_and_relocate() -> Result<()> {
        use std::os::unix::process::CommandExt;

        let appimage_path = match get_appimage_path() {
            Ok(p) => p,
            Err(_) => return Ok(()), // Not an AppImage
        };

        if !is_transient_location(&appimage_path) {
            return Ok(()); // Already in a good location
        }

        if relocation_declined() {
            return Ok(()); // User said no before
        }

        if !show_relocation_dialog() {
            mark_relocation_declined();
            return Ok(()); // User said no
        }

        let new_path = match move_appimage(&appimage_path) {
            Ok(p) => p,
            Err(e) => {
                error!("Failed to relocate AppImage: {}", e);
                return Ok(()); // Continue from current location
            }
        };

        // Re-exec from the new location, preserving all arguments
        let args: Vec<String> = std::env::args().collect();
        let err = std::process::Command::new(&new_path)
            .args(&args[1..])
            .exec();

        // exec() only returns on error
        error!("Failed to re-exec from {}: {}", new_path.display(), err);
        Ok(())
    }
```

**Step 2: Verify it compiles**

Run: `cargo build 2>&1 | head -5`
Expected: compiles

**Step 3: Commit**

```bash
git add src/platform/linux.rs
git commit -m "feat: add check_and_relocate orchestrator for AppImage self-relocation"
```

---

### Task 6: Wire up `check_and_relocate()` in main.rs

**Files:**
- Modify: `src/main.rs:107-109`

**Step 1: Add the relocation call before auto-install**

Change lines 107-109 in `src/main.rs` from:

```rust
    // Auto-install desktop integration on first launch (Linux AppImage only)
    #[cfg(target_os = "linux")]
    check_and_auto_install()?;
```

To:

```rust
    // Relocate AppImage from transient locations on first launch (Linux only)
    #[cfg(target_os = "linux")]
    check_and_relocate_appimage()?;

    // Auto-install desktop integration on first launch (Linux AppImage only)
    #[cfg(target_os = "linux")]
    check_and_auto_install()?;
```

**Step 2: Add the wrapper function**

Add after the `check_and_auto_install()` function (after line 707 in `src/main.rs`):

```rust
#[cfg(target_os = "linux")]
fn check_and_relocate_appimage() -> Result<()> {
    use platform::linux::desktop_integration;

    // Only relevant when running from AppImage
    if !is_running_from_appimage() {
        return Ok(());
    }

    // Skip if user explicitly wants to manage integration manually
    let args: Vec<String> = std::env::args().collect();
    if args
        .iter()
        .any(|arg| arg == "uninstall" || arg == "install")
    {
        return Ok(());
    }

    match desktop_integration::check_and_relocate() {
        Ok(_) => Ok(()),
        Err(e) => {
            error!("AppImage relocation check failed: {}", e);
            Ok(()) // Non-fatal, continue
        }
    }
}
```

**Step 3: Verify it compiles**

Run: `cargo build 2>&1 | head -5`
Expected: compiles

Note: The `desktop_integration` module is currently `mod` (private). If the compiler complains about visibility, change line 15 of `src/platform/linux.rs` from `mod desktop_integration` to `pub mod desktop_integration`. Also ensure `check_and_relocate` is `pub fn`.

**Step 4: Commit**

```bash
git add src/main.rs src/platform/linux.rs
git commit -m "feat: wire up AppImage self-relocation on first launch"
```

---

### Task 7: Run clippy and fix any warnings

**Step 1: Run clippy**

Run: `cargo clippy --all-targets -- -D warnings -A dead_code`

**Step 2: Fix any warnings**

Address all clippy warnings in the changed code.

**Step 3: Run formatter**

Run: `cargo fmt`

**Step 4: Run tests**

Run: `cargo test`

**Step 5: Commit if any fixes were needed**

```bash
git add src/platform/linux.rs src/main.rs
git commit -m "fix: address clippy warnings in AppImage relocation"
```
