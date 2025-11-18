use forgo_lib_yaml::{Lexer, Tok};

#[test]
fn test_unterminated_double_quote_lexer() {
    let input = r#"key: "unterminated"#;
    let mut lx = Lexer::new(input);
    let mut found_error = false;
    loop {
        let tok = lx.next_token();
        if matches!(tok, Tok::Error(_)) {
            found_error = true;
            break;
        }
        if matches!(tok, Tok::Eof) {
            break;
        }
    }
    assert!(found_error, "Should have found Tok::Error for unterminated double quote");
}

#[test]
fn test_unterminated_single_quote_lexer() {
    let input = r#"key: 'unterminated"#;
    let mut lx = Lexer::new(input);
    let mut found_error = false;
    loop {
        let tok = lx.next_token();
        if matches!(tok, Tok::Error(_)) {
            found_error = true;
            break;
        }
        if matches!(tok, Tok::Eof) {
            break;
        }
    }
    assert!(found_error, "Should have found Tok::Error for unterminated single quote");
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
