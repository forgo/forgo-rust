//! Top-Level Element Emission Tests
//!
//! **Related YAML 1.2.2 Spec Sections:**
//! - §9: Document Stream
//! - §9.1: Document Prefix and Markers
//! - §8.1: Block Scalar Styles (top-level)
//! - §7.3: Flow Scalar Styles (top-level)
//!
//! **Purpose:**
//! These are unit tests for top-level element emission. They test implementation-specific
//! behavior and edge cases not explicitly covered by the spec. Unlike spec
//! tests which validate compliance, these tests validate internal correctness
//! and error handling.

use forgo_lib_yaml::Doc;

#[test]
fn top_level_block_scalar_is_emitted_with_pipe() {
    let d = Doc::from_str("|\n  hi\n  there\n").unwrap();
    let out = d.to_string().unwrap();
    assert!(out.starts_with("|\n"));
    assert!(out.contains("  hi\n"));
    assert!(out.contains("  there\n"));
}

#[test]
fn top_level_quoted_scalar_stays_quoted_when_preferred() {
    let d = Doc::from_str("\"k: v\"\n").unwrap();
    let out = d.to_string().unwrap();
    assert!(out.starts_with("\"k: v\"\n"), "{out}");
}
