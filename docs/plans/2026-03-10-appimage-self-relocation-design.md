# AppImage Self-Relocation Design

## Problem

When users download the AppImage to `~/Downloads/` and double-click it, the Downloads folder becomes the permanent install location. This is awkward — Downloads is a transient location users expect to clean up.

## Solution

On first AppImage launch, detect if running from a transient location and offer to move to `~/Applications/` via a zenity dialog. If user accepts, move the file and re-exec from the new path.

## Flow

1. Detect if AppImage is in a transient location (`~/Downloads/`, `/tmp/` excluding mount points, `~/Desktop/`)
2. Check if user previously declined (presence of `~/.config/cook-sync/relocation-declined`)
3. Show zenity dialog asking to move to `~/Applications/`
4. If **yes**: move file to `~/Applications/cook-sync.AppImage`, re-exec from new path
5. If **no**: create decline marker file, continue from current location
6. Existing desktop integration (`.desktop` file, icons) runs after this — points to final location

## Transient Location Detection

Check if the AppImage path starts with:
- `~/Downloads/`
- `/tmp/` (but not `/tmp/.mount_*` which is the AppImage runtime mount)
- `~/Desktop/`

## Target Path

Always `~/Applications/cook-sync.AppImage` — normalized filename regardless of downloaded name (e.g. `cook-sync_0.8.3_x86_64.AppImage` → `cook-sync.AppImage`). Overwrites silently if previous version exists.

## Decline Persistence

Simple file presence check: `~/.config/cook-sync/relocation-declined`. No config values needed.

## Re-launch Mechanism

1. `std::fs::rename()` for same-filesystem moves, copy+delete for cross-filesystem
2. `std::os::unix::process::CommandExt::exec()` to replace current process with new path, preserving arguments

## Zenity Dialog

```
zenity --question \
  --title="Move Cook Sync?" \
  --text="Cook Sync is running from your Downloads folder.\n\nMove to ~/Applications/ for permanent installation?" \
  --ok-label="Yes, move it" \
  --cancel-label="No thanks" \
  --no-markup
```

## Error Handling

- Zenity not installed: skip relocation silently (current behavior)
- Move fails (permissions, disk full): log error, continue from current location
- `~/Applications/` can't be created: log error, continue

## Code Location

New function `check_and_relocate_appimage()` in `src/platform/linux.rs` `desktop_integration` module, called from `main.rs` **before** `check_and_auto_install()`.
