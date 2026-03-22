#!/usr/bin/env bash
set -euo pipefail

# Shared CI/local quality tasks for Marreq.
#
# Usage:
#   ./backend/scripts/run_ci.sh checks
#   ./backend/scripts/run_ci.sh tests
#   ./backend/scripts/run_ci.sh local-ci [--jobs N]

MODE="${1:-}"
if [[ -z "${MODE}" ]]; then
  echo "Usage: $0 <checks|tests|local-ci> [options]" >&2
  exit 1
fi
shift || true

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BACKEND_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
REPO_ROOT="$(cd "${BACKEND_ROOT}/.." && pwd)"
cd "${REPO_ROOT}"

run_local_ci() {
  local jobs=""

  while [[ $# -gt 0 ]]; do
    case "$1" in
      -j|--jobs)
        jobs="${2:-}"
        if [[ -z "${jobs}" ]]; then
          echo "Missing value for $1" >&2
          exit 1
        fi
        shift 2
        ;;
      -h|--help)
        echo "Usage: backend/scripts/run_ci.sh local-ci [OPTIONS]"
        echo ""
        echo "Options:"
        echo "  -j, --jobs NUMBER    Limit the number of parallel jobs (CPUs) to use"
        echo "  -h, --help           Show this help message"
        echo ""
        echo "Example: backend/scripts/run_ci.sh local-ci --jobs 2"
        return 0
        ;;
      *)
        echo "Unknown option: $1" >&2
        echo "Use --help for usage information" >&2
        exit 1
        ;;
    esac
  done

  local cargo_args=()
  if [[ -n "${jobs}" ]]; then
    cargo_args=(-j "${jobs}")
    echo "🔧 Limiting parallel jobs to ${jobs}"
  fi

  echo "🔍 Running all CI checks locally..."

  echo "1️⃣ Checking Rust formatting..."
  cargo fmt --all -- --check

  echo "2️⃣ Linting CSS..."
  npx stylelint "backend/src/html/static/**/*.css" --config .stylelintrc.json

  echo "3️⃣ Checking for unused CSS..."
  npm run check:unused-css

  echo "4️⃣ Running frontend tests..."
  npm test

  echo "5️⃣ Running backend tests with coverage..."
  export DATABASE_URL=postgres://rust:rust@127.0.0.1:5433/marreq
  cargo llvm-cov "${cargo_args[@]}" -p marreq --all-features --doctests --fail-under-lines 70

  echo "✅ All checks passed!"
}

case "${MODE}" in
  checks)
    exec bash "${SCRIPT_DIR}/run_checks.sh" "$@"
    ;;
  tests)
    exec bash "${SCRIPT_DIR}/run_tests.sh" "$@"
    ;;
  local-ci)
    run_local_ci "$@"
    ;;
  *)
    echo "Unknown mode: ${MODE}" >&2
    echo "Usage: $0 <checks|tests|local-ci> [options]" >&2
    exit 1
    ;;
esac
