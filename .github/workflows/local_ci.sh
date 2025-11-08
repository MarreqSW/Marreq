#!/bin/bash
set -e

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
export DATABASE_URL=postgres://rust:rust@127.0.0.1:5432/reqman
cargo llvm-cov --workspace --all-features --doctests --fail-under-lines 70

echo "✅ All checks passed!"