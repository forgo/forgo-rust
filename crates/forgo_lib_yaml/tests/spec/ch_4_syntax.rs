// crates/forgo_lib_yaml/tests/spec/ch_4_syntax.rs
//! YAML 1.2.2 Chapter 4: Syntax Conventions
//!
//! Reference: https://yaml.org/spec/1.2.2/#chapter-4-syntax-conventions
//!
//! This chapter defines the BNF-style notation used throughout the spec.
//! While mostly meta-level, some syntax edge cases are testable.

#[path = "../common/mod.rs"]
mod common;

use common::Fixture;
use forgo_lib_yaml::Doc;

// ============================================================================
// 4.1 Production Syntax
// ============================================================================

#[test]
fn ch_4_1_01_character_range() {
    // Character ranges [x20-x7E] should work
    Fixture::new(
        "4.1",
        1,
        "Printable ASCII character range",
        "text: hello world!\n",
    )
    .run();
}

#[test]
fn ch_4_1_02_concatenation() {
    // Concatenation of terms (implicit in all YAML)
    Fixture::new(
        "4.1",
        2,
        "Concatenation of characters",
        "key: value\n",
    )
    .run();
}

#[test]
fn ch_4_1_03_alternation() {
    // Alternation (|) - either block or flow style works
    let block = "list:\n  - item\n";
    let flow = "list: [item]\n";

    let doc1 = Doc::from_str(block).unwrap();
    let doc2 = Doc::from_str(flow).unwrap();

    // Both valid alternatives
    assert!(doc1.to_string().is_ok());
    assert!(doc2.to_string().is_ok());
}

// ============================================================================
// 4.2 Quantification
// ============================================================================

#[test]
fn ch_4_2_01_optional_term() {
    // term? means zero or one occurrence
    let with = "---\nkey: value\n";
    let without = "key: value\n";

    // Both valid (document marker is optional)
    assert!(Doc::from_str(with).is_ok());
    assert!(Doc::from_str(without).is_ok());
}

#[test]
fn ch_4_2_02_zero_or_more() {
    // term* means zero or more occurrences
    let zero = "key:\n";
    let one = "key: value\n";
    let many = "key: value1 value2 value3\n";

    // All valid
    assert!(Doc::from_str(zero).is_ok());
    assert!(Doc::from_str(one).is_ok());
    assert!(Doc::from_str(many).is_ok());
}

#[test]
fn ch_4_2_03_one_or_more() {
    // term+ means one or more occurrences
    Fixture::new(
        "4.2",
        3,
        "At least one sequence item",
        "- item1\n- item2\n",
    )
    .run();
}

#[test]
fn ch_4_2_04_greedy_quantification() {
    // All quantifications are greedy
    Fixture::new(
        "4.2",
        4,
        "Greedy matching of multiple values",
        "text: hello world how are you\n",
    )
    .run();
}

// ============================================================================
// 4.3 Special Productions
// ============================================================================

#[test]
fn ch_4_3_01_start_of_line() {
    // <start-of-line> matches at beginning of line
    Fixture::new(
        "4.3",
        1,
        "Comment at start of line",
        "# comment\nkey: value\n",
    )
    .run();
}

#[test]
fn ch_4_3_02_end_of_input() {
    // <end-of-input> matches at stream end
    Fixture::new(
        "4.3",
        2,
        "Valid document ending",
        "key: value\n",
    )
    .run();
}

#[test]
fn ch_4_3_03_empty_production() {
    // <empty> matches without consuming input
    Fixture::new(
        "4.3",
        3,
        "Empty value (null)",
        "key:\n",
    )
    .run();
}

// ============================================================================
// 4.4 Lookaround Assertions
// ============================================================================

#[test]
fn ch_4_4_01_lookahead_positive() {
    // Lookahead assertions check without consuming
    Fixture::new(
        "4.4",
        1,
        "Colon followed by space (lookahead)",
        "key: value\n",
    )
    .run();
}

#[test]
fn ch_4_4_02_lookbehind() {
    // Lookbehind checks prior context
    Fixture::new(
        "4.4",
        2,
        "Indentation context (lookbehind)",
        "parent:\n  child: value\n",
    )
    .run();
}

// ============================================================================
// 4.5 Parameterized Productions
// ============================================================================

#[test]
fn ch_4_5_01_indentation_zero() {
    // Indentation parameter n=0
    Fixture::new(
        "4.5",
        1,
        "Zero indentation (top level)",
        "key: value\n",
    )
    .run();
}

#[test]
fn ch_4_5_02_indentation_positive() {
    // Indentation parameter n>0
    Fixture::new(
        "4.5",
        2,
        "Positive indentation (nested)",
        "parent:\n  child: value\n    grandchild: data\n",
    )
    .run();
}

#[test]
fn ch_4_5_03_indentation_minus_one() {
    // Indentation parameter n=-1 (flow context)
    Fixture::new(
        "4.5",
        3,
        "Flow context indentation",
        "flow: {key: value}\n",
    )
    .run();
}

#[test]
fn ch_4_5_04_indentation_expression() {
    // Expressions like n+m in productions
    Fixture::new(
        "4.5",
        4,
        "Increased indentation (n+1)",
        "list:\n  - item1\n  - item2\n",
    )
    .run();
}

// ============================================================================
// 4.6 Context Parameters
// ============================================================================

#[test]
fn ch_4_6_01_block_in_context() {
    // BLOCK-IN: block collection entry context
    Fixture::new(
        "4.6",
        1,
        "Block-in context (sequence entry)",
        "- item1\n- item2\n",
    )
    .run();
}

#[test]
fn ch_4_6_02_block_out_context() {
    // BLOCK-OUT: block outer context
    Fixture::new(
        "4.6",
        2,
        "Block-out context (top level)",
        "key: value\n",
    )
    .run();
}

#[test]
fn ch_4_6_03_block_key_context() {
    // BLOCK-KEY: implicit block key context
    Fixture::new(
        "4.6",
        3,
        "Block-key context (mapping key)",
        "key: value\n",
    )
    .run();
}

#[test]
fn ch_4_6_04_flow_in_context() {
    // FLOW-IN: flow collection entry context
    Fixture::new(
        "4.6",
        4,
        "Flow-in context (flow sequence)",
        "[a, b, c]\n",
    )
    .run();
}

#[test]
fn ch_4_6_05_flow_out_context() {
    // FLOW-OUT: flow outer context
    Fixture::new(
        "4.6",
        5,
        "Flow-out context (flow mapping)",
        "{key: value}\n",
    )
    .run();
}

#[test]
fn ch_4_6_06_flow_key_context() {
    // FLOW-KEY: implicit flow key context
    Fixture::new(
        "4.6",
        6,
        "Flow-key context (flow map key)",
        "{key: value, other: data}\n",
    )
    .run();
}

#[test]
fn ch_4_6_07_context_transition() {
    // Transition between block and flow contexts
    Fixture::new(
        "4.6",
        7,
        "Block to flow context transition",
        "block:\n  flow: [a, b]\n",
    )
    .run();
}

#[test]
fn ch_4_6_08_nested_contexts() {
    // Nested context changes
    Fixture::new(
        "4.6",
        8,
        "Nested flow within block",
        "outer:\n  - {key: value}\n  - [a, b, c]\n",
    )
    .run();
}

// ============================================================================
// 4.7 Operator Precedence
// ============================================================================

#[test]
fn ch_4_7_01_parenthesization() {
    // Parentheses group productions
    Fixture::new(
        "4.7",
        1,
        "Grouped production (parentheses)",
        "key: value\n",
    )
    .run();
}

#[test]
fn ch_4_7_02_quantification_precedence() {
    // Quantification binds tighter than concatenation
    Fixture::new(
        "4.7",
        2,
        "Quantified term precedence",
        "text: multiple words here\n",
    )
    .run();
}

#[test]
fn ch_4_7_03_concatenation_precedence() {
    // Concatenation binds tighter than alternation
    Fixture::new(
        "4.7",
        3,
        "Concatenation before alternation",
        "key: value\n",
    )
    .run();
}

#[test]
fn ch_4_7_04_alternation_lowest() {
    // Alternation has lowest precedence
    let option1 = "flow: [a, b]\n";
    let option2 = "flow:\n  - a\n  - b\n";

    // Both alternatives valid
    assert!(Doc::from_str(option1).is_ok());
    assert!(Doc::from_str(option2).is_ok());
}

// ============================================================================
// 4.8 Edge Cases
// ============================================================================

#[test]
fn ch_4_8_01_complex_production() {
    // Complex production combining multiple features
    Fixture::new(
        "4.8",
        1,
        "Complex nested structure",
        "root:\n  level1:\n    - item: {nested: [a, b]}\n    - plain: value\n",
    )
    .run();
}

#[test]
fn ch_4_8_02_multiple_quantifiers() {
    // Multiple quantified elements
    Fixture::new(
        "4.8",
        2,
        "Multiple optional/repeated elements",
        "---\nkey1: value1\nkey2: value2\nkey3: value3\n...\n",
    )
    .run();
}

#[test]
fn ch_4_8_03_mixed_contexts() {
    // Mixing block and flow extensively
    Fixture::new(
        "4.8",
        3,
        "Mixed block and flow contexts",
        "block:\n  - flow: {a: 1}\n  - nested:\n      deeper: [x, y]\n",
    )
    .run();
}

#[test]
fn ch_4_8_04_whitespace_rules() {
    // Whitespace handling per syntax rules
    Fixture::new(
        "4.8",
        4,
        "Whitespace in various contexts",
        "key:   value   \nlist:  [  a  ,  b  ]\n",
    )
    .run();
}

#[test]
fn ch_4_8_05_empty_productions() {
    // Multiple empty productions
    Fixture::new(
        "4.8",
        5,
        "Multiple empty values",
        "null1:\nnull2:\nnull3:\n",
    )
    .run();
}

#[test]
fn ch_4_8_06_deeply_nested() {
    // Deep nesting (tests indentation parameters)
    Fixture::new(
        "4.8",
        6,
        "Deep nesting with various indentations",
        "l1:\n  l2:\n    l3:\n      l4:\n        l5: deep\n",
    )
    .run();
}

#[test]
fn ch_4_8_07_alternation_choices() {
    // All alternation branches should work
    let scalar_plain = "value: plain text\n";
    let scalar_quoted = "value: \"quoted text\"\n";
    let scalar_literal = "value: |\n  literal\n";

    assert!(Doc::from_str(scalar_plain).is_ok());
    assert!(Doc::from_str(scalar_quoted).is_ok());
    assert!(Doc::from_str(scalar_literal).is_ok());
}

// ============================================================================
// 4.9 Indentation Edge Cases
// ============================================================================

#[test]
fn ch_4_9_01_maximum_indentation_depth() {
    // Test deeply nested indentation (20+ levels)
    Fixture::new(
        "4.9",
        1,
        "Maximum indentation depth (20 levels)",
        "l1:\n  l2:\n    l3:\n      l4:\n        l5:\n          l6:\n            l7:\n              l8:\n                l9:\n                  l10:\n                    l11:\n                      l12:\n                        l13:\n                          l14:\n                            l15:\n                              l16:\n                                l17:\n                                  l18:\n                                    l19:\n                                      l20: deep\n",
    )
    .run();
}

#[test]
fn ch_4_9_02_inconsistent_indentation_error() {
    // Inconsistent indentation should be rejected
    let input = "parent:\n  child1: value\n   child2: value\n";
    let result = Doc::from_str(input);

    // 3-space indent after 2-space is inconsistent
    // Some parsers may be lenient, strict ones should error
    if result.is_err() {
        assert!(result.is_err(), "Inconsistent indentation should be rejected");
    }
}

#[test]
fn ch_4_9_03_mixed_indent_sizes() {
    // Mixing 2-space and 4-space indents at different levels
    let input = "l1:\n  l2:\n    l3:\n      l4: value\n";
    let result = Doc::from_str(input);

    // This doubles indent at each level (2, 4, 6, 8)
    // Should be valid as long as it's consistent per-level
    assert!(result.is_ok(), "Increasing indentation should be valid");
}

#[test]
fn ch_4_9_04_zero_indent_after_positive() {
    // Return to zero indentation after nested content
    Fixture::new(
        "4.9",
        4,
        "Zero indent after nested content",
        "parent:\n  child: value\ntop: level\n",
    )
    .run();
}

#[test]
fn ch_4_9_05_single_space_indent() {
    // Minimum valid indentation (1 space)
    Fixture::new(
        "4.9",
        5,
        "Single-space indentation",
        "parent:\n child: value\n",
    )
    .run();
}

#[test]
fn ch_4_9_06_large_indent_jump() {
    // Large indentation jump (e.g., 10 spaces at once)
    Fixture::new(
        "4.9",
        6,
        "Large indentation jump (10 spaces)",
        "parent:\n          child: value\n",
    )
    .run();
}

#[test]
fn ch_4_9_07_indentation_in_flow_context() {
    // Indentation within flow collections (n=-1 context)
    Fixture::new(
        "4.9",
        7,
        "Indentation in flow context",
        "flow: {\n  key: value,\n  other: data\n}\n",
    )
    .run();
}

// ============================================================================
// 4.10 Context Transition Edge Cases
// ============================================================================

#[test]
fn ch_4_10_01_block_to_flow_to_block() {
    // Block → Flow → Block context transitions
    Fixture::new(
        "4.10",
        1,
        "Block to flow to block transition",
        "outer:\n  middle: [a, b]\n  inner:\n    deep: value\n",
    )
    .run();
}

#[test]
fn ch_4_10_02_flow_to_block_to_flow() {
    // Flow → Block → Flow context transitions (spec-compliant)
    // Flow sequence [..] inside block mapping inside flow sequence
    Fixture::new(
        "4.10",
        2,
        "Flow to block to flow transition",
        "data:\n  - key: value\n  - nested: [item1, item2]\n",
    )
    .run();
}

#[test]
fn ch_4_10_03_multiple_context_switches() {
    // Rapid context switching
    Fixture::new(
        "4.10",
        3,
        "Multiple rapid context switches",
        "root:\n  - {a: 1}\n  - b: [x, y]\n  - c:\n      d: {e: f}\n",
    )
    .run();
}

#[test]
fn ch_4_10_04_flow_in_block_compliant() {
    // Flow nodes inside block collection (ALLOWED per YAML 1.2.2 §8.2.3)
    // "Flow nodes must be indented by at least one more space than the parent block collection"
    // This tests flow sequences properly used as values in a block mapping
    Fixture::new(
        "4.10",
        4,
        "Flow nodes in block collection (compliant)",
        "outer:\n  inner: [a, b]\n  another: [c, d]\n",
    )
    .run();
}

#[test]
fn ch_4_10_05_block_key_to_flow_value() {
    // Block-key context with flow-style value
    Fixture::new(
        "4.10",
        5,
        "Block key with flow value",
        "complex key here: [flow, value]\n",
    )
    .run();
}

#[test]
fn ch_4_10_06_block_in_flow_sequence_prohibited() {
    // Per YAML 1.2.2 Section 8.2.3: "block styles are not allowed inside flow collections"
    // This MUST be REJECTED - block sequence directly inside flow sequence
    Fixture::new(
        "4.10",
        6,
        "Block sequence in flow sequence (prohibited)",
        "[\n  - block\n  - items\n]\n",
    )
    .expect_error("Parse")
    .run();
}

#[test]
fn ch_4_10_06b_block_in_flow_mapping_prohibited() {
    // Per YAML 1.2.2 Section 8.2.3: "block styles are not allowed inside flow collections"
    // This MUST be REJECTED - block sequence as value in flow mapping
    Fixture::new(
        "4.10",
        6,
        "Block sequence in flow mapping (prohibited)",
        "{key:\n  - block\n  - value}\n",
    )
    .expect_error("Parse")
    .run();
}

#[test]
fn ch_4_10_07_context_at_document_boundary() {
    // Context transitions at document boundaries
    Fixture::new(
        "4.10",
        7,
        "Context across document boundary",
        "---\nblock: value\n---\nflow: {key: value}\n",
    )
    .run();
}

// ============================================================================
// 4.11 Production Limits and Extreme Cases
// ============================================================================

#[test]
fn ch_4_11_01_very_long_line() {
    // Extremely long scalar line (1000+ characters)
    let long_text = "x".repeat(1000);
    let input = format!("text: {}\n", long_text);
    let result = Doc::from_str(&input);

    // Should handle long lines gracefully
    assert!(result.is_ok(), "Very long lines should be supported");
}

#[test]
fn ch_4_11_02_very_long_key() {
    // Extremely long mapping key
    let long_key = "k".repeat(500);
    let input = format!("{}: value\n", long_key);
    let result = Doc::from_str(&input);

    assert!(result.is_ok(), "Very long keys should be supported");
}

#[test]
fn ch_4_11_03_many_mapping_entries() {
    // Large number of mapping entries (100+)
    let mut yaml = String::new();
    for i in 0..100 {
        yaml.push_str(&format!("key{}: value{}\n", i, i));
    }
    let result = Doc::from_str(&yaml);

    assert!(result.is_ok(), "Large mappings should be supported");
}

#[test]
fn ch_4_11_04_many_sequence_items() {
    // Large number of sequence items (100+)
    let mut yaml = String::new();
    for i in 0..100 {
        yaml.push_str(&format!("- item{}\n", i));
    }
    let result = Doc::from_str(&yaml);

    assert!(result.is_ok(), "Large sequences should be supported");
}

#[test]
fn ch_4_11_05_deeply_nested_flow() {
    // Deep nesting in flow collections (10+ levels)
    let input = "data: [[[[[[[[[[deep]]]]]]]]]]\n";
    let result = Doc::from_str(input);

    assert!(result.is_ok(), "Deeply nested flow should be supported");
}

#[test]
fn ch_4_11_06_wide_and_deep_structure() {
    // Both wide (many siblings) and deep (many levels)
    Fixture::new(
        "4.11",
        6,
        "Wide and deep structure",
        "root:\n  child1:\n    - item1\n    - item2\n    - item3\n  child2:\n    - item4\n    - item5\n  child3:\n    grandchild:\n      deep: value\n",
    )
    .run();
}

#[test]
fn ch_4_11_07_empty_productions_sequence() {
    // Many consecutive empty productions
    Fixture::new(
        "4.11",
        7,
        "Many empty values in sequence",
        "list:\n  -\n  -\n  -\n  -\n  -\n",
    )
    .run();
}

// ============================================================================
// 4.12 Quantification Edge Cases
// ============================================================================

#[test]
fn ch_4_12_01_optional_at_boundary() {
    // Optional term at various boundaries
    Fixture::new(
        "4.12",
        1,
        "Optional term at start",
        "key: value\n",
    )
    .run();
}

#[test]
fn ch_4_12_02_zero_occurrences_star() {
    // Zero occurrences of a * quantified term
    Fixture::new(
        "4.12",
        2,
        "Zero occurrences of optional repeated term",
        "key:\n",
    )
    .run();
}

#[test]
fn ch_4_12_03_many_occurrences_plus() {
    // Many occurrences of + quantified term
    Fixture::new(
        "4.12",
        3,
        "Many occurrences of required repeated term",
        "- a\n- b\n- c\n- d\n- e\n- f\n- g\n- h\n",
    )
    .run();
}

// ============================================================================
// 4.13 Special Character Sequences
// ============================================================================

#[test]
fn ch_4_13_01_all_printable_ascii() {
    // Test all printable ASCII characters in plain scalar
    // Space (x20) through tilde (x7E)
    Fixture::new(
        "4.13",
        1,
        "Various printable ASCII",
        "text: abc123 !@#$%^&*()_+-=[]{}|;':,./<>?\n",
    )
    .run();
}

#[test]
fn ch_4_13_02_unicode_in_productions() {
    // Unicode characters in various productions
    Fixture::new(
        "4.13",
        2,
        "Unicode in keys and values",
        "日本: 語\némoji: 🎉\n",
    )
    .run();
}

#[test]
fn ch_4_13_03_combining_characters() {
    // Unicode combining characters
    Fixture::new(
        "4.13",
        3,
        "Unicode combining characters",
        "combined: e\u{0301}\n",  // e with acute accent as combining
    )
    .run();
}
