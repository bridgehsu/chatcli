#!/usr/bin/env bash
set -euo pipefail

LABEL="com.chatcli"
PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
INSTALLED_PLIST="$HOME/Library/LaunchAgents/$LABEL.plist"
USER_DOMAIN="gui/$(id -u)"

if [[ "$(uname)" != "Darwin" ]]; then
  printf 'ChatCLI launchd control is supported only on macOS.\n' >&2
  exit 1
fi
if [[ ! -f "$INSTALLED_PLIST" ]]; then
  printf 'ChatCLI is not installed. Run %s first.\n' "$PROJECT_DIR/scripts/install.sh" >&2
  exit 1
fi

# A previous manual `launchctl disable` persists across installations.
launchctl enable "$USER_DOMAIN/$LABEL"
if ! launchctl print "$USER_DOMAIN/$LABEL" >/dev/null 2>&1; then
  launchctl bootstrap "$USER_DOMAIN" "$INSTALLED_PLIST"
fi
launchctl kickstart -k "$USER_DOMAIN/$LABEL"

printf 'Started launchd service: %s\n' "$LABEL"
