#!/usr/bin/env bash
set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${PROJECT_ROOT}"

log_file="test_output.log"

echo "🚀 Running all Marreq tests..."
echo "----------------------------------------"

set -o pipefail
cargo test --features test-helpers --quiet 2>&1 | tee "${log_file}"

echo ""
echo "📊 Test Summary"
echo "----------------------------------------"

total_passed=$(grep "test result: ok" "${log_file}" | awk '{sum += $4} END {print sum+0}')
total_failed=$(grep "test result: ok" "${log_file}" | awk '{sum += $6} END {print sum+0}')
suites_run=$(grep -c "test result: ok" "${log_file}" || true)

echo "✅ Total Passed: ${total_passed}"
echo "❌ Total Failed: ${total_failed}"
echo "📦 Test Suites:  ${suites_run}"
echo "----------------------------------------"

rm -f "${log_file}"

if [[ "${total_failed}" -eq 0 ]]; then
  echo "🎉 All tests passed!"
  exit 0
fi

echo "💥 Some tests failed. See output above for details."
exit 1
