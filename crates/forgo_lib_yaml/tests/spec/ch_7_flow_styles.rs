// crates/forgo_lib_yaml/tests/spec/ch_7_flow_styles.rs
//! YAML 1.2.2 Chapter 7: Flow Style Productions
//!
//! Reference: https://yaml.org/spec/1.2.2/#chapter-7-flow-style-productions
//!
//! Flow styles use explicit indicators rather than indentation to denote structure.

#[path = "../common/mod.rs"]
mod common;

use common::Fixture;
use forgo_lib_yaml::Doc;

// ============================================================================
// 7.1 Alias Nodes
// ============================================================================

#[test]
fn ch_7_1_01_simple_alias() {
    Fixture::new(
        "7.1",
        1,
        "Simple alias node",
        "anchor: &ref value\nalias: *ref\n",
    )
    .run();
}

#[test]
fn ch_7_1_02_alias_in_flow_sequence() {
    Fixture::new(
        "7.1",
        2,
        "Alias in flow sequence",
        "item: &x hello\nlist: [*x, *x]\n",
    )
    .run();
}

#[test]
fn ch_7_1_03_alias_in_flow_mapping() {
    Fixture::new(
        "7.1",
        3,
        "Alias in flow mapping",
        "item: &x value\nmap: {a: *x, b: *x}\n",
    )
    .run();
}

#[test]
fn ch_7_1_04_alias_spacing_in_flow_collection() {
    // Production [110] c-ns-alias-node - alias immediately after flow indicator
    // Test that [*anchor] works (no space required after '[')
    Fixture::new(
        "7.1",
        4,
        "Alias spacing in flow collection",
        "item: &x value\nseq: [*x, other]\nmap: {key: *x}\n",
    )
    .run();
}

// ============================================================================
// 7.2 Empty Nodes
// ============================================================================

#[test]
fn ch_7_2_01_empty_scalar() {
    Fixture::new(
        "7.2",
        1,
        "Empty scalar node",
        "key: \n",
    )
    .run();
}

#[test]
fn ch_7_2_02_empty_in_flow_sequence() {
    Fixture::new(
        "7.2",
        2,
        "Empty nodes in flow sequence",
        "items: [\"\", a, \"\"]\n",
    )
    .run();
}

#[test]
fn ch_7_2_03_empty_value_in_mapping() {
    Fixture::new(
        "7.2",
        3,
        "Empty value in mapping",
        "key:\nother: value\n",
    )
    .run();
}

// ============================================================================
// 7.3 Flow Scalar Styles
// ============================================================================

// 7.3.1 Double-Quoted Style
#[test]
fn ch_7_3_1_01_double_quoted_basic() {
    Fixture::new(
        "7.3.1",
        1,
        "Basic double-quoted scalar",
        "key: \"value\"\n",
    )
    .run();
}

#[test]
fn ch_7_3_1_02_double_quoted_escaped_quote() {
    Fixture::new(
        "7.3.1",
        2,
        "Double-quoted with escaped quote",
        "key: \"say \\\"hello\\\"\"\n",
    )
    .run();
}

#[test]
fn ch_7_3_1_03_double_quoted_escaped_backslash() {
    Fixture::new(
        "7.3.1",
        3,
        "Double-quoted with escaped backslash",
        "key: \"path\\\\to\\\\file\"\n",
    )
    .run();
}

#[test]
fn ch_7_3_1_04_double_quoted_newline() {
    Fixture::new(
        "7.3.1",
        4,
        "Double-quoted with escaped newline",
        "key: \"line1\\nline2\"\n",
    )
    .run();
}

#[test]
fn ch_7_3_1_05_double_quoted_tab() {
    Fixture::new(
        "7.3.1",
        5,
        "Double-quoted with escaped tab",
        "key: \"tab\\there\"\n",
    )
    .run();
}

#[test]
fn ch_7_3_1_06_double_quoted_carriage_return() {
    Fixture::new(
        "7.3.1",
        6,
        "Double-quoted with carriage return",
        "key: \"line1\\rline2\"\n",
    )
    .run();
}

// 7.3.2 Single-Quoted Style
#[test]
fn ch_7_3_2_01_single_quoted_basic() {
    Fixture::new(
        "7.3.2",
        1,
        "Basic single-quoted scalar",
        "key: 'value'\n",
    )
    .run();
}

#[test]
fn ch_7_3_2_02_single_quoted_with_spaces() {
    Fixture::new(
        "7.3.2",
        2,
        "Single-quoted preserves leading/trailing spaces",
        "key: '  spaced  '\n",
    )
    .run();
}

#[test]
fn ch_7_3_2_03_single_quoted_escaped_quote() {
    Fixture::new(
        "7.3.2",
        3,
        "Single-quoted with escaped quote (doubled)",
        "key: 'it''s'\n",
    )
    .run();
}

#[test]
fn ch_7_3_2_04_single_quoted_special_chars() {
    Fixture::new(
        "7.3.2",
        4,
        "Single-quoted with special characters",
        "key: 'a: b'\n",
    )
    .run();
}

// 7.3.3 Plain Style
#[test]
fn ch_7_3_3_01_plain_string() {
    Fixture::new(
        "7.3.3",
        1,
        "Plain scalar string",
        "key: value\n",
    )
    .run();
}

#[test]
fn ch_7_3_3_02_plain_number() {
    Fixture::new(
        "7.3.3",
        2,
        "Plain scalar number",
        "count: 42\n",
    )
    .run();
}

#[test]
fn ch_7_3_3_03_plain_boolean() {
    Fixture::new(
        "7.3.3",
        3,
        "Plain scalar boolean",
        "enabled: true\n",
    )
    .run();
}

#[test]
fn ch_7_3_3_04_plain_null() {
    Fixture::new(
        "7.3.3",
        4,
        "Plain scalar null",
        "value: null\n",
    )
    .run();
}

#[test]
fn ch_7_3_3_05_plain_tilde_null() {
    Fixture::new(
        "7.3.3",
        5,
        "Plain scalar tilde as null",
        "value: ~\n",
    )
    .run();
}

#[test]
fn ch_7_3_3_06_plain_with_spaces() {
    Fixture::new(
        "7.3.3",
        6,
        "Plain scalar with spaces",
        "message: hello world\n",
    )
    .run();
}

#[test]
fn ch_7_3_3_07_plain_url() {
    Fixture::new(
        "7.3.3",
        7,
        "Plain scalar URL",
        "url: https://example.com\n",
    )
    .run();
}

// ============================================================================
// 7.4 Flow Collection Styles
// ============================================================================

// 7.4.1 Flow Sequences
#[test]
fn ch_7_4_1_01_flow_sequence_basic() {
    Fixture::new(
        "7.4.1",
        1,
        "Basic flow sequence",
        "items: [a, b, c]\n",
    )
    .run();
}

#[test]
fn ch_7_4_1_02_flow_sequence_numbers() {
    Fixture::new(
        "7.4.1",
        2,
        "Flow sequence of numbers",
        "nums: [1, 2, 3]\n",
    )
    .run();
}

#[test]
fn ch_7_4_1_03_flow_sequence_quoted() {
    Fixture::new(
        "7.4.1",
        3,
        "Flow sequence with quoted strings",
        "items: [\"first\", \"second\", \"third\"]\n",
    )
    .run();
}

#[test]
fn ch_7_4_1_04_flow_sequence_nested() {
    Fixture::new(
        "7.4.1",
        4,
        "Nested flow sequences",
        "matrix: [[1, 2], [3, 4]]\n",
    )
    .run();
}

#[test]
fn ch_7_4_1_05_flow_sequence_empty() {
    Fixture::new(
        "7.4.1",
        5,
        "Empty flow sequence",
        "empty: []\n",
    )
    .run();
}

#[test]
fn ch_7_4_1_06_flow_sequence_single_item() {
    Fixture::new(
        "7.4.1",
        6,
        "Flow sequence with single item",
        "items: [only]\n",
    )
    .run();
}

// 7.4.2 Flow Mappings
#[test]
fn ch_7_4_2_01_flow_mapping_basic() {
    Fixture::new(
        "7.4.2",
        1,
        "Basic flow mapping",
        "point: {x: 1, y: 2}\n",
    )
    .run();
}

#[test]
fn ch_7_4_2_02_flow_mapping_strings() {
    Fixture::new(
        "7.4.2",
        2,
        "Flow mapping with string values",
        "person: {name: Alice, role: admin}\n",
    )
    .run();
}

#[test]
fn ch_7_4_2_03_flow_mapping_nested() {
    Fixture::new(
        "7.4.2",
        3,
        "Nested flow mappings",
        "config: {db: {host: localhost, port: 5432}}\n",
    )
    .run();
}

#[test]
fn ch_7_4_2_04_flow_mapping_empty() {
    Fixture::new(
        "7.4.2",
        4,
        "Empty flow mapping",
        "empty: {}\n",
    )
    .run();
}

#[test]
fn ch_7_4_2_05_flow_mapping_quoted_keys() {
    Fixture::new(
        "7.4.2",
        5,
        "Flow mapping with quoted keys",
        "map: {\"key with spaces\": value}\n",
    )
    .run();
}

#[test]
fn ch_7_4_2_06_flow_mapping_empty_key() {
    // Section 7.4.2 - empty key in flow mapping (implicit null key)
    // {: value} should be valid with empty/null key
    Fixture::new(
        "7.4.2",
        6,
        "Flow mapping with empty key",
        "map: {: value, key2: value2}\n",
    )
    .run();
}

// 7.5 Flow Nodes
#[test]
fn ch_7_5_01_flow_node_with_anchor() {
    Fixture::new(
        "7.5",
        1,
        "Flow node with anchor",
        "data: &anchor {x: 1, y: 2}\nref: *anchor\n",
    )
    .run();
}

#[test]
fn ch_7_5_02_flow_node_with_tag() {
    Fixture::new(
        "7.5",
        2,
        "Flow node with tag",
        "value: !!str 123\n",
    )
    .run();
}

#[test]
fn ch_7_5_03_flow_sequence_with_tags() {
    Fixture::new(
        "7.5",
        3,
        "Flow sequence with tagged items",
        "items: [!!str 1, !!int 2]\n",
    )
    .run();
}

#[test]
fn ch_7_5_04_mixed_flow_and_block() {
    Fixture::new(
        "7.5",
        4,
        "Mixed flow and block styles",
        "outer:\n  inner: [a, b, c]\n  other: value\n",
    )
    .run();
}

// ============================================================================
// Flow Style Edge Cases
// ============================================================================

#[test]
fn ch_7_6_01_flow_sequence_trailing_comma() {
    // Note: trailing commas may or may not be supported
    Fixture::new(
        "7.6",
        1,
        "Flow sequence with spaces",
        "items: [ a , b , c ]\n",
    )
    .run();
}

#[test]
fn ch_7_6_02_flow_mapping_in_block_sequence() {
    Fixture::new(
        "7.6",
        2,
        "Flow mapping inside block sequence",
        "- {name: Alice, age: 30}\n- {name: Bob, age: 25}\n",
    )
    .run();
}

#[test]
fn ch_7_6_03_flow_sequence_in_block_mapping() {
    Fixture::new(
        "7.6",
        3,
        "Flow sequence inside block mapping",
        "colors: [red, green, blue]\nsizes: [S, M, L]\n",
    )
    .run();
}

#[test]
fn ch_7_6_04_complex_flow_structure() {
    Fixture::new(
        "7.6",
        4,
        "Complex nested flow structure",
        "data: [{id: 1, tags: [a, b]}, {id: 2, tags: [c, d]}]\n",
    )
    .run();
}

// =============================================================================
// 7.7 Flow Collection Errors and Advanced Cases
// =============================================================================

// -----------------------------------------------------------------------------
// 7.7.1 Flow Collection Error Conditions
// -----------------------------------------------------------------------------

#[test]
fn ch_7_7_1_01_missing_comma_in_sequence() {
    let input = "[a b c]\n";
    let result = Doc::from_str(input);
    assert!(
        result.is_err(),
        "Flow sequence should reject missing comma between items"
    );
}

#[test]
fn ch_7_7_1_02_trailing_comma_in_sequence() {
    let input = "[a, b, c,]\n";
    let result = Doc::from_str(input);
    assert!(
        result.is_err(),
        "Flow sequence should reject trailing comma"
    );
}

#[test]
fn ch_7_7_1_03_missing_comma_in_mapping() {
    let input = "{a: 1 b: 2}\n";
    let result = Doc::from_str(input);
    assert!(
        result.is_err(),
        "Flow mapping should reject missing comma between pairs"
    );
}

#[test]
fn ch_7_7_1_04_trailing_comma_in_mapping() {
    let input = "{a: 1, b: 2,}\n";
    let result = Doc::from_str(input);
    assert!(
        result.is_err(),
        "Flow mapping should reject trailing comma"
    );
}

#[test]
fn ch_7_7_1_05_missing_colon_in_mapping() {
    let input = "{a 1, b: 2}\n";
    let result = Doc::from_str(input);
    assert!(
        result.is_err(),
        "Flow mapping should reject missing colon in key-value pair"
    );
}

#[test]
fn ch_7_7_1_06_missing_closing_bracket() {
    let input = "[a, b, c\n";
    let result = Doc::from_str(input);
    assert!(
        result.is_err(),
        "Flow sequence should reject missing closing bracket"
    );
}

#[test]
fn ch_7_7_1_07_missing_closing_brace() {
    let input = "{a: 1, b: 2\n";
    let result = Doc::from_str(input);
    assert!(
        result.is_err(),
        "Flow mapping should reject missing closing brace"
    );
}

#[test]
fn ch_7_7_1_08_extra_closing_bracket() {
    let input = "[a, b, c]]\n";
    let result = Doc::from_str(input);
    assert!(
        result.is_err(),
        "Flow sequence should reject extra closing bracket"
    );
}

// -----------------------------------------------------------------------------
// 7.7.2 Multiline Flow Collections
// -----------------------------------------------------------------------------

#[test]
fn ch_7_7_2_01_multiline_flow_sequence() {
    Fixture::new(
        "7.7.2",
        1,
        "Multiline flow sequence",
        "items: [\n  item1,\n  item2,\n  item3\n]\n",
    )
    .run();
}

#[test]
fn ch_7_7_2_02_multiline_flow_mapping() {
    Fixture::new(
        "7.7.2",
        2,
        "Multiline flow mapping",
        "point: {\n  x: 1,\n  y: 2,\n  z: 3\n}\n",
    )
    .run();
}

#[test]
fn ch_7_7_2_03_multiline_flow_nested() {
    Fixture::new(
        "7.7.2",
        3,
        "Multiline nested flow collections",
        "data: {\n  items: [\n    a,\n    b\n  ],\n  count: 2\n}\n",
    )
    .run();
}

#[test]
fn ch_7_7_2_04_multiline_double_quoted_scalar() {
    Fixture::new(
        "7.7.2",
        4,
        "Multiline double-quoted flow scalar",
        "text: \"This is a long \\\nstring that spans \\\nmultiple lines\"\n",
    )
    .run();
}

#[test]
fn ch_7_7_2_05_multiline_single_quoted_scalar() {
    Fixture::new(
        "7.7.2",
        5,
        "Multiline single-quoted flow scalar",
        "text: 'This is a long\nstring that spans\nmultiple lines'\n",
    )
    .run();
}

#[test]
fn ch_7_7_2_06_multiline_flow_with_empty_lines() {
    Fixture::new(
        "7.7.2",
        6,
        "Multiline flow sequence with empty lines",
        "items: [\n  a,\n\n  b,\n\n  c\n]\n",
    )
    .run();
}

// -----------------------------------------------------------------------------
// 7.7.3 Complex Keys in Flow Mappings
// -----------------------------------------------------------------------------

#[test]
fn ch_7_7_3_01_complex_key_quoted() {
    Fixture::new(
        "7.7.3",
        1,
        "Flow mapping with quoted complex key",
        "{\"key with spaces\": value}\n",
    )
    .run();
}

#[test]
fn ch_7_7_3_02_complex_key_special_chars() {
    Fixture::new(
        "7.7.3",
        2,
        "Flow mapping with special character key",
        "{\"key:with:colons\": value}\n",
    )
    .run();
}

#[test]
fn ch_7_7_3_03_complex_key_explicit_indicator() {
    Fixture::new(
        "7.7.3",
        3,
        "Flow mapping with explicit key indicator",
        "{? complex key : value}\n",
    )
    .run();
}

#[test]
fn ch_7_7_3_04_complex_key_flow_sequence() {
    Fixture::new(
        "7.7.3",
        4,
        "Flow mapping with flow sequence as key",
        "{? [a, b] : value}\n",
    )
    .run();
}

// -----------------------------------------------------------------------------
// 7.7.4 Deeply Nested Flow Collections
// -----------------------------------------------------------------------------

#[test]
fn ch_7_7_4_01_deeply_nested_sequences() {
    Fixture::new(
        "7.7.4",
        1,
        "Deeply nested flow sequences (10 levels)",
        "deep: [[[[[[[[[[value]]]]]]]]]]\n",
    )
    .run();
}

#[test]
fn ch_7_7_4_02_deeply_nested_mappings() {
    Fixture::new(
        "7.7.4",
        2,
        "Deeply nested flow mappings (10 levels)",
        "deep: {a: {b: {c: {d: {e: {f: {g: {h: {i: {j: value}}}}}}}}}}\n",
    )
    .run();
}

#[test]
fn ch_7_7_4_03_deeply_nested_mixed() {
    Fixture::new(
        "7.7.4",
        3,
        "Deeply nested mixed flow collections",
        "deep: [{a: [{b: [{c: [{d: value}]}]}]}]\n",
    )
    .run();
}

#[test]
fn ch_7_7_4_04_deeply_nested_with_values() {
    Fixture::new(
        "7.7.4",
        4,
        "Deeply nested flow with multiple values at each level",
        "deep: [[a, [b, [c, [d, [e, [f]]]]]]]\n",
    )
    .run();
}

// -----------------------------------------------------------------------------
// 7.7.5 Duplicate Keys Error Handling
// -----------------------------------------------------------------------------

#[test]
fn ch_7_7_5_01_duplicate_keys_simple() {
    let input = "{a: 1, a: 2}\n";
    let result = Doc::from_str(input);
    assert!(
        result.is_err(),
        "Flow mapping should reject duplicate keys"
    );
}

#[test]
fn ch_7_7_5_02_duplicate_keys_different_quotes() {
    let input = "{a: 1, 'a': 2}\n";
    let result = Doc::from_str(input);
    assert!(
        result.is_err(),
        "Flow mapping should reject duplicate keys with different quoting"
    );
}

#[test]
fn ch_7_7_5_03_duplicate_keys_nested() {
    let input = "{outer: {a: 1, a: 2}}\n";
    let result = Doc::from_str(input);
    assert!(
        result.is_err(),
        "Nested flow mapping should reject duplicate keys"
    );
}
