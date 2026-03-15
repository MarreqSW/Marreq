#!/usr/bin/env bash
set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${PROJECT_ROOT}"

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
run_step "cargo fmt" cargo fmt --all -- --check
run_step "cargo clippy" cargo clippy --all-targets -- -D warnings
run_step "stylelint" npx stylelint "src/html/static/**/*.css" --config .stylelintrc.json
run_step "purgecss" node .github/workflows/purgecss-ci.mjs
run_step "npm ci" npm ci
run_step "npm test" npm test

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
