#!/bin/bash
set -e

VERSION="$1"

if [ -z "$VERSION" ]; then
  echo "Usage: $0 <version>"
  exit 1
fi

# Create app bundle structure
mkdir -p "Cook Sync.app/Contents/MacOS"
mkdir -p "Cook Sync.app/Contents/Resources"

# Copy binary
cp target/release/cook-sync "Cook Sync.app/Contents/MacOS/"

# Copy icon resources
for icon in icon_black.png icon_white.png; do
  [ -f "assets/$icon" ] && cp "assets/$icon" "Cook Sync.app/Contents/Resources/" || echo "⚠️ Warning: $icon not found"
done

# Copy macOS app icon
if [ -f assets/package.icns ]; then
  cp assets/package.icns "Cook Sync.app/Contents/Resources/AppIcon.icns"
elif [ -f assets/icon.icns ]; then
  cp assets/icon.icns "Cook Sync.app/Contents/Resources/AppIcon.icns"
fi

# Create launcher script
cat > "Cook Sync.app/Contents/MacOS/Cook Sync Launcher" << 'EOF'
#!/bin/bash
exec "$(dirname "$0")/cook-sync" start
EOF
chmod +x "Cook Sync.app/Contents/MacOS/Cook Sync Launcher"

# Create Info.plist
cat > "Cook Sync.app/Contents/Info.plist" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleExecutable</key>
  <string>Cook Sync Launcher</string>
  <key>CFBundleIdentifier</key>
  <string>md.cook.sync</string>
  <key>CFBundleName</key>
  <string>Cook Sync</string>
  <key>CFBundleIconFile</key>
  <string>AppIcon</string>
  <key>CFBundleShortVersionString</key>
  <string>${VERSION}</string>
  <key>CFBundleVersion</key>
  <string>${VERSION}</string>
  <key>LSMinimumSystemVersion</key>
  <string>10.15</string>
  <key>LSUIElement</key>
  <true/>
</dict>
</plist>
EOF

# Sign the app bundle if certificate is available
if [ -n "$APPLE_SIGNING_IDENTITY" ]; then
  echo "Signing app bundle..."
  codesign --force --deep --options runtime --sign "$APPLE_SIGNING_IDENTITY" \
    --entitlements resources/macos/entitlements.plist \
    --timestamp --verbose "Cook Sync.app" || echo "Warning: Signing failed"
fi

# Create DMG
echo "Creating DMG..."
create-dmg \
  --volname "Cook Sync" \
  --volicon "Cook Sync.app/Contents/Resources/AppIcon.icns" \
  --window-pos 200 120 \
  --window-size 500 320 \
  --icon-size 100 \
  --icon "Cook Sync.app" 125 160 \
  --app-drop-link 375 160 \
  "CookSync-${VERSION}.dmg" \
  "Cook Sync.app" || echo "Warning: DMG creation with create-dmg failed, trying hdiutil..."

# Fallback to hdiutil if create-dmg fails
if [ ! -f "CookSync-${VERSION}.dmg" ]; then
  echo "Using hdiutil as fallback..."
  hdiutil create -volname "Cook Sync" -srcfolder "Cook Sync.app" -ov -format UDZO "CookSync-${VERSION}.dmg"
fi

# Notarize if credentials are available
if [ -n "$APPLE_ID" ] && [ -n "$APPLE_PASSWORD" ] && [ -n "$APPLE_TEAM_ID" ]; then
  echo "Notarizing DMG..."
  xcrun notarytool submit "CookSync-${VERSION}.dmg" \
    --apple-id "$APPLE_ID" \
    --password "$APPLE_PASSWORD" \
    --team-id "$APPLE_TEAM_ID" \
    --wait || echo "Warning: Notarization failed"

  xcrun stapler staple "CookSync-${VERSION}.dmg" || echo "Warning: Stapling failed"
fi

echo "DMG_PATH=sync-agent/CookSync-${VERSION}.dmg" >> $GITHUB_ENV
