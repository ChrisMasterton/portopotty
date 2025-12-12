#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
APP_NAME="Port-o-Potty"
SRC_APP="${ROOT_DIR}/src-tauri/target/release/bundle/macos/${APP_NAME}.app"
DEST_APP="/Applications/${APP_NAME}.app"

FORCE=0
NO_BUILD=0

usage() {
  cat <<EOF
Usage: ./install.sh [options]

Builds the Tauri app and installs it into /Applications.

Options:
  --force     Replace existing app without prompting
  --no-build  Skip build; only copy the existing .app
  -h, --help  Show this help
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --force) FORCE=1; shift ;;
    --no-build) NO_BUILD=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *) echo "Unknown option: $1" >&2; usage; exit 2 ;;
  esac
done

cd "$ROOT_DIR"

if [[ "$NO_BUILD" -eq 0 ]]; then
  if [[ ! -d "${ROOT_DIR}/node_modules" ]]; then
    npm install
  fi
  npm run tauri:build
fi

if [[ ! -d "$SRC_APP" ]]; then
  echo "Build output not found at: $SRC_APP" >&2
  echo "Try running: npm run tauri:build" >&2
  exit 1
fi

if [[ -d "$DEST_APP" && "$FORCE" -ne 1 ]]; then
  read -r -p "Replace existing ${DEST_APP}? [y/N] " ans
  if [[ "${ans}" != "y" && "${ans}" != "Y" ]]; then
    echo "Cancelled."
    exit 0
  fi
fi

TMP_APP="/tmp/${APP_NAME}.app.$$"
rm -rf "$TMP_APP"
ditto "$SRC_APP" "$TMP_APP"

remove_dest() {
  rm -rf "$DEST_APP" 2>/dev/null || sudo rm -rf "$DEST_APP"
}

copy_into_apps() {
  ditto "$TMP_APP" "$DEST_APP" 2>/dev/null || sudo ditto "$TMP_APP" "$DEST_APP"
}

if [[ -d "$DEST_APP" ]]; then
  remove_dest
fi

copy_into_apps
rm -rf "$TMP_APP"

echo "Installed: $DEST_APP"
echo "Launch from Applications or Spotlight: ${APP_NAME}"

