//! Document Marker Parsing Tests
//!
//! **Related YAML 1.2.2 Spec Sections:**
//! - §9.1.2: Document Markers (--- and ...)
//! - §9.1.3: Bare Documents
//! - §9.2: Streams
//! - §6.8.1: Block Scalar Context (forbids markers)
//!
//! **Purpose:**
//! These are unit tests for document marker parsing. They test implementation-specific
//! behavior and edge cases not explicitly covered by the spec. Unlike spec
//! tests which validate compliance, these tests validate internal correctness
//! and error handling.

use forgo_lib_yaml::{Doc, Error, Node};

/// Indented `---`/`...` are plain scalars (markers only count at column 0),
/// so these inputs fail because of what follows them, not as markers.
fn assert_block_err(s: &str) {
    match Doc::from_str(s) {
        Ok(doc) => {
            panic!(
                "expected a parse error, but parsing succeeded.\n\nEmitted:\n{}",
                doc.to_string().unwrap_or_else(|_| "<emit error>".into())
            );
        }
        Err(Error::Syntax { msg, .. }) => {
            assert!(!msg.is_empty());
        }
        Err(e) => panic!("expected a syntax error, got: {e:?}"),
    }
}

#[test]
fn indented_doc_markers_with_comments_should_error() {
    // Indented '---' / '...' under a mapping value must error, even with trailing spaces/comments.
    assert_block_err("a:\n  ---   # nope\n  c: 2\n");
    assert_block_err("a:\n  ...   # nope\n  c: 2\n");
}

#[test]
fn indented_marker_like_junk_is_not_a_marker() {
    // '----' or '...x' should be treated as plain scalars, not markers.
    let d = Doc::from_str("a:\n  ----\nc: 2\n").expect("parse");
    let Node::Map(m) = d.root().node() else {
        panic!("root not map");
    };
    assert_eq!(m.len(), 2);
    assert_eq!(m[0].0, "a");
    assert_eq!(m[0].1.node().as_str(), Some("----"));
    let d2 = Doc::from_str("a:\n  ...x\nc: 2\n").expect("parse");
    let Node::Map(m2) = d2.root().node() else {
        panic!("root not map");
    };
    assert_eq!(m2.len(), 2);
    assert_eq!(m2[0].0, "a");
    assert_eq!(m2[0].1.node().as_str(), Some("...x"));
}

#[test]
fn nested_indented_marker_is_a_plain_scalar() {
    // `---` only marks a document at column 0; indented, it is a scalar.
    let d = Doc::from_str("a:\n  b:\n    ---\n").expect("parse");
    let Node::Map(m) = d.root().node() else {
        panic!("root not map");
    };
    let Node::Map(inner) = m[0].1.node() else {
        panic!("a not map");
    };
    assert_eq!(inner[0].1.node().as_str(), Some("---"));
}

#[test]
fn top_level_markers_allow_comments_and_multiple_docs() {
    let s = "--- # start\nk: v\n...\n---\nx: y\n";
    let docs = Doc::from_stream(s).expect("parse stream");
    assert_eq!(docs.len(), 2);
    let Node::Map(m0) = docs[0].root().node() else {
        panic!("doc0 root");
    };
    assert_eq!(m0.len(), 1);
    assert_eq!(m0[0].0, "k");
    assert_eq!(m0[0].1.node().as_str(), Some("v"));
    let Node::Map(m1) = docs[1].root().node() else {
        panic!("doc1 root");
    };
    assert_eq!(m1.len(), 1);
    assert_eq!(m1[0].0, "x");
    assert_eq!(m1[0].1.node().as_str(), Some("y"));
}

#[test]
fn indented_four_hyphens_is_scalar() {
    let d = Doc::from_str("a:\n  ----\nc: 2\n").unwrap();
    let Node::Map(m) = d.root().node() else {
        panic!()
    };
    assert_eq!(m[0].0, "a");
    assert_eq!(m[0].1.node().as_str(), Some("----"));
}

#[test]
fn indented_standalone_doc_marker_errors() {
    // The indented `---` is a scalar value, and `c: 2` then continues it as
    // a multi-line implicit key, which is an error.
    assert!(Doc::from_str("a:\n  ---\n  c: 2\n").is_err());
}

#[test]
fn list_item_followed_by_marker_like_is_not_marker() {
    // "- ---" is a list item containing the scalar "---", not a doc marker.
    let d = Doc::from_str("- ---\n").unwrap();
    let Node::Seq(items) = d.root().node() else {
        panic!()
    };
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].node().as_str(), Some("---"));
}
