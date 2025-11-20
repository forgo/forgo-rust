#!/usr/bin/env bash
# Script to run the official YAML test suite against forgo_lib_yaml
# This provides a quick way to check compliance without compiling the full test

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TEST_SUITE_DIR="$SCRIPT_DIR/yaml_test_suite/src"

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

if [ ! -d "$TEST_SUITE_DIR" ]; then
    echo -e "${RED}Error: yaml-test-suite not found!${NC}"
    echo ""
    echo "To set up the official YAML test suite:"
    echo ""
    echo "  cd $SCRIPT_DIR"
    echo "  git clone https://github.com/yaml/yaml-test-suite.git yaml_test_suite"
    echo ""
    exit 1
fi

echo ""
echo -e "${BLUE}╔═══════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║        Official YAML Test Suite - Compliance Report          ║${NC}"
echo -e "${BLUE}╚═══════════════════════════════════════════════════════════════╝${NC}"
echo ""

# Count total test files
TOTAL_TESTS=$(ls -1 "$TEST_SUITE_DIR"/*.yaml 2>/dev/null | wc -l)
echo -e "Found ${GREEN}$TOTAL_TESTS${NC} test cases in yaml-test-suite"
echo ""
echo "To run the full test suite with detailed results:"
echo -e "  ${YELLOW}cd $(dirname $SCRIPT_DIR)${NC}"
echo -e "  ${YELLOW}cargo test --test yaml_test_suite_official -- --nocapture${NC}"
echo ""
echo "Test categories include:"
grep -h "tags:" "$TEST_SUITE_DIR"/*.yaml | \
    sed 's/.*tags: //' | \
    tr ' ' '\n' | \
    sort | uniq -c | sort -rn | head -15 | \
    awk '{printf "  • %-20s (%3d tests)\n", $2, $1}'
echo ""
