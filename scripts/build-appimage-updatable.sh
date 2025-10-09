#!/bin/bash
set -euo pipefail

VERSION=${1:-"0.0.1"}

echo "Building updatable AppImage for Cook Sync ${VERSION}..."

# Build binary
echo "Building for x86_64-unknown-linux-gnu..."
cargo build --release --target x86_64-unknown-linux-gnu

# Create AppDir structure
APP_DIR="AppDir"
rm -rf "$APP_DIR"
mkdir -p "$APP_DIR/usr/"{bin,lib,share/applications,share/icons/hicolor/256x256/apps}

# Copy binary
cp target/x86_64-unknown-linux-gnu/release/cook-sync "$APP_DIR/usr/bin/"
chmod +x "$APP_DIR/usr/bin/cook-sync"

# Create desktop file
cat > "$APP_DIR/usr/share/applications/cook-sync.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=Cook Sync
Comment=Sync your Cook.md recipes
Exec=cook-sync
Icon=cook-sync
Categories=Utility;Network;
Terminal=false
EOF

# Copy or create icon
if [ -f "resources/icon.png" ]; then
    cp resources/icon.png "$APP_DIR/usr/share/icons/hicolor/256x256/apps/cook-sync.png"
else
    echo "Warning: resources/icon.png not found"
    if command -v convert &> /dev/null; then
        # Create simple placeholder icon using ImageMagick
        convert -size 256x256 xc:blue -fill white -pointsize 48 \
            -gravity center -annotate +0+0 "Cook\nSync" \
            "$APP_DIR/usr/share/icons/hicolor/256x256/apps/cook-sync.png"
        echo "Created placeholder icon"
    else
        echo "ImageMagick not available, skipping icon creation"
    fi
fi

# Download appimagetool if not available
if [ ! -f "appimagetool-x86_64.AppImage" ]; then
    echo "Downloading appimagetool..."
    wget -q https://github.com/AppImage/AppImageKit/releases/download/continuous/appimagetool-x86_64.AppImage
    chmod +x appimagetool-x86_64.AppImage
fi

# Build AppImage with update information
echo "Building AppImage with update support..."
ARCH=x86_64 ./appimagetool-x86_64.AppImage \
  --updateinformation "gh-releases-zsync|Cooklang|sync-agent|latest|cook-sync-*-x86_64.AppImage.zsync" \
  "$APP_DIR" \
  "cook-sync-${VERSION}-x86_64.AppImage"

# Generate zsync file for delta updates
if command -v zsyncmake &> /dev/null; then
    echo "Generating zsync file for delta updates..."
    zsyncmake \
      -u "https://github.com/Cooklang/sync-agent/releases/download/v${VERSION}/cook-sync-${VERSION}-x86_64.AppImage" \
      "cook-sync-${VERSION}-x86_64.AppImage"

    echo "✅ Created cook-sync-${VERSION}-x86_64.AppImage.zsync"
else
    echo "Warning: zsync not installed. Install with: apt-get install zsync"
    echo "Skipping zsync file generation"
fi

chmod +x "cook-sync-${VERSION}-x86_64.AppImage"

echo "✅ Created cook-sync-${VERSION}-x86_64.AppImage with update support"
echo ""
echo "Build complete! Files created:"
ls -lh cook-sync-${VERSION}-x86_64.AppImage*

echo ""
echo "To test the AppImage:"
echo "  ./cook-sync-${VERSION}-x86_64.AppImage --version"
echo ""
echo "To extract and inspect:"
echo "  ./cook-sync-${VERSION}-x86_64.AppImage --appimage-extract"
