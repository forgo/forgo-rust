# YAML Library Tests

Comprehensive test suite for `forgo_lib_yaml` ensuring YAML 1.2.2 compliance.

## Quick Start

```bash
# Run all tests
cargo test

# Run specific test suites
cargo test --test api_tests           # Public API tests (161)
cargo test --test feature_tests       # Feature tests (38)
cargo test --test integration_tests   # Integration tests (16)
cargo test --test regression_tests    # Regression/fuzz tests (13)
cargo test --test unit_tests          # Unit tests (58)
cargo test --test yaml_spec_1_2_2     # YAML 1.2.2 spec tests (671)
cargo test --test yaml_test_suite_official  # Official test suite (351)
```

## Test Status

| Suite | Tests | Status |
|-------|-------|--------|
| API Tests | 161 | ✅ 161 passing |
| Feature Tests | 43 | ✅ 43 passing |
| Integration Tests | 16 | ✅ 16 passing |
| Unit Tests | 58 | ✅ 58 passing |
| Regression Tests | 13 | ⚠️ 12 passing, 1 failing |
| **YAML 1.2.2 Spec** | **671** | **✅ 671 passing (100%)** |
| **Official Suite** | **363** | **⚠️ 233 passing (64.2%)** |
| **Internal Total** | **962** | **961 ✅ / 1 ⚠️** |

**YAML 1.2.2 Compliance: 100%** (671/671 spec tests passing)
**Official Test Suite: 64.2%** (233/363 passing, 78 failing, 52 skipped)

> **Detailed Failures:** See [OFFICIAL_TEST_FAILURES.md](OFFICIAL_TEST_FAILURES.md) for complete analysis of all 78 failing tests with error messages and input YAML.

## Test Organization

```
tests/
├── api/              # Public API surface tests
├── features/         # Feature-based tests (comments, tags, etc.)
├── integration/      # End-to-end integration tests
├── regression/       # Bug prevention and fuzz tests
├── spec/             # YAML 1.2.2 specification tests
├── unit/             # Component unit tests (lexer, parser, emitter)
└── yaml_test_suite/  # Official YAML test suite (external)
```

### API Tests (`api/`)
Tests for the public API ensuring stability and usability.
- Builders and constructors
- Type guards and accessors
- Convenience methods

### Feature Tests (`features/`)
Organized by YAML feature, spanning multiple spec sections:
- Anchors & aliases
- Block scalars
- Comments
- Document streams
- Flow style
- Quoting
- Schemas
- Tags

### Integration Tests (`integration/`)
End-to-end validation of real-world usage:
- AST manipulation
- Editor operations
- Round-trip stability

### Regression Tests (`regression/`)
Prevent known bugs from recurring:
- Property-based tests
- Fuzz testing
- Historical bug reproductions

### Spec Tests (`spec/`)
Systematic YAML 1.2.2 compliance tracking:
- One file per spec chapter (ch_2 through ch_10)
- Direct traceability to spec sections
- Comprehensive coverage of all requirements

### Unit Tests (`unit/`)
Component-level testing:
- Lexer edge cases
- Parser specifics
- Emitter behavior

## Running Official YAML Test Suite

The official test suite validates against 363 community-maintained test cases.

```bash
# Run the official test suite (generates detailed failure report)
cargo test --test yaml_test_suite_official -- --nocapture

# Or use Docker (if local clone not available)
./tests/run-official-tests-docker.sh
```

**Output Files:**
- `OFFICIAL_TEST_FAILURES.md` - Detailed report with error messages and YAML input for each failure
- `failed_test_ids.txt` - List of failing test IDs for quick reference

## Test Output Format

All test failures include:
- Test name and location
- Input YAML
- Expected vs actual output
- Spec section reference (for spec tests)

Example output:
```
---- spec::ch_7_flow_styles::ch_7_4_1_05_flow_sequence_empty stdout ----
Fixture failed: 7.4.1 test 5 - Empty flow sequence
Input YAML:
"[]"
Error: <description>
Location: tests/spec/ch_7_flow_styles.rs:142
```

## Adding Tests

### Spec Test
```rust
#[test]
fn ch_X_Y_ZZ_description() {
    Fixture::new(
        "X.Y",       // Spec section
        ZZ,           // Test number
        "Description",
        "yaml: input\n",
    )
    .run();
}
```

### Feature Test
```rust
#[test]
fn descriptive_test_name() {
    let doc = parse("yaml: here").unwrap();
    assert_eq!(/* validation */);
}
```

## Known Issues

### Failing Tests
- `regression::proptest_yaml::prop_roundtrip_parse_emit_parse` - Complex key roundtrip edge case

## Documentation Cleanup

This directory previously contained extensive documentation (AUDIT.md, COMPLIANCE.md, DOCKER_*.md, etc.).
These have been archived as they became outdated. This README is now the single source of truth for testing.

If you need historical context, check git history for:
- Test failure analysis
- Compliance tracking
- Docker setup guides
