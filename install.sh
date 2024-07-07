#!/bin/bash

set -e

# Check if a custom pg_config path is provided
if [ $# -eq 1 ]; then
    PG_CONFIG="$1"
else
    PG_CONFIG=$(which pg_config)
fi

# Function to get the latest release version
get_latest_release() {
  curl --silent "https://api.github.com/repos/blitss/typeid-postgres/releases/latest" | 
  grep '"tag_name":' |
  sed -E 's/.*"([^"]+)".*/\1/'
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

# Get the latest release version
RELEASE_VERSION=$(get_latest_release)

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
cp "$TMP_DIR"/usr/share/postgresql/*/extension/typeid.control "$EXTENSION_DIR"
cp "$TMP_DIR"/usr/share/postgresql/*/extension/typeid--*.sql "$EXTENSION_DIR"
cp "$TMP_DIR"/usr/lib/postgresql/*/lib/typeid.so "$LIB_DIR"

# Clean up
rm -rf "$TMP_DIR"

echo "TypeID extension installed successfully."
echo "To enable the extension, run: CREATE EXTENSION typeid;"