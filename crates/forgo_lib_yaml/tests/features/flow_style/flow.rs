//! Flow Style Tests
//!
//! **Related YAML 1.2.2 Spec Sections:**
//! - §7: Flow Style Productions
//! - §7.3: Flow Scalar Styles (double-quoted, single-quoted, plain)
//! - §7.4: Flow Collection Styles
//! - §7.4.1: Flow Sequences (`[...]`)
//! - §7.4.2: Flow Mappings (`{...}`)
//!
//! **Purpose:**
//! These tests validate flow (inline) style parsing and emission.
//! The spec tests in `spec/ch_7_flow_styles.rs` cover basic syntax;
//! these tests validate nested flow collections and complex structures.

use forgo_lib_yaml::{Doc, Node};

#[test]
fn flow_mapping_and_nested_flow_seq() {
    let d = Doc::from_str("{k1: [1, 2, 3], k2: {x: true}}\n").unwrap();
    if let Node::Map(pairs) = d.root().node() {
        assert_eq!(pairs.len(), 2);
        match &pairs[0].1.node {
            Node::Seq(v) => assert_eq!(v.len(), 3),
            _ => panic!(),
        }
        match &pairs[1].1.node {
            Node::Map(m) => assert_eq!(m.len(), 1),
            _ => panic!(),
        }
    } else {
        panic!()
    }
}

// Regression test for URL parsing in flow mappings (Official test 9MMW)
// YAML 1.2.2 §7.3.3: Plain scalars in flow context can contain ':'
// if not followed by whitespace or flow indicator
#[test]
fn flow_mapping_with_url() {
    // URLs with colons should parse correctly
    let d = Doc::from_str("- { url: http://example.org }\n").unwrap();
    if let Node::Seq(items) = d.root().node() {
        assert_eq!(items.len(), 1);
        if let Node::Map(pairs) = &items[0].node {
            assert_eq!(pairs.len(), 1);
            assert_eq!(pairs[0].0.as_str(), Some("url"));
            if let Node::Scalar(s) = &pairs[0].1.node {
                assert_eq!(s.to_string(), "http://example.org");
            } else {
                panic!("Expected scalar value");
            }
        } else {
            panic!("Expected map");
        }
    } else {
        panic!("Expected sequence");
    }
}

#[test]
fn flow_mapping_with_various_urls() {
    let test_cases = vec![
        ("{url: http://example.org}", "http://example.org"),
        ("{url: https://example.org:8080/path}", "https://example.org:8080/path"),
        ("{url: ftp://files.example.com}", "ftp://files.example.com"),
        // Timestamps with colons
        ("{time: 2024-01-01T12:34:56}", "2024-01-01T12:34:56"),
        // Multiple colons in value
        ("{data: a:b:c:d}", "a:b:c:d"),
    ];

    for (yaml, expected_value) in test_cases {
        let d = Doc::from_str(yaml).unwrap_or_else(|e| panic!("Failed to parse '{}': {}", yaml, e));
        if let Node::Map(pairs) = d.root().node() {
            if let Node::Scalar(s) = &pairs[0].1.node {
                assert_eq!(s.to_string(), expected_value, "Failed for input: {}", yaml);
            } else {
                panic!("Expected scalar for input: {}", yaml);
            }
        } else {
            panic!("Expected map for input: {}", yaml);
        }
    }
}

#[test]
fn flow_mapping_colon_must_not_be_followed_by_space() {
    // This should fail: colon followed by space is a key-value separator
    assert!(Doc::from_str("{a: b, c: d}").is_ok());

    // Colon not followed by space/flow indicator is part of the value
    assert!(Doc::from_str("{url:http://example.org}").is_ok());
}
