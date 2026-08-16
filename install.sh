#!/usr/bin/env bash

set -Eeuo pipefail

readonly PROGRAM_NAME=${0##*/}
TMP_DIR=

die() {
  printf 'Error: %s\n' "$*" >&2
  exit 1
}

usage() {
  printf 'Usage: %s [/path/to/pg_config]\n' "$PROGRAM_NAME" >&2
}

cleanup() {
  if [ -n "$TMP_DIR" ] && [ -d "$TMP_DIR" ]; then
    rm -rf "$TMP_DIR"
  fi
}

trap cleanup EXIT
trap 'exit 129' HUP
trap 'exit 130' INT
trap 'exit 143' TERM

require_command() {
  command -v "$1" >/dev/null 2>&1 || die "Required command not found: $1"
}

validate_release_version() {
  local version=$1
  local prerelease identifier
  local -a identifiers

  [ -n "$version" ] || die "RELEASE_VERSION must not be empty"
  [[ "$version" != v* ]] || die "RELEASE_VERSION must not start with 'v'"
  [[ "$version" != */* ]] || die "RELEASE_VERSION must not contain '/'"
  [[ "$version" =~ ^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)(-([0-9A-Za-z-]+)(\.[0-9A-Za-z-]+)*)?(\+([0-9A-Za-z-]+)(\.[0-9A-Za-z-]+)*)?$ ]] ||
    die "RELEASE_VERSION must be a bare semantic version"

  prerelease=${version%%+*}
  if [[ "$prerelease" == *-* ]]; then
    prerelease=${prerelease#*-}
    IFS='.' read -r -a identifiers <<<"$prerelease"
    for identifier in "${identifiers[@]}"; do
      if [[ "$identifier" =~ ^[0-9]+$ ]] && [ "${#identifier}" -gt 1 ] && [[ "$identifier" == 0* ]]; then
        die "RELEASE_VERSION has a numeric prerelease identifier with a leading zero"
      fi
    done
  fi
}

if [ "$#" -gt 1 ]; then
  usage
  die "Expected zero or one pg_config argument"
fi

require_command uname
require_command mktemp
require_command rm
require_command tar
require_command install

if [ "$#" -eq 1 ]; then
  PG_CONFIG=$1
else
  PG_CONFIG=$(command -v pg_config 2>/dev/null || true)
fi

[ -n "$PG_CONFIG" ] && [ -x "$PG_CONFIG" ] ||
  die "pg_config was not found or is not executable"

case "$(uname -s)" in
  Linux) OS=linux ;;
  Darwin) OS=darwin ;;
  *) die "Unsupported operating system: $(uname -s)" ;;
esac

case "$(uname -m)" in
  x86_64 | amd64) ARCH=amd64 ;;
  aarch64 | arm64) ARCH=arm64 ;;
  *) die "Unsupported architecture: $(uname -m)" ;;
esac

PG_CONFIG_VERSION=$("$PG_CONFIG" --version) ||
  die "Unable to query PostgreSQL version from pg_config"
if [[ "$PG_CONFIG_VERSION" =~ ^PostgreSQL[[:space:]]+([0-9]+)(\.|$) ]]; then
  PG_MAJOR=${BASH_REMATCH[1]}
else
  die "Unable to parse PostgreSQL version: $PG_CONFIG_VERSION"
fi
if [ "$PG_MAJOR" -lt 13 ] || [ "$PG_MAJOR" -gt 19 ]; then
  die "Unsupported PostgreSQL major version: $PG_MAJOR (expected 13..19)"
fi

RELEASE_VERSION=${RELEASE_VERSION:-}
validate_release_version "$RELEASE_VERSION"

SHARE_DIR=$("$PG_CONFIG" --sharedir) ||
  die "Unable to query PostgreSQL shared directory"
LIB_DIR=$("$PG_CONFIG" --pkglibdir) ||
  die "Unable to query PostgreSQL library directory"
EXTENSION_DIR="$SHARE_DIR/extension"

[ -d "$LIB_DIR" ] && [ -w "$LIB_DIR" ] ||
  die "PostgreSQL library destination is not a writable directory: $LIB_DIR"
[ -d "$EXTENSION_DIR" ] && [ -w "$EXTENSION_DIR" ] ||
  die "PostgreSQL extension destination is not a writable directory: $EXTENSION_DIR"

ASSET_BASENAME="typeid-pg${PG_MAJOR}-${OS}-${ARCH}.tar.gz"
CHECKSUM_BASENAME="${ASSET_BASENAME}.sha256"
LIBRARY_NAME=typeid.so
if [ "$OS" = darwin ]; then
  LIBRARY_NAME=typeid.dylib
fi

if [ "$OS" = linux ]; then
  require_command sha256sum
  CHECKSUM_COMMAND=(sha256sum -c)
else
  require_command shasum
  CHECKSUM_COMMAND=(shasum -a 256 -c)
fi

LOCAL_MODE=false
if [ "${TYPEID_ARCHIVE+x}" = x ] || [ "${TYPEID_ARCHIVE_SHA256+x}" = x ]; then
  LOCAL_MODE=true
  [ -n "${TYPEID_ARCHIVE:-}" ] && [ -n "${TYPEID_ARCHIVE_SHA256:-}" ] ||
    die "Local mode requires TYPEID_ARCHIVE and TYPEID_ARCHIVE_SHA256"
  [ "${TYPEID_ARCHIVE##*/}" = "$ASSET_BASENAME" ] ||
    die "Archive basename must be $ASSET_BASENAME"
  [ "${TYPEID_ARCHIVE_SHA256##*/}" = "$CHECKSUM_BASENAME" ] ||
    die "Checksum basename must be $CHECKSUM_BASENAME"
  [ -f "$TYPEID_ARCHIVE" ] && [ ! -L "$TYPEID_ARCHIVE" ] ||
    die "Archive is not a regular file: $TYPEID_ARCHIVE"
  [ -f "$TYPEID_ARCHIVE_SHA256" ] && [ ! -L "$TYPEID_ARCHIVE_SHA256" ] ||
    die "Checksum sidecar is not a regular file: $TYPEID_ARCHIVE_SHA256"
else
  require_command curl
fi

umask 077
TMP_DIR=$(mktemp -d "${TMPDIR:-/tmp}/typeid-postgres-install.XXXXXX") ||
  die "Unable to create a private temporary directory"
[ -d "$TMP_DIR" ] || die "Temporary directory was not created"

ARCHIVE_PATH="$TMP_DIR/$ASSET_BASENAME"
CHECKSUM_PATH="$TMP_DIR/$CHECKSUM_BASENAME"

if [ "$LOCAL_MODE" = true ]; then
  install -m 0600 "$TYPEID_ARCHIVE" "$ARCHIVE_PATH"
  install -m 0600 "$TYPEID_ARCHIVE_SHA256" "$CHECKSUM_PATH"
else
  DOWNLOAD_BASE="https://github.com/blitss/typeid-postgres/releases/download/v${RELEASE_VERSION}"
  CURL_OPTIONS=(--fail --silent --show-error --location --retry 3)
  if [ -n "${GITHUB_TOKEN:-}" ]; then
    CURL_OPTIONS+=(--header "Authorization: Bearer ${GITHUB_TOKEN}")
  fi
  curl "${CURL_OPTIONS[@]}" --output "$ARCHIVE_PATH" \
    "${DOWNLOAD_BASE}/${ASSET_BASENAME}"
  curl "${CURL_OPTIONS[@]}" --output "$CHECKSUM_PATH" \
    "${DOWNLOAD_BASE}/${CHECKSUM_BASENAME}"
fi

CHECKSUM_LINE=
CHECKSUM_LINE_COUNT=0
while IFS= read -r line || [ -n "$line" ]; do
  CHECKSUM_LINE_COUNT=$((CHECKSUM_LINE_COUNT + 1))
  [ "$CHECKSUM_LINE_COUNT" -eq 1 ] ||
    die "Checksum sidecar must contain exactly one line"
  CHECKSUM_LINE=$line
done <"$CHECKSUM_PATH"
[ "$CHECKSUM_LINE_COUNT" -eq 1 ] ||
  die "Checksum sidecar must contain exactly one line"

DIGEST=${CHECKSUM_LINE:0:64}
[[ "$DIGEST" =~ ^[0-9a-f]{64}$ ]] &&
  [ "$CHECKSUM_LINE" = "$DIGEST  $ASSET_BASENAME" ] ||
  die "Checksum sidecar must contain one lowercase SHA-256 digest, two spaces, and $ASSET_BASENAME"

(
  cd "$TMP_DIR"
  "${CHECKSUM_COMMAND[@]}" "$CHECKSUM_BASENAME"
)

EXPECTED_MEMBERS=(
  "lib/$LIBRARY_NAME"
  "share/extension/typeid.control"
  "share/extension/typeid--0.1.0.sql"
  "share/extension/typeid--0.2.0.sql"
  "share/extension/typeid--0.3.0.sql"
  "share/extension/typeid--0.4.0.sql"
  "share/extension/typeid--0.1.0--0.2.0.sql"
  "share/extension/typeid--0.2.0--0.3.0.sql"
  "share/extension/typeid--0.3.0--0.4.0.sql"
)

MEMBER_LIST="$TMP_DIR/members"
MEMBER_DETAILS="$TMP_DIR/member-details"
tar -tzf "$ARCHIVE_PATH" >"$MEMBER_LIST" ||
  die "Unable to list archive members"
tar -tvzf "$ARCHIVE_PATH" >"$MEMBER_DETAILS" ||
  die "Unable to inspect archive member types"

SEEN_MEMBERS=$'\n'
MEMBER_COUNT=0
while IFS= read -r member || [ -n "$member" ]; do
  MEMBER_COUNT=$((MEMBER_COUNT + 1))
  [ -n "$member" ] || die "Archive contains an empty member name"
  [[ "$member" != /* ]] || die "Archive contains an absolute path: $member"
  [[ ! "$member" =~ (^|/)\.\.(/|$) ]] ||
    die "Archive contains a parent-path component: $member"

  expected=false
  for expected_member in "${EXPECTED_MEMBERS[@]}"; do
    if [ "$member" = "$expected_member" ]; then
      expected=true
      break
    fi
  done
  [ "$expected" = true ] || die "Archive contains an unexpected member: $member"

  case "$SEEN_MEMBERS" in
    *$'\n'"$member"$'\n'*) die "Archive contains a duplicate member: $member" ;;
  esac
  SEEN_MEMBERS+="$member"$'\n'
done <"$MEMBER_LIST"

[ "$MEMBER_COUNT" -eq "${#EXPECTED_MEMBERS[@]}" ] ||
  die "Archive is missing one or more required members"
for expected_member in "${EXPECTED_MEMBERS[@]}"; do
  case "$SEEN_MEMBERS" in
    *$'\n'"$expected_member"$'\n'*) ;;
    *) die "Archive is missing required member: $expected_member" ;;
  esac
done

DETAIL_COUNT=0
while IFS= read -r detail || [ -n "$detail" ]; do
  DETAIL_COUNT=$((DETAIL_COUNT + 1))
  [ "${detail:0:1}" = "-" ] ||
    die "Archive links, directories, and special files are not allowed"
done <"$MEMBER_DETAILS"
[ "$DETAIL_COUNT" -eq "$MEMBER_COUNT" ] ||
  die "Archive member metadata is inconsistent"

EXTRACT_DIR="$TMP_DIR/extract"
install -d -m 0700 "$EXTRACT_DIR"
tar -xzf "$ARCHIVE_PATH" -C "$EXTRACT_DIR" ||
  die "Unable to extract archive"

for expected_member in "${EXPECTED_MEMBERS[@]}"; do
  extracted_path="$EXTRACT_DIR/$expected_member"
  [ -f "$extracted_path" ] && [ ! -L "$extracted_path" ] ||
    die "Extracted member is not a regular file: $expected_member"
done

install -m 0755 "$EXTRACT_DIR/lib/$LIBRARY_NAME" "$LIB_DIR/$LIBRARY_NAME"
for expected_member in "${EXPECTED_MEMBERS[@]:1}"; do
  extension_name=${expected_member##*/}
  install -m 0644 "$EXTRACT_DIR/$expected_member" "$EXTENSION_DIR/$extension_name"
done

printf 'TypeID %s installed for PostgreSQL %s (%s/%s).\n' \
  "$RELEASE_VERSION" "$PG_MAJOR" "$OS" "$ARCH"