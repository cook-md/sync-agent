#!/bin/bash

# Test script for OAuth flow with configurable endpoint

set -e

echo "Testing OAuth flow for Cook Sync Agent"
echo ""

# Check if endpoint is provided
COOK_ENDPOINT=${1:-"https://cook.md"}

echo "Configuration:"
echo "  Cook Endpoint: $COOK_ENDPOINT"
echo "  API Endpoint: $COOK_ENDPOINT/api"
echo ""

# Generate a random state
STATE=$(openssl rand -hex 16)
CALLBACK_URL="http://localhost:12345/auth/callback"

echo "OAuth Flow Test:"
echo "1. The sync-agent would open this URL in a browser:"
echo "   $COOK_ENDPOINT/auth/desktop?callback=$CALLBACK_URL&state=$STATE"
echo ""
echo "2. After successful login, the browser would redirect to:"
echo "   $CALLBACK_URL?token=JWT_TOKEN_HERE&state=$STATE"
echo ""
echo "3. The sync-agent would extract the token and save it locally."
echo ""

# Test with curl (optional)
read -p "Would you like to test the endpoint with curl? (y/n) " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    echo "Testing endpoint..."
    curl -I "$COOK_ENDPOINT/auth/desktop?callback=$CALLBACK_URL&state=$STATE" 2>/dev/null | head -n 1
fi