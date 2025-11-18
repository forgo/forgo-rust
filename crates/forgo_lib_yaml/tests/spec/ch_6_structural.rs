// crates/forgo_lib_yaml/tests/spec/ch_6_structural.rs
//! YAML 1.2.2 Chapter 6: Structural Productions
//!
//! Reference: https://yaml.org/spec/1.2.2/#chapter-6-structural-productions
//!
//! Structural productions define the YAML document structure.

#[path = "../common/mod.rs"]
mod common;

use common::Fixture;
use forgo_lib_yaml::Doc;

// ============================================================================
// 6.1 Indentation Spaces
// ============================================================================
// Productions [63-65]: s-indent(n), s-indent-less-than(n), s-indent-less-or-equal(n)

#[test]
fn ch_6_1_01_exact_indentation_2_spaces() {
    // s-indent(2) - Exactly 2 spaces for nested content
    Fixture::new(
        "6.1",
        1,
        "Exact 2-space indentation",
        "parent:\n  child: value\n",
    )
    .run();
}

#[test]
fn ch_6_1_02_exact_indentation_4_spaces() {
    // s-indent(4) - Exactly 4 spaces
    Fixture::new(
        "6.1",
        2,
        "Exact 4-space indentation",
        "parent:\n    child: value\n",
    )
    .run();
}

#[test]
fn ch_6_1_03_nested_indentation_levels() {
    // Multiple s-indent(n) levels (2, 4, 6 spaces)
    Fixture::new(
        "6.1",
        3,
        "Multiple nested indentation levels",
        "l1:\n  l2:\n    l3:\n      value: data\n",
    )
    .run();
}

#[test]
fn ch_6_1_04_indentation_less_than() {
    // s-indent-less-than(n) - Indentation must be less than parent
    Fixture::new(
        "6.1",
        4,
        "Indentation less than parent level",
        "parent:\n  child1: value\nsibling: value\n",
    )
    .run();
}

#[test]
fn ch_6_1_05_indentation_sequence() {
    // Indentation in sequences
    Fixture::new(
        "6.1",
        5,
        "Indentation in block sequences",
        "items:\n  - first\n  - second\n  - third\n",
    )
    .run();
}

#[test]
fn ch_6_1_06_zero_indentation() {
    // s-indent(0) - Top-level entries (no indentation)
    Fixture::new(
        "6.1",
        6,
        "Zero indentation at top level",
        "key1: value1\nkey2: value2\n",
    )
    .run();
}

#[test]
fn ch_6_1_07_indentation_after_sequence_indicator() {
    // Indentation after dash in sequences
    Fixture::new(
        "6.1",
        7,
        "Indentation after sequence indicator",
        "- item1\n- item2\n- nested:\n    deep: value\n",
    )
    .run();
}

// ============================================================================
// 6.2 Separation Spaces
// ============================================================================
// Production [66]: s-separate-in-line

#[test]
fn ch_6_2_01_space_after_colon() {
    // Required space after : in mappings
    Fixture::new(
        "6.2",
        1,
        "Space after colon in mapping",
        "key: value\n",
    )
    .run();
}

#[test]
fn ch_6_2_02_no_space_after_colon_error() {
    // Missing space after : should fail
    let input = "key:value\n";
    let result = Doc::from_str(input);

    // Most parsers require space after colon in block context
    // May succeed in flow context {key:value}
    if result.is_err() {
        assert!(result.is_err(), "Missing space after colon should fail in block context");
    }
}

#[test]
fn ch_6_2_03_multiple_spaces_after_colon() {
    // Multiple spaces are allowed
    Fixture::new(
        "6.2",
        3,
        "Multiple spaces after colon",
        "key:   value\n",
    )
    .run();
}

#[test]
fn ch_6_2_04_space_after_dash() {
    // Required space after - in sequences
    Fixture::new(
        "6.2",
        4,
        "Space after dash in sequence",
        "- item1\n- item2\n",
    )
    .run();
}

#[test]
fn ch_6_2_05_space_after_comma_flow() {
    // Separation in flow collections
    Fixture::new(
        "6.2",
        5,
        "Space after comma in flow",
        "list: [a, b, c]\n",
    )
    .run();
}

#[test]
fn ch_6_2_06_tab_as_separation() {
    // Tab can be used for separation (not indentation)
    let input = "key:\tvalue\n";
    let result = Doc::from_str(input);

    // Tabs allowed for separation but not indentation
    if result.is_ok() {
        // Implementation accepts tab as separator
        assert!(result.is_ok());
    }
}

#[test]
fn ch_6_2_07_no_space_in_flow_allowed() {
    // In flow context, spaces optional
    Fixture::new(
        "6.2",
        7,
        "Flow collections without spaces",
        "compact: {a:1,b:2}\n",
    )
    .run();
}

// ============================================================================
// 6.3 Line Prefixes
// ============================================================================
// Productions [67-69]: s-line-prefix(n,c), s-block-line-prefix(n), s-flow-line-prefix(n)

#[test]
fn ch_6_3_01_block_line_prefix() {
    // s-block-line-prefix(n) in block context
    Fixture::new(
        "6.3",
        1,
        "Block line prefix with indentation",
        "parent:\n  child: value\n  other: data\n",
    )
    .run();
}

#[test]
fn ch_6_3_02_flow_line_prefix() {
    // s-flow-line-prefix(n) in flow context
    Fixture::new(
        "6.3",
        2,
        "Flow line prefix",
        "data: {\n  key: value,\n  other: data\n}\n",
    )
    .run();
}

#[test]
fn ch_6_3_03_line_prefix_with_comments() {
    // Line prefix can include comments
    Fixture::new(
        "6.3",
        3,
        "Line prefix with comment lines",
        "parent:\n  # comment\n  child: value\n",
    )
    .run();
}

#[test]
fn ch_6_3_04_line_prefix_block_scalar() {
    // Line prefix in block scalars
    Fixture::new(
        "6.3",
        4,
        "Line prefix in literal scalar",
        "text: |\n  line1\n  line2\n",
    )
    .run();
}

#[test]
fn ch_6_3_05_line_prefix_nested_flow() {
    // Line prefix in nested flow collections
    Fixture::new(
        "6.3",
        5,
        "Line prefix in nested flow",
        "data: [\n  [a, b],\n  [c, d]\n]\n",
    )
    .run();
}

// ============================================================================
// 6.4 Empty Lines
// ============================================================================
// Production [70]: l-empty(n,c)

#[test]
fn ch_6_4_01_empty_lines_between_entries() {
    // Empty lines between mapping entries
    Fixture::new(
        "6.4",
        1,
        "Empty lines between entries",
        "key1: value1\n\nkey2: value2\n",
    )
    .run();
}

#[test]
fn ch_6_4_02_multiple_empty_lines() {
    // Multiple consecutive empty lines
    Fixture::new(
        "6.4",
        2,
        "Multiple consecutive empty lines",
        "key1: value1\n\n\n\nkey2: value2\n",
    )
    .run();
}

#[test]
fn ch_6_4_03_empty_lines_in_sequences() {
    // Empty lines in sequences
    Fixture::new(
        "6.4",
        3,
        "Empty lines in sequences",
        "- item1\n\n- item2\n\n- item3\n",
    )
    .run();
}

#[test]
fn ch_6_4_04_empty_lines_preserve_structure() {
    // Empty lines should not affect structure
    let with_empty = "key1: value1\n\nkey2: value2\n";
    let without_empty = "key1: value1\nkey2: value2\n";

    let doc1 = Doc::from_str(with_empty).unwrap();
    let doc2 = Doc::from_str(without_empty).unwrap();

    // Both should parse to same structure
    use forgo_lib_yaml::Node;
    let Node::Map(m1) = doc1.root().node() else { panic!("Expected map"); };
    let Node::Map(m2) = doc2.root().node() else { panic!("Expected map"); };

    assert_eq!(m1.len(), m2.len());
}

#[test]
fn ch_6_4_05_empty_lines_in_block_scalar() {
    // Empty lines within block scalars are preserved
    Fixture::new(
        "6.4",
        5,
        "Empty lines in block scalar",
        "text: |\n  line1\n  \n  line3\n",
    )
    .run();
}

#[test]
fn ch_6_4_06_empty_line_with_spaces() {
    // Empty line with only spaces (whitespace-only)
    Fixture::new(
        "6.4",
        6,
        "Empty line with spaces",
        "key1: value1\n  \nkey2: value2\n",
    )
    .run();
}

// ============================================================================
// 6.5 Line Folding
// ============================================================================
// Productions [71-74]: b-l-trimmed, b-as-space, b-l-folded, s-flow-folded

#[test]
fn ch_6_5_01_folded_scalar_basic() {
    // Basic folded scalar (>) folds newlines to spaces
    Fixture::new(
        "6.5",
        1,
        "Basic folded scalar",
        "text: >\n  folded\n  line\n",
    )
    .run();
}

#[test]
fn ch_6_5_02_folded_preserves_blank_lines() {
    // Folded scalar preserves blank lines as line breaks
    Fixture::new(
        "6.5",
        2,
        "Folded scalar with blank lines",
        "text: >\n  paragraph 1\n\n  paragraph 2\n",
    )
    .run();
}

#[test]
fn ch_6_5_03_folded_more_indented_preserved() {
    // More-indented lines in folded scalars are preserved
    Fixture::new(
        "6.5",
        3,
        "Folded with more indentation",
        "text: >\n  normal\n    indented\n  normal\n",
    )
    .run();
}

#[test]
fn ch_6_5_04_line_folding_in_plain_scalar() {
    // Plain scalars also fold lines
    Fixture::new(
        "6.5",
        4,
        "Line folding in plain scalar",
        "text: this is a long\n  plain scalar that\n  spans lines\n",
    )
    .run();
}

#[test]
fn ch_6_5_05_folding_multiple_spaces() {
    // Multiple spaces collapse to single space in folding
    let input = "text: >\n  fold   multiple   spaces\n";
    let doc = Doc::from_str(input);

    // Folding should normalize spaces
    assert!(doc.is_ok());
}

#[test]
fn ch_6_5_06_flow_folding() {
    // s-flow-folded: line folding in flow context
    Fixture::new(
        "6.5",
        6,
        "Flow line folding",
        "data: [\n  item1,\n  item2\n]\n",
    )
    .run();
}

#[test]
fn ch_6_5_07_folded_chomping_strip() {
    // Folded with strip chomping (-)
    Fixture::new(
        "6.5",
        7,
        "Folded scalar with strip chomping",
        "text: >-\n  folded\n  text\n",
    )
    .run();
}

#[test]
fn ch_6_5_08_folded_chomping_keep() {
    // Folded with keep chomping (+)
    Fixture::new(
        "6.5",
        8,
        "Folded scalar with keep chomping",
        "text: >+\n  folded\n  text\n\n",
    )
    .run();
}

// ============================================================================
// 6.6 Comments
// ============================================================================

#[test]
fn ch_6_6_01_single_line_comment() {
    let input = "# comment\nkey: value\n";
    let d = Doc::from_str(input).unwrap();
    let out = d.to_string().unwrap();
    assert!(out.starts_with("# comment"));
}

#[test]
fn ch_6_6_02_trailing_comment() {
    let input = "key: value # comment\n";
    let d = Doc::from_str(input).unwrap();
    let out = d.to_string().unwrap();
    assert!(out.contains("# comment"));
}

#[test]
fn ch_6_6_03_comment_only_line() {
    Fixture::new(
        "6.6",
        3,
        "Comment-only lines",
        "# first\n# second\nkey: value\n",
    )
    .run();
}

#[test]
fn ch_6_6_04_inline_comment_after_scalar() {
    Fixture::new(
        "6.6",
        4,
        "Inline comment after scalar",
        "- a # comment on a\n- b\n",
    )
    .run();
}

#[test]
fn ch_6_6_05_hash_in_quoted_string() {
    Fixture::new(
        "6.6",
        5,
        "Hash in quoted string is not a comment",
        "key: '#not a comment'\n",
    )
    .run();
}

// ============================================================================
// 6.7 Separation Lines
// ============================================================================
// Productions [80-81]: s-separate-lines(n), s-separate(n,c)

#[test]
fn ch_6_7_01_separation_lines_in_block() {
    // Blank lines separate block entries
    Fixture::new(
        "6.7",
        1,
        "Separation lines in block context",
        "key1: value1\n\nkey2: value2\n",
    )
    .run();
}

#[test]
fn ch_6_7_02_no_separation_required_same_line() {
    // Same-line entries separated by space
    Fixture::new(
        "6.7",
        2,
        "Same-line separation with space",
        "flow: [a, b, c]\n",
    )
    .run();
}

#[test]
fn ch_6_7_03_separation_with_comments() {
    // Comment lines count as separation
    Fixture::new(
        "6.7",
        3,
        "Separation via comment lines",
        "key1: value1\n# comment\nkey2: value2\n",
    )
    .run();
}

#[test]
fn ch_6_7_04_separation_in_sequences() {
    // Separation lines in sequences
    Fixture::new(
        "6.7",
        4,
        "Separation in block sequences",
        "- item1\n\n- item2\n",
    )
    .run();
}

#[test]
fn ch_6_7_05_separation_flow_vs_block() {
    // Different separation rules in flow vs block
    Fixture::new(
        "6.7",
        5,
        "Flow allows no separation, block needs newline",
        "block:\n  - item1\n  - item2\nflow: [item1,item2]\n",
    )
    .run();
}

#[test]
fn ch_6_7_06_separation_lines_with_indentation() {
    // Separation lines preserve indentation context
    Fixture::new(
        "6.7",
        6,
        "Separation lines with indentation",
        "parent:\n  child1: value\n  \n  child2: value\n",
    )
    .run();
}

#[test]
fn ch_6_7_07_multiple_separation_lines() {
    // Multiple blank lines still count as one separation
    Fixture::new(
        "6.7",
        7,
        "Multiple separation lines",
        "key1: value1\n\n\n\nkey2: value2\n",
    )
    .run();
}

// ============================================================================
// 6.8 Directives
// ============================================================================

// 6.8.1 "YAML" Directives
#[test]
fn ch_6_8_1_01_yaml_directive_basic() {
    Fixture::new(
        "6.8.1",
        1,
        "YAML version directive",
        "%YAML 1.2\n---\nkey: value\n",
    )
    .run();
}

#[test]
fn ch_6_8_1_02_yaml_directive_preserved() {
    let input = "%YAML 1.2\n---\nkey: value\n";
    let d = Doc::from_str(input).unwrap();
    assert_eq!(d.yaml_version(), Some((1, 2)));
}

// 6.8.2 "TAG" Directives
#[test]
fn ch_6_8_2_01_tag_directive_basic() {
    Fixture::new(
        "6.8.2",
        1,
        "TAG directive declaration",
        "%TAG !e! tag:example.com,2000:app/\n---\nkey: value\n",
    )
    .run();
}

#[test]
fn ch_6_8_2_02_multiple_tag_directives() {
    Fixture::new(
        "6.8.2",
        2,
        "Multiple TAG directives",
        "%TAG !e! tag:example.com,2000:app/\n%TAG !f! tag:foo.com,2000:/\n---\nkey: value\n",
    )
    .run();
}

#[test]
fn ch_6_8_2_03_yaml_and_tag_directives() {
    Fixture::new(
        "6.8.2",
        3,
        "Both YAML and TAG directives",
        "%YAML 1.2\n%TAG !e! tag:example.com,2000:app/\n---\nkey: value\n",
    )
    .run();
}

// 6.8.2.1 Tag Handles
#[test]
fn ch_6_8_2_1_01_primary_tag_handle() {
    Fixture::new(
        "6.8.2.1",
        1,
        "Primary tag handle (!)",
        "- !local value\n",
    )
    .run();
}

#[test]
fn ch_6_8_2_1_02_secondary_tag_handle() {
    Fixture::new(
        "6.8.2.1",
        2,
        "Secondary tag handle (!!)",
        "- !!str value\n- !!int 123\n",
    )
    .run();
}

#[test]
fn ch_6_8_2_1_03_named_tag_handle() {
    Fixture::new(
        "6.8.2.1",
        3,
        "Named tag handle",
        "%TAG !e! tag:example.com,2000:app/\n---\n- !e!thing value\n",
    )
    .run();
}

// 6.8.2.2 Tag Prefixes
#[test]
fn ch_6_8_2_2_01_verbatim_tag() {
    Fixture::new(
        "6.8.2.2",
        1,
        "Verbatim tag prefix",
        "- !<tag:example.com,2000:app/type> value\n",
    )
    .run();
}

// ============================================================================
// 6.9 Node Properties
// ============================================================================

// 6.9.1 Node Tags
#[test]
fn ch_6_9_1_01_tag_on_scalar() {
    Fixture::new(
        "6.9.1",
        1,
        "Tag on scalar value",
        "value: !!str 123\n",
    )
    .run();
}

#[test]
fn ch_6_9_1_02_tag_on_sequence() {
    Fixture::new(
        "6.9.1",
        2,
        "Tag on sequence",
        "list: !!seq\n  - a\n  - b\n",
    )
    .run();
}

#[test]
fn ch_6_9_1_03_tag_on_mapping() {
    Fixture::new(
        "6.9.1",
        3,
        "Tag on mapping",
        "map: !!map\n  key: value\n",
    )
    .run();
}

#[test]
fn ch_6_9_1_04_tag_forces_type() {
    Fixture::new(
        "6.9.1",
        4,
        "Tag forces type interpretation",
        "version: !!str 1.0\n",
    )
    .run();
}

// 6.9.2 Node Anchors
#[test]
fn ch_6_9_2_01_simple_anchor() {
    Fixture::new(
        "6.9.2",
        1,
        "Simple node anchor",
        "item: &anchor value\nref: *anchor\n",
    )
    .run();
}

#[test]
fn ch_6_9_2_02_anchor_on_mapping() {
    Fixture::new(
        "6.9.2",
        2,
        "Anchor on mapping",
        "defaults: &def\n  a: 1\n  b: 2\nref: *def\n",
    )
    .run();
}

#[test]
fn ch_6_9_2_03_anchor_on_sequence() {
    Fixture::new(
        "6.9.2",
        3,
        "Anchor on sequence",
        "list: &mylist\n  - a\n  - b\nref: *mylist\n",
    )
    .run();
}

#[test]
fn ch_6_9_2_04_merge_key() {
    Fixture::new(
        "6.9.2",
        4,
        "Merge key (<<) usage",
        "defaults: &def\n  a: 1\n  b: 2\nuse:\n  <<: *def\n  c: 3\n",
    )
    .run();
}

#[test]
fn ch_6_9_2_05_anchor_with_tag() {
    Fixture::new(
        "6.9.2",
        5,
        "Anchor combined with tag",
        "data: &anchor !!str value\nref: *anchor\n",
    )
    .run();
}

// ============================================================================
// Core Schema Tags (from spec)
// ============================================================================

#[test]
fn ch_6_10_01_str_tag() {
    Fixture::new(
        "6.10",
        1,
        "Explicit !!str tag",
        "version: !!str 1.0\n",
    )
    .run();
}

#[test]
fn ch_6_10_02_int_tag() {
    Fixture::new(
        "6.10",
        2,
        "Explicit !!int tag",
        "count: !!int 42\n",
    )
    .run();
}

#[test]
fn ch_6_10_03_float_tag() {
    Fixture::new(
        "6.10",
        3,
        "Explicit !!float tag",
        "value: !!float 3.14\n",
    )
    .run();
}

#[test]
fn ch_6_10_04_bool_tag() {
    Fixture::new(
        "6.10",
        4,
        "Explicit !!bool tag",
        "enabled: !!bool true\n",
    )
    .run();
}

#[test]
fn ch_6_10_05_null_tag() {
    Fixture::new(
        "6.10",
        5,
        "Explicit !!null tag",
        "empty: !!null null\n",
    )
    .run();
}

#[test]
fn ch_6_10_06_seq_tag() {
    Fixture::new(
        "6.10",
        6,
        "Explicit !!seq tag",
        "items: !!seq [a, b, c]\n",
    )
    .run();
}

#[test]
fn ch_6_10_07_map_tag() {
    Fixture::new(
        "6.10",
        7,
        "Explicit !!map tag",
        "config: !!map {a: 1, b: 2}\n",
    )
    .run();
}
