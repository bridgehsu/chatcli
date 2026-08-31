#!/usr/bin/env bash
set -euo pipefail

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LABEL="com.chatcli"
USER_DOMAIN="gui/$(id -u)"

usage() {
  cat <<'EOF'
Usage: ./bin/cmd.sh <command>

Commands:
  build       Format-check, test, lint, and build the release binary
  run         Run ChatCLI in the foreground
  install     Build, install, and start the service for this login session
  start       Start the installed macOS launchd service
  stop        Stop the running macOS launchd service
  restart     Restart the installed macOS launchd service
  uninstall   Stop and remove the macOS launchd service
  status      Show launchd service status
  logs        Follow launchd stdout and stderr logs
  help        Show this help message
EOF
}

require_macos() {
  if [[ "$(uname)" != "Darwin" ]]; then
    printf 'This command is supported only on macOS.\n' >&2
    exit 1
  fi
}

command="${1:-help}"
case "$command" in
  build)
    exec "$PROJECT_DIR/scripts/build.sh"
    ;;
  run)
    exec "$PROJECT_DIR/bin/run.sh"
    ;;
  install)
    exec "$PROJECT_DIR/scripts/install.sh"
    ;;
  start)
    exec "$PROJECT_DIR/bin/start.sh"
    ;;
  stop)
    exec "$PROJECT_DIR/bin/stop.sh"
    ;;
  restart)
    exec "$PROJECT_DIR/bin/restart.sh"
    ;;
  uninstall)
    exec "$PROJECT_DIR/scripts/uninstall.sh"
    ;;
  status)
    require_macos
    launchctl print "$USER_DOMAIN/$LABEL"
    ;;
  logs)
    require_macos
    mkdir -p "$PROJECT_DIR/logs"
    exec tail -n 100 -f "$PROJECT_DIR/logs/launchd.out.log" "$PROJECT_DIR/logs/launchd.err.log"
    ;;
  help|-h|--help)
    usage
    ;;
  *)
    printf 'Unknown command: %s\n\n' "$command" >&2
    usage >&2
    exit 2
    ;;
esac
