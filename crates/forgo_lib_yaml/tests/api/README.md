# API Tests

This directory contains comprehensive tests for the `forgo_lib_yaml` public API surface.

## Organization

Tests are organized by API category to mirror the structure in `API_PLAN.md`:

- `mod.rs` - Common test utilities and helpers
- `layer1_simple.rs` - Layer 1 simple API: `parse()`, `stringify()`, `parse_all()`
- `doc_construction.rs` - Document construction & I/O (from_str, from_slice, from_reader, read_file, etc.)
- `doc_access.rs` - Direct AST access (root, root_mut, yaml_version, etc.)
- `visitor_pattern.rs` - Path-based traversal (visit_sequences_mut, visit_maps_mut, visit_values_mut)
- `mutation_helpers.rs` - High-level mutation (set, delete, push_at, get_at, merge_at, etc.)
- `node_constructors.rs` - Node construction helpers (Node::string(), Node::map(), etc.)
- `node_guards.rs` - Node type guards (is_map, is_seq, as_map, etc.)
- `node_map_ops.rs` - Node map operations (get, insert, remove, keys, values, etc.)
- `node_seq_ops.rs` - Node sequence operations (push, pop, insert, remove, etc.)
- `elem_builders.rs` - Elem construction and fluent builders
- `scalar_api.rs` - Scalar type system and conversions
- `error_types.rs` - Error type handling and messages
- `editor_utils.rs` - Editor utilities (normalize_string_list, edit_file_in_place)
- `debug_utils.rs` - Debug utilities (debug_token_dump, debug_ast_dump, etc.)
- `into_path.rs` - IntoPath trait and path handling

## Test Style

Each test follows this pattern:

```rust
#[test]
fn test_specific_behavior() {
    // Arrange: Set up test data
    let input = "...";

    // Act: Perform the operation
    let result = operation(input);

    // Assert: Verify the outcome
    assert_eq!(result, expected);
}
```

Tests are:
- **Focused**: One behavior per test
- **Named clearly**: `test_verb_noun_condition`
- **Well-documented**: Comments explain "why" not "what"
- **Independent**: No shared mutable state between tests
- **Comprehensive**: Cover happy path, edge cases, and error cases
