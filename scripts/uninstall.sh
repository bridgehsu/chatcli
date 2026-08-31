#!/usr/bin/env bash
set -euo pipefail

LABEL="com.chatcli"
INSTALLED_PLIST="$HOME/Library/LaunchAgents/$LABEL.plist"
USER_DOMAIN="gui/$(id -u)"

if [[ "$(uname)" != "Darwin" ]]; then
  printf 'ChatCLI launchd uninstallation is supported only on macOS.\n' >&2
  exit 1
fi

"$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/bin/stop.sh"
rm -f "$INSTALLED_PLIST"

printf 'Removed launchd service: %s\n' "$LABEL"
