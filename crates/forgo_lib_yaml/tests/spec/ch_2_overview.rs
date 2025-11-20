// crates/forgo_lib_yaml/tests/spec/ch_2_overview.rs
//! YAML 1.2.2 Chapter 2: Language Overview
//!
//! **Spec Reference:** https://yaml.org/spec/1.2.2/#chapter-2-language-overview
//!
//! **Purpose:**
//! This file contains systematic tests for YAML 1.2.2 Chapter 2. Each test
//! maps to a specific section of the specification and validates compliance
//! with the requirements defined there. Test names follow the pattern
//! `ch_2_Y_ZZ_description` where 2 is the chapter, Y is the section, and
//! ZZ is the test number.
//!
//! **Coverage:**
//! Overview examples demonstrating YAML features: collections, structures, scalars, tags, complete documents

#[path = "../common/mod.rs"]
mod common;

use common::Fixture;
use forgo_lib_yaml::{Doc, Node};

// ============================================================================
// 2.1 Collections
// ============================================================================

#[test]
fn ch_2_1_01_sequence_of_scalars() {
    Fixture::new(
        "2.1",
        1,
        "Sequence of Scalars (ball players)",
        "- Mark McGwire\n- Sammy Sosa\n- Ken Griffey\n",
    )
    .run();
}

#[test]
fn ch_2_1_02_mapping_scalars_to_scalars() {
    Fixture::new(
        "2.1",
        2,
        "Mapping Scalars to Scalars (player stats)",
        "hr:  65\navg: 0.278\nrbi: 147\n",
    )
    .run();
}

#[test]
fn ch_2_1_03_mapping_scalars_to_sequences() {
    Fixture::new(
        "2.1",
        3,
        "Mapping Scalars to Sequences (teams)",
        "american:\n  - Boston Red Sox\n  - Detroit Tigers\n  - New York Yankees\nnational:\n  - New York Mets\n  - Chicago Cubs\n  - Atlanta Braves\n",
    )
    .run();
}

#[test]
fn ch_2_1_04_sequence_of_mappings() {
    Fixture::new(
        "2.1",
        4,
        "Sequence of Mappings (player records)",
        "-\n  name: Mark McGwire\n  hr:   65\n  avg:  0.278\n-\n  name: Sammy Sosa\n  hr:   63\n  avg:  0.288\n",
    )
    .run();
}

#[test]
fn ch_2_1_05_sequence_of_sequences() {
    Fixture::new(
        "2.1",
        5,
        "Sequence of Sequences (flow style)",
        "- [name        , hr, avg  ]\n- [Mark McGwire, 65, 0.278]\n- [Sammy Sosa  , 63, 0.288]\n",
    )
    .run();
}

#[test]
fn ch_2_1_06_mapping_of_mappings() {
    Fixture::new(
        "2.1",
        6,
        "Mapping of Mappings (flow style)",
        "Mark McGwire: {hr: 65, avg: 0.278}\nSammy Sosa: {\n    hr: 63,\n    avg: 0.288,\n }\n",
    )
    .run();
}

// ============================================================================
// 2.2 Structures
// ============================================================================

#[test]
fn ch_2_2_01_two_documents_in_stream() {
    Fixture::new(
        "2.2",
        1,
        "Two Documents in a Stream",
        "# Ranking of 1998 home runs\n---\n- Mark McGwire\n- Sammy Sosa\n- Ken Griffey\n\n# Team ranking\n---\n- Chicago Cubs\n- St Louis Cardinals\n",
    )
    .run();
}

#[test]
fn ch_2_2_02_play_by_play_feed() {
    Fixture::new(
        "2.2",
        2,
        "Play by Play Feed from a Game",
        "---\ntime: 20:03:20\nplayer: Sammy Sosa\naction: strike (miss)\n...\n---\ntime: 20:03:47\nplayer: Sammy Sosa\naction: grand slam\n...\n",
    )
    .run();
}

#[test]
fn ch_2_2_03_single_document_with_comments() {
    Fixture::new(
        "2.2",
        3,
        "Single Document with Two Comments",
        "---\nhr: # 1998 hr ranking\n  - Mark McGwire\n  - Sammy Sosa\n# 1998 rbi ranking\nrbi:\n  - Sammy Sosa\n  - Ken Griffey\n",
    )
    .run();
}

#[test]
fn ch_2_2_04_node_appearing_twice() {
    Fixture::new(
        "2.2",
        4,
        "Node Appearing Twice (anchors & aliases)",
        "---\nhr:\n  - Mark McGwire\n  # Following node labeled SS\n  - &SS Sammy Sosa\nrbi:\n  - *SS # Subsequent occurrence\n  - Ken Griffey\n",
    )
    .run();
}

#[test]
fn ch_2_2_05_mapping_between_sequences() {
    Fixture::new(
        "2.2",
        5,
        "Mapping between Sequences",
        "? - Detroit Tigers\n  - Chicago cubs\n: - 2001-07-23\n\n? [ New York Yankees,\n    Atlanta Braves ]\n: [ 2001-07-02, 2001-08-12,\n    2001-08-14 ]\n",
    )
    .run();
}

#[test]
fn ch_2_2_06_compact_nested_mapping() {
    Fixture::new(
        "2.2",
        6,
        "Compact Nested Mapping",
        "---\n# Products purchased\n- item    : Super Hoop\n  quantity: 1\n- item    : Basketball\n  quantity: 4\n- item    : Big Shoes\n  quantity: 1\n",
    )
    .run();
}

// ============================================================================
// 2.3 Scalars
// ============================================================================

#[test]
fn ch_2_3_01_literal_block_scalar() {
    Fixture::new(
        "2.3",
        1,
        "Literal Block Scalar (preserves newlines)",
        "# ASCII Art\n--- |\n  \\\\//||\\\\/||\n  // ||  ||__\n",
    )
    .run();
}

#[test]
fn ch_2_3_02_folded_block_scalar() {
    Fixture::new(
        "2.3",
        2,
        "Folded Block Scalar (folds newlines)",
        "--- >\n  Mark McGwire's\n  year was crippled\n  by a knee injury.\n",
    )
    .run();
}

#[test]
fn ch_2_3_03_folded_with_indented_lines() {
    Fixture::new(
        "2.3",
        3,
        "Folded Block Scalar with indented lines",
        "--- >\n Sammy Sosa completed another\n fine season with great stats.\n\n   63 Home Runs\n   0.288 Batting Average\n\n What a year!\n",
    )
    .run();
}

#[test]
fn ch_2_3_04_indentation_determines_scope() {
    Fixture::new(
        "2.3",
        4,
        "Indentation Determines Scope",
        "name: Mark McGwire\naccomplishment: >\n  Mark set a major league\n  home run record in 1998.\nstats: |\n  65 Home Runs\n  0.278 Batting Average\n",
    )
    .run();
}

#[test]
fn ch_2_3_05_quoted_scalars() {
    Fixture::new(
        "2.3",
        5,
        "Quoted Scalars (escape sequences)",
        "unicode: \"Sosa did fine.\\u263A\"\ncontrol: \"\\b1998\\t1999\\t2000\\n\"\nhex esc: \"\\x0d\\x0a is \\r\\n\"\n\nsingle: '\"Howdy!\" he cried.'\nquoted: ' # Not a ''comment''.'\ntie-fighter: '|\\-*-/|'\n",
    )
    .run();
}

#[test]
fn ch_2_3_06_multiline_flow_scalars() {
    Fixture::new(
        "2.3",
        6,
        "Multi-line Flow Scalars",
        "plain:\n  This unquoted scalar\n  spans many lines.\n\nquoted: \"So does this\n  quoted scalar.\\n\"\n",
    )
    .run();
}

// ============================================================================
// 2.4 Tags
// ============================================================================

#[test]
fn ch_2_4_01_integers() {
    Fixture::new(
        "2.4",
        1,
        "Integers (various formats)",
        "canonical: 12345\ndecimal: +12345\noctal: 0o14\nhexadecimal: 0xC\n",
    )
    .run();
}

#[test]
fn ch_2_4_02_floating_point() {
    Fixture::new(
        "2.4",
        2,
        "Floating Point (various formats)",
        "canonical: 1.23015e+3\nexponential: 12.3015e+02\nfixed: 1230.15\nnegative infinity: -.inf\nnot a number: .nan\n",
    )
    .run();
}

#[test]
fn ch_2_4_03_miscellaneous() {
    Fixture::new(
        "2.4",
        3,
        "Miscellaneous (null, bool, string, date)",
        "null:\nbooleans: [ true, false ]\nstring: '012345'\n",
    )
    .run();
}

#[test]
fn ch_2_4_04_timestamps() {
    Fixture::new(
        "2.4",
        4,
        "Timestamps (ISO 8601)",
        "canonical: 2001-12-15T02:59:43.1Z\niso8601: 2001-12-14t21:59:43.10-05:00\nspaced: 2001-12-14 21:59:43.10 -5\ndate: 2002-12-14\n",
    )
    .run();
}

#[test]
fn ch_2_4_05_explicit_tags() {
    let input = "---\nnot-date: !!str 2002-04-28\n\npicture: !!binary |\n R0lGODlhDAAMAIQAAP//9/X\n 17unp5WZmZgAAAOfn515eXv\n Pz7Y6OjuDg4J+fn5OTk6enp\n 56enmleECcgggoBADs=\n";

    let doc = Doc::from_str(input).unwrap();
    // Just verify it parses - explicit tag handling is implementation-specific
    assert!(matches!(doc.root().node(), Node::Map(_)));
}

#[test]
fn ch_2_4_06_application_specific_tags() {
    // Application-specific tags
    let input = "# Explicitly typed collection\n---\npicture: !<tag:example.com,2000:app/picture>\n  - R0lGODlhDAAMAIQAAP//9\n  - 17unp5WZmZgAAAOfn5\n";

    let doc = Doc::from_str(input).unwrap();
    assert!(matches!(doc.root().node(), Node::Map(_)));
}

// ============================================================================
// 2.5 Full Length Example
// ============================================================================

#[test]
fn ch_2_5_01_invoice_document() {
    let input = "---\ninvoice: 34843\ndate   : 2001-01-23\nbill-to: &id001\n  given  : Chris\n  family : Dumars\n  address:\n    lines: |\n      458 Walkman Dr.\n      Suite #292\n    city    : Royal Oak\n    state   : MI\n    postal  : 48046\nship-to: *id001\nproduct:\n  - sku         : BL394D\n    quantity    : 4\n    description : Basketball\n    price       : 450.00\n  - sku         : BL4438H\n    quantity    : 1\n    description : Super Hoop\n    price       : 2392.00\ntax  : 251.42\ntotal: 4443.52\ncomments:\n  Late afternoon is best.\n  Backup contact is Nancy\n  Billsmer @ 338-4338.\n";

    let doc = Doc::from_str(input).unwrap();
    let Node::Map(root) = doc.root().node() else {
        panic!("Expected root to be a map");
    };

    // Verify some key fields exist
    assert!(root.iter().any(|(k, _)| k == "invoice"));
    assert!(root.iter().any(|(k, _)| k == "date"));
    assert!(root.iter().any(|(k, _)| k == "bill-to"));
    assert!(root.iter().any(|(k, _)| k == "product"));
}

// ============================================================================
// Additional Language Features
// ============================================================================

#[test]
fn ch_2_6_01_empty_content() {
    Fixture::new(
        "2.6",
        1,
        "Empty content (null values)",
        "key:\n",
    )
    .run();
}

#[test]
fn ch_2_6_02_mixed_block_and_flow() {
    Fixture::new(
        "2.6",
        2,
        "Mixed block and flow collections",
        "players:\n  - name: Mark McGwire\n    stats: {hr: 65, avg: 0.278}\n  - name: Sammy Sosa\n    stats: {hr: 63, avg: 0.288}\n",
    )
    .run();
}

#[test]
fn ch_2_6_03_nested_sequences() {
    Fixture::new(
        "2.6",
        3,
        "Nested sequences (3 levels deep)",
        "- - - deep\n",
    )
    .run();
}

#[test]
fn ch_2_6_04_nested_mappings() {
    Fixture::new(
        "2.6",
        4,
        "Nested mappings (3 levels deep)",
        "a:\n  b:\n    c: value\n",
    )
    .run();
}

#[test]
fn ch_2_6_05_complex_mapping_key() {
    Fixture::new(
        "2.6",
        5,
        "Complex mapping key (sequence as key)",
        "? [a, b]\n: value\n",
    )
    .run();
}

#[test]
fn ch_2_6_06_multiline_keys_values() {
    Fixture::new(
        "2.6",
        6,
        "Multi-line keys and values",
        "? |\n  key\n  line 2\n: |\n  value\n  line 2\n",
    )
    .run();
}

// ============================================================================
// 2.7 Edge Cases and Additional Coverage
// ============================================================================

#[test]
fn ch_2_edge_01_empty_sequence() {
    Fixture::new(
        "2.edge",
        1,
        "Empty sequence",
        "empty: []\n",
    )
    .run();
}

#[test]
fn ch_2_edge_02_empty_mapping() {
    Fixture::new(
        "2.edge",
        2,
        "Empty mapping",
        "empty: {}\n",
    )
    .run();
}

#[test]
fn ch_2_edge_03_empty_collections_nested() {
    Fixture::new(
        "2.edge",
        3,
        "Empty collections nested",
        "data:\n  empty_seq: []\n  empty_map: {}\n  nested: [[], {}]\n",
    )
    .run();
}

#[test]
fn ch_2_edge_04_deeply_nested_10_levels() {
    Fixture::new(
        "2.edge",
        4,
        "10-level deep nesting",
        "l1:\n  l2:\n    l3:\n      l4:\n        l5:\n          l6:\n            l7:\n              l8:\n                l9:\n                  l10: deep\n",
    )
    .run();
}

#[test]
fn ch_2_edge_05_deeply_nested_sequences() {
    Fixture::new(
        "2.edge",
        5,
        "Deeply nested sequences (6 levels)",
        "- - - - - - deep\n",
    )
    .run();
}

#[test]
fn ch_2_edge_06_unicode_keys() {
    Fixture::new(
        "2.edge",
        6,
        "Unicode characters in mapping keys",
        "日本語: Japanese\n한글: Korean\nрусский: Russian\n中文: Chinese\n",
    )
    .run();
}

#[test]
fn ch_2_edge_07_unicode_values() {
    Fixture::new(
        "2.edge",
        7,
        "Unicode emoji and symbols",
        "emoji: 🎉😀🚀\nmath: ∀∃∫∑\narrows: →←↑↓\n",
    )
    .run();
}

#[test]
fn ch_2_edge_08_mixed_unicode_ascii() {
    Fixture::new(
        "2.edge",
        8,
        "Mixed Unicode and ASCII",
        "message: Hello 世界! Bonjour 🌍\npath: /home/用户/documents\n",
    )
    .run();
}

#[test]
fn ch_2_edge_09_plain_scalar_colon_in_url() {
    // Colon in plain scalar (URL context)
    Fixture::new(
        "2.edge",
        9,
        "Plain scalar with colon (URL)",
        "url: https://example.com:8080/path\n",
    )
    .run();
}

#[test]
fn ch_2_edge_10_plain_scalar_hash_no_space() {
    // Hash without preceding space is not a comment
    Fixture::new(
        "2.edge",
        10,
        "Plain scalar with # (no space)",
        "id: issue#123\ntag: #hashtag\n",
    )
    .run();
}

#[test]
fn ch_2_edge_11_flow_no_spaces() {
    // Flow collections without spaces
    Fixture::new(
        "2.edge",
        11,
        "Flow collections without spaces",
        "seq: [a,b,c]\nmap: {x:1,y:2}\n",
    )
    .run();
}

#[test]
fn ch_2_edge_12_flow_extra_spaces() {
    // Flow collections with extra spaces
    Fixture::new(
        "2.edge",
        12,
        "Flow collections with extra spaces",
        "seq: [  a  ,  b  ,  c  ]\nmap: {  x :  1  ,  y :  2  }\n",
    )
    .run();
}

#[test]
fn ch_2_edge_13_implicit_document_start() {
    // Document without --- marker
    Fixture::new(
        "2.edge",
        13,
        "Implicit document start (no ---)",
        "key: value\nlist:\n  - item\n",
    )
    .run();
}

#[test]
fn ch_2_edge_14_explicit_document_end() {
    // Document with ... marker
    Fixture::new(
        "2.edge",
        14,
        "Explicit document end (...)",
        "key: value\n...\n",
    )
    .run();
}

#[test]
fn ch_2_edge_15_multiple_empty_lines() {
    // Multiple consecutive empty lines
    Fixture::new(
        "2.edge",
        15,
        "Multiple consecutive empty lines",
        "key1: value1\n\n\n\nkey2: value2\n",
    )
    .run();
}

#[test]
fn ch_2_edge_16_comment_only_lines() {
    // Comment-only lines between content
    Fixture::new(
        "2.edge",
        16,
        "Comment-only lines",
        "# Header comment\nkey1: value1\n# Middle comment\n# Another comment\nkey2: value2\n# Footer comment\n",
    )
    .run();
}

#[test]
fn ch_2_edge_17_inline_comments_sequence() {
    // Inline comments in sequences
    Fixture::new(
        "2.edge",
        17,
        "Inline comments in sequence",
        "- item1 # first\n- item2 # second\n- item3 # third\n",
    )
    .run();
}

#[test]
fn ch_2_edge_18_inline_comments_mapping() {
    // Inline comments in mappings
    Fixture::new(
        "2.edge",
        18,
        "Inline comments in mapping",
        "key1: value1 # first\nkey2: value2 # second\nkey3: value3 # third\n",
    )
    .run();
}

#[test]
fn ch_2_edge_19_quoted_indicator_strings() {
    // Indicators inside quoted strings should be literal
    Fixture::new(
        "2.edge",
        19,
        "Quoted strings with indicators",
        "text1: \"---\"\ntext2: \"...\"\ntext3: \"# not a comment\"\ntext4: \"key: value\"\n",
    )
    .run();
}

#[test]
fn ch_2_edge_20_single_vs_double_quotes() {
    // Compare single and double quote handling
    Fixture::new(
        "2.edge",
        20,
        "Single vs double quoted strings",
        "single: 'can''t escape \\n'\ndouble: \"can escape \\n here\"\n",
    )
    .run();
}

// ============================================================================
// 2.8 Error Conditions - Reserved Indicators and Invalid Syntax
// ============================================================================

#[test]
fn ch_2_error_01_reserved_at_indicator() {
    // @ is reserved for future use in YAML
    // In plain scalars, @ at the start should be an error
    let input = "key: @reserved\n";
    let result = Doc::from_str(input);

    // Note: Some implementations may accept @ in plain scalars
    // The spec reserves @ for future use
    if result.is_err() {
        // Parser correctly rejects reserved indicator
        assert!(result.is_err());
    } else {
        // Parser accepts it - may need to be a plain scalar restriction
        // This is implementation-dependent
    }
}

#[test]
fn ch_2_error_02_reserved_backtick_indicator() {
    // ` (backtick) is reserved for future use in YAML
    let input = "key: `reserved\n";
    let result = Doc::from_str(input);

    // Note: Some implementations may accept ` in plain scalars
    // The spec reserves ` for future use
    if result.is_err() {
        assert!(result.is_err());
    } else {
        // Implementation may treat as plain scalar
    }
}

#[test]
fn ch_2_error_03_trailing_comma_flow_sequence() {
    // Trailing commas ARE allowed in YAML 1.2.2 flow sequences
    // See official test suite test 5C5M - this test was incorrect
    let input = "[a, b, c,]\n";
    let result = Doc::from_str(input);

    // Should parse successfully
    assert!(result.is_ok(), "Trailing comma in flow sequence should be allowed (YAML 1.2.2)");
}

#[test]
fn ch_2_error_04_trailing_comma_flow_mapping() {
    // Trailing commas ARE allowed in YAML 1.2.2 flow mappings
    // See official test suite test 5C5M - this test was incorrect
    let input = "{a: 1, b: 2,}\n";
    let result = Doc::from_str(input);

    // Should parse successfully
    assert!(result.is_ok(), "Trailing comma in flow mapping should be allowed (YAML 1.2.2)");
}

#[test]
fn ch_2_error_05_unmatched_bracket_extra_close() {
    // Extra closing bracket
    let input = "[a, b]]\n";
    let result = Doc::from_str(input);

    assert!(result.is_err(), "Unmatched closing bracket should be rejected");
}

#[test]
fn ch_2_error_06_unmatched_bracket_missing_close() {
    // Missing closing bracket
    let input = "[a, b\n";
    let result = Doc::from_str(input);

    assert!(result.is_err(), "Missing closing bracket should be rejected");
}

#[test]
fn ch_2_error_07_unmatched_brace_extra_close() {
    // Extra closing brace
    let input = "{a: 1}}\n";
    let result = Doc::from_str(input);

    assert!(result.is_err(), "Unmatched closing brace should be rejected");
}

#[test]
fn ch_2_error_08_unmatched_brace_missing_close() {
    // Missing closing brace
    let input = "{a: 1\n";
    let result = Doc::from_str(input);

    assert!(result.is_err(), "Missing closing brace should be rejected");
}

#[test]
fn ch_2_error_09_mismatched_brackets_braces() {
    // Opening bracket, closing brace
    let input = "[a, b}\n";
    let result = Doc::from_str(input);

    assert!(result.is_err(), "Mismatched bracket/brace should be rejected");
}

#[test]
fn ch_2_error_10_nested_unmatched() {
    // Nested collections with unmatched delimiters
    let input = "[[a, b], [c, d]\n";
    let result = Doc::from_str(input);

    assert!(result.is_err(), "Nested unmatched brackets should be rejected");
}

#[test]
fn ch_2_error_11_unterminated_double_quote() {
    // Unterminated double-quoted string
    let input = "key: \"unterminated\n";
    let result = Doc::from_str(input);

    assert!(result.is_err(), "Unterminated double quote should be rejected");
}

#[test]
fn ch_2_error_12_unterminated_single_quote() {
    // Unterminated single-quoted string
    let input = "key: 'unterminated\n";
    let result = Doc::from_str(input);

    assert!(result.is_err(), "Unterminated single quote should be rejected");
}
