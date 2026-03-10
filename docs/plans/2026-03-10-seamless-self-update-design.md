# Seamless Self-Update Design

**Date:** 2026-03-10

## Problem

- macOS: Downloads DMG, opens Finder. User must quit, drag-and-drop to /Applications, relaunch.
- Linux: Replaces AppImage binary but never restarts. User must find file and relaunch.
- Windows: Works well (NSIS handles everything).

## Root Causes

1. macOS updater code bypasses `cargo-packager-updater`'s `download_and_install()` with a custom DMG download path
2. No restart logic after `download_and_install()` on any platform

## Solution

### A) macOS: Switch from DMG to tar.gz for auto-updates

- CI builds tar.gz of the signed/notarized .app bundle alongside the DMG
- Update manifest points to tar.gz URLs instead of DMG
- DMG still built for first-time downloads from website
- Remove custom DMG code in `src/updater/mod.rs`; use `download_and_install()` on all platforms
- `download_and_install()` handles atomic .app replacement with AppleScript elevation if needed

### B) Add restart_app() for all platforms

- macOS: `open -n /path/to/Cook\ Sync.app --args start` then `exit(0)`
- Linux: `exec()` using `$APPIMAGE` env var (fallback: `current_exe()`)
- Windows: no-op (NSIS already handles restart)

### C) Auto-restart flow

1. Detect update → download and install (atomic replacement)
2. Show notification: "Updated to vX. Restarting..."
3. 2s delay for notification visibility
4. Graceful sync manager shutdown
5. Auto-restart via platform-specific mechanism

## Files Changed

| File | Change |
|------|--------|
| `src/updater/mod.rs` | Remove macOS DMG block, unify to `download_and_install()`, add `restart_app()` |
| `src/daemon.rs` | Call restart after successful update |
| `src/tray/tray_icon_impl.rs` | Update messages, call restart after manual update check |
| `src/tray/linux.rs` | Update messages, call restart after manual update check |
| `scripts/generate-manifest.sh` | macOS URLs point to tar.gz, update signature file patterns |
| `.github/workflows/release-please.yml` | Add tar.gz creation + signing step for macOS builds |
