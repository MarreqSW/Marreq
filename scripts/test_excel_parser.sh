#!/bin/bash

echo "🧪 Testing Excel Parser for Marreq"
echo "=================================="

# Check if Marreq server is running
echo "🔍 Checking if Marreq server is running..."
if curl -s http://127.0.0.1:8000 > /dev/null; then
    echo "✅ Marreq server is running"
else
    echo "❌ Marreq server is not running. Please start it first with 'cargo run'"
    exit 1
fi

# Create test directory
mkdir -p test_exports

echo ""
echo "📥 Downloading test Excel files..."

# Download requirements export
echo "📋 Downloading requirements.xls..."
curl -s -o test_exports/requirements.xls http://127.0.0.1:8000/requirements.xls

# Download tests export
echo "🧪 Downloading tests.xls..."
curl -s -o test_exports/tests.xls http://127.0.0.1:8000/tests.xls

# Download matrix export
echo "📊 Downloading matrix.xls..."
curl -s -o test_exports/matrix.xls http://127.0.0.1:8000/matrix.xls

echo ""
echo "🔍 Testing Excel Parser..."

# Test requirements parsing (dry run)
echo "📋 Testing requirements parsing (dry run)..."
./excel_parser/target/release/excel_parser -f test_exports/requirements.xls --dry-run

echo ""
echo "🧪 Testing tests parsing (dry run)..."
./excel_parser/target/release/excel_parser -f test_exports/tests.xls --dry-run

echo ""
echo "💾 Generating JSON files..."

# Generate JSON files
echo "📋 Generating requirements.json..."
./excel_parser/target/release/excel_parser -f test_exports/requirements.xls --json-only -o test_exports/requirements.json

echo "🧪 Generating tests.json..."
./excel_parser/target/release/excel_parser -f test_exports/tests.xls --json-only -o test_exports/tests.json

echo ""
echo "📁 Generated files:"
ls -la test_exports/

echo ""
echo "🎉 Test completed! Check the test_exports/ directory for generated files."
echo ""
echo "To import data into Marreq API, run:"
echo "  ./excel_parser/target/release/excel_parser -f test_exports/requirements.xls"
echo "  ./excel_parser/target/release/excel_parser -f test_exports/tests.xls" 