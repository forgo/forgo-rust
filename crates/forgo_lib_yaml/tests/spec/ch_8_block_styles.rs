// crates/forgo_lib_yaml/tests/spec/ch_8_block_styles.rs
//! YAML 1.2.2 Chapter 8: Block Style Productions
//!
//! Reference: https://yaml.org/spec/1.2.2/#chapter-8-block-style-productions
//!
//! Block styles use indentation to denote structure.

#[path = "../common/mod.rs"]
mod common;

use common::Fixture;
use forgo_lib_yaml::Doc;

// ============================================================================
// 8.1 Block Scalar Styles
// ============================================================================

// 8.1.1 Block Scalar Headers
#[test]
fn ch_8_1_1_01_literal_basic() {
    Fixture::new(
        "8.1.1",
        1,
        "Basic literal block scalar",
        "|\nfoo\nbar\n",
    )
    .run();
}

#[test]
fn ch_8_1_1_02_folded_basic() {
    Fixture::new(
        "8.1.1",
        2,
        "Basic folded block scalar",
        ">\nfoo\nbar\n",
    )
    .run();
}

// 8.1.1.1 Block Indentation Indicator
#[test]
fn ch_8_1_1_1_01_indent_indicator_explicit() {
    Fixture::new(
        "8.1.1.1",
        1,
        "Literal with explicit indentation indicator",
        "|2\n  foo\n  bar\n",
    )
    .run();
}

#[test]
fn ch_8_1_1_1_02_indent_indicator_filters_lines() {
    Fixture::new(
        "8.1.1.1",
        2,
        "Indent indicator filters insufficient indent",
        "|3\n   acceptable\n    also acceptable\n  too little\n",
    )
    .run();
}

#[test]
fn ch_8_1_1_1_03_no_indicator_uses_min_indent() {
    Fixture::new(
        "8.1.1.1",
        3,
        "No indicator uses minimum indent",
        "|\n   foo\n     bar\n   baz\n",
    )
    .run();
}

#[test]
fn ch_8_1_1_1_04_indent_indicator_zero_invalid() {
    Fixture::new(
        "8.1.1.1",
        4,
        "Indent indicator 0 is invalid",
        "|0\n  foo\n",
    )
    .expect_error("Parse")
    .run();
}

// 8.1.1.2 Block Chomping Indicator
#[test]
fn ch_8_1_1_2_01_chomp_strip() {
    Fixture::new(
        "8.1.1.2",
        1,
        "Strip chomping removes all trailing newlines",
        "|-\n  content\n\n\n",
    )
    .run();
}

#[test]
fn ch_8_1_1_2_02_chomp_keep() {
    Fixture::new(
        "8.1.1.2",
        2,
        "Keep chomping preserves all trailing newlines",
        "|+\n  content\n\n\n",
    )
    .run();
}

#[test]
fn ch_8_1_1_2_03_chomp_clip_default() {
    Fixture::new(
        "8.1.1.2",
        3,
        "Clip chomping (default) keeps one trailing newline",
        "|\n  content\n\n\n",
    )
    .run();
}

#[test]
fn ch_8_1_1_2_04_indicator_order_independent() {
    Fixture::new(
        "8.1.1.2",
        4,
        "Chomping and indent indicators in either order",
        "|2-\n  foo\n  bar\n",
    )
    .run();
}

#[test]
fn ch_8_1_1_2_05_indicator_order_reversed() {
    Fixture::new(
        "8.1.1.2",
        5,
        "Indicators in reverse order",
        "|-2\n  foo\n  bar\n",
    )
    .run();
}

// 8.1.2 Literal Style
#[test]
fn ch_8_1_2_01_literal_preserves_newlines() {
    Fixture::new(
        "8.1.2",
        1,
        "Literal preserves all line breaks",
        "|\nline 1\nline 2\n\nline 4\n",
    )
    .run();
}

#[test]
fn ch_8_1_2_02_literal_preserves_leading_spaces() {
    Fixture::new(
        "8.1.2",
        2,
        "Literal preserves leading spaces",
        "|\n  indented\n    more indented\n  back\n",
    )
    .run();
}

#[test]
fn ch_8_1_2_03_literal_preserves_empty_lines() {
    Fixture::new(
        "8.1.2",
        3,
        "Literal preserves empty lines",
        "|\nfoo\n\n\nbar\n",
    )
    .run();
}

#[test]
fn ch_8_1_2_04_literal_single_line() {
    Fixture::new(
        "8.1.2",
        4,
        "Literal with single line",
        "|\n  single line\n",
    )
    .run();
}

// 8.1.3 Folded Style
#[test]
fn ch_8_1_3_01_folded_single_newlines_to_space() {
    Fixture::new(
        "8.1.3",
        1,
        "Folded: single newlines become spaces",
        ">\nfoo\nbar\n",
    )
    .run();
}

#[test]
fn ch_8_1_3_02_folded_blank_lines_preserved() {
    Fixture::new(
        "8.1.3",
        2,
        "Folded: blank lines preserved as breaks",
        ">\nfoo\nbar\n\nbaz\n",
    )
    .run();
}

#[test]
fn ch_8_1_3_03_folded_more_indented_preserved() {
    Fixture::new(
        "8.1.3",
        3,
        "Folded: more-indented lines preserved",
        ">\nfoo\n bar\n  baz\nqux\n",
    )
    .run();
}

#[test]
fn ch_8_1_3_04_folded_complex_paragraphs() {
    Fixture::new(
        "8.1.3",
        4,
        "Folded: complex text with paragraphs",
        ">\nFirst paragraph\nstill first\n\nSecond paragraph\n  indented\nback to second\n",
    )
    .run();
}

// ============================================================================
// 8.2 Block Collection Styles
// ============================================================================

// 8.2.1 Block Sequences
#[test]
fn ch_8_2_1_01_block_sequence_basic() {
    Fixture::new(
        "8.2.1",
        1,
        "Basic block sequence",
        "- item1\n- item2\n- item3\n",
    )
    .run();
}

#[test]
fn ch_8_2_1_02_block_sequence_numbers() {
    Fixture::new(
        "8.2.1",
        2,
        "Block sequence of numbers",
        "- 1\n- 2\n- 3\n",
    )
    .run();
}

#[test]
fn ch_8_2_1_03_block_sequence_mixed_types() {
    Fixture::new(
        "8.2.1",
        3,
        "Block sequence with mixed types",
        "- string\n- 42\n- true\n- null\n",
    )
    .run();
}

#[test]
fn ch_8_2_1_04_block_sequence_nested() {
    Fixture::new(
        "8.2.1",
        4,
        "Nested block sequences",
        "- - nested1\n  - nested2\n- item2\n",
    )
    .run();
}

#[test]
fn ch_8_2_1_05_block_sequence_of_mappings() {
    Fixture::new(
        "8.2.1",
        5,
        "Block sequence of mappings",
        "- name: Alice\n  age: 30\n- name: Bob\n  age: 25\n",
    )
    .run();
}

// 8.2.2 Block Mappings
#[test]
fn ch_8_2_2_01_block_mapping_basic() {
    Fixture::new(
        "8.2.2",
        1,
        "Basic block mapping",
        "name: Alice\nage: 30\n",
    )
    .run();
}

#[test]
fn ch_8_2_2_02_block_mapping_numbers() {
    Fixture::new(
        "8.2.2",
        2,
        "Block mapping with numeric values",
        "width: 1920\nheight: 1080\n",
    )
    .run();
}

#[test]
fn ch_8_2_2_03_block_mapping_booleans() {
    Fixture::new(
        "8.2.2",
        3,
        "Block mapping with boolean values",
        "enabled: true\nvisible: false\n",
    )
    .run();
}

#[test]
fn ch_8_2_2_04_block_mapping_nested() {
    Fixture::new(
        "8.2.2",
        4,
        "Nested block mappings",
        "outer:\n  inner: value\n  another: 42\n",
    )
    .run();
}

#[test]
fn ch_8_2_2_05_block_mapping_with_sequences() {
    Fixture::new(
        "8.2.2",
        5,
        "Block mapping with sequence values",
        "fruits:\n  - apple\n  - banana\nveggies:\n  - carrot\n",
    )
    .run();
}

#[test]
fn ch_8_2_2_06_block_mapping_quoted_keys() {
    Fixture::new(
        "8.2.2",
        6,
        "Block mapping with quoted keys",
        "\"key with spaces\": value\n\"another:key\": value2\n",
    )
    .run();
}

#[test]
fn ch_8_2_2_07_block_mapping_very_long_key() {
    // Production [157] ns-l-compact-mapping - very long key (500+ chars)
    let long_key = "a".repeat(500);
    let yaml = format!("{}: value\n", long_key);

    // Parse and verify it works
    let doc = Doc::from_str(&yaml).expect("Should parse very long key");
    let output = doc.to_string().expect("Should emit");

    // Verify the key is preserved
    assert!(output.len() > 500, "Output should contain the long key");
}

// 8.2.3 Block Nodes
#[test]
fn ch_8_2_3_01_block_node_with_anchor() {
    Fixture::new(
        "8.2.3",
        1,
        "Block node with anchor",
        "data: &anchor\n  key: value\nref: *anchor\n",
    )
    .run();
}

#[test]
fn ch_8_2_3_02_block_node_with_tag() {
    Fixture::new(
        "8.2.3",
        2,
        "Block node with tag",
        "value: !!str\n  123\n",
    )
    .run();
}

#[test]
fn ch_8_2_3_03_block_scalar_in_mapping() {
    Fixture::new(
        "8.2.3",
        3,
        "Block scalar as map value",
        "key: |\n  value\n  multi-line\n",
    )
    .run();
}

#[test]
fn ch_8_2_3_04_block_scalar_in_sequence() {
    Fixture::new(
        "8.2.3",
        4,
        "Block scalar as sequence item",
        "- |\n  first\n  item\n- |\n  second\n  item\n",
    )
    .run();
}

// ============================================================================
// Block Style Edge Cases
// ============================================================================

#[test]
fn ch_8_3_01_deeply_nested_blocks() {
    Fixture::new(
        "8.3",
        1,
        "Deeply nested block structures",
        "level1:\n  level2:\n    level3:\n      value: deep\n",
    )
    .run();
}

#[test]
fn ch_8_3_02_mixed_block_collections() {
    Fixture::new(
        "8.3",
        2,
        "Mixed block sequences and mappings",
        "users:\n  - name: Alice\n    roles:\n      - admin\n      - user\n  - name: Bob\n    roles:\n      - user\n",
    )
    .run();
}

#[test]
fn ch_8_3_03_block_scalar_with_comment() {
    Fixture::new(
        "8.3",
        3,
        "Block scalar with comment on header",
        "text: | # comment\n  content\n",
    )
    .run();
}

#[test]
fn ch_8_3_04_empty_block_scalar() {
    Fixture::new(
        "8.3",
        4,
        "Empty literal block scalar",
        "|\n",
    )
    .run();
}

// =============================================================================
// 8.4 Chomping Indicator Combinations
// =============================================================================

#[test]
fn ch_8_4_01_folded_strip_chomping() {
    Fixture::new(
        "8.4",
        1,
        "Folded with strip chomping (>-)",
        ">-\n  folded\n  text\n\n\n",
    )
    .run();
}

#[test]
fn ch_8_4_02_folded_keep_chomping() {
    Fixture::new(
        "8.4",
        2,
        "Folded with keep chomping (>+)",
        ">+\n  folded\n  text\n\n\n",
    )
    .run();
}

#[test]
fn ch_8_4_03_folded_clip_chomping_default() {
    Fixture::new(
        "8.4",
        3,
        "Folded with clip chomping (default)",
        ">\n  folded\n  text\n\n\n",
    )
    .run();
}

#[test]
fn ch_8_4_04_literal_with_indent_and_strip() {
    Fixture::new(
        "8.4",
        4,
        "Literal with indent indicator and strip (|2-)",
        "|2-\n  line1\n  line2\n\n",
    )
    .run();
}

#[test]
fn ch_8_4_05_literal_with_strip_and_indent() {
    Fixture::new(
        "8.4",
        5,
        "Literal with strip and indent (|-2)",
        "|-2\n  line1\n  line2\n\n",
    )
    .run();
}

#[test]
fn ch_8_4_06_folded_with_indent_and_keep() {
    Fixture::new(
        "8.4",
        6,
        "Folded with indent indicator and keep (>2+)",
        ">2+\n  text\n  here\n\n\n",
    )
    .run();
}

#[test]
fn ch_8_4_07_folded_with_keep_and_indent() {
    Fixture::new(
        "8.4",
        7,
        "Folded with keep and indent (>+2)",
        ">+2\n  text\n  here\n\n\n",
    )
    .run();
}

// =============================================================================
// 8.5 Full Indentation Indicator Range (1-9)
// =============================================================================

#[test]
fn ch_8_5_01_indent_indicator_1() {
    Fixture::new(
        "8.5",
        1,
        "Indent indicator 1",
        "|1\n text\n",
    )
    .run();
}

#[test]
fn ch_8_5_02_indent_indicator_2() {
    Fixture::new(
        "8.5",
        2,
        "Indent indicator 2",
        "|2\n  text\n",
    )
    .run();
}

#[test]
fn ch_8_5_03_indent_indicator_3() {
    Fixture::new(
        "8.5",
        3,
        "Indent indicator 3",
        "|3\n   text\n",
    )
    .run();
}

#[test]
fn ch_8_5_04_indent_indicator_4() {
    Fixture::new(
        "8.5",
        4,
        "Indent indicator 4",
        "|4\n    text\n",
    )
    .run();
}

#[test]
fn ch_8_5_05_indent_indicator_5() {
    Fixture::new(
        "8.5",
        5,
        "Indent indicator 5",
        "|5\n     text\n",
    )
    .run();
}

#[test]
fn ch_8_5_06_indent_indicator_6() {
    Fixture::new(
        "8.5",
        6,
        "Indent indicator 6",
        "|6\n      text\n",
    )
    .run();
}

#[test]
fn ch_8_5_07_indent_indicator_7() {
    Fixture::new(
        "8.5",
        7,
        "Indent indicator 7",
        "|7\n       text\n",
    )
    .run();
}

#[test]
fn ch_8_5_08_indent_indicator_8() {
    Fixture::new(
        "8.5",
        8,
        "Indent indicator 8",
        "|8\n        text\n",
    )
    .run();
}

#[test]
fn ch_8_5_09_indent_indicator_9() {
    Fixture::new(
        "8.5",
        9,
        "Indent indicator 9",
        "|9\n         text\n",
    )
    .run();
}

// =============================================================================
// 8.6 Explicit Key/Value Indicators
// =============================================================================

#[test]
fn ch_8_6_01_explicit_key_indicator_simple() {
    Fixture::new(
        "8.6",
        1,
        "Explicit key indicator (?)",
        "? key\n: value\n",
    )
    .run();
}

#[test]
fn ch_8_6_02_explicit_key_multiline() {
    Fixture::new(
        "8.6",
        2,
        "Explicit key with multiline value",
        "? key\n: value\n  continuation\n",
    )
    .run();
}

#[test]
fn ch_8_6_03_explicit_key_with_complex_value() {
    Fixture::new(
        "8.6",
        3,
        "Explicit key with complex value",
        "? simple key\n:\n  - value1\n  - value2\n",
    )
    .run();
}

#[test]
fn ch_8_6_04_multiple_explicit_keys() {
    Fixture::new(
        "8.6",
        4,
        "Multiple explicit keys in mapping",
        "? key1\n: value1\n? key2\n: value2\n",
    )
    .run();
}

#[test]
fn ch_8_6_05_explicit_key_with_quoted_string() {
    Fixture::new(
        "8.6",
        5,
        "Explicit key with quoted string",
        "? \"complex: key\"\n: value\n",
    )
    .run();
}

// =============================================================================
// 8.7 Complex Keys (Sequences and Mappings as Keys)
// =============================================================================

#[test]
fn ch_8_7_01_sequence_as_key() {
    Fixture::new(
        "8.7",
        1,
        "Block sequence as mapping key",
        "? - item1\n  - item2\n: value\n",
    )
    .run();
}

#[test]
fn ch_8_7_02_mapping_as_key() {
    Fixture::new(
        "8.7",
        2,
        "Block mapping as mapping key",
        "? key1: val1\n  key2: val2\n: outer value\n",
    )
    .run();
}

#[test]
fn ch_8_7_03_nested_sequence_as_key() {
    Fixture::new(
        "8.7",
        3,
        "Nested sequence as key",
        "? - - nested\n: value\n",
    )
    .run();
}

#[test]
fn ch_8_7_04_complex_key_complex_value() {
    Fixture::new(
        "8.7",
        4,
        "Complex key with complex value",
        "? - key_item\n:\n  - value_item\n",
    )
    .run();
}

#[test]
fn ch_8_7_05_flow_sequence_as_block_key() {
    Fixture::new(
        "8.7",
        5,
        "Flow sequence as block mapping key",
        "? [a, b, c]\n: value\n",
    )
    .run();
}

#[test]
fn ch_8_7_06_flow_mapping_as_block_key() {
    Fixture::new(
        "8.7",
        6,
        "Flow mapping as block mapping key",
        "? {x: 1, y: 2}\n: value\n",
    )
    .run();
}

// =============================================================================
// 8.8 Multiline Plain Scalar Keys
// =============================================================================

#[test]
fn ch_8_8_01_multiline_plain_scalar_key() {
    Fixture::new(
        "8.8",
        1,
        "Multiline plain scalar as key",
        "? multiline\n  plain\n  key\n: value\n",
    )
    .run();
}

#[test]
fn ch_8_8_02_multiline_plain_key_with_block_value() {
    Fixture::new(
        "8.8",
        2,
        "Multiline plain key with block value",
        "? long key\n  spanning lines\n:\n  - value1\n  - value2\n",
    )
    .run();
}

#[test]
fn ch_8_8_03_multiline_plain_key_folding() {
    Fixture::new(
        "8.8",
        3,
        "Multiline plain key with line folding",
        "? this is a\n  long key\n: value\n",
    )
    .run();
}

// =============================================================================
// 8.9 Block Scalar Edge Cases
// =============================================================================

#[test]
fn ch_8_9_01_literal_empty_content() {
    Fixture::new(
        "8.9",
        1,
        "Literal block scalar with empty content",
        "text: |\n",
    )
    .run();
}

#[test]
fn ch_8_9_02_folded_empty_content() {
    Fixture::new(
        "8.9",
        2,
        "Folded block scalar with empty content",
        "text: >\n",
    )
    .run();
}

#[test]
fn ch_8_9_03_literal_whitespace_only() {
    Fixture::new(
        "8.9",
        3,
        "Literal with whitespace-only lines",
        "text: |\n  \n  \n",
    )
    .run();
}

#[test]
fn ch_8_9_04_folded_whitespace_only() {
    Fixture::new(
        "8.9",
        4,
        "Folded with whitespace-only lines",
        "text: >\n  \n  \n",
    )
    .run();
}

#[test]
fn ch_8_9_05_literal_single_space_lines() {
    Fixture::new(
        "8.9",
        5,
        "Literal with single space on each line",
        "text: |\n   \n   \n   \n",
    )
    .run();
}

#[test]
fn ch_8_9_06_block_scalar_only_newlines() {
    Fixture::new(
        "8.9",
        6,
        "Block scalar with only newlines",
        "text: |\n\n\n\n",
    )
    .run();
}

#[test]
fn ch_8_9_07_literal_trailing_spaces() {
    Fixture::new(
        "8.9",
        7,
        "Literal preserves trailing spaces",
        "text: |\n  content   \n  more   \n",
    )
    .run();
}

#[test]
fn ch_8_9_08_folded_very_long_line() {
    // Very long line (500+ characters) in folded scalar
    Fixture::new(
        "8.9",
        8,
        "Folded with very long line",
        "text: >\n  This is a very long line that contains many words and continues for a long time to test how the parser handles extremely long lines in folded block scalars which should fold properly according to the YAML specification and not cause any issues with parsing or memory allocation even when the line is exceptionally long and exceeds typical line length limits that might be encountered in normal usage scenarios and this line should continue to be valid YAML despite its extreme length and should be folded appropriately when processed by the parser implementation.\n",
    )
    .run();
}

#[test]
fn ch_8_9_09_literal_mixed_empty_content_lines() {
    Fixture::new(
        "8.9",
        9,
        "Literal with mixed empty and content lines",
        "text: |\n  line1\n\n  line3\n  \n  line5\n",
    )
    .run();
}

#[test]
fn ch_8_9_10_block_scalar_single_char() {
    Fixture::new(
        "8.9",
        10,
        "Block scalar with single character",
        "text: |\n  x\n",
    )
    .run();
}
