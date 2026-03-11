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

**DMG:**
Download from [releases](https://github.com/cook-md/sync-agent/releases/latest)
- Supports automatic updates

### Windows

**Installer (.exe):**
Download from [releases](https://github.com/cook-md/sync-agent/releases/latest)

### Linux

**AppImage:**
Download from [releases](https://github.com/cook-md/sync-agent/releases/latest)

1. Download the `.AppImage` file from the [releases page](https://github.com/cook-md/sync-agent/releases/latest)

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

Cook Sync runs as a background agent with a system tray icon. It also provides a command line interface:

```
cook-sync [COMMAND]

Commands:
  start       Start the sync agent daemon
  stop        Stop the running sync agent
  status      Show sync status
  login       Open browser for login
  logout      Logout and clear session
  config      Configure sync settings
  update      Check for updates
  install     Install desktop integration (Linux AppImage only)
  uninstall   Uninstall desktop integration (Linux AppImage only)
  reset       Reset all configuration and data (stops daemon if running)
```

### Examples

```bash
# Start the sync agent
cook-sync start

# Show status
cook-sync status

# Login (opens browser)
cook-sync login

# Configure recipes directory
cook-sync config --recipes-dir ~/Documents/CookRecipes

# Enable auto-start and auto-update
cook-sync config --auto-start true
cook-sync config --auto-update true

# Show current configuration
cook-sync config --show

# Reset all data (with confirmation prompt)
cook-sync reset

# Stop the agent
cook-sync stop
```

## Auto-Updates

Cook Sync automatically checks for updates and notifies you when new versions are available.

The update mechanism varies by installation method:
- **DMG**: Updates via built-in updater
- **Windows**: Built-in updater via `cook-sync update`
- **AppImage**: Built-in updater via `cook-sync update`

You can check for updates at any time:
```bash
cook-sync update
```

## Configuration

Configuration files are stored in:
- macOS: `~/Library/Application Support/cook-sync/`
- Linux: `~/.config/cook-sync/`
- Windows: `%APPDATA%\cook-sync\`

## Development

See [DEVELOPMENT.md](DEVELOPMENT.md) for development setup, environment variables, and architecture details.

## License

Copyright (c) Alexey Dubovskoy