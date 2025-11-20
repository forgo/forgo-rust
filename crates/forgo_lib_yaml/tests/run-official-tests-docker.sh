#!/usr/bin/env bash
# Run the official YAML test suite in Docker
# This provides a reproducible, platform-independent test environment

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo ""
echo -e "${BLUE}╔═══════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║   Official YAML Test Suite - Docker Runner                   ║${NC}"
echo -e "${BLUE}╚═══════════════════════════════════════════════════════════════╝${NC}"
echo ""

# Check if Docker is available
if ! command -v docker &> /dev/null; then
    echo -e "${YELLOW}Error: Docker is not installed or not in PATH${NC}"
    echo ""
    echo "Please install Docker Desktop from:"
    echo "  https://www.docker.com/products/docker-desktop"
    echo ""
    exit 1
fi

# Parse command line arguments
REBUILD=false
SHELL_MODE=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --rebuild)
            REBUILD=true
            shift
            ;;
        --shell)
            SHELL_MODE=true
            shift
            ;;
        --help|-h)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --rebuild    Rebuild the Docker image from scratch"
            echo "  --shell      Open a shell in the container instead of running tests"
            echo "  --help       Show this help message"
            echo ""
            echo "Examples:"
            echo "  $0                    # Run tests with cached image"
            echo "  $0 --rebuild          # Rebuild image and run tests"
            echo "  $0 --shell            # Open interactive shell"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            echo "Run '$0 --help' for usage information"
            exit 1
            ;;
    esac
done

# Navigate to repo root for correct build context
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

# Build or rebuild the image
if [ "$REBUILD" = true ]; then
    echo -e "${BLUE}Building Docker image (this may take a few minutes)...${NC}"
    cd "$REPO_ROOT"
    docker build -f crates/forgo_lib_yaml/tests/Dockerfile.yaml-test-suite -t forgo-yaml-test-suite:latest .
    echo -e "${GREEN}✓ Image built successfully${NC}"
    echo ""
elif ! docker image inspect forgo-yaml-test-suite:latest &> /dev/null; then
    echo -e "${BLUE}Building Docker image for the first time...${NC}"
    echo -e "${YELLOW}(Subsequent runs will be faster using the cached image)${NC}"
    echo ""
    cd "$REPO_ROOT"
    docker build -f crates/forgo_lib_yaml/tests/Dockerfile.yaml-test-suite -t forgo-yaml-test-suite:latest .
    echo -e "${GREEN}✓ Image built successfully${NC}"
    echo ""
fi

# Run the container
if [ "$SHELL_MODE" = true ]; then
    echo -e "${BLUE}Opening interactive shell in test container...${NC}"
    echo -e "${YELLOW}Test suite is available at: /workspace/crates/forgo_lib_yaml/tests/yaml_test_suite${NC}"
    echo -e "${YELLOW}Run tests with: cargo test --test yaml_test_suite_official -- --nocapture${NC}"
    echo ""
    docker run --rm -it \
        -v "$SCRIPT_DIR/../src:/workspace/crates/forgo_lib_yaml/src:ro" \
        -v "$SCRIPT_DIR/common:/workspace/crates/forgo_lib_yaml/tests/common:ro" \
        -v "$SCRIPT_DIR/yaml_test_suite_official.rs:/workspace/crates/forgo_lib_yaml/tests/yaml_test_suite_official.rs:ro" \
        forgo-yaml-test-suite:latest \
        /bin/bash
else
    echo -e "${BLUE}Running official YAML test suite in Docker...${NC}"
    echo ""

    # Run tests and mount the tests directory to capture output files
    docker run --rm \
        -v "$SCRIPT_DIR/../src:/workspace/crates/forgo_lib_yaml/src:ro" \
        -v "$SCRIPT_DIR/common:/workspace/crates/forgo_lib_yaml/tests/common:ro" \
        -v "$SCRIPT_DIR/yaml_test_suite_official.rs:/workspace/crates/forgo_lib_yaml/tests/yaml_test_suite_official.rs:ro" \
        -v "$SCRIPT_DIR:/workspace/crates/forgo_lib_yaml/tests/output" \
        forgo-yaml-test-suite:latest \
        bash -c "cargo test --package forgo_lib_yaml --test yaml_test_suite_official -- --nocapture && cp -f /workspace/crates/forgo_lib_yaml/tests/failed_test_ids.txt /workspace/crates/forgo_lib_yaml/tests/output/failed_test_ids.txt 2>/dev/null || true"
fi

echo ""
echo -e "${GREEN}✓ Complete${NC}"
echo ""
