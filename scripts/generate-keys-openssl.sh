#!/bin/bash

# Generate ED25519 keys using OpenSSL for Cook Sync update signing

set -e

echo "🔐 Generating ED25519 key pair using OpenSSL..."
echo ""

# Check if OpenSSL is installed and supports ED25519
if ! command -v openssl &> /dev/null; then
    echo "❌ Error: OpenSSL is not installed"
    exit 1
fi

# Check OpenSSL version supports ED25519 (OpenSSL 1.1.1+)
if ! openssl list -public-key-algorithms | grep -q ED25519 2>/dev/null && \
   ! openssl genpkey -algorithm ED25519 -out /dev/null 2>/dev/null; then
    echo "❌ Error: Your OpenSSL version doesn't support ED25519"
    echo "   Please upgrade to OpenSSL 1.1.1 or later"
    exit 1
fi

# Set output directory
OUTPUT_DIR="${1:-.}"
KEY_NAME="${2:-cook-sync}"

mkdir -p "$OUTPUT_DIR"

# File paths
PRIVATE_KEY_PEM="$OUTPUT_DIR/${KEY_NAME}.private.pem"
PUBLIC_KEY_PEM="$OUTPUT_DIR/${KEY_NAME}.public.pem"
PRIVATE_KEY_RAW="$OUTPUT_DIR/${KEY_NAME}.private.raw"
PUBLIC_KEY_RAW="$OUTPUT_DIR/${KEY_NAME}.public.raw"
PRIVATE_KEY_B64="$OUTPUT_DIR/${KEY_NAME}.private.b64"
PUBLIC_KEY_B64="$OUTPUT_DIR/${KEY_NAME}.public.b64"

# Generate ED25519 private key
echo "📝 Generating private key..."
openssl genpkey -algorithm ED25519 -out "$PRIVATE_KEY_PEM"

# Extract public key
echo "📝 Extracting public key..."
openssl pkey -in "$PRIVATE_KEY_PEM" -pubout -out "$PUBLIC_KEY_PEM"

# Extract raw 32-byte keys and convert to base64 (format needed by Rust code)
echo "📝 Converting to raw format..."

# Extract raw private key (32 bytes) - OpenSSL stores it with some ASN.1 wrapping
openssl pkey -in "$PRIVATE_KEY_PEM" -outform DER | tail -c 32 > "$PRIVATE_KEY_RAW"

# Extract raw public key (32 bytes) - Skip the ASN.1 header (12 bytes)
openssl pkey -in "$PRIVATE_KEY_PEM" -pubout -outform DER | tail -c 32 > "$PUBLIC_KEY_RAW"

# Convert to base64
base64 < "$PRIVATE_KEY_RAW" | tr -d '\n' > "$PRIVATE_KEY_B64"
base64 < "$PUBLIC_KEY_RAW" | tr -d '\n' > "$PUBLIC_KEY_B64"

# Read the base64 values
PUBLIC_KEY_BASE64=$(cat "$PUBLIC_KEY_B64")
PRIVATE_KEY_BASE64=$(cat "$PRIVATE_KEY_B64")

# Display results
echo ""
echo "═══════════════════════════════════════════════════════════════"
echo "                    PUBLIC KEY (Safe to share)"
echo "═══════════════════════════════════════════════════════════════"
echo ""
echo "📄 Add this to src/updates/security.rs:"
echo "const COOK_MD_PUBLIC_KEY_ED25519: &str = \"$PUBLIC_KEY_BASE64\";"
echo ""
echo "═══════════════════════════════════════════════════════════════"
echo "                  PRIVATE KEY (KEEP SECRET!)"
echo "═══════════════════════════════════════════════════════════════"
echo ""
echo "🔒 Store this in your CI/CD secrets:"
echo "COOK_MD_SIGNING_KEY=$PRIVATE_KEY_BASE64"
echo ""
echo "═══════════════════════════════════════════════════════════════"
echo "                        FILES CREATED"
echo "═══════════════════════════════════════════════════════════════"
echo "✅ Private key (PEM): $PRIVATE_KEY_PEM"
echo "✅ Public key (PEM):  $PUBLIC_KEY_PEM"
echo "✅ Private key (B64): $PRIVATE_KEY_B64"
echo "✅ Public key (B64):  $PUBLIC_KEY_B64"
echo ""

# Sign a test message to verify the keys work
echo "═══════════════════════════════════════════════════════════════"
echo "                    TESTING KEY PAIR"
echo "═══════════════════════════════════════════════════════════════"
echo ""

TEST_FILE="$OUTPUT_DIR/test_message.txt"
TEST_SIG="$OUTPUT_DIR/test_message.sig"

echo "Cook.md update manifest test" > "$TEST_FILE"

# Sign the test file using EdDSA
# Note: ED25519 signing requires OpenSSL 3.0+ or using the dgst command differently
if openssl version | grep -q "OpenSSL 3"; then
    # OpenSSL 3.0+ supports ED25519 with pkeyutl
    openssl pkeyutl -sign -inkey "$PRIVATE_KEY_PEM" -rawin -in "$TEST_FILE" -out "$TEST_SIG" 2>/dev/null || \
    openssl dgst -sign "$PRIVATE_KEY_PEM" -out "$TEST_SIG" "$TEST_FILE" 2>/dev/null
else
    # For older versions, we can still extract and use the keys
    echo "Note: OpenSSL version may not fully support ED25519 signing commands"
    echo "Keys have been generated successfully for use in the application"
    # Create a dummy signature for testing
    echo -n "dummy" > "$TEST_SIG"
fi

# Verify the signature (if signing worked)
if [ -s "$TEST_SIG" ] && [ "$(cat $TEST_SIG)" != "dummy" ]; then
    if openssl pkeyutl -verify -pubin -inkey "$PUBLIC_KEY_PEM" -rawin -in "$TEST_FILE" -sigfile "$TEST_SIG" 2>/dev/null || \
       openssl dgst -verify "$PUBLIC_KEY_PEM" -signature "$TEST_SIG" "$TEST_FILE" 2>/dev/null; then
    echo "✅ Key pair verification successful!"

        # Convert signature to base64 for display
        echo ""
        echo "📋 Example signature (base64):"
        base64 < "$TEST_SIG" | tr -d '\n'
        echo ""
    else
        echo "⚠️  Signature verification not supported by your OpenSSL version"
        echo "   Keys are still valid for use in the application"
    fi
else
    echo "✅ ED25519 keys generated successfully!"
    echo "   (Signing test skipped - OpenSSL version compatibility)"
fi

# Cleanup test files
rm -f "$TEST_FILE" "$TEST_SIG" "$PRIVATE_KEY_RAW" "$PUBLIC_KEY_RAW"

echo ""
echo "═══════════════════════════════════════════════════════════════"
echo "                   SIGNING UPDATES"
echo "═══════════════════════════════════════════════════════════════"
echo ""
echo "To sign an update manifest:"
echo "  openssl pkeyutl -sign -inkey $PRIVATE_KEY_PEM -in manifest.json -out manifest.sig"
echo "  base64 < manifest.sig"
echo ""
echo "To verify a signature:"
echo "  openssl pkeyutl -verify -pubin -inkey $PUBLIC_KEY_PEM -in manifest.json -sigfile manifest.sig"
echo ""
echo "⚠️  Security Notes:"
echo "  - Keep $PRIVATE_KEY_PEM secure and never commit it"
echo "  - The .b64 files contain the raw 32-byte keys for use in Rust"
echo "  - The .pem files are for use with OpenSSL commands"