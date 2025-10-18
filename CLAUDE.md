# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Cook Sync Agent is a lightweight, cross-platform synchronization agent for Cook.md that runs in the background and syncs recipes. It's built in Rust and replaces the deprecated Tauri-based desktop app.

## Development Commands

### Building and Running

```bash
# Standard development build
cargo build

# Run tests
cargo test

# Run specific test with output
cargo test test_name -- --nocapture

# Format code
cargo fmt

# Lint code
cargo clippy

# Build release version
cargo build --release

# Run the sync agent
./target/debug/cook-sync [command]
```

### Local Development with Rails

When developing with the Rails server locally, use the development script:

```bash
# Default: uses localhost:3000 for API and localhost:8000 for sync
./scripts/dev.sh start

# Custom endpoints
./scripts/dev.sh --endpoint=https://cook.md start
./scripts/dev.sh --sync-endpoint=http://localhost:8001 start

# Login with custom endpoint
./scripts/dev.sh --endpoint=https://cook.md login
```

### Common CLI Commands

```bash
# Start the sync agent daemon
cook-sync start

# Stop the daemon
cook-sync stop

# Check status
cook-sync status

# Login (opens browser)
cook-sync login

# Logout
cook-sync logout

# Configure settings
cook-sync config --recipes-dir ~/Documents/CookRecipes
cook-sync config --auto-start true
cook-sync config --auto-update true
cook-sync config --show

# Check for updates
cook-sync update

# Install desktop integration (AppImage/Windows)
cook-sync install

# Uninstall desktop integration
cook-sync uninstall
```

## Architecture Overview

### Core Components

1. **Main Entry Point** (`src/main.rs`)
   - CLI command parsing using clap
   - Daemon lifecycle management (start/stop/status)
   - Configuration management
   - Platform-specific integration (AppImage on Linux, Start Menu on Windows)

2. **Daemon** (`src/daemon.rs`)
   - Runs the background sync process
   - Manages system tray integration
   - Coordinates sync intervals and JWT token rotation
   - Handles PID file management for process tracking

3. **Authentication** (`src/auth/`)
   - OAuth browser-based login flow
   - JWT token management with automatic rotation (daily)
   - Secure session storage using platform keyrings (macOS Keychain, Linux Secret Service, Windows Credential Manager)
   - Session persistence across restarts

4. **Sync Manager** (`src/sync/`)
   - Uses `cooklang-sync-client` crate for actual syncing
   - Manages sync intervals (default 12 seconds, configurable)
   - Handles sync status reporting

5. **Configuration** (`src/config/`)
   - Settings stored in platform-specific locations:
     - macOS: `~/Library/Application Support/cook-sync/`
     - Linux: `~/.config/cook-sync/`
     - Windows: `%APPDATA%\cook-sync\`
   - Manages recipes directory, auto-start, auto-update preferences

6. **Updates** (`src/updates/`)
   - Custom package update system (not using self_update crate directly)
   - Downloads and verifies signed update packages
   - Platform-specific installer execution

7. **Platform Integration** (`src/platform/`)
   - Platform-specific auto-start implementation
   - System tray icon management (light/dark mode support)
   - Desktop integration (AppImage on Linux, shortcuts on Windows)

8. **API Client** (`src/api/`)
   - HTTP client for Cook.md API
   - Handles authentication endpoints and sync operations
   - Environment-aware endpoint configuration

### Environment Variables

- `COOK_ENDPOINT`: Base URL for Cook.md (default: `https://cook.md`)
  - Used for both OAuth login and API calls
  - API calls use `$COOK_ENDPOINT/api`
- `SYNC_ENDPOINT`: Sync server endpoint (default: derived from COOK_ENDPOINT)
- `RUST_LOG`: Logging configuration (e.g., `cook_sync=debug,info`)
- `SENTRY_DSN`: Sentry error tracking endpoint (production only)

### Key Design Decisions

1. **No Traditional Daemonization**: The agent avoids fork-based daemonization to maintain GUI/display server access required for system tray
2. **Secure Token Storage**: Uses platform-native keyrings instead of file-based storage for authentication tokens
3. **Graceful Migration**: Automatically detects and migrates from the old Tauri desktop app
4. **Cross-platform Consistency**: Single binary works across macOS, Linux, and Windows with platform-specific adaptations

## Testing Approach

### Running Tests

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test module
cargo test auth::tests

# Run with debug logging
RUST_LOG=debug cargo test
```

### Key Test Areas

- Authentication flow (`src/auth/*_test.rs`)
- Sync manager operations (`src/sync/*_test.rs`)
- JWT token handling and rotation
- Secure session storage and retrieval
- Configuration persistence

## Release Process

The project uses GitHub Actions for CI/CD:

1. **release-please.yml**: Unified workflow that manages version bumping, changelog generation, builds platform packages (AppImage, DMG, MSI), and publishes releases
2. **sync-agent-ci.yml**: Runs tests and linting on PRs

The release-please workflow uses conditional job execution - build and packaging jobs only run when a release is created. Releases are signed and include platform-specific packages with auto-update support.

## Dependencies Note

- Uses `cooklang-sync-client` crate for sync functionality
- Platform-specific dependencies are conditionally compiled
- Minimal external dependencies for security and size optimization