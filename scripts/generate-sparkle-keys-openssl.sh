#!/bin/bash
set -euo pipefail

echo "Generating Sparkle EdDSA signing keys using OpenSSL..."

# Create keys directory
KEYS_DIR="sparkle-keys"
mkdir -p "$KEYS_DIR"

# Generate Ed25519 private key
openssl genpkey -algorithm ED25519 -out "$KEYS_DIR/sparkle_private_key.pem"

# Extract public key
openssl pkey -in "$KEYS_DIR/sparkle_private_key.pem" -pubout -out "$KEYS_DIR/sparkle_public_key.pem"

# For Sparkle, we need the public key in base64 format (without headers)
PUBLIC_KEY_BASE64=$(grep -v "BEGIN PUBLIC KEY" "$KEYS_DIR/sparkle_public_key.pem" | grep -v "END PUBLIC KEY" | tr -d '\n')

echo ""
echo "✅ Keys generated successfully!"
echo ""
echo "📁 Keys location: $KEYS_DIR/"
echo "   - sparkle_private_key.pem (KEEP SECRET!)"
echo "   - sparkle_public_key.pem"
echo ""
echo "🔐 Next steps:"
echo "1. Store sparkle_private_key.pem securely (GitHub Secrets: SPARKLE_PRIVATE_KEY)"
echo "2. Use this base64 public key in Info.plist:"
echo ""
echo "$PUBLIC_KEY_BASE64"
echo ""

# Save the base64 version for convenience
echo "$PUBLIC_KEY_BASE64" > "$KEYS_DIR/sparkle_public_key_base64.txt"

echo "Base64 key also saved to: $KEYS_DIR/sparkle_public_key_base64.txt"
