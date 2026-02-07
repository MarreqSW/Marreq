#!/usr/bin/env bash

FAILED=()

run() {
    local name="$1"
    shift
    if "$@" 2>&1; then
        echo "  ✓ $name"
        return 0
    else
        echo "  ✗ $name"
        FAILED+=("$name")
        return 1
    fi
}

echo "Running checks..."
run "cargo fmt" cargo fmt --all -- --check
run "cargo clippy" cargo clippy --all-targets -- -D warnings
run "stylelint" npx stylelint "src/html/static/**/*.css" --config .stylelintrc.json
run "purgecss" node .github/workflows/purgecss-ci.mjs
run "npm ci" npm ci
run "npm test" npm test

echo ""
if [ ${#FAILED[@]} -eq 0 ]; then
    echo "OK — all checks passed."
    exit 0
else
    echo "Problems:"
    for name in "${FAILED[@]}"; do
        echo "  - $name"
    done
    exit 1
fi
