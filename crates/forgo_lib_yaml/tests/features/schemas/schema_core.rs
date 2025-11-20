//! Core Schema Tests
//!
//! **Related YAML 1.2.2 Spec Sections:**
//! - §10.1.1.3: Core Schema
//! - §10.3.2: Boolean Type (case-sensitive: `true`, `false`)
//! - §10.3.3: Null Type (case-sensitive: `null`, `~`)
//!
//! **Purpose:**
//! These tests validate that the Core Schema's type resolution rules are
//! correctly implemented, particularly case-sensitivity for boolean and null
//! values. In YAML 1.2, only lowercase `true`/`false`/`null` are recognized
//! as special values; capitalized variants like `True`, `FALSE`, `Null` must
//! be treated as plain strings. This differs from YAML 1.1 behavior.

use forgo_lib_yaml::{Doc, Node, Scalar};

#[test]
fn core_case_sensitive_bools_null() {
    let d = Doc::from_str("k1: true\nk2: false\nk3: null\nk4: ~\nk5: True\nk6: FALSE\nk7: Null\n")
        .unwrap();
    match &d.root().node {
        Node::Map(m) => {
            assert!(matches!(m[0].1.node, Node::Scalar(Scalar::Bool(true))));
            assert!(matches!(m[1].1.node, Node::Scalar(Scalar::Bool(false))));
            assert!(matches!(m[2].1.node, Node::Scalar(Scalar::Null)));
            assert!(matches!(m[3].1.node, Node::Scalar(Scalar::Null)));
            // Capitalized should be strings in YAML 1.2
            assert!(matches!(m[4].1.node, Node::Scalar(Scalar::Str(_))));
            assert!(matches!(m[5].1.node, Node::Scalar(Scalar::Str(_))));
            assert!(matches!(m[6].1.node, Node::Scalar(Scalar::Str(_))));
        }
        _ => panic!("not a map"),
    }
}
