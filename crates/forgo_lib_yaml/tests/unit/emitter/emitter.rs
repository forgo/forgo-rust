//! General Emitter Tests
//!
//! **Related YAML 1.2.2 Spec Sections:**
//! - §7: Flow Styles
//! - §8: Block Styles
//! - §10.2: Serialization and presentation
//!
//! **Purpose:**
//! These are unit tests for the YAML emitter. They test implementation-specific
//! behavior and edge cases not explicitly covered by the spec. Unlike spec
//! tests which validate compliance, these tests validate internal correctness
//! and error handling.

use forgo_lib_yaml::Doc;

fn norm_indent(s: &str) -> String {
    s.lines()
        .map(|l| l.trim_start())
        .collect::<Vec<_>>()
        .join("\n")
}

fn has_quoted_value(line: &str, val: &str) -> bool {
    line.contains(&format!(r#""{val}""#)) || line.contains(&format!("'{val}'"))
}

#[test]
fn emit_keeps_inline_flow_list_unless_block_requested() {
    let mut d = Doc::from_str("m:\n  n:\n    list: [zeta, alpha, alpha]\n").unwrap();
    let out = d.to_string().unwrap();
    // A flow list in the source stays a flow list.
    assert!(out.contains("    list: [zeta, alpha, alpha]\n"), "{out}");

    // Requesting block style turns it into dash items.
    let path: &[forgo_lib_yaml::Seg] = &["m".into(), "n".into(), "list".into()];
    d.visit_values_mut(path, |e| {
        e.meta.style = Some(forgo_lib_yaml::Style::Block);
        true
    });
    let out = d.to_string().unwrap();
    assert!(
        out.contains("    list:\n      - zeta\n      - alpha\n      - alpha\n"),
        "{out}"
    );
}

#[test]
fn emit_quotes_when_needed() {
    // Plain text with spaces needs no quotes...
    let d = Doc::from_str("key: value with spaces\n").unwrap();
    assert_eq!(d.to_string().unwrap(), "key: value with spaces\n");
    // ...but a value that would read back differently does.
    let mut d = Doc::from_str("key: x\n").unwrap();
    d.visit_values_mut(&["key"], |e| {
        *e = forgo_lib_yaml::Elem::string("a: b");
        true
    });
    let out = d.to_string().unwrap();
    let norm = norm_indent(&out);
    let rhs = norm.lines().find_map(|l| l.strip_prefix("key: ")).unwrap();
    assert!(has_quoted_value(rhs, "a: b"), "got:\n{out}");
}

#[test]
fn roundtrip_map_seq_scalar() {
    let s = "jobs:\n  build:\n    strategy:\n      matrix:\n        project: [b, a, a]\n";
    let d1 = Doc::from_str(s).unwrap();
    let s2 = d1.to_string().unwrap();
    let d2 = Doc::from_str(&s2).unwrap();
    assert_eq!(d1, d2);
}

#[test]
fn emit_quotes_for_yaml_keywords() {
    // Typed scalars stay bare (and the flow list stays flow).
    let d = Doc::from_str("list: [true, null, 2]\n").unwrap();
    assert_eq!(d.to_string().unwrap(), "list: [true, null, 2]\n");
    // Strings that look like keywords are quoted.
    let d = Doc::from_str("list: ['true', 'null', '2']\n").unwrap();
    let out = d.to_string().unwrap();
    assert_eq!(out, "list: ['true', 'null', '2']\n");
}

#[test]
fn emit_block_scalar_for_multiline_strings() {
    let d = Doc::from_str("k: |\n  line1\n  line2\n").unwrap();
    let out = d.to_string().unwrap();
    assert!(out.contains("k: |"));
    assert!(out.contains("\n  line1\n"));
    assert!(out.contains("\n  line2\n"));
}

#[test]
fn quotes_colon_space_and_dot_keywords() {
    // `a: foo: bar` is invalid YAML (a mapping can't start on a key's line),
    // so set the string values directly and check the emitter quotes them.
    let mut d = Doc::from_str("a: x\nb: y\n").unwrap();
    d.visit_values_mut(&["a"], |e| {
        *e = forgo_lib_yaml::Elem::string("foo: bar");
        true
    });
    d.visit_values_mut(&["b"], |e| {
        *e = forgo_lib_yaml::Elem::string(".NaN");
        true
    });
    let out = d.to_string().unwrap();
    assert!(out.contains(r#"a: "foo: bar""#), "{out}"); // colon-space
    assert!(out.contains(r#"b: ".NaN""#), "{out}"); // would otherwise be a float
    // Plain `.NaN` in the source is a float under the core schema.
    let f = Doc::from_str("b: .NaN\n").unwrap();
    assert!(
        f.root().node().as_map().unwrap()[0]
            .1
            .node()
            .as_f64()
            .unwrap()
            .is_nan()
    );
}
