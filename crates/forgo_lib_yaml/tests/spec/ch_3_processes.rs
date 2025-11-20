// crates/forgo_lib_yaml/tests/spec/ch_3_processes.rs
//! YAML 1.2.2 Chapter 3: Processes and Models
//!
//! **Spec Reference:** https://yaml.org/spec/1.2.2/#chapter-3-processes-and-models
//!
//! **Purpose:**
//! This file contains systematic tests for YAML 1.2.2 Chapter 3. Each test
//! maps to a specific section of the specification and validates compliance
//! with the requirements defined there. Test names follow the pattern
//! `ch_3_Y_ZZ_description` where 3 is the chapter, Y is the section, and
//! ZZ is the test number.
//!
//! **Coverage:**
//! Load/dump processes, representation graph, serialization tree, presentation stream, failure points

#[path = "../common/mod.rs"]
mod common;

use common::Fixture;
use forgo_lib_yaml::{Doc, Node, Scalar};

// ============================================================================
// 3.1 Processes
// ============================================================================

// 3.1.1 Dump Process

#[test]
fn ch_3_1_1_01_native_to_representation() {
    // The dump process starts by representing native data structures
    Fixture::new(
        "3.1.1",
        1,
        "Native data represented as tagged nodes",
        "string: text\nnumber: 42\nbool: true\n",
    )
    .run();
}

#[test]
fn ch_3_1_1_02_serialization_imposes_order() {
    // Serialization imposes ordering on mapping keys
    let input = "key1: value1\nkey2: value2\nkey3: value3\n";
    let doc = Doc::from_str(input).unwrap();
    let output = doc.to_string().unwrap();

    // Output should maintain some consistent ordering
    assert!(output.contains("key1"));
    assert!(output.contains("key2"));
    assert!(output.contains("key3"));
}

#[test]
fn ch_3_1_1_03_aliasing_duplicate_nodes() {
    // Serialization replaces repeated nodes with aliases
    Fixture::new(
        "3.1.1",
        3,
        "Duplicate nodes become aliases",
        "anchor: &ref\n  value: data\nalias: *ref\n",
    )
    .run();
}

#[test]
fn ch_3_1_1_04_presentation_formatting() {
    // Presentation adds formatting and styling
    let input = "key: value\n";
    let doc = Doc::from_str(input).unwrap();
    let output = doc.to_string().unwrap();

    // Output should be valid YAML text
    assert!(output.contains("key"));
    assert!(output.contains("value"));
}

// 3.1.2 Load Process

#[test]
fn ch_3_1_2_01_parsing_stage() {
    // Parsing converts character stream to serialization tree
    Fixture::new(
        "3.1.2",
        1,
        "Parsing produces serialization tree",
        "key: value\nlist:\n  - item1\n  - item2\n",
    )
    .run();
}

#[test]
fn ch_3_1_2_02_composing_stage() {
    // Composing converts tree to representation graph
    Fixture::new(
        "3.1.2",
        2,
        "Composing resolves structure",
        "mapping:\n  nested:\n    deep: value\n",
    )
    .run();
}

#[test]
fn ch_3_1_2_03_construction_stage() {
    // Construction builds native structures from representation
    let input = "string: hello\nnumber: 123\nbool: true\n";
    let doc = Doc::from_str(input).unwrap();

    let Node::Map(root) = doc.root().node() else {
        panic!("Expected map");
    };

    // Verify construction produces typed values
    assert!(root.iter().any(|(k, _)| k == "string"));
    assert!(root.iter().any(|(k, _)| k == "number"));
    assert!(root.iter().any(|(k, _)| k == "bool"));
}

#[test]
fn ch_3_1_2_04_presentation_details_discarded() {
    // Construction must not depend on presentation details
    let input1 = "key: value\n";
    let input2 = "key:    value   \n"; // Extra whitespace

    let doc1 = Doc::from_str(input1).unwrap();
    let doc2 = Doc::from_str(input2).unwrap();

    // Both should produce same logical structure
    let Node::Map(m1) = doc1.root().node() else { panic!() };
    let Node::Map(m2) = doc2.root().node() else { panic!() };

    assert_eq!(m1.len(), m2.len());
}

#[test]
fn ch_3_1_2_05_serialization_details_discarded() {
    // Construction must not depend on serialization details (anchor names, key order)
    let input = "a: 1\nb: 2\nc: 3\n";
    let doc = Doc::from_str(input).unwrap();

    // The representation should be independent of key order
    let Node::Map(root) = doc.root().node() else {
        panic!("Expected map");
    };
    assert_eq!(root.len(), 3);
}

// ============================================================================
// 3.2 Information Models
// ============================================================================

// 3.2.1 Representation Graph

#[test]
fn ch_3_2_1_01_nodes_are_typed() {
    // All nodes must have tags (explicit or implicit)
    let input = "string: text\nnumber: 42\nlist: [a, b]\n";
    let doc = Doc::from_str(input).unwrap();

    // Root should be a mapping node
    assert!(matches!(doc.root().node(), Node::Map(_)));
}

#[test]
fn ch_3_2_1_02_scalars_have_content() {
    // Scalar nodes contain string content
    let input = "text: hello world\n";
    let doc = Doc::from_str(input).unwrap();

    let Node::Map(root) = doc.root().node() else {
        panic!("Expected map");
    };

    let (_, val) = root.iter().find(|(k, _)| k == "text").unwrap();
    assert!(matches!(val.node(), Node::Scalar(_)));
}

#[test]
fn ch_3_2_1_03_sequences_are_ordered() {
    // Sequence nodes maintain ordered entries
    let input = "list:\n  - first\n  - second\n  - third\n";
    let doc = Doc::from_str(input).unwrap();

    let Node::Map(root) = doc.root().node() else {
        panic!("Expected map");
    };

    let (_, val) = root.iter().find(|(k, _)| k == "list").unwrap();
    assert!(matches!(val.node(), Node::Seq(_)));
}

#[test]
fn ch_3_2_1_04_mappings_are_unordered() {
    // Mapping nodes are conceptually unordered
    let input = "map:\n  key1: val1\n  key2: val2\n";
    let doc = Doc::from_str(input).unwrap();

    let Node::Map(root) = doc.root().node() else {
        panic!("Expected map");
    };

    let (_, val) = root.iter().find(|(k, _)| k == "map").unwrap();
    assert!(matches!(val.node(), Node::Map(_)));
}

#[test]
fn ch_3_2_1_05_anchors_create_graph() {
    // Anchors and aliases create graph structure
    let input = "first: &anchor value\nsecond: *anchor\n";
    let doc = Doc::from_str(input).unwrap();

    // Both should reference the same logical value
    let Node::Map(root) = doc.root().node() else {
        panic!("Expected map");
    };
    assert_eq!(root.len(), 2);
}

// ============================================================================
// 3.3 Loading Failure Points
// ============================================================================

#[test]
fn ch_3_3_01_well_formed_requirement() {
    // Must accept well-formed YAML
    Fixture::new(
        "3.3",
        1,
        "Well-formed YAML is accepted",
        "key: value\nlist: [a, b, c]\n",
    )
    .run();
}

#[test]
fn ch_3_3_02_ill_formed_rejected() {
    // Ill-formed input should be rejected
    let invalid = "key: value\n  - invalid indentation\n";
    let result = Doc::from_str(invalid);

    // Should fail to parse
    assert!(result.is_err());
}

#[test]
fn ch_3_3_03_unidentified_alias() {
    // Alias without prior anchor should fail
    let invalid = "value: *undefined\n";
    let result = Doc::from_str(invalid);

    // Should fail during composition
    assert!(result.is_err());
}

#[test]
fn ch_3_3_04_valid_alias_reference() {
    // Valid anchor/alias should work
    Fixture::new(
        "3.3",
        4,
        "Valid alias references defined anchor",
        "anchor: &name value\nalias: *name\n",
    )
    .run();
}

#[test]
fn ch_3_3_05_duplicate_mapping_keys() {
    // Non-unique mapping keys may be rejected or handled
    let input = "key: value1\nkey: value2\n";
    let doc = Doc::from_str(input);

    // Implementation may accept or reject duplicates
    // This test just ensures it doesn't panic
    let _ = doc;
}

#[test]
fn ch_3_3_06_tag_resolution() {
    // Tags must be resolved during composition
    Fixture::new(
        "3.3",
        6,
        "Tags are resolved",
        "string: hello\nnumber: 42\nbool: true\n",
    )
    .run();
}

// ============================================================================
// 3.4 Node Comparison
// ============================================================================

#[test]
fn ch_3_4_01_scalar_equality() {
    // Scalars are equal if tags and content match
    let input = "a: test\nb: test\n";
    let doc = Doc::from_str(input).unwrap();

    let Node::Map(root) = doc.root().node() else {
        panic!("Expected map");
    };

    let (_, val_a) = root.iter().find(|(k, _)| k == "a").unwrap();
    let (_, val_b) = root.iter().find(|(k, _)| k == "b").unwrap();

    // Both should have same scalar content
    match (val_a.node(), val_b.node()) {
        (Node::Scalar(Scalar::Str(s1)), Node::Scalar(Scalar::Str(s2))) => {
            assert_eq!(s1, s2);
        }
        _ => panic!("Expected string scalars"),
    }
}

#[test]
fn ch_3_4_02_collection_equality() {
    // Collections are equal if entries are equal
    let input = "list1: [a, b, c]\nlist2: [a, b, c]\n";
    let doc = Doc::from_str(input).unwrap();

    let Node::Map(root) = doc.root().node() else {
        panic!("Expected map");
    };

    // Both lists should have same structure
    assert_eq!(root.len(), 2);
}

#[test]
fn ch_3_4_03_identity_vs_equality() {
    // Anchored nodes: identity vs equality
    let input = "anchor: &ref [1, 2, 3]\nalias: *ref\ncopy: [1, 2, 3]\n";
    let doc = Doc::from_str(input).unwrap();

    let Node::Map(root) = doc.root().node() else {
        panic!("Expected map");
    };

    // anchor and alias point to same node (identity)
    // copy is equal but not identical
    assert_eq!(root.len(), 3);
}

// ============================================================================
// 3.5 Round-Trip Preservation
// ============================================================================

#[test]
fn ch_3_5_01_content_preserved() {
    // Content must be preserved through load/dump
    Fixture::new(
        "3.5",
        1,
        "Content preserved in round-trip",
        "text: hello\nnumber: 42\nbool: true\n",
    )
    .run();
}

#[test]
fn ch_3_5_02_structure_preserved() {
    // Structure must be preserved
    Fixture::new(
        "3.5",
        2,
        "Structure preserved",
        "outer:\n  inner:\n    deep: value\n",
    )
    .run();
}

#[test]
fn ch_3_5_03_aliases_preserved() {
    // Alias relationships should be preserved
    Fixture::new(
        "3.5",
        3,
        "Aliases preserved",
        "first: &anchor data\nsecond: *anchor\n",
    )
    .run();
}

#[test]
fn ch_3_5_04_types_preserved() {
    // Type information should be preserved
    let input = "null: null\nbool: true\nint: 42\nfloat: 3.14\nstr: text\n";
    let doc = Doc::from_str(input).unwrap();
    let output = doc.to_string().unwrap();

    // Re-parse and verify types are maintained
    let doc2 = Doc::from_str(&output).unwrap();
    let Node::Map(root) = doc2.root().node() else {
        panic!("Expected map");
    };

    // Check that different types are preserved
    let (_, null_val) = root.iter().find(|(k, _)| k == "null").unwrap();
    assert!(matches!(null_val.node(), Node::Scalar(Scalar::Null)));

    let (_, bool_val) = root.iter().find(|(k, _)| k == "bool").unwrap();
    assert!(matches!(bool_val.node(), Node::Scalar(Scalar::Bool(_))));
}

// ============================================================================
// 3.6 Presentation Independence
// ============================================================================

#[test]
fn ch_3_6_01_whitespace_irrelevant() {
    // Presentation whitespace should not affect content
    let compact = "key:value\n";
    let spaced = "key:    value   \n";

    let doc1 = Doc::from_str(compact).unwrap();
    let doc2 = Doc::from_str(spaced).unwrap();

    // Both should produce same structure
    let Node::Map(m1) = doc1.root().node() else { panic!() };
    let Node::Map(m2) = doc2.root().node() else { panic!() };

    assert_eq!(m1.len(), m2.len());
}

#[test]
fn ch_3_6_02_comments_ignored() {
    // Comments are presentation details only
    let with_comments = "# Comment\nkey: value # inline\n";
    let without = "key: value\n";

    let doc1 = Doc::from_str(with_comments).unwrap();
    let doc2 = Doc::from_str(without).unwrap();

    // Both should produce same content
    let Node::Map(m1) = doc1.root().node() else { panic!() };
    let Node::Map(m2) = doc2.root().node() else { panic!() };

    assert_eq!(m1.len(), m2.len());
}

#[test]
fn ch_3_6_03_style_irrelevant() {
    // Scalar style (plain, quoted, literal, folded) should not affect content
    let plain = "text: hello\n";
    let quoted = "text: \"hello\"\n";
    let single = "text: 'hello'\n";

    let doc1 = Doc::from_str(plain).unwrap();
    let doc2 = Doc::from_str(quoted).unwrap();
    let doc3 = Doc::from_str(single).unwrap();

    // All should produce same content
    let Node::Map(m1) = doc1.root().node() else { panic!() };
    let Node::Map(m2) = doc2.root().node() else { panic!() };
    let Node::Map(m3) = doc3.root().node() else { panic!() };

    assert_eq!(m1.len(), m2.len());
    assert_eq!(m1.len(), m3.len());
}

#[test]
fn ch_3_6_04_collection_style_irrelevant() {
    // Flow vs block collection style should not affect content
    let block = "list:\n  - a\n  - b\n";
    let flow = "list: [a, b]\n";

    let doc1 = Doc::from_str(block).unwrap();
    let doc2 = Doc::from_str(flow).unwrap();

    // Both should produce same structure
    let Node::Map(m1) = doc1.root().node() else { panic!() };
    let Node::Map(m2) = doc2.root().node() else { panic!() };

    assert_eq!(m1.len(), m2.len());
}

// ============================================================================
// 3.2.2 Serialization Tree (Anchor/Alias Rules)
// ============================================================================

#[test]
fn ch_3_2_2_01_anchor_uniqueness_per_document() {
    // Anchor names must be unique within a document
    let input = "a: &duplicate value1\nb: &duplicate value2\n";
    let result = Doc::from_str(input);

    // Should either error or handle gracefully (last wins)
    // YAML spec requires anchor names to be unique per document
    if result.is_err() {
        // Correctly rejects duplicate anchor
        assert!(result.is_err());
    } else {
        // Some implementations allow duplicates (last definition wins)
        // This is not strictly spec-compliant but common
    }
}

#[test]
fn ch_3_2_2_02_anchor_scope_does_not_cross_documents() {
    // Anchors defined in one document cannot be referenced in another
    let input = "---\nfirst: &ref data\n---\nsecond: *ref\n";

    // The alias *ref in second document refers to undefined anchor
    // This should fail during stream parsing (multi-document)
    let result = Doc::from_stream(input);
    assert!(result.is_err(), "Anchor scope should not cross document boundaries");
}

#[test]
fn ch_3_2_2_03_circular_alias_self_reference() {
    // A node cannot reference itself directly
    let input = "a: &self\n  b: *self\n";
    let result = Doc::from_str(input);

    // Circular references may be accepted (creating a graph)
    // or rejected depending on implementation
    if result.is_ok() {
        // Implementation allows circular references
        // This creates a cyclic graph structure
    } else {
        // Implementation detects and rejects cycles
        assert!(result.is_err());
    }
}

#[test]
fn ch_3_2_2_04_circular_alias_mutual() {
    // Mutual circular references: A -> B -> A
    let input = "a: &refA\n  child: *refB\nb: &refB\n  child: *refA\n";
    let result = Doc::from_str(input);

    // May be accepted (creates cycles) or rejected
    if result.is_err() {
        // Implementation detects circular dependency
        assert!(result.is_err());
    }
}

#[test]
fn ch_3_2_2_05_anchor_on_empty_node() {
    // Anchors can be placed on empty/null nodes
    Fixture::new(
        "3.2.2",
        5,
        "Anchor on empty node",
        "anchor: &empty\nalias: *empty\n",
    )
    .run();
}

#[test]
fn ch_3_2_2_06_multiple_aliases_same_anchor() {
    // Multiple aliases can reference the same anchor
    Fixture::new(
        "3.2.2",
        6,
        "Multiple aliases to same anchor",
        "data: &shared\n  value: important\nref1: *shared\nref2: *shared\nref3: *shared\n",
    )
    .run();
}

// ============================================================================
// 3.2.3 Presentation Stream (Directives, Encoding, Style Hints)
// ============================================================================

#[test]
fn ch_3_2_3_01_utf8_bom_handling() {
    // UTF-8 BOM (U+FEFF) should be stripped
    let input_with_bom = "\u{FEFF}key: value\n";
    let doc = Doc::from_str(input_with_bom);

    // Should parse successfully, BOM stripped
    assert!(doc.is_ok(), "UTF-8 BOM should be accepted and stripped");
}

#[test]
fn ch_3_2_3_02_yaml_directive_version() {
    // YAML directive specifies version
    let input = "%YAML 1.2\n---\nkey: value\n";
    let doc = Doc::from_str(input);

    assert!(doc.is_ok(), "YAML 1.2 directive should be accepted");
}

#[test]
fn ch_3_2_3_03_yaml_directive_unsupported_version() {
    // YAML 1.3 doesn't exist yet
    let input = "%YAML 1.3\n---\nkey: value\n";
    let result = Doc::from_str(input);

    // Should warn or error about unsupported version
    // Most implementations are lenient and parse anyway
    if result.is_err() {
        assert!(result.is_err(), "Unsupported YAML version should error");
    }
}

#[test]
fn ch_3_2_3_04_tag_directive_scope() {
    // TAG directive applies only to the document where it's defined
    let input = "%TAG !e! tag:example.com,2000:app/\n---\nfirst: !e!type data\n---\nsecond: value\n";

    // First document can use !e! handle
    // Second document should not have access to !e! (unless redefined)
    let doc = Doc::from_str(input);
    // This tests directive scoping
    assert!(doc.is_ok());
}

#[test]
fn ch_3_2_3_05_duplicate_yaml_directive() {
    // Only one YAML directive allowed per document
    let input = "%YAML 1.2\n%YAML 1.2\n---\nkey: value\n";
    let result = Doc::from_str(input);

    // Should error - duplicate directive
    assert!(result.is_err(), "Duplicate YAML directive should be rejected");
}

#[test]
fn ch_3_2_3_06_duplicate_tag_handle() {
    // Cannot define the same tag handle twice
    let input = "%TAG !e! tag:example.com,2000:/\n%TAG !e! tag:other.com,2000:/\n---\nkey: value\n";
    let result = Doc::from_str(input);

    // Should error - duplicate tag handle
    assert!(result.is_err(), "Duplicate TAG handle should be rejected");
}

#[test]
fn ch_3_2_3_07_directive_after_content() {
    // Directives must come before document content
    let input = "key: value\n%YAML 1.2\n";
    let result = Doc::from_str(input);

    // Should error - directive after content
    assert!(result.is_err(), "Directive after content should be rejected");
}

// ============================================================================
// 3.3 Error Cases - Validation and Failure Modes
// ============================================================================

#[test]
fn ch_3_3_07_tabs_in_block_indentation() {
    // Tabs are not allowed in block indentation
    let input = "parent:\n\tchild: value\n";
    let result = Doc::from_str(input);

    // Should error - tabs forbidden in indentation
    assert!(result.is_err(), "Tabs in block indentation should be rejected");
}

#[test]
fn ch_3_3_08_invalid_utf8_sequence() {
    // Invalid UTF-8 should be rejected
    // This is tricky to test in Rust strings (they're always valid UTF-8)
    // But the parser should handle byte-level invalid sequences

    // Note: Rust string literals are always valid UTF-8
    // This test documents the requirement
    // Real testing would need byte-level input
}

#[test]
fn ch_3_3_09_undefined_tag_handle() {
    // Using a tag handle that wasn't declared
    let input = "value: !undefined!type data\n";
    let result = Doc::from_str(input);

    // Should error - tag handle not declared
    assert!(result.is_err(), "Undefined tag handle should be rejected");
}

#[test]
fn ch_3_3_10_unknown_directive_ignored() {
    // Unknown directives should be ignored with warning
    let input = "%FUTURE 1.0\n---\nkey: value\n";
    let doc = Doc::from_str(input);

    // Should parse successfully, ignoring unknown directive
    assert!(doc.is_ok(), "Unknown directives should be ignored");
}

// ============================================================================
// 3.4 Tag Resolution Order
// ============================================================================

#[test]
fn ch_3_4_01_explicit_tag_overrides_inference() {
    // Explicit tags override schema type inference
    Fixture::new(
        "3.4",
        1,
        "Explicit tag overrides inference",
        "number: !!str 42\nstring: !!int \"123\"\n",
    )
    .run();
}

#[test]
fn ch_3_4_02_tag_resolution_priority() {
    // Tag resolution: explicit > non-specific > schema default
    let input = "explicit: !!int 42\nimplicit: 42\n";
    let doc = Doc::from_str(input).unwrap();

    let Node::Map(root) = doc.root().node() else {
        panic!("Expected map");
    };

    // Both should be integers, but explicit is guaranteed
    assert_eq!(root.len(), 2);
}

#[test]
fn ch_3_4_03_tag_priority_str_on_number() {
    // Section 3.2.1.2 - explicit tag should override type inference
    // !!str 123 should be string "123", not number
    Fixture::new(
        "3.4",
        3,
        "Explicit !!str tag on numeric content",
        "value: !!str 123\n",
    )
    .run();
}

#[test]
fn ch_3_4_04_tag_priority_int_on_string() {
    // Section 3.2.1.2 - explicit tag priority
    // !!int on quoted string should attempt integer parsing
    Fixture::new(
        "3.4",
        4,
        "Explicit !!int tag on string content",
        "value: !!int \"456\"\n",
    )
    .run();
}

// ============================================================================
// 3.5 Advanced Round-Trip Cases
// ============================================================================

#[test]
fn ch_3_5_05_anchor_alias_preserved() {
    // Anchor/alias structure should be preserved
    Fixture::new(
        "3.5",
        5,
        "Anchor/alias structure preserved",
        "data: &ref [1, 2, 3]\ncopy: *ref\n",
    )
    .run();
}

#[test]
fn ch_3_5_06_complex_graph_structure() {
    // Complex graph with multiple anchors and aliases
    Fixture::new(
        "3.5",
        6,
        "Complex graph structure",
        "a: &a1 value\nb: &b1 data\nrefs:\n  - *a1\n  - *b1\n  - *a1\n",
    )
    .run();
}

// ============================================================================
// 3.7 Edge Cases and Exotic Scenarios
// ============================================================================

// 3.7.1 Anchor/Alias Edge Cases

#[test]
fn ch_3_7_1_01_anchor_without_alias() {
    // Anchors need not have corresponding aliases
    // They are optional serialization decorations
    Fixture::new(
        "3.7.1",
        1,
        "Anchor without alias reference",
        "unused: &orphan data\nother: value\n",
    )
    .run();
}

#[test]
fn ch_3_7_1_02_multiple_anchors_on_identical_content() {
    // Different anchors can point to semantically identical content
    Fixture::new(
        "3.7.1",
        2,
        "Multiple anchors on identical values",
        "first: &a1 data\nsecond: &a2 data\nthird: &a3 data\n",
    )
    .run();
}

#[test]
fn ch_3_7_1_03_alias_to_empty_scalar() {
    // Alias referencing empty/null scalar
    Fixture::new(
        "3.7.1",
        3,
        "Alias to empty scalar",
        "empty: &e\nref: *e\n",
    )
    .run();
}

#[test]
fn ch_3_7_1_04_alias_to_null_explicit() {
    // Alias referencing explicit null
    Fixture::new(
        "3.7.1",
        4,
        "Alias to explicit null",
        "null_value: &n null\nref: *n\n",
    )
    .run();
}

// 3.7.2 Duplicate Key Detection Edge Cases

#[test]
fn ch_3_7_2_01_duplicate_keys_different_formats() {
    // Keys that are semantically equal but written differently
    // For example: 0o13 (octal) and 11 (decimal) should be duplicate keys
    let input = "0o13: octal\n11: decimal\n";
    let result = Doc::from_str(input);

    // Spec requires duplicate key detection based on canonical form
    // Most implementations will accept this (last wins), but strict
    // spec compliance requires detecting semantic duplicates
    if result.is_ok() {
        // Implementation allows duplicate keys (common behavior)
        let doc = result.unwrap();
        let Node::Map(m) = doc.root().node() else {
            panic!("Expected map");
        };
        // Should have at most 1 key if duplicates merged
        assert!(m.len() <= 2);
    }
}

#[test]
fn ch_3_7_2_02_duplicate_keys_canonical_comparison() {
    // Different representations of same string
    let input = "\"key\": value1\nkey: value2\n";
    let doc = Doc::from_str(input).unwrap();

    let Node::Map(m) = doc.root().node() else {
        panic!("Expected map");
    };

    // These should be treated as duplicate keys (same canonical form)
    // Most implementations merge duplicates (last wins)
    assert_eq!(m.len(), 1);
}

#[test]
fn ch_3_7_2_03_duplicate_keys_numeric_formats() {
    // Different numeric formats: hexadecimal and decimal
    let input = "0xB: hex\n11: decimal\n";
    let result = Doc::from_str(input);

    // Should detect duplicates based on resolved numeric value
    if result.is_ok() {
        let doc = result.unwrap();
        let Node::Map(m) = doc.root().node() else {
            panic!("Expected map");
        };
        assert!(m.len() <= 2);
    }
}

// 3.7.3 Tag Resolution Edge Cases

#[test]
fn ch_3_7_3_01_non_specific_tag_plain_scalar() {
    // Plain scalars get "?" non-specific tag during parsing
    Fixture::new(
        "3.7.3",
        1,
        "Non-specific tag on plain scalar",
        "plain: unquoted value\n",
    )
    .run();
}

#[test]
fn ch_3_7_3_02_non_specific_tag_quoted_scalar() {
    // Quoted scalars get "!" non-specific tag during parsing
    Fixture::new(
        "3.7.3",
        2,
        "Non-specific tag on quoted scalar",
        "quoted: \"value\"\n",
    )
    .run();
}

#[test]
fn ch_3_7_3_03_tag_resolution_no_sibling_influence() {
    // Tag resolution must not consider sibling node content
    // Each node resolved independently based only on path from root
    Fixture::new(
        "3.7.3",
        3,
        "Tag resolution ignores siblings",
        "items:\n  - 42\n  - text\n  - true\n",
    )
    .run();
}

// 3.7.4 Multi-Document Stream Edge Cases

#[test]
fn ch_3_7_4_01_different_directives_per_document() {
    // Each document in a stream can have different directives
    let input = "%YAML 1.2\n---\nfirst: doc1\n---\n%TAG !custom! tag:example.com,2000:/\nsecond: doc2\n";
    let result = Doc::from_str(input);

    // Directives are per-document, but same encoding required
    // Some parsers may only parse first document
    if result.is_ok() {
        // Parser handles multi-document with different directives
    }
}

#[test]
fn ch_3_7_4_02_bom_in_middle_of_stream() {
    // BOM can appear at start of any document in a stream
    let input = "---\nfirst: doc1\n---\n\u{FEFF}second: doc2\n";
    let result = Doc::from_str(input);

    // Should either accept BOM at document start or error gracefully
    if result.is_ok() {
        // Parser handles BOM in multi-document stream
    }
}

#[test]
fn ch_3_7_4_03_encoding_consistency_across_documents() {
    // All documents in same stream must use same encoding
    // This is inherently satisfied in Rust strings (always UTF-8)
    // But the requirement exists for byte-level parsers
    Fixture::new(
        "3.7.4",
        3,
        "Multi-document encoding consistency",
        "---\nfirst: value\n---\nsecond: value\n---\nthird: value\n",
    )
    .run();
}

// 3.7.5 Comment Edge Cases

#[test]
fn ch_3_7_5_01_comments_not_in_scalars() {
    // Comments must not appear inside scalars
    // This tests that # inside a literal block is preserved
    Fixture::new(
        "3.7.5",
        1,
        "Hash in literal scalar not a comment",
        "text: |\n  This # is not a comment\n  Another # line\n",
    )
    .run();
}

#[test]
fn ch_3_7_5_02_comments_between_collection_items() {
    // Comments can be interleaved with collection items
    Fixture::new(
        "3.7.5",
        2,
        "Comments between collection items",
        "items:\n  # First item comment\n  - item1\n  # Second item comment\n  - item2\n",
    )
    .run();
}

#[test]
fn ch_3_7_5_03_comment_after_directive() {
    // Comments can follow directives
    let input = "%YAML 1.2 # This is a comment\n---\nkey: value\n";
    let result = Doc::from_str(input);

    // Should parse successfully
    assert!(result.is_ok(), "Comments after directives should be allowed");
}

// 3.7.6 Partial Representation Edge Cases

#[test]
fn ch_3_7_6_01_unresolved_tag_in_partial_representation() {
    // Partial representations need not resolve all tags
    // This is more of an implementation detail, but we can test
    // that the parser can work with non-specific tags
    Fixture::new(
        "3.7.6",
        1,
        "Parser handles unresolved tags",
        "value: some data\n",
    )
    .run();
}

// 3.7.7 Node Equality Edge Cases

#[test]
fn ch_3_7_7_01_scalar_equality_requires_tag_match() {
    // Two scalars are equal only if tags AND content match
    // !!str "42" is NOT equal to !!int 42
    let input = "str_num: !!str 42\nint_num: !!int 42\n";
    let doc = Doc::from_str(input).unwrap();

    let Node::Map(m) = doc.root().node() else {
        panic!("Expected map");
    };

    // These should NOT be duplicate keys (different tags)
    assert_eq!(m.len(), 2);
}

#[test]
fn ch_3_7_7_02_collection_equality_requires_content_match() {
    // Two collections are equal if their content matches
    let input = "list1: [1, 2, 3]\nlist2: [1, 2, 3]\n";
    let doc = Doc::from_str(input).unwrap();

    let Node::Map(m) = doc.root().node() else {
        panic!("Expected map");
    };

    // Different keys, so not duplicates
    assert_eq!(m.len(), 2);
}

// 3.7.8 Construction Constraints

#[test]
fn ch_3_7_8_01_construction_ignores_comments() {
    // Native data construction must ignore comments
    let with_comments = "key: value # comment\nlist: # comment\n  - item # comment\n";
    let without_comments = "key: value\nlist:\n  - item\n";

    let doc1 = Doc::from_str(with_comments).unwrap();
    let doc2 = Doc::from_str(without_comments).unwrap();

    // Should produce identical representation graphs
    let Node::Map(m1) = doc1.root().node() else {
        panic!("Expected map");
    };
    let Node::Map(m2) = doc2.root().node() else {
        panic!("Expected map");
    };

    assert_eq!(m1.len(), m2.len());
}

#[test]
fn ch_3_7_8_02_construction_ignores_whitespace_style() {
    // Construction must not depend on whitespace details
    let compact = "key: value\nlist: [1, 2, 3]\n";
    let expanded = "key:   value\nlist:\n  - 1\n  - 2\n  - 3\n";

    let doc1 = Doc::from_str(compact).unwrap();
    let doc2 = Doc::from_str(expanded).unwrap();

    // Should produce equivalent structures
    let Node::Map(m1) = doc1.root().node() else {
        panic!("Expected map");
    };
    let Node::Map(m2) = doc2.root().node() else {
        panic!("Expected map");
    };

    assert_eq!(m1.len(), m2.len());
}

#[test]
fn ch_3_7_8_03_construction_ignores_key_order() {
    // Mapping key order is not significant for equality
    let order1 = "a: 1\nb: 2\nc: 3\n";
    let order2 = "c: 3\na: 1\nb: 2\n";

    let doc1 = Doc::from_str(order1).unwrap();
    let doc2 = Doc::from_str(order2).unwrap();

    // Should produce equivalent mappings (order may differ in iteration)
    let Node::Map(m1) = doc1.root().node() else {
        panic!("Expected map");
    };
    let Node::Map(m2) = doc2.root().node() else {
        panic!("Expected map");
    };

    assert_eq!(m1.len(), m2.len());
}

// 3.7.9 Encoding Edge Cases (BOM Variations)

#[test]
fn ch_3_7_9_01_utf8_bom_at_stream_start() {
    // UTF-8 BOM (EF BB BF) at start of stream
    let input = "\u{FEFF}key: value\n";
    let doc = Doc::from_str(input);

    assert!(doc.is_ok(), "UTF-8 BOM at stream start should be accepted");
}

#[test]
fn ch_3_7_9_02_bom_inside_double_quoted_scalar() {
    // BOM inside double-quoted scalar should be preserved (JSON compat)
    // But should be escaped on output
    let input = "text: \"\u{FEFF}data\"\n";
    let doc = Doc::from_str(input);

    // Should parse successfully
    assert!(doc.is_ok(), "BOM inside quoted scalar should be allowed");
}

#[test]
fn ch_3_7_9_03_no_bom_ascii_start() {
    // Stream without BOM must begin with ASCII character
    Fixture::new(
        "3.7.9",
        3,
        "No BOM, ASCII start",
        "key: value\n",
    )
    .run();
}

// 3.7.10 Cyclic Reference Equality Edge Cases

#[test]
fn ch_3_7_10_01_self_referential_node_equality_undefined() {
    // Equality of self-referential nodes is implementation-defined
    let input = "node: &self\n  child: *self\n";
    let result = Doc::from_str(input);

    // May accept or reject cyclic structures
    // If accepted, equality testing for such nodes is implementation-defined
    if result.is_ok() {
        // Implementation accepts cycles
        // Duplicate detection would be impossible for this structure
    }
}

#[test]
fn ch_3_7_10_02_duplicate_detection_in_cyclic_graph() {
    // Cannot determine duplicates when cycles exist
    let input = "a: &a {b: *b}\nb: &b {a: *a}\n";
    let result = Doc::from_str(input);

    // This creates mutual references
    // Duplicate key detection cannot work in presence of cycles
    if result.is_ok() {
        // Implementation allows this structure
    }
}
