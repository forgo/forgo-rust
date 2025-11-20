//! Comment Emission Tests
//!
//! **Related YAML 1.2.2 Spec Sections:**
//! - §6.6: Comments (s-b-comment)
//! - §3.2.3.3: Comments in presentation stream
//! - Throughout §7 (Flow Styles): Comments in flow collections
//! - Throughout §8 (Block Styles): Comments in block collections
//!
//! **Purpose:**
//! These are unit tests for comment emission. They test implementation-specific
//! behavior and edge cases not explicitly covered by the spec. Unlike spec
//! tests which validate compliance, these tests validate internal correctness
//! and error handling.

use forgo_lib_yaml::Doc;

fn doc(s: &str) -> Doc {
    Doc::from_str(s).expect("parse")
}

#[test]
fn trailing_comment_on_scalar_value_is_preserved() {
    let d = doc("k: v # t\n");
    let out = d.to_string().unwrap();
    assert!(out.contains("k: v # t"), "{out}");
}

#[test]
fn leading_comment_before_key_is_preserved() {
    let d = doc("# header\nk: v\n");
    let out = d.to_string().unwrap();
    assert!(out.starts_with("# header\nk: v"), "{out}");
}

#[test]
fn seq_item_trailing_comment_is_preserved() {
    let d = doc("- 1 # a\n- 2 # b\n");
    let out = d.to_string().unwrap();
    assert!(out.contains("- 1 # a"), "{out}");
    assert!(out.contains("- 2 # b"), "{out}");
}

#[test]
fn trailing_comment_does_not_force_quotes_for_simple_value() {
    let d = Doc::from_str("k: v # t\n").unwrap();
    let out = d.to_string().unwrap();
    assert!(out.contains("k: v # t"), "{out}");
}

#[test]
fn trailing_comment_keeps_quotes_when_needed() {
    // value contains a colon -> must be quoted
    let d = Doc::from_str("k: a: b # t\n").unwrap();
    let out = d.to_string().unwrap();
    assert!(
        out.contains("k: \"a: b\" # t") || out.contains("k: 'a: b' # t"),
        "{out}"
    );
}

#[test]
fn seq_item_trailing_comment_roundtrip() {
    let d = Doc::from_str("- 1 # a\n- 2 # b\n").unwrap();
    let out = d.to_string().unwrap();
    assert!(out.contains("- 1 # a"), "{out}");
    assert!(out.contains("- 2 # b"), "{out}");
}
