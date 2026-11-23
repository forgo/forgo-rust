//! Lexer Edge Case Tests
//!
//! **Related YAML 1.2.2 Spec Sections:**
//! - §5: Characters
//! - §5.4: Line Break Characters
//! - §5.2: Character Encodings (BOM handling)
//! - §6.2: Indentation Spaces (tab rejection)
//!
//! **Purpose:**
//! These are unit tests for lexer edge cases. They test implementation-specific
//! behavior and edge cases not explicitly covered by the spec. Unlike spec
//! tests which validate compliance, these tests validate internal correctness
//! and error handling.

use forgo_lib_yaml::Doc;

#[test]
fn ignores_utf8_bom() {
    let s = "\u{FEFF}key: val\n";
    let d = Doc::from_str(s).unwrap();
    let out = d.to_string().unwrap();
    assert!(out.starts_with("key: val"));
}

#[test]
fn rejects_leading_tab_indent() {
    let s = "k:\n\t- x\n"; // tab before dash
    let d = Doc::from_str(s);
    assert!(d.is_err(), "expected parse error on tab indentation");
}

#[test]
fn handles_crlf_as_newline() {
    let s = "a: 1\r\nb: 2\r\n";
    let d = Doc::from_str(s).unwrap();
    assert_eq!(d.root().node().as_map().unwrap().len(), 2);
    // CRLF input is written back with CRLF line breaks.
    let out = d.to_string().unwrap();
    assert_eq!(out, "a: 1\r\nb: 2\r\n");
}

#[test]
fn handles_bare_cr_as_newline() {
    let s = "x: 1\ry: 2\r";
    let d = Doc::from_str(s).unwrap();
    assert_eq!(d.root().node().as_map().unwrap().len(), 2);
    // CR line breaks are written back as CRLF.
    let out = d.to_string().unwrap();
    assert_eq!(out, "x: 1\r\ny: 2\r\n");
}
