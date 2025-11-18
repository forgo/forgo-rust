// crates/forgo_lib_yaml/tests/api/elem_builders.rs
//! Tests for Elem construction and fluent builders
//!
//! Covers:
//! - Elem::new(), map(), seq(), string(), number(), boolean(), null()
//! - Elem::with_comment(), with_leading_comments(), with_anchor()
//! - Elem::prefer_block(), prefer_quoted()

use crate::api::*;
use forgo_lib_yaml::{Elem, Node, Scalar};

// ============================================================================
// Elem Constructors Tests
// ============================================================================

#[test]
fn test_elem_new_wraps_node() {
    let node = Node::string("test");
    let elem = Elem::new(node.clone());
    assert_eq!(elem.node(), &node);
}

#[test]
fn test_elem_map_creates_empty_map() {
    let elem = Elem::map();
    assert!(elem.node().is_map());
    assert_eq!(elem.node().as_map().unwrap().len(), 0);
}

#[test]
fn test_elem_seq_creates_empty_sequence() {
    let elem = Elem::seq();
    assert!(elem.node().is_seq());
    assert_eq!(elem.node().as_seq().unwrap().len(), 0);
}

#[test]
fn test_elem_string_creates_string_scalar() {
    let elem = Elem::string("hello");
    assert_eq!(elem.node().as_str(), Some("hello"));
}

#[test]
fn test_elem_string_accepts_string_type() {
    let s = String::from("world");
    let elem = Elem::string(s);
    assert_eq!(elem.node().as_str(), Some("world"));
}

#[test]
fn test_elem_number_creates_number_scalar() {
    let elem = Elem::number(42);
    assert_eq!(elem.node().as_i64(), Some(42));
}

#[test]
fn test_elem_number_accepts_float() {
    let elem = Elem::number(3.14);
    assert_eq!(elem.node().as_f64(), Some(3.14));
}

#[test]
fn test_elem_number_accepts_string() {
    let elem = Elem::number("0x42");
    if let Node::Scalar(Scalar::Num { text }) = elem.node() {
        assert_eq!(text, "0x42");
    } else {
        panic!("Expected Num scalar");
    }
}

#[test]
fn test_elem_boolean_true() {
    let elem = Elem::boolean(true);
    assert_eq!(elem.node().as_bool(), Some(true));
}

#[test]
fn test_elem_boolean_false() {
    let elem = Elem::boolean(false);
    assert_eq!(elem.node().as_bool(), Some(false));
}

#[test]
fn test_elem_null_creates_null_scalar() {
    let elem = Elem::null();
    assert!(elem.node().is_null());
}

// ============================================================================
// Fluent Builder Tests - with_comment
// ============================================================================

#[test]
fn test_with_comment_sets_trailing_comment() {
    let elem = Elem::string("value").with_comment("This is a comment");
    assert_eq!(elem.meta.trailing_comment, Some("This is a comment".into()));
}

#[test]
fn test_with_comment_chainable() {
    let elem = Elem::string("test")
        .with_comment("comment");

    assert_eq!(elem.node().as_str(), Some("test"));
    assert_eq!(elem.meta.trailing_comment, Some("comment".into()));
}

#[test]
fn test_with_comment_accepts_string() {
    let comment = String::from("dynamic comment");
    let elem = Elem::number(42).with_comment(comment);
    assert_eq!(elem.meta.trailing_comment, Some("dynamic comment".into()));
}

#[test]
fn test_with_comment_on_map() {
    let elem = Elem::map().with_comment("map comment");
    assert!(elem.node().is_map());
    assert_eq!(elem.meta.trailing_comment, Some("map comment".into()));
}

// ============================================================================
// Fluent Builder Tests - with_leading_comments
// ============================================================================

#[test]
fn test_with_leading_comments_sets_comments() {
    let comments = vec!["Line 1".into(), "Line 2".into()];
    let elem = Elem::string("value").with_leading_comments(comments.clone());
    assert_eq!(elem.meta.leading_comments, comments);
}

#[test]
fn test_with_leading_comments_chainable() {
    let elem = Elem::number(42)
        .with_leading_comments(vec!["First line".into()])
        .with_comment("trailing");

    assert_eq!(elem.meta.leading_comments, vec!["First line".to_string()]);
    assert_eq!(elem.meta.trailing_comment, Some("trailing".into()));
}

#[test]
fn test_with_leading_comments_empty_vec() {
    let elem = Elem::boolean(true).with_leading_comments(vec![]);
    assert_eq!(elem.meta.leading_comments.len(), 0);
}

#[test]
fn test_with_leading_comments_multiple_lines() {
    let comments = vec![
        "This is a header comment".into(),
        "It spans multiple lines".into(),
        "And provides context".into(),
    ];
    let elem = Elem::seq().with_leading_comments(comments.clone());
    assert_eq!(elem.meta.leading_comments, comments);
}

// ============================================================================
// Fluent Builder Tests - with_anchor
// ============================================================================

#[test]
fn test_with_anchor_sets_anchor() {
    let elem = Elem::string("value").with_anchor("my_anchor");
    assert_eq!(elem.meta.anchor, Some("my_anchor".into()));
}

#[test]
fn test_with_anchor_chainable() {
    let elem = Elem::map()
        .with_anchor("config")
        .with_comment("Configuration map");

    assert_eq!(elem.meta.anchor, Some("config".into()));
    assert_eq!(elem.meta.trailing_comment, Some("Configuration map".into()));
}

#[test]
fn test_with_anchor_accepts_string() {
    let anchor_name = String::from("dynamic_anchor");
    let elem = Elem::seq().with_anchor(anchor_name);
    assert_eq!(elem.meta.anchor, Some("dynamic_anchor".into()));
}

// ============================================================================
// Fluent Builder Tests - prefer_block
// ============================================================================

#[test]
fn test_prefer_block_sets_flag() {
    let elem = Elem::map().prefer_block();
    assert!(elem.meta.prefer_block);
}

#[test]
fn test_prefer_block_chainable() {
    let elem = Elem::seq()
        .prefer_block()
        .with_comment("block style");

    assert!(elem.meta.prefer_block);
    assert_eq!(elem.meta.trailing_comment, Some("block style".into()));
}

#[test]
fn test_prefer_block_on_scalar() {
    let elem = Elem::string("multiline\nstring").prefer_block();
    assert!(elem.meta.prefer_block);
}

// ============================================================================
// Fluent Builder Tests - prefer_quoted
// ============================================================================

#[test]
fn test_prefer_quoted_sets_flag() {
    let elem = Elem::string("needs quotes").prefer_quoted();
    assert!(elem.meta.prefer_quoted);
}

#[test]
fn test_prefer_quoted_chainable() {
    let elem = Elem::string("test")
        .prefer_quoted()
        .with_comment("quoted string");

    assert!(elem.meta.prefer_quoted);
    assert_eq!(elem.meta.trailing_comment, Some("quoted string".into()));
}

// ============================================================================
// Complex Chaining Tests
// ============================================================================

#[test]
fn test_full_chain_all_metadata() {
    let elem = Elem::string("important value")
        .with_leading_comments(vec![
            "This is important".into(),
            "Handle with care".into(),
        ])
        .with_comment("inline note")
        .with_anchor("important_value")
        .prefer_quoted();

    assert_eq!(elem.node().as_str(), Some("important value"));
    assert_eq!(elem.meta.leading_comments.len(), 2);
    assert_eq!(elem.meta.trailing_comment, Some("inline note".into()));
    assert_eq!(elem.meta.anchor, Some("important_value".into()));
    assert!(elem.meta.prefer_quoted);
}

#[test]
fn test_chain_order_independence() {
    // Different order should produce same result
    let elem1 = Elem::number(42)
        .with_anchor("num")
        .with_comment("comment");

    let elem2 = Elem::number(42)
        .with_comment("comment")
        .with_anchor("num");

    assert_eq!(elem1.meta.anchor, elem2.meta.anchor);
    assert_eq!(elem1.meta.trailing_comment, elem2.meta.trailing_comment);
}

#[test]
fn test_build_map_with_metadata() {
    let elem = Elem::map()
        .with_leading_comments(vec!["Configuration section".into()])
        .with_anchor("config")
        .prefer_block();

    assert!(elem.node().is_map());
    assert_eq!(elem.meta.leading_comments[0], "Configuration section");
    assert_eq!(elem.meta.anchor, Some("config".into()));
    assert!(elem.meta.prefer_block);
}

#[test]
fn test_build_sequence_with_metadata() {
    let elem = Elem::seq()
        .with_leading_comments(vec!["List of items".into()])
        .with_comment("todo items");

    assert!(elem.node().is_seq());
    assert_eq!(elem.meta.leading_comments[0], "List of items");
    assert_eq!(elem.meta.trailing_comment, Some("todo items".into()));
}

// ============================================================================
// Integration Tests - Building Complex Structures
// ============================================================================

#[test]
fn test_build_documented_config() {
    let config = Elem::map()
        .with_leading_comments(vec![
            "Application Configuration".into(),
            "Version 1.0".into(),
        ])
        .with_anchor("app_config");

    assert!(config.node().is_map());
    assert_eq!(config.meta.leading_comments.len(), 2);
    assert_eq!(config.meta.anchor, Some("app_config".into()));
}

#[test]
fn test_constructors_produce_default_metadata() {
    let elem = Elem::string("test");
    assert!(elem.meta.leading_comments.is_empty());
    assert!(elem.meta.trailing_comment.is_none());
    assert!(elem.meta.anchor.is_none());
    assert!(!elem.meta.prefer_block);
    assert!(!elem.meta.prefer_quoted);
}

#[test]
fn test_metadata_preserved_after_chaining() {
    let elem = Elem::number(100)
        .with_comment("first")
        .with_anchor("num");

    // Metadata should be preserved
    assert_eq!(elem.meta.trailing_comment, Some("first".into()));
    assert_eq!(elem.meta.anchor, Some("num".into()));

    // Node value should be correct
    assert_eq!(elem.node().as_i64(), Some(100));
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_with_comment_empty_string() {
    let elem = Elem::string("value").with_comment("");
    assert_eq!(elem.meta.trailing_comment, Some("".into()));
}

#[test]
fn test_with_anchor_empty_string() {
    let elem = Elem::string("value").with_anchor("");
    assert_eq!(elem.meta.anchor, Some("".into()));
}

#[test]
fn test_multiple_flags_combined() {
    let elem = Elem::string("test")
        .prefer_block()
        .prefer_quoted();

    assert!(elem.meta.prefer_block);
    assert!(elem.meta.prefer_quoted);
}
