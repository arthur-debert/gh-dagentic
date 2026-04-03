#!/usr/bin/env bash
set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

echo "=== cargo clippy ==="
cargo clippy --all-targets -- -D warnings

echo "=== cargo test ==="
cargo test
