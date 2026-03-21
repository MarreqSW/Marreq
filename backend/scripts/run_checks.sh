#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BACKEND_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
REPO_ROOT="$(cd "${BACKEND_ROOT}/.." && pwd)"

failed=()

run_step() {
  local name="$1"
  shift
  if "$@" 2>&1; then
    echo "  ✓ ${name}"
    return 0
  else
    echo "  ✗ ${name}"
    failed+=("${name}")
    return 1
  fi
}

echo "Running checks..."
(
  cd "${REPO_ROOT}"
  run_step "cargo fmt" cargo fmt --all -- --check
  run_step "cargo clippy" cargo clippy --all-targets -- -D warnings
)
(
  cd "${REPO_ROOT}"
  run_step "stylelint" npx stylelint "backend/src/html/static/**/*.css" --config .stylelintrc.json
  run_step "purgecss" node .github/workflows/purgecss-ci.mjs
  run_step "npm ci" npm ci
  run_step "npm test" npm test
)

echo ""
if [[ ${#failed[@]} -eq 0 ]]; then
  echo "OK — all checks passed."
  exit 0
fi

echo "Problems:"
for name in "${failed[@]}"; do
  echo "  - ${name}"
done
exit 1
