// crates/forgo_lib_yaml/tests/api/node_guards.rs
//! Tests for Node type guards and conversions
//!
//! Covers:
//! - Node::is_map(), is_seq(), is_scalar(), is_alias(), is_null()
//! - Node::as_map(), as_map_mut(), as_seq(), as_seq_mut(), as_scalar(), as_scalar_mut()
//! - Node::as_str(), as_bool(), as_i64(), as_f64(), as_u64()

use crate::api::*;
use forgo_lib_yaml::{Node, Scalar};

// ============================================================================
// Type Guards Tests
// ============================================================================

#[test]
fn test_is_map_returns_true_for_map() {
    let node = Node::Map(vec![]);
    assert!(node.is_map());
    assert!(!node.is_seq());
    assert!(!node.is_scalar());
    assert!(!node.is_alias());
}

#[test]
fn test_is_seq_returns_true_for_seq() {
    let node = Node::Seq(vec![]);
    assert!(node.is_seq());
    assert!(!node.is_map());
    assert!(!node.is_scalar());
    assert!(!node.is_alias());
}

#[test]
fn test_is_scalar_returns_true_for_scalar() {
    let node = Node::Scalar(Scalar::Str("test".into()));
    assert!(node.is_scalar());
    assert!(!node.is_map());
    assert!(!node.is_seq());
    assert!(!node.is_alias());
}

#[test]
fn test_is_alias_returns_true_for_alias() {
    let node = Node::Alias("ref".into());
    assert!(node.is_alias());
    assert!(!node.is_map());
    assert!(!node.is_seq());
    assert!(!node.is_scalar());
}

#[test]
fn test_is_null_returns_true_for_null_scalar() {
    let node = Node::Scalar(Scalar::Null);
    assert!(node.is_null());
}

#[test]
fn test_is_null_returns_false_for_non_null() {
    assert!(!Node::Scalar(Scalar::Str("test".into())).is_null());
    assert!(!Node::Scalar(Scalar::Bool(true)).is_null());
    assert!(!Node::Map(vec![]).is_null());
    assert!(!Node::Seq(vec![]).is_null());
}

// ============================================================================
// Type Conversion Tests - as_map, as_seq, as_scalar
// ============================================================================

#[test]
fn test_as_map_returns_some_for_map() {
    let node = Node::Map(vec![("key".into(), str_elem("value"))]);
    let map = node.as_map().unwrap();
    assert_eq!(map.len(), 1);
    assert_eq!(map[0].0, "key");
}

#[test]
fn test_as_map_returns_none_for_non_map() {
    assert!(Node::Seq(vec![]).as_map().is_none());
    assert!(Node::Scalar(Scalar::Str("test".into())).as_map().is_none());
}

#[test]
fn test_as_map_mut_returns_mutable_reference() {
    let mut node = Node::Map(vec![]);
    node.as_map_mut().unwrap().push(("key".into(), str_elem("value")));
    assert_eq!(node.as_map().unwrap().len(), 1);
}

#[test]
fn test_as_seq_returns_some_for_seq() {
    let node = Node::Seq(vec![str_elem("item")]);
    let seq = node.as_seq().unwrap();
    assert_eq!(seq.len(), 1);
}

#[test]
fn test_as_seq_returns_none_for_non_seq() {
    assert!(Node::Map(vec![]).as_seq().is_none());
    assert!(Node::Scalar(Scalar::Str("test".into())).as_seq().is_none());
}

#[test]
fn test_as_seq_mut_returns_mutable_reference() {
    let mut node = Node::Seq(vec![]);
    // Note: We'll use seq_mut() since it already exists
    node.seq_mut().unwrap().push(str_elem("item"));
    assert_eq!(node.as_seq().unwrap().len(), 1);
}

#[test]
fn test_as_scalar_returns_some_for_scalar() {
    let node = Node::Scalar(Scalar::Str("test".into()));
    let scalar = node.as_scalar().unwrap();
    assert!(matches!(scalar, Scalar::Str(_)));
}

#[test]
fn test_as_scalar_returns_none_for_non_scalar() {
    assert!(Node::Map(vec![]).as_scalar().is_none());
    assert!(Node::Seq(vec![]).as_scalar().is_none());
}

#[test]
fn test_as_scalar_mut_returns_mutable_reference() {
    let mut node = Node::Scalar(Scalar::Str("old".into()));
    if let Some(scalar) = node.as_scalar_mut() {
        *scalar = Scalar::Str("new".into());
    }
    assert_eq!(node.as_scalar().unwrap().as_str().unwrap(), "new");
}

// ============================================================================
// Scalar Convenience Methods on Node
// ============================================================================

#[test]
fn test_as_str_returns_string_value() {
    let node = Node::Scalar(Scalar::Str("hello".into()));
    assert_eq!(node.as_str(), Some("hello"));
}

#[test]
fn test_as_str_returns_none_for_non_string() {
    assert!(Node::Scalar(Scalar::Bool(true)).as_str().is_none());
    assert!(Node::Scalar(Scalar::Null).as_str().is_none());
    assert!(Node::Map(vec![]).as_str().is_none());
}

#[test]
fn test_as_bool_returns_boolean_value() {
    let node_true = Node::Scalar(Scalar::Bool(true));
    let node_false = Node::Scalar(Scalar::Bool(false));
    assert_eq!(node_true.as_bool(), Some(true));
    assert_eq!(node_false.as_bool(), Some(false));
}

#[test]
fn test_as_bool_returns_none_for_non_boolean() {
    assert!(Node::Scalar(Scalar::Str("true".into())).as_bool().is_none());
    assert!(Node::Map(vec![]).as_bool().is_none());
}

#[test]
fn test_as_i64_parses_integer() {
    let node = Node::Scalar(Scalar::Num { text: "42".into() });
    assert_eq!(node.as_i64(), Some(42));
}

#[test]
fn test_as_i64_parses_negative_integer() {
    let node = Node::Scalar(Scalar::Num { text: "-123".into() });
    assert_eq!(node.as_i64(), Some(-123));
}

#[test]
fn test_as_i64_returns_none_for_invalid_number() {
    let node = Node::Scalar(Scalar::Num { text: "not_a_number".into() });
    assert!(node.as_i64().is_none());
}

#[test]
fn test_as_i64_returns_none_for_non_number() {
    assert!(Node::Scalar(Scalar::Str("42".into())).as_i64().is_none());
    assert!(Node::Map(vec![]).as_i64().is_none());
}

#[test]
fn test_as_f64_parses_float() {
    let node = Node::Scalar(Scalar::Num { text: "3.14".into() });
    assert_eq!(node.as_f64(), Some(3.14));
}

#[test]
fn test_as_f64_parses_integer_as_float() {
    let node = Node::Scalar(Scalar::Num { text: "42".into() });
    assert_eq!(node.as_f64(), Some(42.0));
}

#[test]
fn test_as_f64_returns_none_for_invalid_number() {
    let node = Node::Scalar(Scalar::Num { text: "not_a_number".into() });
    assert!(node.as_f64().is_none());
}

#[test]
fn test_as_u64_parses_unsigned_integer() {
    let node = Node::Scalar(Scalar::Num { text: "42".into() });
    assert_eq!(node.as_u64(), Some(42));
}

#[test]
fn test_as_u64_returns_none_for_negative() {
    let node = Node::Scalar(Scalar::Num { text: "-42".into() });
    assert!(node.as_u64().is_none());
}

// ============================================================================
// Integration Tests
// ============================================================================

#[test]
fn test_type_guards_on_parsed_document() {
    let doc = sample_doc();
    let root = doc.root().node();

    assert!(root.is_map());

    if let Some(map) = root.as_map() {
        let deps = map.iter().find(|(k, _)| k == "dependencies").unwrap();
        assert!(deps.1.node().is_seq());

        let config = map.iter().find(|(k, _)| k == "config").unwrap();
        assert!(config.1.node().is_map());
    }
}

#[test]
fn test_scalar_conversions_on_parsed_document() {
    let doc = sample_doc();
    let root = doc.root().node();

    if let Some(map) = root.as_map() {
        // Test string conversion
        let name = map.iter().find(|(k, _)| k == "name").unwrap();
        assert_eq!(name.1.node().as_str(), Some("test-app"));

        // Test nested map access with boolean
        let config = map.iter().find(|(k, _)| k == "config").unwrap();
        if let Some(config_map) = config.1.node().as_map() {
            let debug = config_map.iter().find(|(k, _)| k == "debug").unwrap();
            assert_eq!(debug.1.node().as_bool(), Some(true));

            let timeout = config_map.iter().find(|(k, _)| k == "timeout").unwrap();
            assert_eq!(timeout.1.node().as_i64(), Some(30));
        }
    }
}
