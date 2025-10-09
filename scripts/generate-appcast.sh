#!/bin/bash
set -euo pipefail

VERSION=${1:-}

if [ -z "$VERSION" ]; then
    echo "Usage: $0 <version>"
    echo "Example: $0 v0.1.0"
    exit 1
fi

# Extract version number without 'v' prefix
VERSION_NUMBER="${VERSION#v}"

# Check if required files exist
APP_ZIP="cook-sync-${VERSION_NUMBER}.app.zip"
if [ ! -f "$APP_ZIP" ]; then
    echo "Error: $APP_ZIP not found"
    exit 1
fi

# Get file size
if [[ "$OSTYPE" == "darwin"* ]]; then
    APP_SIZE=$(stat -f%z "$APP_ZIP" 2>/dev/null)
else
    APP_SIZE=$(stat -c%s "$APP_ZIP" 2>/dev/null)
fi

# Get Sparkle signature if available
APP_SIG=""
if [ -f "${APP_ZIP}.sig" ]; then
    APP_SIG=$(cat "${APP_ZIP}.sig")
    echo "Found Sparkle signature"
else
    echo "Warning: No Sparkle signature found at ${APP_ZIP}.sig"
    echo "The appcast will be generated without a signature"
fi

# Generate appcast.xml
echo "Generating appcast.xml for version ${VERSION_NUMBER}..."

cat > appcast.xml <<EOF
<?xml version="1.0" encoding="utf-8"?>
<rss version="2.0" xmlns:sparkle="http://www.andymatuschak.org/xml-namespaces/sparkle">
  <channel>
    <title>Cook Sync Updates</title>
    <link>https://cook.md/appcast.xml</link>
    <description>Updates for Cook Sync</description>
    <language>en</language>

    <item>
      <title>Version ${VERSION_NUMBER}</title>
      <sparkle:version>${VERSION_NUMBER}</sparkle:version>
      <sparkle:releaseNotesLink>
        https://github.com/Cooklang/sync-agent/releases/tag/${VERSION}
      </sparkle:releaseNotesLink>
      <pubDate>$(date -R)</pubDate>
      <enclosure
        url="https://github.com/Cooklang/sync-agent/releases/download/${VERSION}/${APP_ZIP}"
        sparkle:version="${VERSION_NUMBER}"
EOF

# Add signature if available
if [ -n "$APP_SIG" ]; then
    cat >> appcast.xml <<EOF
        sparkle:edSignature="${APP_SIG}"
EOF
fi

cat >> appcast.xml <<EOF
        length="${APP_SIZE}"
        type="application/octet-stream" />
    </item>
  </channel>
</rss>
EOF

echo "✅ Generated appcast.xml"
echo ""
echo "Contents:"
cat appcast.xml
echo ""
echo "Next steps:"
echo "1. Review the appcast.xml file"
echo "2. Commit it to the repository: git add appcast.xml && git commit -m 'Update appcast for ${VERSION}'"
echo "3. Push to GitHub: git push"
echo ""
echo "The appcast will be available at:"
echo "https://raw.githubusercontent.com/Cooklang/sync-agent/main/appcast.xml"
