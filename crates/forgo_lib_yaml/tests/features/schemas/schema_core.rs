//! Core Schema Tests
//!
//! **Related YAML 1.2.2 Spec Sections:**
//! - §10.1.1.3: Core Schema
//! - §10.3.2: Tag Resolution (core schema)
//!
//! **Purpose:**
//! These tests validate the Core Schema's resolution of plain scalars. The
//! core schema (unlike the JSON schema) accepts `true|True|TRUE`,
//! `false|False|FALSE` and `null|Null|NULL|~|<empty>`; any other spelling
//! (`tRuE`, `yes`, `on`) is a string. YAML 1.1 words like `yes` are not
//! booleans in YAML 1.2.

use forgo_lib_yaml::{Doc, Node, Scalar};

#[test]
fn core_case_sensitive_bools_null() {
    let d = Doc::from_str(
        "k1: true\nk2: false\nk3: null\nk4: ~\nk5: True\nk6: FALSE\nk7: Null\nk8: tRuE\nk9: yes\nk10: nUll\n",
    )
    .unwrap();
    match &d.root().node {
        Node::Map(m) => {
            assert!(matches!(m[0].1.node, Node::Scalar(Scalar::Bool(true))));
            assert!(matches!(m[1].1.node, Node::Scalar(Scalar::Bool(false))));
            assert!(matches!(m[2].1.node, Node::Scalar(Scalar::Null)));
            assert!(matches!(m[3].1.node, Node::Scalar(Scalar::Null)));
            // The core schema's capitalized spellings
            assert!(matches!(m[4].1.node, Node::Scalar(Scalar::Bool(true))));
            assert!(matches!(m[5].1.node, Node::Scalar(Scalar::Bool(false))));
            assert!(matches!(m[6].1.node, Node::Scalar(Scalar::Null)));
            // Other spellings are strings
            assert!(matches!(m[7].1.node, Node::Scalar(Scalar::Str(_))));
            assert!(matches!(m[8].1.node, Node::Scalar(Scalar::Str(_))));
            assert!(matches!(m[9].1.node, Node::Scalar(Scalar::Str(_))));
        }
        _ => panic!("not a map"),
    }
}
