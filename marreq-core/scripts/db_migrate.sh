#!/bin/bash
# One supported runner for local development and release operations.
set -euo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$root"
case "${1:-up}" in
  up) command=apply ;;
  dev-init) command=dev-init ;;
  check|list) command=check ;;
  *) echo 'Usage: db_migrate.sh {up|dev-init|check}; destructive Diesel commands are development-only manual operations.' >&2; exit 1 ;;
esac
# dotenv reads without printing secrets; explicit environment takes precedence.
exec cargo run --locked -p marreq-migrations -- "$command"
