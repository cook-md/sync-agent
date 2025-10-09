#!/bin/bash

set -e

echo "Building and testing Cook Sync Agent locally..."

# Build the agent
cargo build

echo -e "\n=== Testing CLI commands ===\n"

# Test help
echo "1. Testing help command:"
./target/debug/cook-sync --help

echo -e "\n2. Testing status command:"
./target/debug/cook-sync status

echo -e "\n3. Testing config command:"
./target/debug/cook-sync config --show

echo -e "\nTo test the full sync functionality:"
echo "  1. Run: ./target/debug/cook-sync start"
echo "  2. In another terminal, run: ./target/debug/cook-sync login"
echo "  3. Configure recipes directory: ./target/debug/cook-sync config --recipes-dir ~/Documents/Recipes"
echo "  4. Check status: ./target/debug/cook-sync status"
echo "  5. Stop the agent: ./target/debug/cook-sync stop"