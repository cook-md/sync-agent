#!/bin/bash

# Generate update manifest for cook-sync
# Usage: ./generate-manifest.sh <version> <release_notes> <release_dir>

set -e

VERSION=$1
NOTES=$2
RELEASE_DIR=$3

if [ -z "$VERSION" ] || [ -z "$NOTES" ] || [ -z "$RELEASE_DIR" ]; then
    echo "Usage: $0 <version> <release_notes> <release_dir>"
    exit 1
fi

PUB_DATE=$(date -u +%Y-%m-%dT%H:%M:%SZ)

# Helper function to get file info
get_file_info() {
    local platform=$1
    local extension=$2
    local file="${RELEASE_DIR}/cook-sync_${VERSION}_${platform}.${extension}"
    local sha_file="${file}.sha256"

    if [ -f "$file" ] && [ -f "$sha_file" ]; then
        local sha256=$(cat "$sha_file" | cut -d' ' -f1)
        local size=$(stat -f%z "$file" 2>/dev/null || stat -c%s "$file" 2>/dev/null || echo 0)
        cat <<EOF
    {
      "url": "https://downloads.cook.md/sync-agent/v${VERSION}/cook-sync_${VERSION}_${platform}.${extension}",
      "signature": "${sha256}",
      "sha256": "${sha256}",
      "size": ${size}
    }
EOF
    else
        echo "null"
    fi
}

# Generate the manifest
cat > "${RELEASE_DIR}/../latest.json" <<EOF
{
  "version": "${VERSION}",
  "notes": "${NOTES}",
  "pub_date": "${PUB_DATE}",
  "platforms": {
    "darwin-x86_64": $(get_file_info "darwin-x86_64" "tar.gz"),
    "darwin-aarch64": $(get_file_info "darwin-aarch64" "tar.gz"),
    "linux-x86_64": $(get_file_info "linux-x86_64" "tar.gz"),
    "windows-x86_64": $(get_file_info "windows-x86_64" "zip"),
    "windows-i686": $(get_file_info "windows-i686" "zip")
  }
}
EOF

echo "Manifest generated at ${RELEASE_DIR}/../latest.json"
cat "${RELEASE_DIR}/../latest.json"