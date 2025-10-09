#!/bin/bash
set -euo pipefail

VERSION=${1:-"0.0.1"}

echo "Building Cook Sync ${VERSION} with Sparkle support..."

# Build universal binary
echo "Building for x86_64-apple-darwin..."
cargo build --release --target x86_64-apple-darwin

echo "Building for aarch64-apple-darwin..."
cargo build --release --target aarch64-apple-darwin

# Create universal binary
echo "Creating universal binary..."
lipo -create \
  target/x86_64-apple-darwin/release/cook-sync \
  target/aarch64-apple-darwin/release/cook-sync \
  -output cook-sync-universal

# Create app bundle structure
APP_DIR="Cook Sync.app"
rm -rf "$APP_DIR"
mkdir -p "$APP_DIR/Contents/"{MacOS,Resources,Frameworks}

# Copy binary
cp cook-sync-universal "$APP_DIR/Contents/MacOS/cook-sync"
chmod +x "$APP_DIR/Contents/MacOS/cook-sync"

# Copy Sparkle framework (required for auto-updates)
SPARKLE_SOURCE="/tmp/Sparkle.framework"

if [ ! -d "$SPARKLE_SOURCE" ]; then
    echo "Downloading Sparkle.framework..."
    cd /tmp
    curl -L -o Sparkle-2.6.4.tar.xz https://github.com/sparkle-project/Sparkle/releases/download/2.6.4/Sparkle-2.6.4.tar.xz
    tar -xf Sparkle-2.6.4.tar.xz
    cd - > /dev/null
fi

if [ -d "$SPARKLE_SOURCE" ]; then
    echo "Copying Sparkle.framework..."
    cp -R "$SPARKLE_SOURCE" "$APP_DIR/Contents/Frameworks/"
    echo "✅ Sparkle.framework bundled"
else
    echo "❌ ERROR: Sparkle.framework not found!"
    echo "Please download from: https://github.com/sparkle-project/Sparkle/releases"
    exit 1
fi

# Create Info.plist
cat > "$APP_DIR/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>cook-sync</string>
    <key>CFBundleIdentifier</key>
    <string>org.cooklang.sync</string>
    <key>CFBundleName</key>
    <string>Cook Sync</string>
    <key>CFBundleVersion</key>
    <string>${VERSION}</string>
    <key>CFBundleShortVersionString</key>
    <string>${VERSION}</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>LSMinimumSystemVersion</key>
    <string>10.13</string>
    <key>SUFeedURL</key>
    <string>https://raw.githubusercontent.com/Cooklang/sync-agent/main/appcast.xml</string>
    <key>SUPublicEDKey</key>
    <string>${SPARKLE_PUBLIC_KEY}</string>
    <key>SUEnableAutomaticChecks</key>
    <true/>
    <key>SUAutomaticallyUpdate</key>
    <false/>
    <key>SUAllowsAutomaticUpdates</key>
    <true/>
</dict>
</plist>
EOF

# Copy icon (create placeholder if doesn't exist)
if [ -f "resources/icon.icns" ]; then
    cp resources/icon.icns "$APP_DIR/Contents/Resources/"
else
    echo "Warning: resources/icon.icns not found, app will use default icon"
fi

# Code sign app bundle (if certificates available)
if [ -n "${APPLE_CERTIFICATE:-}" ]; then
    echo "Signing app bundle..."
    if [ -d "$APP_DIR/Contents/Frameworks/Sparkle.framework" ]; then
        codesign --deep --force --options runtime \
          --sign "$APPLE_CERTIFICATE" \
          "$APP_DIR/Contents/Frameworks/Sparkle.framework"
    fi

    codesign --deep --force --options runtime \
      --sign "$APPLE_CERTIFICATE" \
      "$APP_DIR"
    echo "App bundle signed"
else
    echo "Skipping code signing (APPLE_CERTIFICATE not set)"
fi

# Create .zip for Sparkle updates (more efficient than DMG for updates)
echo "Creating Sparkle update package..."
rm -f "cook-sync-${VERSION}.app.zip"
ditto -c -k --keepParent "$APP_DIR" "cook-sync-${VERSION}.app.zip"

# Generate or use existing Sparkle keys
SPARKLE_KEYS_DIR="sparkle-keys"
PRIVATE_KEY="${SPARKLE_KEYS_DIR}/sparkle_private_key.pem"
PUBLIC_KEY_BASE64_FILE="${SPARKLE_KEYS_DIR}/sparkle_public_key_base64.txt"

if [ ! -f "$PRIVATE_KEY" ] || [ ! -f "$PUBLIC_KEY_BASE64_FILE" ]; then
    echo "Generating Sparkle signing keys..."
    ./scripts/generate-sparkle-keys-openssl.sh
    echo "✅ Sparkle keys generated in $SPARKLE_KEYS_DIR/"
    echo "⚠️  IMPORTANT: Store sparkle_private_key.pem securely (e.g., GitHub Secrets)"
    echo "⚠️  Public key has been generated for Info.plist"
fi

# Read the public key for Info.plist (base64 format)
if [ -f "$PUBLIC_KEY_BASE64_FILE" ]; then
    SPARKLE_PUBLIC_KEY=$(cat "$PUBLIC_KEY_BASE64_FILE")
    echo "Using Sparkle public key from $PUBLIC_KEY_BASE64_FILE"
else
    SPARKLE_PUBLIC_KEY="<!-- No public key found -->"
    echo "Warning: No Sparkle public key found"
fi

# Generate Sparkle signature for the update package
if [ -f "$PRIVATE_KEY" ]; then
    echo "Signing update package..."
    ./scripts/sign-update-package.sh "cook-sync-${VERSION}.app.zip" "$PRIVATE_KEY"
    echo "✅ Update package signed"
else
    echo "⚠️  Skipping Sparkle signing (private key not found)"
fi

# Create DMG for initial distribution
if command -v create-dmg &> /dev/null; then
    echo "Creating DMG..."
    create-dmg \
      --volname "Cook Sync" \
      --window-pos 200 120 \
      --window-size 600 400 \
      --icon-size 100 \
      --icon "Cook Sync.app" 175 120 \
      --hide-extension "Cook Sync.app" \
      --app-drop-link 425 120 \
      "cook-sync-${VERSION}.dmg" \
      "$APP_DIR"
    echo "✅ Created cook-sync-${VERSION}.dmg"
else
    echo "Warning: create-dmg not installed. Install with: brew install create-dmg"
    echo "Creating simple DMG with hdiutil instead..."
    hdiutil create -volname "Cook Sync" -srcfolder "$APP_DIR" -ov -format UDZO "cook-sync-${VERSION}.dmg"
    echo "✅ Created cook-sync-${VERSION}.dmg (basic format)"
fi

echo "✅ Created cook-sync-${VERSION}.app.zip (for Sparkle updates)"
echo ""
echo "Build complete! Files created:"
ls -lh cook-sync-${VERSION}.*
