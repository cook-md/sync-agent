#!/bin/bash
set -euo pipefail

echo "Generating Sparkle EdDSA signing keys..."

# Check if Sparkle.framework is available
if [ ! -d "/tmp/Sparkle.framework" ]; then
    echo "Downloading Sparkle.framework..."
    cd /tmp
    curl -L -o Sparkle-2.6.4.tar.xz https://github.com/sparkle-project/Sparkle/releases/download/2.6.4/Sparkle-2.6.4.tar.xz
    tar -xf Sparkle-2.6.4.tar.xz
    cd - > /dev/null
fi

# Create keys directory
KEYS_DIR="sparkle-keys"
mkdir -p "$KEYS_DIR"

# Generate keys using Sparkle's generate_keys tool
/tmp/Sparkle.framework/Resources/generate_keys "$KEYS_DIR"

echo ""
echo "✅ Keys generated successfully!"
echo ""
echo "📁 Keys location: $KEYS_DIR/"
echo "   - sparkle_private_key.pem (KEEP SECRET!)"
echo "   - sparkle_public_key.pem (add to repository)"
echo ""
echo "🔐 Next steps:"
echo "1. Store sparkle_private_key.pem securely (GitHub Secrets: SPARKLE_PRIVATE_KEY)"
echo "2. Add sparkle_public_key.pem to your repository"
echo "3. The public key will be embedded in Info.plist during build"
echo ""
echo "Public key content:"
cat "$KEYS_DIR/sparkle_public_key.pem"
