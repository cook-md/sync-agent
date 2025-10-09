#!/bin/bash

# Development script for running sync-agent with local Rails server

set -e

# Function to display help
show_help() {
    echo "Cook Sync Agent Development Script"
    echo ""
    echo "Usage: $0 [--endpoint=URL] [--sync-endpoint=URL] [sync-agent-args...]"
    echo ""
    echo "Options:"
    echo "  --endpoint=URL        Specify custom API endpoint (default: http://localhost:3000)"
    echo "  --sync-endpoint=URL   Specify custom sync endpoint (default: http://127.0.0.1:8000)"
    echo "  --help                Show this help message"
    echo ""
    echo "Environment Variables:"
    echo "  COOK_ENDPOINT     Alternative way to set the API endpoint"
    echo "  SYNC_ENDPOINT     Alternative way to set the sync endpoint"
    echo "  RUST_LOG          Set logging level (default: cook_sync=debug,info)"
    echo ""
    echo "Examples:"
    echo "  $0                                    # Use default localhost:3000 + sync-server:8000"
    echo "  $0 --endpoint=https://cook.md         # Use production API for both"
    echo "  $0 --sync-endpoint=http://localhost:8001  # Use custom sync server"
    echo "  $0 start                               # Pass 'start' to sync-agent"
    echo "  $0 --endpoint=https://cook.md login   # Use custom endpoint and login"
    echo ""
    exit 0
}

# Check for help flag
if [ "$1" == "--help" ] || [ "$1" == "-h" ]; then
    show_help
fi

# Parse command line arguments
CUSTOM_ENDPOINT=""
CUSTOM_SYNC_ENDPOINT=""

while [[ $# -gt 0 ]]; do
    case $1 in
        --endpoint=*)
            CUSTOM_ENDPOINT="${1#--endpoint=}"
            shift
            ;;
        --sync-endpoint=*)
            CUSTOM_SYNC_ENDPOINT="${1#--sync-endpoint=}"
            shift
            ;;
        *)
            break # Stop parsing when we hit non-option arguments
            ;;
    esac
done

# Set defaults if not provided
if [ -z "$CUSTOM_ENDPOINT" ]; then
    CUSTOM_ENDPOINT="${COOK_ENDPOINT:-http://localhost:3000}"
fi

if [ -z "$CUSTOM_SYNC_ENDPOINT" ]; then
    CUSTOM_SYNC_ENDPOINT="${SYNC_ENDPOINT:-http://127.0.0.1:8000}"
fi

echo "🚀 Starting Cook Sync Agent in development mode..."
echo ""

# Set environment variables for development
export COOK_ENDPOINT="$CUSTOM_ENDPOINT"
export SYNC_ENDPOINT="$CUSTOM_SYNC_ENDPOINT"
export RUST_LOG="${RUST_LOG:-cook_sync=debug,info}"

# Build in debug mode
echo "📦 Building sync-agent..."
cargo build

echo ""
echo "📋 Configuration:"
echo "  • Cook Endpoint: $COOK_ENDPOINT"
echo "  • API Endpoint: $COOK_ENDPOINT/api"
echo "  • Sync Endpoint: $SYNC_ENDPOINT"
echo "  • Log Level: $RUST_LOG"
if [ $# -gt 0 ]; then
    echo "  • Arguments: $@"
fi
echo ""
echo "─────────────────────────────────────────"
echo ""

# Run the sync agent
./target/debug/cook-sync "$@"
