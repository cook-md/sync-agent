# Development

## Local Development Setup

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

## Environment Variables

- `COOK_ENDPOINT`: Base URL for Cook.md (default: `https://cook.md`)
  - Used for both OAuth login and API calls
  - API calls will use `$COOK_ENDPOINT/api`
- `COOK_SYNC_ENDPOINT`: Override sync server endpoint (default: derived from `COOK_ENDPOINT`)
- `RUST_LOG`: Logging configuration (e.g., `cook_sync=debug`)

See `.env.example` for more details.

## Standard Development Commands

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
- **cargo-packager-updater**: For cross-platform auto-updates
