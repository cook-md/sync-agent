#!/bin/bash
set -e

VERSION="$1"
LINUXDEPLOY="$2"

if [ -z "$VERSION" ] || [ -z "$LINUXDEPLOY" ]; then
  echo "Usage: $0 <version> <linuxdeploy-path>"
  exit 1
fi

# Prepare directories
mkdir -p pkg/usr/local/bin
mkdir -p pkg/usr/share/applications
mkdir -p pkg/usr/share/cook-sync
mkdir -p pkg/etc/systemd/user

# Copy binary
cp target/release/cook-sync pkg/usr/local/bin/

# Copy tray icons (needed at runtime)
for icon in icon_black.png icon_white.png icon_black_tray_*.png icon_white_tray_*.png; do
  [ -f "assets/$icon" ] && cp "assets/$icon" pkg/usr/share/cook-sync/ || true
done

# Copy application icons in multiple sizes for desktop entry
for size in 16 22 32 48 128 256; do
  mkdir -p pkg/usr/share/icons/hicolor/${size}x${size}/apps
  if [ -f "assets/icon-${size}.png" ]; then
    cp "assets/icon-${size}.png" pkg/usr/share/icons/hicolor/${size}x${size}/apps/cook-sync.png
  fi
done

# Copy desktop file
cp resources/cook-sync.desktop pkg/usr/share/applications/cook-sync.desktop
# Update Exec path for system installation
sed -i 's|Exec=cook-sync|Exec=/usr/local/bin/cook-sync|g' pkg/usr/share/applications/cook-sync.desktop

# Create systemd service
cat > pkg/etc/systemd/user/cook-sync.service << EOF
[Unit]
Description=Cook Sync Agent
After=network.target

[Service]
Type=simple
ExecStart=/usr/local/bin/cook-sync daemon
Restart=on-failure

[Install]
WantedBy=default.target
EOF

# Build DEB package
fpm -s dir -t deb \
  -n cook-sync \
  -v "$VERSION" \
  --description "Cook.md sync agent" \
  --url "https://cook.md" \
  --maintainer "Cook.md <support@cook.md>" \
  --license "MIT" \
  --category "utils" \
  --depends "libgtk-3-0" \
  --depends "libappindicator3-1 | libayatana-appindicator3-1" \
  --depends "gir1.2-appindicator3-0.1 | gir1.2-ayatanaappindicator3-0.1" \
  --depends "xdg-user-dirs" \
  --after-install scripts/post-install.sh \
  -C pkg \
  .

# Build RPM package
fpm -s dir -t rpm \
  -n cook-sync \
  -v "$VERSION" \
  --description "Cook.md sync agent" \
  --url "https://cook.md" \
  --maintainer "Cook.md <support@cook.md>" \
  --license "MIT" \
  --category "Applications/Internet" \
  --depends "gtk3" \
  --depends "libappindicator" \
  --depends "xdg-utils" \
  --after-install scripts/post-install.sh \
  -C pkg \
  .

# Build AppImage with bundled dependencies
echo "============================================"
echo "Building AppImage..."
echo "============================================"

# Prepare icon (linuxdeploy needs properly named icon files)
echo "🎨 Preparing icons in multiple sizes..."
for size in 16 22 32 48 128 256; do
  mkdir -p AppDir/usr/share/icons/hicolor/${size}x${size}/apps
  if [ -f "assets/icon-${size}.png" ]; then
    cp "assets/icon-${size}.png" AppDir/usr/share/icons/hicolor/${size}x${size}/apps/cook-sync.png
    echo "✅ Copied icon-${size}.png"
  fi
done

# Copy runtime tray icons to share directory (needed for system tray)
# These icons are loaded at runtime by the tray code
mkdir -p AppDir/usr/share/cook-sync
for icon in icon_black.png icon_white.png icon_black_tray_*.png icon_white_tray_*.png; do
  [ -f "assets/$icon" ] && cp "assets/$icon" AppDir/usr/share/cook-sync/ && echo "✅ Copied tray icon: $icon" || true
done

# Copy desktop file to AppDir
echo "📄 Creating desktop file..."
mkdir -p AppDir/usr/share/applications
cp resources/cook-sync.desktop AppDir/usr/share/applications/cook-sync.desktop
# Update Exec for AppImage (just the binary name, linuxdeploy will handle the path)
sed -i 's|Exec=cook-sync start|Exec=cook-sync|g' AppDir/usr/share/applications/cook-sync.desktop
echo "✅ Desktop file created"

# Verify binary exists
echo "🔍 Verifying binary..."
ls -lh target/release/cook-sync
file target/release/cook-sync
ldd target/release/cook-sync 2>&1 | head -20 || echo "Note: ldd check complete"

# Build AppImage with linuxdeploy
echo "🔧 Setting environment variables..."
export DEPLOY_GTK_VERSION=3
export OUTPUT="CookSync-${VERSION}-x86_64.AppImage"
export DISABLE_COPYRIGHT_FILES_DEPLOYMENT=1
export NO_STRIP=true
export LINUXDEPLOY_OUTPUT_VERSION="${VERSION}"

echo "Environment:"
echo "  DEPLOY_GTK_VERSION=$DEPLOY_GTK_VERSION"
echo "  OUTPUT=$OUTPUT"
echo "  NO_STRIP=$NO_STRIP"
echo "  VERSION=$VERSION"

echo "🚀 Running linuxdeploy..."
set -x  # Enable verbose output
"$LINUXDEPLOY" \
  --appdir AppDir \
  --executable target/release/cook-sync \
  --desktop-file AppDir/usr/share/applications/cook-sync.desktop \
  --icon-file AppDir/usr/share/icons/hicolor/256x256/apps/cook-sync.png \
  --icon-file AppDir/usr/share/icons/hicolor/128x128/apps/cook-sync.png \
  --icon-file AppDir/usr/share/icons/hicolor/48x48/apps/cook-sync.png \
  --icon-file AppDir/usr/share/icons/hicolor/32x32/apps/cook-sync.png \
  --icon-file AppDir/usr/share/icons/hicolor/22x22/apps/cook-sync.png \
  --icon-file AppDir/usr/share/icons/hicolor/16x16/apps/cook-sync.png \
  --plugin gtk \
  --output appimage
LINUXDEPLOY_EXIT=$?
set +x  # Disable verbose output

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "linuxdeploy exit code: $LINUXDEPLOY_EXIT"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

if [ $LINUXDEPLOY_EXIT -ne 0 ]; then
  echo "❌ linuxdeploy failed with exit code $LINUXDEPLOY_EXIT"
  echo "Listing generated files:"
  ls -la
  exit $LINUXDEPLOY_EXIT
fi

# Rename to our desired format
echo "📦 Renaming AppImage..."
ls -la CookSync-*.AppImage
mv CookSync-*.AppImage "CookSync-${VERSION}.AppImage"
echo "✅ AppImage created: CookSync-${VERSION}.AppImage"

echo "DEB_PATH=sync-agent/cook-sync_${VERSION}_amd64.deb" >> $GITHUB_ENV
echo "RPM_PATH=sync-agent/cook-sync-${VERSION}-1.x86_64.rpm" >> $GITHUB_ENV
echo "APPIMAGE_PATH=sync-agent/CookSync-${VERSION}.AppImage" >> $GITHUB_ENV
