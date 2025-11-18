#!/bin/bash
# scripts/check_compliance.sh
#
# Runs the YAML 1.2.2 compliance test suite and checks for regressions.
#
# Usage:
#   ./scripts/check_compliance.sh           # Run all tests
#   ./scripts/check_compliance.sh --report  # Generate detailed report
#   ./scripts/check_compliance.sh --ci      # CI mode (exits 1 on regression)

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CRATE_DIR="$(dirname "$SCRIPT_DIR")"
BASELINE_FILE="$CRATE_DIR/test_baseline.json"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Baseline (from test_baseline.json)
BASELINE_PASSED=607
BASELINE_TOTAL=671

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  YAML 1.2.2 Compliance Test Suite"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Navigate to crate directory
cd "$CRATE_DIR"

# Run spec tests
echo "🧪 Running YAML 1.2.2 specification tests..."
echo ""

SPEC_OUTPUT=$(cargo test --test yaml_spec_1_2_2 2>&1 || true)

# Extract results
PASSED=$(echo "$SPEC_OUTPUT" | grep "test result:" | sed -E 's/.*\. ([0-9]+) passed.*/\1/' | head -1)
FAILED=$(echo "$SPEC_OUTPUT" | grep "test result:" | sed -E 's/.*passed; ([0-9]+) failed.*/\1/' | head -1)
IGNORED=$(echo "$SPEC_OUTPUT" | grep "test result:" | sed -E 's/.*failed; ([0-9]+) ignored.*/\1/' | head -1)

# Handle cases where fields might be missing
if [ -z "$PASSED" ]; then PASSED=0; fi
if [ -z "$FAILED" ]; then FAILED=0; fi
if [ -z "$IGNORED" ]; then IGNORED=0; fi

TOTAL=$((PASSED + FAILED + IGNORED))
PASS_RATE=$(awk "BEGIN {printf \"%.1f\", ($PASSED / $BASELINE_TOTAL) * 100}")

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  RESULTS"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
printf "  Total:   %3d tests\n" "$TOTAL"
printf "  ${GREEN}Passed:  %3d tests${NC}\n" "$PASSED"
if [ "$FAILED" -gt 0 ]; then
    printf "  ${RED}Failed:  %3d tests${NC}\n" "$FAILED"
else
    printf "  Failed:  %3d tests\n" "$FAILED"
fi
if [ "$IGNORED" -gt 0 ]; then
    printf "  ${YELLOW}Ignored: %3d tests${NC}\n" "$IGNORED"
fi
echo ""
printf "  Compliance: ${BLUE}%s%%${NC} (%d/%d)\n" "$PASS_RATE" "$PASSED" "$BASELINE_TOTAL"
echo ""

# Check for regression
REGRESSION=0
if [ "$PASSED" -lt "$BASELINE_PASSED" ]; then
    REGRESSION=1
    DIFF=$((BASELINE_PASSED - PASSED))
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    printf "  ${RED}⚠️  REGRESSION DETECTED${NC}\n"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo ""
    printf "  Expected: ${GREEN}≥ %d${NC} passing\n" "$BASELINE_PASSED"
    printf "  Actual:   ${RED}%d${NC} passing\n" "$PASSED"
    printf "  Lost:     ${RED}%d${NC} tests\n" "$DIFF"
    echo ""
elif [ "$PASSED" -gt "$BASELINE_PASSED" ]; then
    GAIN=$((PASSED - BASELINE_PASSED))
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    printf "  ${GREEN}🎉 PROGRESS!${NC}\n"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo ""
    printf "  Baseline: ${YELLOW}%d${NC} passing\n" "$BASELINE_PASSED"
    printf "  Current:  ${GREEN}%d${NC} passing\n" "$PASSED"
    printf "  Gained:   ${GREEN}+%d${NC} tests\n" "$GAIN"
    echo ""
    echo "  🎯 Remember to update test_baseline.json!"
    echo ""
else
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    printf "  ${GREEN}✅ No regression${NC} (baseline maintained)\n"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo ""
fi

# Run implementation tests if requested
if [ "$1" = "--report" ] || [ "$1" = "--all" ]; then
    echo ""
    echo "🧪 Running implementation tests..."
    echo ""

    # List of implementation test files
    IMPL_TESTS=(
        "alias_anchor_weird"
        "anchors"
        "ast"
        "block_scalars"
        "block_scalars_strict"
        "flow"
        "merge_key_roundtrip"
        "schema_core"
        "single_double_quotes"
        "tags"
    )

    IMPL_PASSED=0
    IMPL_FAILED=0

    for test in "${IMPL_TESTS[@]}"; do
        if cargo test --test "$test" --quiet 2>&1 | grep -q "test result: ok"; then
            IMPL_PASSED=$((IMPL_PASSED + 1))
            printf "  ${GREEN}✓${NC} %s\n" "$test"
        else
            IMPL_FAILED=$((IMPL_FAILED + 1))
            printf "  ${RED}✗${NC} %s\n" "$test"
        fi
    done

    echo ""
    echo "  Implementation: $IMPL_PASSED/${#IMPL_TESTS[@]} passing"
    echo ""
fi

# Show chapter breakdown if requested
if [ "$1" = "--report" ]; then
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "  CHAPTER BREAKDOWN"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo ""

    for ch in {2..10}; do
        CH_OUTPUT=$(cargo test --test yaml_spec_1_2_2 ch_${ch}_ 2>&1 || true)
        CH_PASSED=$(echo "$CH_OUTPUT" | grep "test result:" | sed -E 's/.*([0-9]+) passed.*/\1/' | head -1)
        CH_FAILED=$(echo "$CH_OUTPUT" | grep "test result:" | sed -E 's/.*; ([0-9]+) failed.*/\1/' | head -1)

        if [ -z "$CH_PASSED" ]; then CH_PASSED=0; fi
        if [ -z "$CH_FAILED" ]; then CH_FAILED=0; fi

        CH_TOTAL=$((CH_PASSED + CH_FAILED))

        if [ "$CH_TOTAL" -gt 0 ]; then
            if [ "$CH_FAILED" -eq 0 ]; then
                printf "  Ch %2d: ${GREEN}✅ %3d/%3d${NC} (100%%)\n" "$ch" "$CH_PASSED" "$CH_TOTAL"
            elif [ "$CH_PASSED" -gt 0 ]; then
                CH_RATE=$(awk "BEGIN {printf \"%.0f\", ($CH_PASSED / $CH_TOTAL) * 100}")
                printf "  Ch %2d: ${YELLOW}⚡ %3d/%3d${NC} (%s%%)\n" "$ch" "$CH_PASSED" "$CH_TOTAL" "$CH_RATE"
            else
                printf "  Ch %2d: ${RED}⚠️  %3d/%3d${NC} (failing)\n" "$ch" "$CH_PASSED" "$CH_TOTAL"
            fi
        fi
    done
    echo ""
fi

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Exit with error code if regression detected in CI mode
if [ "$1" = "--ci" ] && [ "$REGRESSION" -eq 1 ]; then
    exit 1
fi

exit 0
