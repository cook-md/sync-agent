#!/bin/bash
set -euo pipefail

PACKAGE=${1:-}
PRIVATE_KEY=${2:-"sparkle-keys/sparkle_private_key.pem"}

if [ -z "$PACKAGE" ]; then
    echo "Usage: $0 <package-file> [private-key-path]"
    echo "Example: $0 cook-sync-0.1.0.app.zip"
    exit 1
fi

if [ ! -f "$PACKAGE" ]; then
    echo "Error: Package file not found: $PACKAGE"
    exit 1
fi

if [ ! -f "$PRIVATE_KEY" ]; then
    echo "Error: Private key not found: $PRIVATE_KEY"
    exit 1
fi

echo "Signing package: $PACKAGE"
echo "Using private key: $PRIVATE_KEY"

# Sign the package using Ed25519
# Sparkle expects the signature in base64 format
SIGNATURE=$(openssl pkeyutl -sign -inkey "$PRIVATE_KEY" -rawin -in "$PACKAGE" | base64)

# Save signature to file
SIGNATURE_FILE="${PACKAGE}.sig"
echo "$SIGNATURE" > "$SIGNATURE_FILE"

echo "✅ Signature created: $SIGNATURE_FILE"
echo ""
echo "Signature (base64):"
echo "$SIGNATURE"
