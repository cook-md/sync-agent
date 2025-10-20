#!/bin/bash
set -e

# Usage: ./scripts/generate-manifest.sh <version> [artifacts-dir]
# Example: ./scripts/generate-manifest.sh 0.4.3 artifacts

VERSION=${1:-"0.0.0"}
ARTIFACTS_DIR=${2:-"target/release/bundle"}
REPO="cook-md/sync-agent"
BASE_URL="https://github.com/${REPO}/releases/download/cook-sync-v${VERSION}"

echo "Generating manifest.json for version ${VERSION}"
echo "Artifacts directory: ${ARTIFACTS_DIR}"

# Function to find and read signature file
read_signature() {
    local pattern="$1"
    local sig_file=$(find "${ARTIFACTS_DIR}" -name "${pattern}" | head -1)
    
    if [ -z "$sig_file" ]; then
        echo "Warning: Signature file not found for pattern: ${pattern}" >&2
        echo "SIG_NOT_FOUND"
        return
    fi
    
    cat "$sig_file"
}

# Find signature files
DARWIN_X64_SIG=$(read_signature "*-x64.dmg.sig")
DARWIN_ARM64_SIG=$(read_signature "*-arm64.dmg.sig" || read_signature "*-aarch64.dmg.sig")
LINUX_X64_SIG=$(read_signature "*-x86_64.AppImage.sig")
WINDOWS_X64_NSIS_SIG=$(read_signature "*Setup*.exe.sig" || read_signature "*-x64.exe.sig")
WINDOWS_X64_MSI_SIG=$(read_signature "*.msi.sig")

# Generate manifest.json
cat > manifest.json <<EOF_MANIFEST
{
  "version": "${VERSION}",
  "pub_date": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "notes": "See CHANGELOG.md for details. Download and install the appropriate package for your platform.",
  "platforms": {
EOF_MANIFEST

# Add platform entries (only if signature found)
FIRST_ENTRY=true

# macOS x86_64
if [ "$DARWIN_X64_SIG" != "SIG_NOT_FOUND" ] && [ -n "$DARWIN_X64_SIG" ]; then
    [ "$FIRST_ENTRY" = false ] && echo "," >> manifest.json
    cat >> manifest.json <<EOF_ENTRY
    "darwin-x86_64": {
      "signature": "${DARWIN_X64_SIG}",
      "url": "${BASE_URL}/Cook-Sync-${VERSION}-x64.dmg",
      "format": "app"
    }
EOF_ENTRY
    FIRST_ENTRY=false
fi

# macOS ARM64
if [ "$DARWIN_ARM64_SIG" != "SIG_NOT_FOUND" ] && [ -n "$DARWIN_ARM64_SIG" ]; then
    [ "$FIRST_ENTRY" = false ] && echo "," >> manifest.json
    cat >> manifest.json <<EOF_ENTRY
    "darwin-aarch64": {
      "signature": "${DARWIN_ARM64_SIG}",
      "url": "${BASE_URL}/Cook-Sync-${VERSION}-aarch64.dmg",
      "format": "app"
    }
EOF_ENTRY
    FIRST_ENTRY=false
fi

# Linux x86_64
if [ "$LINUX_X64_SIG" != "SIG_NOT_FOUND" ] && [ -n "$LINUX_X64_SIG" ]; then
    [ "$FIRST_ENTRY" = false ] && echo "," >> manifest.json
    cat >> manifest.json <<EOF_ENTRY
    "linux-x86_64": {
      "signature": "${LINUX_X64_SIG}",
      "url": "${BASE_URL}/cook-sync-${VERSION}-x86_64.AppImage",
      "format": "appimage"
    }
EOF_ENTRY
    FIRST_ENTRY=false
fi

# Windows x86_64 NSIS
if [ "$WINDOWS_X64_NSIS_SIG" != "SIG_NOT_FOUND" ] && [ -n "$WINDOWS_X64_NSIS_SIG" ]; then
    [ "$FIRST_ENTRY" = false ] && echo "," >> manifest.json
    cat >> manifest.json <<EOF_ENTRY
    "windows-x86_64": {
      "signature": "${WINDOWS_X64_NSIS_SIG}",
      "url": "${BASE_URL}/Cook-Sync-Setup-${VERSION}-x64.exe",
      "format": "nsis"
    }
EOF_ENTRY
    FIRST_ENTRY=false
fi

# Close the JSON
cat >> manifest.json <<EOF_CLOSE
  }
}
EOF_CLOSE

echo "Generated manifest.json for version ${VERSION}"
cat manifest.json

# Validate JSON
if command -v jq &> /dev/null; then
    echo "Validating manifest.json..."
    jq . manifest.json > /dev/null && echo "✓ manifest.json is valid JSON"
else
    echo "jq not found, skipping JSON validation"
fi
