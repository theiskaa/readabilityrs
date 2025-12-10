#!/bin/bash
# Benchmark runner script for comparing ReadabilityRS with Mozilla Readability.js
#
# Usage: ./run_benchmarks.sh
#
# Prerequisites:
#   - Rust toolchain (cargo)
#   - Node.js (npm)

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

echo "=============================================="
echo "ReadabilityRS Benchmark Suite"
echo "=============================================="
echo ""

# Check prerequisites
command -v cargo >/dev/null 2>&1 || { echo "Error: cargo is required but not installed."; exit 1; }
command -v node >/dev/null 2>&1 || { echo "Error: node is required but not installed."; exit 1; }
command -v npm >/dev/null 2>&1 || { echo "Error: npm is required but not installed."; exit 1; }

echo "Step 1: Installing JavaScript dependencies..."
cd "$SCRIPT_DIR"
npm install --silent
echo "Done."
echo ""

echo "Step 2: Running Rust benchmark..."
cd "$PROJECT_DIR"
cargo run --release --example benchmark 2>/dev/null
echo ""

echo "Step 3: Running JavaScript benchmark..."
cd "$SCRIPT_DIR"
node benchmark.js
echo ""

echo "Step 4: Comparing results..."
node compare.js
echo ""

echo "=============================================="
echo "Benchmarks complete!"
echo "=============================================="
echo ""
echo "Results:"
echo "  - Rust results: $SCRIPT_DIR/rust-results.json"
echo "  - JS results: $SCRIPT_DIR/js-results.json"
echo "  - Comparison: $SCRIPT_DIR/BENCHMARK_RESULTS.md"
