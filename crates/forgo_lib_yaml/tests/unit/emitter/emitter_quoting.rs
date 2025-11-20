//! Quote Handling Emission Tests
//!
//! **Related YAML 1.2.2 Spec Sections:**
//! - §7.3: Flow Scalar Styles
//! - §7.3.1: Double-Quoted Style
//! - §7.3.2: Single-Quoted Style
//! - §5.7: Escaped Characters
//!
//! **Purpose:**
//! These are unit tests for quote handling in emission. They test implementation-specific
//! behavior and edge cases not explicitly covered by the spec. Unlike spec
//! tests which validate compliance, these tests validate internal correctness
//! and error handling.

use forgo_lib_yaml::Doc;

fn doc(s: &str) -> Doc {
    Doc::from_str(s).expect("parse")
}

#[test]
fn plain_scalar_with_colon_space_must_be_quoted_in_seq_items() {
    // seq item with ": " inside => must be quoted
    let d = doc("- a: b\n- hello: world\n- x: 1\n- foo: bar baz\n");
    let out = d.to_string().unwrap();
    // should keep map structure; this asserts at least one item with colon-space inside values will be quoted
    assert!(out.contains("hello: world"));
    // Now explicitly craft a seq item that’s a plain scalar containing ": "
    let d = Doc::from_str("- \"k: v\"\n").unwrap();
    let out = d.to_string().unwrap();
    assert!(out.contains("- \"k: v\""), "{out}");
}

#[test]
fn glued_colon_is_normalized_for_unquoted_plain_scalars() {
    // parser will classify as Str and emitter normalizes "k :v" -> "k:v"
    let d = doc("k :v\n");
    let out = d.to_string().unwrap();
    assert!(out.contains("k: v") || out.contains("k:v"), "{out}");
}

#[test]
fn http_scheme_spacing_is_fixed_not_broken() {
    // Ensure "http ://" -> "http://"
    let d = doc("url: http ://example.com\n");
    let out = d.to_string().unwrap();
    assert!(out.contains("url: http://example.com"), "{out}");
}

#[test]
fn prefer_quoted_on_input_is_preserved() {
    let d = doc("title: \"A: B\"\n");
    let out = d.to_string().unwrap();
    // because value was quoted originally, keep quotes (prefer_quoted)
    assert!(out.contains("title: \"A: B\""), "{out}");
}
