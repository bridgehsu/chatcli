#!/usr/bin/env bash
set -euo pipefail

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

"$PROJECT_DIR/bin/stop.sh"
"$PROJECT_DIR/bin/start.sh"
