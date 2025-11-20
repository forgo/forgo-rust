//! Block Scalar Basic Tests
//!
//! **Related YAML 1.2.2 Spec Sections:**
//! - §8.1: Block Scalar Styles (literal `|` and folded `>`)
//! - §8.1.1: Block Scalar Headers (indicators and chomping)
//! - §8.1.2: Literal Style
//! - §8.1.3: Folded Style
//! - §6.5: Line Folding
//!
//! **Purpose:**
//! These tests validate basic block scalar functionality including indent
//! indicators and folding behavior. They focus on the most common use cases
//! to ensure correct parsing and emission. More comprehensive edge cases are
//! covered in `block_scalars_strict.rs`, and style preservation is tested in
//! `block_style.rs`.

use forgo_lib_yaml::Doc;

#[test]
fn indent_indicator_and_folding() {
    // Test literal block scalar with explicit indent indicator
    // Indicator 2 means content must have 2 spaces of indentation
    let s = "|2\n  a\n  b\n\n  c\n";

    let d = Doc::from_str(s).unwrap();
    let out = d.to_string().unwrap();

    // The parsed body should be "a\nb\n\nc\n" (no leading spaces - stripped by base_indent)
    // The emitted output will have consistent 2-space indentation for block scalar lines
    assert!(
        out.contains("  a\n  b\n  \n  c\n"),
        "Expected substring '  a\\n  b\\n  \\n  c\\n' (with 2-space indents) in output.\nGot: {:?}",
        out
    );

    // Test folded block scalar
    // Lines fold to space except when blank or more-indented
    let s2 = ">\n foo\n bar\n\n  baz\n";
    let d2 = Doc::from_str(s2).unwrap();
    let out2 = d2.to_string().unwrap();

    // For folded scalars: "foo\nbar" folds to "foo bar", blank line preserved,
    // " baz" (with extra leading space) is preserved as more-indented
    // When emitted, we get 2-space base indent for the folded content
    assert!(
        out2.contains("foo bar"),
        "Expected folding 'foo bar'.\nGot: {:?}",
        out2
    );
}

// Regression test for multi-digit indent indicator validation (Official test 2G84)
// YAML 1.2.2 §8.1.1.1: Indent indicators must be 1-9, not 0 or multi-digit values > 9
#[test]
fn indent_indicator_validation() {
    // Valid: single digit 1-9
    assert!(Doc::from_str("--- |1\n foo\n").is_ok());
    assert!(Doc::from_str("--- |9\n         foo\n").is_ok());

    // Invalid: 0 is not allowed
    assert!(Doc::from_str("--- |0\n  foo\n").is_err());

    // Invalid: multi-digit > 9 is not allowed
    assert!(Doc::from_str("--- |10\n").is_err());
    assert!(Doc::from_str("--- |99\n").is_err());

    // Valid: indicators can come before or after chomping
    assert!(Doc::from_str("--- |2-\n  foo\n").is_ok());
    assert!(Doc::from_str("--- |-2\n  foo\n").is_ok());
    assert!(Doc::from_str("--- |2+\n  foo\n").is_ok());
    assert!(Doc::from_str("--- |+2\n  foo\n").is_ok());
}

#[test]
fn indent_indicator_with_chomping() {
    // Test all combinations of indent indicator and chomping
    let test_cases = vec![
        ("|1-\n foo\n", true),  // indicator 1, strip
        ("|1+\n foo\n", true),  // indicator 1, keep
        ("|2-\n  foo\n", true), // indicator 2, strip
        ("|2+\n  foo\n", true), // indicator 2, keep
        ("|-1\n foo\n", true),  // strip, indicator 1
        ("|+1\n foo\n", true),  // keep, indicator 1
        ("|-2\n  foo\n", true), // strip, indicator 2
        ("|+2\n  foo\n", true), // keep, indicator 2
    ];

    for (yaml, should_pass) in test_cases {
        let result = Doc::from_str(yaml);
        assert_eq!(
            result.is_ok(),
            should_pass,
            "Failed for input: '{}' - expected {}, got {:?}",
            yaml.replace('\n', "\\n"),
            if should_pass { "Ok" } else { "Err" },
            result
        );
    }
}
