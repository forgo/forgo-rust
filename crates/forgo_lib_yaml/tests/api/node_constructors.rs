// crates/forgo_lib_yaml/tests/api/node_constructors.rs
//! Tests for Node construction helpers
//!
//! Covers:
//! - Node::map(), seq(), string(), number(), boolean(), null()

use crate::api::*;
use forgo_lib_yaml::{MapKey, Node, Scalar};

// ============================================================================
// Node Constructors Tests
// ============================================================================

#[test]
fn test_node_map_creates_empty_map() {
    let node = Node::map();
    assert!(node.is_map());
    assert_eq!(node.as_map().unwrap().len(), 0);
}

#[test]
fn test_node_seq_creates_empty_sequence() {
    let node = Node::seq();
    assert!(node.is_seq());
    assert_eq!(node.as_seq().unwrap().len(), 0);
}

#[test]
fn test_node_string_creates_string_scalar() {
    let node = Node::string("hello");
    assert!(node.is_scalar());
    assert_eq!(node.as_str(), Some("hello"));
}

#[test]
fn test_node_string_accepts_string() {
    let s = String::from("world");
    let node = Node::string(s);
    assert_eq!(node.as_str(), Some("world"));
}

#[test]
fn test_node_string_accepts_str_slice() {
    let node = Node::string("test");
    assert_eq!(node.as_str(), Some("test"));
}

#[test]
fn test_node_number_creates_number_scalar() {
    let node = Node::number(42);
    assert!(node.is_scalar());
    if let Node::Scalar(Scalar::Num { text }) = node {
        assert_eq!(text, "42");
    } else {
        panic!("Expected Num scalar");
    }
}

#[test]
fn test_node_number_accepts_string() {
    let node = Node::number("3.14");
    assert_eq!(node.as_f64(), Some(3.14));
}

#[test]
fn test_node_number_accepts_integer() {
    let node = Node::number(123);
    assert_eq!(node.as_i64(), Some(123));
}

#[test]
fn test_node_number_accepts_float() {
    let node = Node::number(2.5);
    assert_eq!(node.as_f64(), Some(2.5));
}

#[test]
fn test_node_number_preserves_formatting() {
    let node = Node::number("0x42");
    if let Node::Scalar(Scalar::Num { text }) = node {
        assert_eq!(text, "0x42");
    } else {
        panic!("Expected Num scalar");
    }
}

#[test]
fn test_node_boolean_true_creates_bool_scalar() {
    let node = Node::boolean(true);
    assert!(node.is_scalar());
    assert_eq!(node.as_bool(), Some(true));
}

#[test]
fn test_node_boolean_false_creates_bool_scalar() {
    let node = Node::boolean(false);
    assert!(node.is_scalar());
    assert_eq!(node.as_bool(), Some(false));
}

#[test]
fn test_node_null_creates_null_scalar() {
    let node = Node::null();
    assert!(node.is_scalar());
    assert!(node.is_null());
}

// ============================================================================
// Integration Tests - Building Documents
// ============================================================================

#[test]
fn test_build_simple_map_with_constructors() {
    let mut map_entries = vec![];
    map_entries.push(("name".into(), str_elem("test-app")));
    map_entries.push(("version".into(), num_elem("1.0")));
    map_entries.push(("enabled".into(), bool_elem(true)));

    let node = Node::Map(map_entries);

    assert!(node.is_map());
    let map = node.as_map().unwrap();
    assert_eq!(map.len(), 3);

    // Verify values
    let name = &map[0];
    assert_eq!(name.0, "name");
    assert_eq!(name.1.node().as_str(), Some("test-app"));

    let version = &map[1];
    assert_eq!(version.0, "version");
    // Numbers are stored as Num { text }, check as f64 instead
    assert_eq!(version.1.node().as_f64(), Some(1.0));

    let enabled = &map[2];
    assert_eq!(enabled.0, "enabled");
    assert_eq!(enabled.1.node().as_bool(), Some(true));
}

#[test]
fn test_build_sequence_with_constructors() {
    let seq = vec![
        str_elem("first"),
        num_elem("42"),
        bool_elem(false),
        null_elem(),
    ];

    let node = Node::Seq(seq);

    assert!(node.is_seq());
    let items = node.as_seq().unwrap();
    assert_eq!(items.len(), 4);
    assert_eq!(items[0].node().as_str(), Some("first"));
    assert_eq!(items[1].node().as_i64(), Some(42));
    assert_eq!(items[2].node().as_bool(), Some(false));
    assert!(items[3].node().is_null());
}

#[test]
fn test_nested_structure_with_constructors() {
    // Build: { outer: { inner: "value" } }
    let mut inner_map: Vec<(String, _)> = vec![];
    inner_map.push(("inner".into(), str_elem("value")));

    let mut outer_map: Vec<(MapKey, _)> = vec![];
    outer_map.push(("outer".into(), map_elem()));

    // This shows we need better helpers - we'll add them next!
    let node = Node::Map(outer_map);
    assert!(node.is_map());
}

// ============================================================================
// Type Conversion Tests
// ============================================================================

#[test]
fn test_constructors_produce_correct_types() {
    assert!(Node::map().is_map());
    assert!(Node::seq().is_seq());
    assert!(Node::string("test").is_scalar());
    assert!(Node::number(42).is_scalar());
    assert!(Node::boolean(true).is_scalar());
    assert!(Node::null().is_scalar());
    assert!(Node::null().is_null());
}

#[test]
fn test_string_constructor_handles_empty_string() {
    let node = Node::string("");
    assert_eq!(node.as_str(), Some(""));
}

#[test]
fn test_string_constructor_handles_special_chars() {
    let node = Node::string("hello\nworld\t!");
    assert_eq!(node.as_str(), Some("hello\nworld\t!"));
}

#[test]
fn test_number_constructor_handles_zero() {
    let node = Node::number(0);
    assert_eq!(node.as_i64(), Some(0));
}

#[test]
fn test_number_constructor_handles_negative() {
    let node = Node::number(-42);
    assert_eq!(node.as_i64(), Some(-42));
}
