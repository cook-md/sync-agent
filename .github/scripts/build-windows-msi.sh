#!/bin/bash
set -e

VERSION="$1"

if [ -z "$VERSION" ]; then
  echo "Usage: $0 <version>"
  exit 1
fi

# Download and extract WiX toolset
curl -L https://github.com/wixtoolset/wix3/releases/download/wix3112rtm/wix311-binaries.zip -o wix.zip
unzip -q wix.zip -d wix
export PATH="$PWD/wix:$PATH"

# Create WXS file
cat > cook-sync.wxs << EOF
<?xml version="1.0" encoding="UTF-8"?>
<Wix xmlns="http://schemas.microsoft.com/wix/2006/wi">
  <Product Id="*"
           Name="Cook Sync"
           Language="1033"
           Version="${VERSION}.0"
           Manufacturer="Cook.md"
           UpgradeCode="8F7A9B3C-2E1D-4F6A-9C8B-7E5D4A3B2C1F">

    <Package InstallerVersion="200"
             Compressed="yes"
             InstallScope="perMachine"
             Platform="x64" />

    <MajorUpgrade DowngradeErrorMessage="A newer version of [ProductName] is already installed." />
    <MediaTemplate EmbedCab="yes" />

    <Feature Id="ProductFeature" Title="Cook Sync" Level="1" Description="Cook Sync application" ConfigurableDirectory="INSTALLFOLDER">
      <ComponentGroupRef Id="ProductComponents" />
      <ComponentRef Id="ApplicationShortcut" />

      <Feature Id="DesktopShortcutFeature" Title="Desktop Shortcut" Level="1" Description="Add a shortcut to your Desktop">
        <ComponentRef Id="DesktopShortcut" />
      </Feature>

      <Feature Id="AutoStartFeature" Title="Run at Startup" Level="1" Description="Automatically start Cook Sync when you log in">
        <ComponentRef Id="AutoStartEntry" />
      </Feature>
    </Feature>

    <Directory Id="TARGETDIR" Name="SourceDir">
      <Directory Id="ProgramFiles64Folder">
        <Directory Id="INSTALLFOLDER" Name="Cook Sync" />
      </Directory>
      <Directory Id="ProgramMenuFolder">
        <Directory Id="ApplicationProgramsFolder" Name="Cook Sync"/>
      </Directory>
      <Directory Id="DesktopFolder" Name="Desktop" />
      <Directory Id="StartupFolder" />
    </Directory>

    <ComponentGroup Id="ProductComponents" Directory="INSTALLFOLDER">
      <Component Id="CookSyncExecutable" Guid="1A2B3C4D-5E6F-7A8B-9C0D-1E2F3A4B5C6D">
        <File Id="CookSyncEXE"
              Source="target/release/cook-sync.exe"
              KeyPath="yes" />
        <Environment Id="PATH"
                    Name="PATH"
                    Value="[INSTALLFOLDER]"
                    Permanent="no"
                    Part="last"
                    Action="set"
                    System="no" />
      </Component>
    </ComponentGroup>

    <!-- Start Menu Shortcut -->
    <DirectoryRef Id="ApplicationProgramsFolder">
      <Component Id="ApplicationShortcut" Guid="2B3C4D5E-6F7A-8B9C-0D1E-2F3A4B5C6D7E">
        <Shortcut Id="ApplicationStartMenuShortcut"
                  Name="Cook Sync"
                  Description="Cook Sync - Recipe synchronization agent"
                  Target="[INSTALLFOLDER]cook-sync.exe"
                  Arguments="start"
                  WorkingDirectory="INSTALLFOLDER"
                  Icon="icon.ico" />
        <RemoveFolder Id="CleanUpShortCut" Directory="ApplicationProgramsFolder" On="uninstall"/>
        <RegistryValue Root="HKCU"
                      Key="Software\Cook.md\CookSync"
                      Name="installed"
                      Type="integer"
                      Value="1"
                      KeyPath="yes"/>
      </Component>
    </DirectoryRef>

    <!-- Desktop Shortcut -->
    <DirectoryRef Id="DesktopFolder">
      <Component Id="DesktopShortcut" Guid="4D5E6F7A-8B9C-0D1E-2F3A-4B5C6D7E8F9A">
        <Shortcut Id="ApplicationDesktopShortcut"
                  Name="Cook Sync"
                  Description="Cook Sync - Recipe synchronization agent"
                  Target="[INSTALLFOLDER]cook-sync.exe"
                  Arguments="start"
                  WorkingDirectory="INSTALLFOLDER"
                  Icon="icon.ico" />
        <RegistryValue Root="HKCU"
                      Key="Software\Cook.md\CookSync"
                      Name="desktop_shortcut"
                      Type="integer"
                      Value="1"
                      KeyPath="yes"/>
      </Component>
    </DirectoryRef>

    <!-- Auto-start Entry -->
    <DirectoryRef Id="TARGETDIR">
      <Component Id="AutoStartEntry" Guid="3C4D5E6F-7A8B-9C0D-1E2F-3A4B5C6D7E8F">
        <RegistryValue Root="HKCU"
                      Key="Software\Microsoft\Windows\CurrentVersion\Run"
                      Name="CookSync"
                      Value="&quot;[INSTALLFOLDER]cook-sync.exe&quot; start"
                      Type="string"
                      KeyPath="yes" />
      </Component>
    </DirectoryRef>

    <Icon Id="icon.ico" SourceFile="assets/package.ico"/>
    <Property Id="ARPPRODUCTICON" Value="icon.ico" />

    <!-- Custom UI with feature selection -->
    <UI>
      <UIRef Id="WixUI_FeatureTree" />
      <Publish Dialog="WelcomeDlg" Control="Next" Event="NewDialog" Value="CustomizeDlg" Order="2">1</Publish>
      <Publish Dialog="CustomizeDlg" Control="Back" Event="NewDialog" Value="WelcomeDlg" Order="2">1</Publish>
    </UI>

    <!-- Set installation directory property for UI -->
    <Property Id="WIXUI_INSTALLDIR" Value="INSTALLFOLDER" />
  </Product>
</Wix>
EOF

# Check if icon exists, create placeholder if not
if [ ! -f "assets/package.ico" ]; then
  echo "Error: assets/package.ico not found"
  exit 1
fi

# Compile and link
./wix/candle.exe -arch x64 cook-sync.wxs
./wix/light.exe -ext WixUIExtension cook-sync.wixobj -out "CookSync-${VERSION}-windows-x86_64.msi"

echo "MSI_PATH=sync-agent/CookSync-${VERSION}-windows-x86_64.msi" >> $GITHUB_ENV
