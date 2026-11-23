//! Scalar Emission Tests
//!
//! **Related YAML 1.2.2 Spec Sections:**
//! - §7.3: Flow Scalar Styles
//! - §8.1: Block Scalar Styles
//! - §6.9: Node Properties
//! - §10.3: Core Schema (type resolution)
//!
//! **Purpose:**
//! These are unit tests for scalar emission. They test implementation-specific
//! behavior and edge cases not explicitly covered by the spec. Unlike spec
//! tests which validate compliance, these tests validate internal correctness
//! and error handling.

use forgo_lib_yaml::Doc;

#[test]
fn numeric_text_is_preserved_verbatim() {
    // Leading zeros should not be normalized by emitter; we keep original text.
    let d = Doc::from_str("n: 0012\n").unwrap();
    // Re-emit and ensure we didn't turn it into "12"
    let out = d.to_string().unwrap();
    assert!(out.contains("n: 0012"), "{out}");
}

#[test]
fn booleans_and_null_emit_canonically() {
    let d = Doc::from_str("t: true\nf: false\nz: null\n").unwrap();
    let out = d.to_string().unwrap();
    assert!(out.contains("t: true"));
    assert!(out.contains("f: false"));
    assert!(out.contains("z: null"));
}
