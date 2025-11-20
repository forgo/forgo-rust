//! Merge Key Tests
//!
//! **Related YAML 1.2.2 Spec Sections:**
//! - §10.1.1.1: Failsafe Schema (merge key extension origin)
//! - §10.1.1.2: JSON Schema (inherits failsafe)
//! - §10.1.1.3: Core Schema (inherits failsafe)
//!
//! **Purpose:**
//! These tests validate that merge keys (`<<: *anchor`) are preserved during
//! round-trip without semantic expansion. While merge keys are a common YAML
//! extension, forgo_lib_yaml treats them as plain key-value pairs, preserving
//! the `<<` key literally. This differs from libraries that expand merge keys
//! into multiple key-value pairs.

use forgo_lib_yaml::Doc;

#[test]
fn merge_key_roundtrips_as_plain() {
    let s = "base: &b {a: 1}\nobj:\n <<: *b\n a: 2\n";
    let d = Doc::from_str(s).unwrap();
    let out = d.to_string().unwrap();
    assert!(out.contains("<<: *b")); // no semantics, just round-trip
}
