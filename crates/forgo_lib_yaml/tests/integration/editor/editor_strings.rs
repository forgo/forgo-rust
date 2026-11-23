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

#[test]
fn normalize_list_with_typed_values() {
    let mut d = Doc::from_str(
        r#"
list: [true, 2, "x", null, true]
"#,
    )
    .unwrap();

    let path: &[Seg] = &["list".into()];
    d.visit_sequences_mut(path, |seq| normalize_string_list(seq));
    let out = d.to_string().unwrap();

    // Everything becomes a string, sorted and deduped; the flow style stays.
    // Strings that would read back as other types are quoted.
    assert!(out.contains(r#"list: ["2", "null", "true", x]"#), "{out}");
    let back = Doc::from_str(&out).unwrap();
    let items: Vec<_> = back.root().node().as_map().unwrap()[0]
        .1
        .node()
        .as_seq()
        .unwrap()
        .iter()
        .map(|e| e.node().as_str().map(str::to_string))
        .collect();
    assert_eq!(
        items,
        ["2", "null", "true", "x"].map(|s| Some(s.to_string()))
    );
}

#[test]
fn normalize_handles_exponents_negatives_and_prefixes() {
    let mut d = Doc::from_str(r#"list: ["01", -3, 1.0e3, "5", x]"#).unwrap();
    let path: &[Seg] = &["list".into()];
    d.visit_sequences_mut(path, |seq| normalize_string_list(seq));
    let out = d.to_string().unwrap();
    // All strings now; ones that look like numbers must be quoted.
    for quoted in ["01", "-3", "5"] {
        assert!(out.contains(&format!("\"{quoted}\"")), "{out}");
    }
    assert!(
        out.contains("\"1.0e3\"") || out.contains("\"1.0E3\""),
        "{out}"
    );
    let back = Doc::from_str(&out).unwrap();
    for e in back.root().node().as_map().unwrap()[0]
        .1
        .node()
        .as_seq()
        .unwrap()
    {
        assert!(e.node().as_str().is_some(), "not a string: {e:?}\n{out}");
    }
}
