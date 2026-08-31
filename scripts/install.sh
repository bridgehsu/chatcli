#!/usr/bin/env bash
set -euo pipefail

LABEL="com.chatcli"
PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CONFIG_PATH="$PROJECT_DIR/config.yaml"
TEMPLATE_PATH="$PROJECT_DIR/com.chatcli.plist"
LAUNCH_AGENTS_DIR="$HOME/Library/LaunchAgents"
INSTALLED_PLIST="$LAUNCH_AGENTS_DIR/$LABEL.plist"
USER_DOMAIN="gui/$(id -u)"

if [[ "$(uname)" != "Darwin" ]]; then
  printf 'ChatCLI launchd installation is supported only on macOS.\n' >&2
  exit 1
fi
if [[ ! -f "$CONFIG_PATH" ]]; then
  printf 'Missing %s. Copy config.example.yaml to config.yaml and configure Telegram first.\n' "$CONFIG_PATH" >&2
  exit 1
fi
if [[ ! -f "$TEMPLATE_PATH" ]]; then
  printf 'Missing launchd template: %s\n' "$TEMPLATE_PATH" >&2
  exit 1
fi

"$PROJECT_DIR/scripts/build.sh"
mkdir -p "$PROJECT_DIR/logs" "$LAUNCH_AGENTS_DIR"

# Keep the service tied to this checkout, so config.yaml and logs stay local.
cp "$TEMPLATE_PATH" "$INSTALLED_PLIST"
/usr/libexec/PlistBuddy -c "Set :ProgramArguments:0 $PROJECT_DIR/target/release/chatcli" "$INSTALLED_PLIST"
/usr/libexec/PlistBuddy -c "Set :WorkingDirectory $PROJECT_DIR" "$INSTALLED_PLIST"
/usr/libexec/PlistBuddy -c "Set :StandardOutPath $PROJECT_DIR/logs/launchd.out.log" "$INSTALLED_PLIST"
/usr/libexec/PlistBuddy -c "Set :StandardErrorPath $PROJECT_DIR/logs/launchd.err.log" "$INSTALLED_PLIST"

# Reload if an earlier installation exists. `bootout` returns non-zero if it does not.
launchctl bootout "$USER_DOMAIN/$LABEL" 2>/dev/null || true
"$PROJECT_DIR/bin/start.sh"

printf 'ChatCLI is running as %s. Logs: %s\n' "$LABEL" "$PROJECT_DIR/logs"
