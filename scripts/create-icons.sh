#!/bin/bash

# Create icons for all platforms
# This script generates both package icons (from color logo) and tray icons (from monochrome icons)
# Usage: ./create-icons.sh

set -e

# Change to assets directory
cd "$(dirname "$0")/../assets"

# Check for required tools
if ! command -v magick &> /dev/null; then
    echo "Error: ImageMagick 7 (magick) is not installed"
    echo "Install with: brew install imagemagick"
    exit 1
fi

echo "==========================================
Creating Package Icons from Color Logo
=========================================="

# Use the full color logo for package icons
LOGO_SOURCE="logo-1024.png"

if [ ! -f "$LOGO_SOURCE" ]; then
    echo "Error: Logo source image '$LOGO_SOURCE' not found!"
    exit 1
fi

echo "Creating package icons from: $LOGO_SOURCE"

# ===========================
# 1. Create Windows Package Icon
# ===========================
echo ""
echo "Creating Windows package icon (package.ico)..."

# Create multiple sizes for Windows
magick "$LOGO_SOURCE" -resize 16x16 temp-16.png
magick "$LOGO_SOURCE" -resize 32x32 temp-32.png
magick "$LOGO_SOURCE" -resize 48x48 -filter Lanczos temp-48.png
magick "$LOGO_SOURCE" -resize 64x64 -filter Lanczos temp-64.png
magick "$LOGO_SOURCE" -resize 128x128 -filter Lanczos temp-128.png
magick "$LOGO_SOURCE" -resize 256x256 -filter Lanczos temp-256.png

# Combine into ICO file
magick temp-16.png temp-32.png temp-48.png temp-64.png temp-128.png temp-256.png package.ico

# Clean up temporary files
rm temp-16.png temp-32.png temp-48.png temp-64.png temp-128.png temp-256.png

echo "✅ Created package.ico"

# ===========================
# 2. Create Linux Package Icon
# ===========================
echo "Creating Linux package icon (package.png)..."

# Create 256x256 PNG for Linux AppImage
magick "$LOGO_SOURCE" -resize 256x256 -filter Lanczos package.png

echo "✅ Created package.png (256x256)"

# ===========================
# 3. Create macOS Package Icon
# ===========================
if [[ "$OSTYPE" == "darwin"* ]]; then
    echo "Creating macOS package icon (package.icns)..."

    # Create iconset directory
    mkdir -p package.iconset

    # Generate all required sizes for macOS
    magick "$LOGO_SOURCE" -resize 16x16 package.iconset/icon_16x16.png
    magick "$LOGO_SOURCE" -resize 32x32 package.iconset/icon_16x16@2x.png
    magick "$LOGO_SOURCE" -resize 32x32 package.iconset/icon_32x32.png
    magick "$LOGO_SOURCE" -resize 64x64 -filter Lanczos package.iconset/icon_32x32@2x.png
    magick "$LOGO_SOURCE" -resize 128x128 -filter Lanczos package.iconset/icon_128x128.png
    magick "$LOGO_SOURCE" -resize 256x256 -filter Lanczos package.iconset/icon_128x128@2x.png
    magick "$LOGO_SOURCE" -resize 256x256 -filter Lanczos package.iconset/icon_256x256.png
    magick "$LOGO_SOURCE" -resize 512x512 -filter Lanczos package.iconset/icon_256x256@2x.png
    magick "$LOGO_SOURCE" -resize 512x512 -filter Lanczos package.iconset/icon_512x512.png
    magick "$LOGO_SOURCE" -resize 1024x1024 -filter Lanczos package.iconset/icon_512x512@2x.png

    # Convert to ICNS
    iconutil -c icns package.iconset

    # Clean up
    rm -rf package.iconset

    echo "✅ Created package.icns"
else
    echo "⚠️  Skipping macOS package icon creation (iconutil only available on macOS)"
fi

# ===========================
# 4. Create Desktop Environment Icons
# ===========================
echo ""
echo "Creating desktop environment icons for Linux packages..."

for size in 16 22 32 48 128 256; do
    magick "$LOGO_SOURCE" -resize ${size}x${size} -filter Lanczos icon-${size}.png
    echo "✅ Created icon-${size}.png"
done

echo ""
echo "==========================================
Creating Tray Icons from Monochrome Icons
=========================================="

# Function to create tray icons from a monochrome source
create_tray_icons() {
    local SOURCE_IMAGE=$1
    local PREFIX=$2

    echo ""
    echo "Creating tray icons from: $SOURCE_IMAGE"

    # ===========================
    # 1. Create Windows Tray Icon
    # ===========================
    echo "Creating Windows tray icon (${PREFIX}tray.ico)..."

    # Create multiple sizes for Windows tray
    magick "$SOURCE_IMAGE" -resize 16x16 temp-16.png
    magick "$SOURCE_IMAGE" -resize 24x24 temp-24.png
    magick "$SOURCE_IMAGE" -resize 32x32 temp-32.png
    magick "$SOURCE_IMAGE" -resize 48x48 -filter Lanczos temp-48.png
    magick "$SOURCE_IMAGE" -resize 64x64 -filter Lanczos temp-64.png

    # Combine into ICO file
    magick temp-16.png temp-24.png temp-32.png temp-48.png temp-64.png ${PREFIX}tray.ico

    # Clean up temporary files
    rm temp-16.png temp-24.png temp-32.png temp-48.png temp-64.png

    echo "✅ Created ${PREFIX}tray.ico"

    # ===========================
    # 2. Create PNG Tray Icons
    # ===========================
    echo "Creating PNG tray icons..."

    # Create various sizes for different platforms
    magick "$SOURCE_IMAGE" -resize 16x16 ${PREFIX}tray_16.png
    magick "$SOURCE_IMAGE" -resize 24x24 ${PREFIX}tray_24.png
    magick "$SOURCE_IMAGE" -resize 32x32 ${PREFIX}tray_32.png
    magick "$SOURCE_IMAGE" -resize 48x48 -filter Lanczos ${PREFIX}tray_48.png
    magick "$SOURCE_IMAGE" -resize 64x64 -filter Lanczos ${PREFIX}tray_64.png

    # Keep the original name for backward compatibility (if not already named that)
    if [ "$SOURCE_IMAGE" != "${PREFIX}.png" ]; then
        cp "$SOURCE_IMAGE" ${PREFIX}.png
    fi

    echo "✅ Created PNG tray icons (16x16, 24x24, 32x32, 48x48, 64x64)"
}

# Create black tray icons (for light theme)
# Use the higher quality 64px source if available, otherwise fall back to 32px
if [ -f "logo-black-64.png" ]; then
    create_tray_icons "logo-black-64.png" "icon_black"
elif [ -f "icon_black.png" ]; then
    create_tray_icons "icon_black.png" "icon_black"
else
    echo "⚠️  Skipping black tray icons (no black icon source found)"
fi

# Create white tray icons (for dark theme)
# Use the higher quality 64px source if available, otherwise fall back to 32px
if [ -f "logo-white-64.png" ]; then
    create_tray_icons "logo-white-64.png" "icon_white"
elif [ -f "icon_white.png" ]; then
    create_tray_icons "icon_white.png" "icon_white"
else
    echo "⚠️  Skipping white tray icons (no white icon source found)"
fi

# ===========================
# Summary
# ===========================
echo ""
echo "=========================================="
echo "Icon creation complete! Generated files:"
echo "=========================================="
echo ""
echo "Package Icons (from color logo):"
echo "  • package.ico       - Windows installer/package icon"
echo "  • package.png       - Linux AppImage icon"
if [[ "$OSTYPE" == "darwin"* ]]; then
    echo "  • package.icns      - macOS application icon"
fi
echo ""
echo "Tray Icons (monochrome):"
if [ -f "icon_black.png" ]; then
    echo "  Black icons (for light theme):"
    echo "    • icon_black_tray.ico     - Windows tray icon"
    echo "    • icon_black_tray_*.png   - PNG tray icons (various sizes)"
    echo "    • icon_black.png          - Original (backward compatibility)"
fi
if [ -f "icon_white.png" ]; then
    echo "  White icons (for dark theme):"
    echo "    • icon_white_tray.ico     - Windows tray icon"
    echo "    • icon_white_tray_*.png   - PNG tray icons (various sizes)"
    echo "    • icon_white.png          - Original (backward compatibility)"
fi
echo ""
echo "Usage:"
echo "  • Package icons: Use for app installers, dock, and app switcher"
echo "  • Tray icons: Use for system tray (automatically switch based on theme)"