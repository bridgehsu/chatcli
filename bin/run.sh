#!/usr/bin/env bash
set -euo pipefail

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CONFIG_PATH="$PROJECT_DIR/config.yaml"

if [[ ! -f "$CONFIG_PATH" ]]; then
  printf 'Missing %s. Copy config.example.yaml to config.yaml and configure Telegram first.\n' "$CONFIG_PATH" >&2
  exit 1
fi

cd "$PROJECT_DIR"
exec cargo run --release
