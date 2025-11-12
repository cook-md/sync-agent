# Cook Sync Agent

A lightweight, cross-platform sync agent for Cook.md that runs in the background and syncs your recipes with Cook.md.

## Features

- **Cross-platform**: Works on macOS, Linux, and Windows
- **System tray integration**: Shows sync status and provides quick actions
- **Automatic syncing**: Syncs recipes every 12 seconds (configurable)
- **OAuth authentication**: Browser-based login flow for security
- **JWT token rotation**: Automatically refreshes authentication tokens daily
- **Auto-start**: Can be configured to start with the system
- **Auto-updates**: Multiple update mechanisms for seamless updates
- **Lightweight**: Pure Rust implementation without web technology dependencies

## Installation

### macOS

**Homebrew (recommended):**
```bash
brew install cooklang/tap/cook-sync
```

**DMG:**
Download from [releases](https://github.com/Cooklang/sync-agent/releases/latest)
- Includes Sparkle framework for automatic updates

**Shell installer:**
```bash
curl -sSL https://github.com/Cooklang/sync-agent/releases/latest/download/cook-sync-installer.sh | sh
```

### Windows

**MSI installer:**
Download from [releases](https://github.com/Cooklang/sync-agent/releases/latest)

**PowerShell:**
```powershell
irm https://github.com/Cooklang/sync-agent/releases/latest/download/cook-sync-installer.ps1 | iex
```

### Linux

**Shell installer (recommended):**
```bash
curl -sSL https://github.com/Cooklang/sync-agent/releases/latest/download/cook-sync-installer.sh | sh
```

**AppImage:**
Download from [releases](https://github.com/Cooklang/sync-agent/releases/latest)

1. Download the AppImage:
   ```bash
   wget https://github.com/Cooklang/sync-agent/releases/latest/download/cook-sync-x86_64.AppImage
   ```

2. Make it executable:
   ```bash
   chmod +x cook-sync-*.AppImage
   ```

3. Double-click the AppImage or run:
   ```bash
   ./cook-sync-*.AppImage start
   ```

**Desktop Integration:**

On first launch, Cook Sync will automatically:
- Install desktop integration (menu entry and icons)
- Add itself to your application menu
- Set up the system tray icon

You can then launch Cook Sync from your application menu like any other app.

**Manual Desktop Integration:**

If you prefer manual control:

```bash
# Install desktop integration
./cook-sync-*.AppImage install

# Uninstall desktop integration
cook-sync uninstall
```

**Note:** If you move the AppImage file after installation, you'll need to uninstall and reinstall desktop integration from the new location.

### From Source

```bash
# Build from source
cargo build --release

# The binary will be at: target/release/cook-sync
# Run it directly or copy it to your PATH
./target/release/cook-sync start
```

### Linux Requirements

**System Tray Support:**

The system tray icon requires specific packages depending on your desktop environment:

**GNOME Users:**
The system tray requires the AppIndicator extension:
```bash
sudo apt install gnome-shell-extension-appindicator
gnome-extensions enable appindicator@ubuntu.com
```
Then restart GNOME Shell (Alt+F2, type 'r', press Enter).

**XFCE Users:**
Install the indicator plugin:
```bash
sudo apt install xfce4-indicator-plugin
```
Then add the Indicator Plugin to your panel.

**KDE/MATE Users:**
System tray works out of the box - no additional setup needed.

**Development Dependencies:**

If building from source, you'll need these packages:

**Ubuntu/Debian:**
```bash
sudo apt install libayatana-appindicator3-dev libgtk-3-dev libxdo-dev
```

**Fedora/RHEL:**
```bash
sudo dnf install libappindicator-gtk3-devel gtk3-devel
```

**Arch Linux:**
```bash
sudo pacman -S libappindicator-gtk3 gtk3
```

## Usage

```bash
# Start the sync agent
cook-sync start

# Show status
cook-sync status

# Configure recipes directory
cook-sync config --recipes-dir ~/Documents/CookRecipes

# Enable auto-start
cook-sync config --auto-start true

# Login (opens browser)
cook-sync login

# Check for updates
cook-sync update --check-only

# Install updates
cook-sync update

# Install desktop integration (Linux AppImage only)
cook-sync install

# Uninstall desktop integration (Linux AppImage only)
cook-sync uninstall

# Stop the agent
cook-sync stop
```

## Auto-Updates

Cook Sync automatically checks for updates and notifies you when new versions are available.

The update mechanism varies by installation method:
- **Homebrew**: Use `brew upgrade cook-sync`
- **DMG**: Updates via Sparkle framework (automatic)
- **MSI/Shell/PowerShell**: Built-in updater via `cook-sync update`
- **AppImage**: Built-in AppImageUpdate support

You can manually check for updates at any time:
```bash
cook-sync update --check-only
```

## Configuration

Configuration files are stored in:
- macOS: `~/Library/Application Support/cook-sync/`
- Linux: `~/.config/cook-sync/`
- Windows: `%APPDATA%\cook-sync\`

## Development

### Local Development Setup

When developing with a local Rails server:

1. Start the Rails server on localhost:3000:
   ```bash
   cd ../web
   bundle exec rails server
   ```

2. Use the development script to run sync-agent:
   ```bash
   ./scripts/dev.sh start
   ```

   This script automatically sets:
   - `COOK_ENDPOINT=http://localhost:3000`
   - `RUST_LOG=cook_sync=debug,info`

3. Or manually set environment variables:
   ```bash
   export COOK_ENDPOINT=http://localhost:3000
   cargo run
   ```

### Environment Variables

- `COOK_ENDPOINT`: Base URL for Cook.md (default: `https://cook.md`)
  - Used for both OAuth login and API calls
  - API calls will use `$COOK_ENDPOINT/api`
- `RUST_LOG`: Logging configuration (e.g., `cook_sync=debug`)

See `.env.example` for more details.

### Standard Development Commands

```bash
# Run in development mode
cargo run

# Run tests
cargo test

# Format code
cargo fmt

# Check for issues
cargo clippy
```

## Architecture

The sync agent is built with:
- **Rust**: For performance and cross-platform compatibility
- **cooklang-sync-client**: For syncing with CookCloud
- **tray-icon**: For system tray integration
- **tokio**: For async runtime
- **axoupdater**: For cross-platform auto-updates (shell/PowerShell/MSI)
- **Sparkle**: For macOS .app bundle updates
- **AppImageUpdate**: For Linux AppImage updates

## Migration from Desktop App

This sync agent replaces the deprecated Tauri-based desktop app. It will automatically:
- Detect existing desktop app installations
- Migrate your authentication and settings
- Continue syncing from where the desktop app left off

The desktop app will be automatically updated to this new sync agent through the auto-update mechanism.

## License

Copyright (c) Alexey Dubovskoy