//! Anchor and Alias Emission Tests
//!
//! **Related YAML 1.2.2 Spec Sections:**
//! - §6.9.2: Anchor (c-ns-anchor-property)
//! - §7.1: Alias nodes (c-ns-alias-node)
//! - §9.1: Documents with anchors and aliases
//!
//! **Purpose:**
//! These are unit tests for anchor and alias emission. They test implementation-specific
//! behavior and edge cases not explicitly covered by the spec. Unlike spec
//! tests which validate compliance, these tests validate internal correctness
//! and error handling.

use forgo_lib_yaml::Doc;

fn doc(s: &str) -> Doc {
    Doc::from_str(s).expect("parse")
}

#[test]
fn inline_alias_as_map_value_is_emitted_inline() {
    let d = doc("def: &a 1\nref: *a\n");
    let out = d.to_string().unwrap();
    assert!(out.contains("ref: *a"), "{out}");
}

#[test]
fn anchor_on_seq_item_and_alias_roundtrip() {
    let d = doc("- &x 1\n- *x\n");
    let out = d.to_string().unwrap();
    assert!(out.contains("- &x 1"), "{out}");
    assert!(out.contains("- *x"), "{out}");
    // roundtrip stable
    let d2 = Doc::from_str(&out).unwrap();
    assert_eq!(d, d2);
}

#[test]
fn merge_key_is_emitted_as_inline_merge() {
    let d = doc("base: &def\n  a: 1\n  b: 2\n\nobj:\n  <<: *def\n  c: 3\n");
    let out = d.to_string().unwrap();
    assert!(out.contains("<<: *def"), "{out}");
}
