#!/usr/bin/env bash
set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

echo "=== cargo fmt ==="
cargo fmt --check
