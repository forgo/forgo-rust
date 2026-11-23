//! Quote Character Validation Tests
//!
//! **Related YAML 1.2.2 Spec Sections:**
//! - §7.3.1: Double-Quoted Style
//! - §7.3.2: Single-Quoted Style
//! - §5.7: Escaped Characters
//! - §7.3: Flow Scalar Styles
//!
//! **Purpose:**
//! These are unit tests for quote character validation in the scanner. They test implementation-specific
//! behavior and edge cases not explicitly covered by the spec. Unlike spec
//! tests which validate compliance, these tests validate internal correctness
//! and error handling.

/// The event stream for `input` ends in a syntax error.
fn events_fail(input: &str) -> bool {
    forgo_lib_yaml::events(input).any(|r| r.is_err())
}

#[test]
fn test_unterminated_double_quote_lexer() {
    let input = r#"key: "unterminated"#;
    assert!(
        events_fail(input),
        "unterminated double quote must be a syntax error"
    );
    assert!(forgo_lib_yaml::Doc::from_str(input).is_err());
}

#[test]
fn test_unterminated_single_quote_lexer() {
    let input = r#"key: 'unterminated"#;
    assert!(
        events_fail(input),
        "unterminated single quote must be a syntax error"
    );
    assert!(forgo_lib_yaml::Doc::from_str(input).is_err());
}

#[test]
fn test_extra_closing_bracket_debug() {
    use forgo_lib_yaml::Doc;
    let input = "[a, b]]\n";
    let result = Doc::from_str(input);
    eprintln!("Result for '[a, b]]': {:?}", result);
    assert!(result.is_err(), "Should reject extra closing bracket");
}

#[test]
fn test_null_character_rejected() {
    use forgo_lib_yaml::Doc;
    let input = "key: value\0more\n";
    let result = Doc::from_str(input);
    eprintln!("Result: {:?}", result);
    assert!(result.is_err(), "Should reject NULL character");
}
