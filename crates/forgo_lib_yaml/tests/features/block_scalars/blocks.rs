//! Block Scalar Tests
//!
//! **Related YAML 1.2.2 Spec Sections:**
//! - §8.1: Block Scalar Styles (literal `|` and folded `>`)
//! - §8.1.1: Block Scalar Headers (indicators and chomping)
//! - §8.1.1.1: Block Indentation Indicator
//! - §8.1.1.2: Block Chomping Indicator (strip `-`, keep `+`, clip default)
//! - §8.1.2: Literal Style (preserves newlines)
//! - §8.1.3: Folded Style (folds newlines to spaces)
//! - §6.5: Line Folding (how folding works)
//!
//! **Purpose:**
//! These tests validate block scalar parsing, emission, and round-trip behavior.
//! The spec tests in `spec/ch_8_block_styles.rs` cover basic compliance;
//! these tests validate implementation-specific behavior like trailing space
//! handling, chomping semantics, and edge cases.

use forgo_lib_yaml;

fn show(s: &str) -> String {
    // Make whitespace/newlines visible so we can see trailing spaces, etc.
    let mut out = String::new();
    for (_, ch) in s.chars().enumerate() {
        match ch {
            '\n' => out.push_str("\\n\n"),
            '\t' => out.push_str("\\t"),
            ' ' => out.push('·'), // middle dot for spaces
            _ => out.push(ch),
        }
    }
    // Also include byte dump (handy if some non-printable sneaks in)
    let bytes = s
        .as_bytes()
        .iter()
        .map(|b| format!("{:02X}", b))
        .collect::<Vec<_>>()
        .join(" ");
    format!(
        "--- EMITTED YAML (visible) ---\n{}\n--- BYTES ---\n{}\n",
        out, bytes
    )
}

#[test]
fn literal_block_pipe_preserves_newlines_default_one_final() {
    let input = "msg: |\n  line 1\n  line 2\n";
    let d = forgo_lib_yaml::Doc::from_str(input).unwrap();
    let out = d.to_string().unwrap();

    if !out.contains("msg: |") {
        panic!("expected header not found\n{}", show(&out));
    }
    if !out.contains("line 1\n") {
        panic!("expected 'line 1\\n' not found\n{}", show(&out));
    }
    if !out.contains("line 2\n") {
        panic!("expected 'line 2\\n' not found\n{}", show(&out));
    }
}

#[test]
fn literal_block_chomp_strip() {
    let input = "m: |-\n  a\n";
    let d = forgo_lib_yaml::Doc::from_str(input).unwrap();
    let out = d.to_string().unwrap();

    if !out.contains("m: |") {
        panic!("expected 'm: |' header not found\n{}", show(&out));
    }
    if !out.contains("a\n") {
        panic!("expected 'a\\n' in body\n{}", show(&out));
    }
}

#[test]
fn folded_block_gt_folds_single_newlines() {
    let input = "wrap: >\n  this\n  is\n  one\n  paragraph\n";
    let d = forgo_lib_yaml::Doc::from_str(input).unwrap();
    let v = &d.root().node().as_map().unwrap()[0].1;
    assert_eq!(v.node().as_str(), Some("this is one paragraph\n"));
    let out = d.to_string().unwrap();
    // The folded style survives emission.
    if !out.contains("wrap: >\n  this is one paragraph\n") {
        panic!("expected folded '>' to be preserved\n{}", show(&out));
    }
}

#[test]
fn block_body_trailing_spaces_are_content() {
    // Trailing white space inside a literal block is content: emission
    // keeps the literal style and the exact value.
    let input = "msg: |\n  line 1  \n  line 2\t \n";
    let d = forgo_lib_yaml::Doc::from_str(input).unwrap();
    let out = d.to_string().unwrap();
    if !out.contains("msg: |\n") {
        panic!("expected literal style\n{}", show(&out));
    }
    let d2 = forgo_lib_yaml::Doc::from_str(&out).unwrap();
    assert_eq!(
        d2.root().node().as_map().unwrap()[0].1.node().as_str(),
        Some("line 1  \nline 2\t \n")
    );
}

#[test]
fn new_multiline_values_with_trailing_spaces_are_quoted() {
    // A value not written as a block scalar gets double quotes when a line
    // ends in white space, so the output has no invisible trailing spaces.
    let mut d = forgo_lib_yaml::Doc::from_str("msg: x\n").unwrap();
    d.visit_values_mut(&["msg"], |e| {
        *e = forgo_lib_yaml::Elem::string("line 1  \nline 2\n");
        true
    });
    let out = d.to_string().unwrap();
    for (i, l) in out.lines().enumerate() {
        if l.ends_with(' ') || l.ends_with('\t') {
            panic!("line {} ends with trailing whitespace\n{}", i, show(&out));
        }
    }
    assert!(out.contains(r#"msg: "line 1  \nline 2\n""#), "{out}");
}
