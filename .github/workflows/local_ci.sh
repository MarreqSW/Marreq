#!/bin/bash
set -e

# Parse command line arguments
JOBS=""
while [[ $# -gt 0 ]]; do
    case $1 in
        -j|--jobs)
            JOBS="$2"
            shift 2
            ;;
        -h|--help)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  -j, --jobs NUMBER    Limit the number of parallel jobs (CPUs) to use"
            echo "  -h, --help          Show this help message"
            echo ""
            echo "Example: $0 --jobs 2"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

# Build cargo arguments with job limit if specified
CARGO_ARGS=""
if [ -n "$JOBS" ]; then
    CARGO_ARGS="-j $JOBS"
    echo "🔧 Limiting parallel jobs to $JOBS"
fi

echo "🔍 Running all CI checks locally..."

echo "1️⃣ Checking Rust formatting..."
cargo +nightly fmt --all -- --check

echo "2️⃣ Linting CSS..."
npx stylelint "src/html/static/**/*.css" --config .stylelintrc.json

echo "3️⃣ Checking for unused CSS..."
npm run check:unused-css

echo "4️⃣ Running frontend tests..."
npm test

echo "5️⃣ Running backend tests with coverage..."
export DATABASE_URL=postgres://rust:rust@127.0.0.1:5432/marreq
cargo +nightly llvm-cov $CARGO_ARGS --workspace --all-features --doctests --fail-under-lines 70

echo "✅ All checks passed!"