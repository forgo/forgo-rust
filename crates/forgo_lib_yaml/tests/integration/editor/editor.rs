//! Editor API Integration Tests
//!
//! **Related YAML 1.2.2 Spec Sections:**
//! - §3.2: Information Models (document structure)
//! - §6-8: YAML syntax (for editing operations)
//!
//! **Purpose:**
//! These integration tests validate the editor API for programmatic YAML manipulation.
//! They test operations like path-based editing, element insertion/deletion, and
//! structure normalization across multiple components. The editor API is not part of
//! the YAML spec but is a high-level interface for working with YAML documents
//! programmatically.

use forgo_lib_yaml::{Doc, Elem, Node, Scalar, Seg, normalize_string_list};

fn doc(s: &str) -> Doc {
    Doc::from_str(s).unwrap()
}

/// Collapse indentation differences so assertions don't depend on exact spaces.
fn normalize_indent(s: &str) -> String {
    s.lines()
        .map(|l| l.trim_start()) // drop leading spaces on each line
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn edit_and_normalize_projects() {
    let mut d = doc(r#"
jobs:
  x:
    strategy:
      matrix:
        project: [forgo, tooling]
  y:
    strategy:
      matrix:
        project:
          - forgo_lib_cli
          - forgo
"#);

    let path: &[Seg] = &[
        "jobs".into(),
        Seg::Wildcard,
        "strategy".into(),
        "matrix".into(),
        "project".into(),
    ];

    let mut touches = 0usize;
    let visited = d.visit_sequences_mut(path, |seq| {
        touches += 1;

        // read items through Elem -> Node
        let has_alpha = seq.iter().any(|e| e.node().as_str() == Some("alpha"));
        if !has_alpha {
            // write items as Elem(Node::Scalar(Scalar::Str))
            seq.push(Elem::new(Node::Scalar(Scalar::Str("alpha".into()))));
        }

        normalize_string_list(seq)
    });

    assert!(visited);
    assert_eq!(touches, 2);

    let out = d.to_string().unwrap();
    // alpha + sorted list (spacing-insensitive)
    let norm = normalize_indent(&out);
    // x was a flow list and stays one; y was a block list and stays one.
    assert!(
        norm.contains("project: [alpha, forgo, tooling]\n"),
        "unexpected YAML output:\n{out}"
    );
    assert!(
        // ASCII order puts "forgo" before "forgo_lib_cli"
        norm.contains("project:\n- alpha\n- forgo\n- forgo_lib_cli"),
        "unexpected YAML output:\n{out}"
    );
    // both matrices should have gained "alpha" (one flow, one block list)
    let back = Doc::from_str(&out).unwrap();
    let mut seen = 0;
    let mut probe = back.clone();
    probe.visit_sequences_mut(path, |seq| {
        seen += usize::from(seq.iter().any(|e| e.node().as_str() == Some("alpha")));
        false
    });
    assert_eq!(seen, 2, "unexpected YAML output:\n{out}");
}

#[test]
fn removing_items() {
    let mut d = doc("list:\n  - a\n  - b\n  - c\n");

    let path: &[Seg] = &["list".into()];

    d.visit_sequences_mut(path, |seq| {
        // retain based on Elem -> Node string view
        seq.retain(|e| e.node().as_str() != Some("b"));
        true
    });

    let out = d.to_string().unwrap();
    // spacing-insensitive removal check
    let norm = normalize_indent(&out);
    assert!(!norm.contains("\n- b\n"), "unexpected YAML output:\n{out}");
}

#[test]
fn missing_path_is_noop() {
    let mut d = doc("root: ok\n");
    let visited = d.visit_sequences_mut(&["nope"], |_s| true);
    assert!(!visited);
}

#[test]
fn consistent_indentation() {
    let d = doc("root:\n  list:\n    - a\n    - b\n");
    let out = d.to_string().unwrap();
    // Count spaces before '-'; they should all be equal.
    let indents: Vec<_> = out
        .lines()
        .filter(|l| l.trim_start().starts_with('-'))
        .map(|l| l.len() - l.trim_start().len())
        .collect();
    assert!(indents.windows(2).all(|w| w[0] == w[1]));
}
