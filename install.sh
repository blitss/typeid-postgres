#!/bin/bash

set -e

# Check if a custom pg_config path is provided
if [ $# -eq 1 ]; then
    PG_CONFIG="$1"
else
    PG_CONFIG=$(which pg_config 2>/dev/null || true)
fi

# Check if pg_config exists and is executable
if [ -z "$PG_CONFIG" ] || [ ! -x "$PG_CONFIG" ]; then
    echo "Error: pg_config not found or not executable."
    echo "Please install PostgreSQL development packages or provide the path to pg_config as an argument."
    echo "Usage: $0 [/path/to/pg_config]"
    exit 1
fi

echo "Using pg_config: $PG_CONFIG"

# Function to get the latest release version
get_latest_release() {
  if [ -n "$GITHUB_TOKEN" ]; then
    curl --silent -H "Authorization: Bearer $GITHUB_TOKEN" \
      "https://api.github.com/repos/blitss/typeid-postgres/releases/latest"
  else
    curl --silent "https://api.github.com/repos/blitss/typeid-postgres/releases/latest"
  fi \
  | grep '"tag_name":' \
  | sed -E 's/.*"([^"]+)".*/\1/'
}

# Detect OS and architecture
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

if [ "$ARCH" = "x86_64" ]; then
  ARCH="amd64"
elif [ "$ARCH" = "aarch64" ] || [ "$ARCH" = "arm64" ]; then
  ARCH="arm64"
else
  echo "Unsupported architecture: $ARCH"
  exit 1
fi

# Get PostgreSQL version
PG_VERSION=$("$PG_CONFIG" --version | awk '{print $2}' | cut -d. -f1)

if [ -z "$PG_VERSION" ]; then
  echo "PostgreSQL not found. Please install PostgreSQL and make sure 'pg_config' is in your PATH or provide the path to pg_config as an argument."
  exit 1
fi

# Determine which TypeID-Postgres release to install.
# If the user sets the RELEASE_VERSION environment variable we honour it,
# otherwise fall back to fetching the latest release tag from GitHub.
if [ -z "$RELEASE_VERSION" ]; then
  echo "RELEASE_VERSION not provided – fetching the latest release tag from GitHub …"
  RELEASE_VERSION=$(get_latest_release)

  echo "Latest release version: $RELEASE_VERSION"
else
  echo "Using user-supplied RELEASE_VERSION: $RELEASE_VERSION"
fi

# Download URL
DOWNLOAD_URL="https://github.com/blitss/typeid-postgres/releases/download/${RELEASE_VERSION}/typeid-pg${PG_VERSION}-${OS}-${ARCH}.tar.gz"

# Temporary directory for extraction
TMP_DIR=$(mktemp -d)

# Download and extract
echo "Downloading TypeID extension from $DOWNLOAD_URL"
curl -L "$DOWNLOAD_URL" | tar xz -C "$TMP_DIR"

# Get PostgreSQL directories
EXTENSION_DIR=$("$PG_CONFIG" --sharedir)/extension
LIB_DIR=$("$PG_CONFIG" --pkglibdir)

# Install files
echo "Installing TypeID extension..."
if [ "$OS" = "darwin" ]; then
  # macOS with Homebrew paths
  cp "$TMP_DIR"/opt/homebrew/opt/postgresql*/share/postgresql*/extension/typeid.control "$EXTENSION_DIR"
  cp "$TMP_DIR"/opt/homebrew/opt/postgresql*/share/postgresql*/extension/typeid--*.sql "$EXTENSION_DIR"
  cp "$TMP_DIR"/opt/homebrew/opt/postgresql*/lib/postgresql/typeid.dylib "$LIB_DIR"
else
  # Linux paths
  cp "$TMP_DIR"/usr/share/postgresql/*/extension/typeid.control "$EXTENSION_DIR"
  cp "$TMP_DIR"/usr/share/postgresql/*/extension/typeid--*.sql "$EXTENSION_DIR"
  cp "$TMP_DIR"/usr/lib/postgresql/*/lib/typeid.so "$LIB_DIR"
fi

# Clean up
rm -rf "$TMP_DIR"

echo "TypeID extension installed successfully."
echo "To enable the extension, run: CREATE EXTENSION typeid;"