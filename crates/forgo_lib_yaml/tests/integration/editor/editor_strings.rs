//! Editor String Normalization Tests
//!
//! **Related YAML 1.2.2 Spec Sections:**
//! - §7.4.1: Flow Sequences (target format for normalization)
//! - §8.2.1: Block Sequences (source format)
//! - §7.3: Flow Scalar Styles (quoting behavior)
//!
//! **Purpose:**
//! These integration tests validate the editor API's string list normalization
//! functionality, which converts between block and flow sequence representations.
//! This tests the `normalize_string_list` utility function across different
//! quoting scenarios and formats.

use forgo_lib_yaml::{Doc, Seg, normalize_string_list};

fn normalize_indent(s: &str) -> String {
    s.lines()
        .map(|l| l.trim_start())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Accept single- or double-quoted items, and no indentation.
fn has_quoted_item(norm: &str, val: &str) -> bool {
    norm.contains(&format!(r#"- "{}""#, val)) || norm.contains(&format!("- '{}'", val))
}

/// Accept a bare list line "- val" regardless of whether there's a trailing '\n'.
fn has_bare_item(norm: &str, val: &str) -> bool {
    norm.lines().any(|l| l.trim_start() == format!("- {}", val))
}

#[test]
fn normalize_list_with_typed_values() {
    let mut d = Doc::from_str(
        r#"
list: [true, 2, "x", null, true]
"#,
    )
    .unwrap();

    // Make the path type explicit so `"list".into()` resolves to Seg
    let path: &[Seg] = &["list".into()];
    d.visit_sequences_mut(path, |seq| normalize_string_list(seq));

    let out = d.to_string().unwrap();
    let norm = normalize_indent(&out);

    // After normalization, everything becomes strings, sorted & deduped.
    // Expected order (ASCII sort): "2", "null", "true", "x"
    // Emitter may quote keywords/numbers; check for the key bits robustly.
    assert!(norm.contains("list:\n"));
    assert!(
        has_quoted_item(&norm, "2"),
        "expected quoted numeric string:\n{out}"
    );

    // "null" and "true" should be strings now; emitter is allowed to quote them.
    // We accept either quoted or (if your needs_quotes changes later) unquoted.
    let has_null = has_quoted_item(&norm, "null") || has_bare_item(&norm, "null");
    let has_true = has_quoted_item(&norm, "true") || has_bare_item(&norm, "true");
    assert!(
        has_null && has_true,
        "normalized list missing null/true as strings:\n{out}"
    );

    // "x" may be bare or quoted.
    assert!(
        has_bare_item(&norm, "x") || has_quoted_item(&norm, "x"),
        "{out}"
    );
}

#[test]
fn normalize_handles_exponents_negatives_and_prefixes() {
    let mut d = Doc::from_str(r#"list: ["01", -3, 1.0e3, "5", x]"#).unwrap();
    let path: &[Seg] = &["list".into()];
    d.visit_sequences_mut(path, |seq| normalize_string_list(seq));
    let out = d.to_string().unwrap();
    let norm = normalize_indent(&out);
    // All as strings; numbers and sign-prefixed should be quoted.
    assert!(has_quoted_item(&norm, "01"), "{out}");
    assert!(has_quoted_item(&norm, "-3"), "{out}");
    assert!(
        has_quoted_item(&norm, "1.0e3") || has_quoted_item(&norm, "1.0E3"),
        "{out}"
    );
    assert!(has_quoted_item(&norm, "5"), "{out}");
    assert!(
        has_bare_item(&norm, "x") || has_quoted_item(&norm, "x"),
        "{out}"
    );
}
