#!/bin/bash
set -e

echo "🚀 Running all ReqMan tests..."
echo "----------------------------------------"

# Run tests and capture output while also showing it
# We use pipefail to catch build errors
set -o pipefail
cargo test --features test-helpers --quiet 2>&1 | tee test_output.log

echo ""
echo "📊 Test Summary"
echo "----------------------------------------"

# Extract and sum up results
TOTAL_PASSED=$(grep "test result: ok" test_output.log | awk '{sum += $4} END {print sum}')
TOTAL_FAILED=$(grep "test result: ok" test_output.log | awk '{sum += $6} END {print sum}')
SUITES_RUN=$(grep -c "test result: ok" test_output.log)

echo "✅ Total Passed: $TOTAL_PASSED"
if [ "$TOTAL_FAILED" -gt 0 ]; then
    echo "❌ Total Failed: $TOTAL_FAILED"
else
    echo "❌ Total Failed: 0"
fi
echo "📦 Test Suites:  $SUITES_RUN"
echo "----------------------------------------"

if [ "$TOTAL_FAILED" -eq 0 ]; then
    echo "🎉 All tests passed!"
    rm test_output.log
    exit 0
else
    echo "💥 Some tests failed. See output above for details."
    rm test_output.log
    exit 1
fi
