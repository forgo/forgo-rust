# Test Organization

This document describes the reorganized test structure for `forgo_lib_yaml`.

## Overview

Tests have been reorganized from a flat structure into a hierarchical, purpose-driven organization that makes it easier to:
- Find and navigate related tests
- Understand what components are being tested
- Add new tests in the appropriate location
- Run specific test categories independently

## Directory Structure

```
tests/
├── unit/                          # Unit tests for core components
│   ├── lexer/                     # Lexer tests (8 tests)
│   │   ├── lexer_edges.rs         # UTF-8 BOM, tabs, newlines
│   │   └── quote_handling.rs      # Quote errors, null chars
│   ├── parser/                    # Parser tests (22 tests)
│   │   ├── parser_basics.rs       # Core parsing functionality
│   │   └── document_markers.rs    # Document marker edge cases
│   └── emitter/                   # Emitter tests (28 tests)
│       ├── emitter.rs             # Core emission logic
│       ├── emitter_alias_anchor.rs
│       ├── emitter_block_scalars.rs
│       ├── emitter_comments.rs
│       ├── emitter_layout.rs
│       ├── emitter_quoting.rs
│       ├── emitter_scalars.rs
│       └── emitter_top_level.rs
│
├── features/                      # Feature-specific tests
│   ├── anchors/                   # Anchor & alias tests (4 tests)
│   │   ├── anchors.rs
│   │   ├── alias_anchor_weird.rs
│   │   └── merge_keys.rs
│   ├── block_scalars/             # Block scalar tests (16 tests)
│   │   ├── block_scalars.rs
│   │   ├── block_scalars_strict.rs
│   │   ├── block_style.rs
│   │   └── blocks.rs
│   ├── comments/                  # Comment tests (6 tests)
│   │   └── comments.rs
│   ├── document_streams/          # Multi-document tests (5 tests)
│   │   └── document_streams.rs
│   ├── flow_style/                # Flow syntax tests (2 tests)
│   │   ├── flow.rs
│   │   └── flow_seq_preserve_quotes.rs
│   ├── quoting/                   # Quote handling tests (1 test)
│   │   └── single_double_quotes.rs
│   ├── tags/                      # Tag tests (1 test)
│   │   └── tags.rs
│   └── schemas/                   # Schema/typing tests (3 tests)
│       ├── typing.rs
│       └── schema_core.rs
│
├── integration/                   # Integration tests
│   ├── editor/                    # Editor tests (6 tests)
│   │   ├── editor.rs
│   │   └── editor_strings.rs
│   └── ast/                       # AST tests (10 tests)
│       └── ast.rs
│
├── regression/                    # Regression & property tests
│   ├── fuzzish.rs                 # Fuzz testing (2 tests)
│   └── proptest_yaml.rs           # Property-based tests (11 tests)
│
├── spec/                          # YAML 1.2.2 spec compliance tests
│   └── [ch_2_overview.rs, ...]
│
├── api/                           # API surface tests
│   └── [various api tests]        # 161 tests
│
├── common/                        # Shared test utilities
│   └── mod.rs
│
├── fixtures/                      # Test data files
│
├── unit_tests.rs                  # Loader for unit tests
├── feature_tests.rs               # Loader for feature tests
├── integration_tests.rs           # Loader for integration tests
├── regression_tests.rs            # Loader for regression tests
├── api_tests.rs                   # Loader for API tests
└── yaml_spec_1_2_2.rs            # Loader for spec tests
```

## Test Categories

### Unit Tests (58 tests)
Tests for individual components in isolation:
- **Lexer** (8 tests): Tokenization, encoding, error handling
- **Parser** (22 tests): AST construction, document structure
- **Emitter** (28 tests): YAML output generation, formatting

### Feature Tests (38 tests)
Tests for specific YAML features:
- **Anchors & Aliases** (4 tests): Reference mechanism
- **Block Scalars** (16 tests): Literal/folded blocks, chomping, indentation
- **Comments** (6 tests): Comment preservation
- **Document Streams** (5 tests): Multi-document handling
- **Flow Style** (2 tests): Inline sequences/mappings
- **Quoting** (1 test): Quote escaping
- **Tags** (1 test): Type annotations
- **Schemas** (3 tests): Type resolution

### Integration Tests (16 tests)
End-to-end tests combining multiple components:
- **Editor** (6 tests): Document manipulation APIs
- **AST** (10 tests): Complete document handling

### Regression Tests (13 tests)
Tests preventing known issues from reoccurring:
- **Fuzz Testing** (2 tests): Randomized input
- **Property Testing** (11 tests): QuickCheck-style tests

### Spec Tests
YAML 1.2.2 specification compliance tests (organized by spec chapters)

### API Tests (161 tests)
Tests for the public API surface (in `api/` subdirectory)

## Running Tests

### Run all tests:
```bash
cargo test -p forgo_lib_yaml
```

### Run specific test categories:
```bash
# Unit tests only
cargo test -p forgo_lib_yaml --test unit_tests

# Feature tests only
cargo test -p forgo_lib_yaml --test feature_tests

# Integration tests only
cargo test -p forgo_lib_yaml --test integration_tests

# Regression tests only
cargo test -p forgo_lib_yaml --test regression_tests

# API tests only
cargo test -p forgo_lib_yaml --test api_tests

# Spec compliance tests only
cargo test -p forgo_lib_yaml --test yaml_spec_1_2_2
```

### Run tests for specific components:
```bash
# Lexer tests only
cargo test -p forgo_lib_yaml --test unit_tests lexer

# Emitter tests only
cargo test -p forgo_lib_yaml --test unit_tests emitter

# Block scalar tests only
cargo test -p forgo_lib_yaml --test feature_tests block_scalars
```

## Test Statistics

- **Total test files**: 35 (excluding loaders and infrastructure)
- **Total test cases**: ~300+ across all categories
- **Unit tests**: 58
- **Feature tests**: 38
- **Integration tests**: 16
- **Regression tests**: 13
- **API tests**: 161
- **Spec tests**: Variable (based on YAML 1.2.2 spec)

## Known Failing Tests

The following tests are currently failing and need investigation:

### Block Scalar Issues (8 failures)
1. `block_scalars_strict::chomping_plus_keeps_all_trailing_newlines`
2. `block_scalars_strict::content_lines_must_meet_required_indent_when_indicator_present`
3. `block_scalars_strict::folded_preserves_paragraphs_and_more_indented_runs`
4. `block_scalars_strict::folded_single_newline_becomes_single_space_no_double_spaces`
5. `block_scalars_strict::indicator_order_is_free_mixture`
6. `block_scalars_strict::literal_with_indent_indicator_removes_exact_k_spaces`
7. `blocks::block_body_has_no_trailing_spaces_before_newline`
8. `blocks::folded_block_gt_folds_single_newlines`

### Other Feature Issues (3 failures)
9. `comments::preserves_leading_and_trailing_comments`
10. `schema_core::core_case_sensitive_bools_null`
11. `tags::simple_short_and_verbatim_tags_parse_and_emit`

These failures existed before the reorganization and represent known limitations or areas needing implementation work.

## Migration Notes

All test files were moved using `git mv` to preserve history. The reorganization:
- ✅ Preserves all existing tests (zero tests lost)
- ✅ Maintains test functionality (passing tests still pass)
- ✅ Improves discoverability
- ✅ Enables selective test execution
- ✅ Provides clear structure for future tests

## Benefits

1. **Better Organization**: Tests grouped by purpose/component
2. **Easier Navigation**: Find related tests quickly
3. **Clearer Intent**: Directory structure shows what's being tested
4. **Scalability**: Easy to add new tests in the right place
5. **Maintenance**: Related tests are co-located
6. **CI Optimization**: Can run test categories independently
7. **Documentation**: Structure itself documents the test coverage
