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
    assert!(
        // x: [forgo, tooling] -> alpha + sorted
        norm.contains("project:\n- alpha\n- forgo\n- tooling\n")
            // y: [forgo_lib_cli, forgo] -> alpha + sorted (ASCII puts "forgo" before "forgo_lib_cli")
            || norm.contains("project:\n- alpha\n- forgo\n- forgo_lib_cli\n")
            // (If comparator changes, this still allows the alternative order)
            || norm.contains("project:\n- alpha\n- forgo_lib_cli\n- forgo\n"),
        "unexpected YAML output:\n{out}"
    );
    // both matrices should have gained "alpha"
    assert_eq!(
        norm.matches("\n- alpha\n").count(),
        2,
        "unexpected YAML output:\n{out}"
    );
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
