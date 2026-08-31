#!/usr/bin/env bash
set -euo pipefail

LABEL="com.chatcli"
USER_DOMAIN="gui/$(id -u)"

if [[ "$(uname)" != "Darwin" ]]; then
  printf 'ChatCLI launchd control is supported only on macOS.\n' >&2
  exit 1
fi

if launchctl print "$USER_DOMAIN/$LABEL" >/dev/null 2>&1; then
  launchctl bootout "$USER_DOMAIN/$LABEL"
  printf 'Stopped launchd service: %s\n' "$LABEL"
else
  printf 'ChatCLI is not running.\n'
fi
