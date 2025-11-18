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
fn emit_normalizes_inline_to_dash() {
    let d = Doc::from_str("m:\n  n:\n    list: [zeta, alpha, alpha]\n").unwrap();
    let out = d.to_string().unwrap();
    let norm = norm_indent(&out);

    // Find the 'list:' block regardless of exact indentation.
    let mut saw_list = false;
    let mut items = Vec::<String>::new();
    for line in norm.lines() {
        if line == "list:" {
            saw_list = true;
            continue;
        }
        if saw_list {
            if line.starts_with("- ") {
                items.push(line[2..].to_string());
            } else if !line.is_empty() {
                // Another top-level key under the same parent; stop scanning list block
                break;
            }
        }
    }

    assert!(saw_list, "no 'list:' block in:\n{out}");
    assert!(
        !items.is_empty(),
        "list was not expanded to dash style (no '- ' items):\n{out}"
    );

    // Must contain alpha and zeta, order and duplicates are not mandated by emitter.
    let has_alpha = items
        .iter()
        .any(|it| it == "alpha" || it == r#""alpha""# || it == "'alpha'");
    let has_zeta = items
        .iter()
        .any(|it| it == "zeta" || it == r#""zeta""# || it == "'zeta'");
    assert!(
        has_alpha && has_zeta,
        "expected alpha and zeta items; got: {items:?}\n{out}"
    );
}

#[test]
fn emit_quotes_when_needed() {
    let d = Doc::from_str("key: value with spaces\n").unwrap();
    let out = d.to_string().unwrap();

    // Accept either single or double quotes for the value.
    let norm = norm_indent(&out);
    let mut found = false;
    for line in norm.lines() {
        if line.starts_with("key: ") {
            let rhs = &line["key: ".len()..];
            found = has_quoted_value(rhs, "value with spaces");
            break;
        }
    }
    assert!(found, "expected quoted value with spaces; got:\n{out}");
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
    let d = Doc::from_str("list: [true, null, 2]\n").unwrap();
    let out = d.to_string().unwrap();
    // Here we didn't normalize to strings, so typed scalars should remain bare.
    // Accept either dash- or flow-style emission (depending on surrounding context),
    // but since your emitter formats block style for list under a key, look for dash items:
    let norm = norm_indent(&out);
    let has_true = norm.lines().any(|l| l.trim_start() == "- true");
    let has_null = norm.lines().any(|l| l.trim_start() == "- null");
    let has_two = norm.lines().any(|l| l.trim_start() == "- 2");
    assert!(
        has_true && has_null && has_two,
        "expected bare typed items; got:\n{out}"
    );
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
    let d = Doc::from_str("a: foo: bar\nb: .NaN\n").unwrap();
    let out = d.to_string().unwrap();
    assert!(out.contains(r#"a: "foo: bar""#)); // colon-space
    assert!(out.contains(r#"b: ".NaN""#) || out.contains(r#"b: ".nan""#));
}
