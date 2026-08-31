#!/usr/bin/env bash
set -euo pipefail

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

cd "$PROJECT_DIR"
cargo fmt --check
cargo test
cargo clippy -- -D warnings
cargo build --release

printf 'Built: %s\n' "$PROJECT_DIR/target/release/chatcli"
