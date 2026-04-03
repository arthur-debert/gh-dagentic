#!/usr/bin/env bash
# Usage: hook-wrapper.sh <name> <command> [args...]
# Runs command, prints PASS/FAIL, saves full log to temp file on failure.

set -euo pipefail

name="$1"
shift

logfile="$(mktemp "/tmp/dagentic-hook-${name}-XXXXXX.log")"

if "$@" >"$logfile" 2>&1; then
    echo "PASS  ${name}"
    rm -f "$logfile"
else
    status=$?
    echo "FAIL  ${name}  (full log: ${logfile})"
    exit $status
fi
