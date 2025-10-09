#!/bin/bash

set -e

# Get the directory of this script
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_DIR="$SCRIPT_DIR/.."

# Parse arguments
PLATFORM=${1:-}
VERSION=${2:-"0.1.0"}

if [ -z "$PLATFORM" ]; then
    echo "Usage: $0 <platform> [version]"
    echo "Platform can be: macos, linux, windows"
    exit 1
fi

cd "$PROJECT_DIR"

echo "Building Cook Sync Agent v$VERSION for $PLATFORM..."

case "$PLATFORM" in
    macos)
        # Build universal binary for macOS
        echo "Building for x86_64..."
        cargo build --release --target x86_64-apple-darwin
        
        echo "Building for aarch64..."
        cargo build --release --target aarch64-apple-darwin
        
        echo "Creating universal binary..."
        mkdir -p dist/macos
        lipo -create \
            target/x86_64-apple-darwin/release/cook-sync \
            target/aarch64-apple-darwin/release/cook-sync \
            -output dist/macos/cook-sync
        
        chmod +x dist/macos/cook-sync
        
        # Create .app bundle
        APP_NAME="Cook Sync.app"
        APP_DIR="dist/macos/$APP_NAME"
        
        mkdir -p "$APP_DIR/Contents/MacOS"
        mkdir -p "$APP_DIR/Contents/Resources"
        
        cp dist/macos/cook-sync "$APP_DIR/Contents/MacOS/"

        # Copy tray icons for runtime use
        cp assets/icon_black.png "$APP_DIR/Contents/Resources/icon_black.png"
        cp assets/icon_white.png "$APP_DIR/Contents/Resources/icon_white.png"

        # Copy package icon
        cp assets/package.icns "$APP_DIR/Contents/Resources/AppIcon.icns"
        
        # Create Info.plist
        cat > "$APP_DIR/Contents/Info.plist" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>cook-sync</string>
    <key>CFBundleIdentifier</key>
    <string>com.cooklang.sync</string>
    <key>CFBundleName</key>
    <string>Cook Sync</string>
    <key>CFBundleIconFile</key>
    <string>AppIcon</string>
    <key>CFBundleVersion</key>
    <string>$VERSION</string>
    <key>CFBundleShortVersionString</key>
    <string>$VERSION</string>
    <key>LSMinimumSystemVersion</key>
    <string>10.12</string>
    <key>LSUIElement</key>
    <true/>
    <key>NSHighResolutionCapable</key>
    <true/>
</dict>
</plist>
EOF
        
        # Create DMG
        echo "Creating DMG..."
        hdiutil create -volname "Cook Sync" -srcfolder "dist/macos/$APP_NAME" -ov -format UDZO "dist/cook-sync-$VERSION-macos.dmg"
        
        echo "macOS installer created: dist/cook-sync-$VERSION-macos.dmg"
        ;;
        
    linux)
        # Build for Linux
        cargo build --release --target x86_64-unknown-linux-gnu
        
        mkdir -p dist/linux/cook-sync
        cp target/x86_64-unknown-linux-gnu/release/cook-sync dist/linux/cook-sync/

        # Copy package icon for desktop
        cp assets/package.png dist/linux/cook-sync/cook-sync.png

        # Copy tray icons
        cp assets/icon_black.png dist/linux/cook-sync/
        cp assets/icon_white.png dist/linux/cook-sync/
        
        # Create desktop entry
        cat > dist/linux/cook-sync/cook-sync.desktop << EOF
[Desktop Entry]
Type=Application
Name=Cook Sync
Comment=Sync your recipes with Cook.md
Exec=/opt/cook-sync/cook-sync
Icon=/opt/cook-sync/cook-sync.png
Terminal=false
Categories=Utility;
StartupNotify=false
EOF
        
        # Create install script
        cat > dist/linux/cook-sync/install.sh << 'EOF'
#!/bin/bash
set -e

INSTALL_DIR="/opt/cook-sync"
DESKTOP_FILE="/usr/share/applications/cook-sync.desktop"
AUTOSTART_FILE="$HOME/.config/autostart/cook-sync.desktop"

echo "Installing Cook Sync..."

# Create installation directory
sudo mkdir -p "$INSTALL_DIR"
sudo cp cook-sync "$INSTALL_DIR/"
sudo cp cook-sync.png "$INSTALL_DIR/"
sudo cp icon_black.png "$INSTALL_DIR/"
sudo cp icon_white.png "$INSTALL_DIR/"
sudo chmod +x "$INSTALL_DIR/cook-sync"

# Install desktop file
sudo cp cook-sync.desktop "$DESKTOP_FILE"

# Create autostart entry
mkdir -p "$(dirname "$AUTOSTART_FILE")"
cp cook-sync.desktop "$AUTOSTART_FILE"

echo "Cook Sync installed successfully!"
echo "You can start it from your application menu or run: /opt/cook-sync/cook-sync"
EOF
        
        chmod +x dist/linux/cook-sync/install.sh
        
        # Create tarball
        cd dist/linux
        tar czf "../cook-sync-$VERSION-linux-x64.tar.gz" cook-sync
        cd ../..
        
        echo "Linux installer created: dist/cook-sync-$VERSION-linux-x64.tar.gz"
        ;;
        
    windows)
        # Build for Windows
        cargo build --release --target x86_64-pc-windows-msvc
        
        mkdir -p dist/windows
        cp target/x86_64-pc-windows-msvc/release/cook-sync.exe dist/windows/

        # Copy icons
        cp assets/package.ico dist/windows/cook-sync.ico
        cp assets/icon_black.png dist/windows/
        cp assets/icon_white.png dist/windows/
        
        # Create installer script for NSIS
        cat > dist/windows/installer.nsi << EOF
!define APPNAME "Cook Sync"
!define COMPANYNAME "Cooklang"
!define DESCRIPTION "Sync your recipes with Cook.md"
!define VERSIONMAJOR ${VERSION%%.*}
!define VERSIONMINOR 1
!define VERSIONBUILD 0
!define HELPURL "https://cook.md/support"
!define UPDATEURL "https://cook.md/download"
!define ABOUTURL "https://cook.md"

RequestExecutionLevel admin

InstallDir "\$PROGRAMFILES\${COMPANYNAME}\${APPNAME}"

Name "${COMPANYNAME} - ${APPNAME}"
Icon "cook-sync.ico"
outFile "cook-sync-${VERSION}-windows-setup.exe"

!include LogicLib.nsh

page directory
Page instfiles

!macro VerifyUserIsAdmin
UserInfo::GetAccountType
pop \$0
\${If} \$0 != "admin"
    messageBox mb_iconstop "Administrator rights required!"
    setErrorLevel 740
    quit
\${EndIf}
!macroend

function .onInit
    setShellVarContext all
    !insertmacro VerifyUserIsAdmin
functionEnd

section "install"
    setOutPath \$INSTDIR
    file "cook-sync.exe"
    file "cook-sync.ico"
    file "icon_black.png"
    file "icon_white.png"
    
    # Create uninstaller
    writeUninstaller "\$INSTDIR\uninstall.exe"
    
    # Start Menu
    createDirectory "\$SMPROGRAMS\${COMPANYNAME}"
    createShortCut "\$SMPROGRAMS\${COMPANYNAME}\${APPNAME}.lnk" "\$INSTDIR\cook-sync.exe" "" "\$INSTDIR\cook-sync.ico"
    
    # Startup
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "CookSync" "\$INSTDIR\cook-sync.exe"
    
    # Registry information for add/remove programs
    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${COMPANYNAME} ${APPNAME}" "DisplayName" "${COMPANYNAME} - ${APPNAME} - ${DESCRIPTION}"
    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${COMPANYNAME} ${APPNAME}" "UninstallString" "\$INSTDIR\uninstall.exe"
    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${COMPANYNAME} ${APPNAME}" "DisplayIcon" "\$INSTDIR\cook-sync.ico"
    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${COMPANYNAME} ${APPNAME}" "Publisher" "${COMPANYNAME}"
    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${COMPANYNAME} ${APPNAME}" "HelpLink" "${HELPURL}"
    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${COMPANYNAME} ${APPNAME}" "URLUpdateInfo" "${UPDATEURL}"
    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${COMPANYNAME} ${APPNAME}" "URLInfoAbout" "${ABOUTURL}"
    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${COMPANYNAME} ${APPNAME}" "DisplayVersion" "${VERSIONMAJOR}.${VERSIONMINOR}.${VERSIONBUILD}"
    WriteRegDWORD HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${COMPANYNAME} ${APPNAME}" "VersionMajor" ${VERSIONMAJOR}
    WriteRegDWORD HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${COMPANYNAME} ${APPNAME}" "VersionMinor" ${VERSIONMINOR}
    WriteRegDWORD HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${COMPANYNAME} ${APPNAME}" "NoModify" 1
    WriteRegDWORD HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${COMPANYNAME} ${APPNAME}" "NoRepair" 1
sectionEnd

section "uninstall"
    delete "\$SMPROGRAMS\${COMPANYNAME}\${APPNAME}.lnk"
    rmDir "\$SMPROGRAMS\${COMPANYNAME}"
    
    delete \$INSTDIR\cook-sync.exe
    delete \$INSTDIR\cook-sync.ico
    delete \$INSTDIR\icon_black.png
    delete \$INSTDIR\icon_white.png
    delete \$INSTDIR\uninstall.exe
    rmDir \$INSTDIR
    
    DeleteRegKey HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${COMPANYNAME} ${APPNAME}"
    DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "CookSync"
sectionEnd
EOF
        
        echo "Windows installer files created in dist/windows/"
        echo "Use NSIS to compile installer.nsi to create the setup executable"
        ;;
        
    *)
        echo "Unknown platform: $PLATFORM"
        exit 1
        ;;
esac

echo "Build complete!"