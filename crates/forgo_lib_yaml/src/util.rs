// crates/forgo_lib_yaml/src/util.rs
fn is_yaml_dot_keyword(s: &str) -> bool {
    // YAML 1.1 style float keywords. We quote them to keep string semantics.
    // Accepts any case: .nan, .NaN, .NAN, .inf, +.inf, -.inf, etc.
    let low = s.to_ascii_lowercase();
    matches!(low.as_str(), ".nan" | ".inf" | "+.inf" | "-.inf")
}

/// Return true if string must be quoted in YAML to be unambiguous.
/// Besides whitespace/colon/leading-dash, we also quote anything that YAML
/// would otherwise parse as a non-string scalar (numbers, true/false/null/~).
pub fn needs_quotes(s: &str) -> bool {
    if s.is_empty() {
        return true;
    }

    // Leading/trailing whitespace
    if s.trim() != s {
        return true;
    }

    // Any internal ASCII space requires quotes
    if s.contains(' ') {
        return true;
    }

    // Any control-ish stuff? (tabs/newlines)
    if s.chars().any(|c| matches!(c, '\n' | '\r' | '\t')) {
        return true;
    }

    // Comment / inline-comment hazards
    if s.contains('#') {
        return true;
    }

    // YAML keywords that would change type if left plain
    match s {
        "null" | "Null" | "NULL" | "~" | "true" | "True" | "TRUE" | "false" | "False" | "FALSE" => {
            return true;
        }
        _ => {}
    }

    // YAML 1.1-style "dot" float keywords (.nan/.inf with optional sign)
    if is_yaml_dot_keyword(s) {
        return true;
    }

    // Numeric-looking literals: keep them quoted if you want stable string type
    if is_number_like(s) {
        return true;
    }

    // A leading "- " is hazardous (could look like a list item if projected)
    if s.starts_with("- ") {
        return true;
    }

    // Trailing colon looks like a mapping key if reflowed: quote it
    if s.ends_with(':') {
        return true;
    }

    // *** Colon rule ***
    // Only a colon *followed by space* is hazardous and must be quoted.
    // A glued colon (e.g., "k:v") is allowed as a plain scalar.
    if s.contains(": ") {
        return true;
    }

    // Otherwise fine — allow Unicode and e.g. "k:v" as plain.
    false
}

pub fn is_number_like(text: &str) -> bool {
    // Strict but simple: [-+]?\d+(\.\d+)?([eE][-+]?\d+)?  (no hex/oct/bin)
    let bytes = text.as_bytes();
    if bytes.is_empty() {
        return false;
    }
    let mut i = 0;
    if matches!(bytes[0], b'+' | b'-') {
        i += 1;
    }
    let mut digits = 0usize;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
        digits += 1;
    }
    if digits == 0 {
        return false;
    }
    if i < bytes.len() && bytes[i] == b'.' {
        i += 1;
        let mut frac = 0usize;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
            frac += 1;
        }
        if frac == 0 {
            return false;
        }
    }
    if i < bytes.len() && (bytes[i] == b'e' || bytes[i] == b'E') {
        i += 1;
        if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
            i += 1;
        }
        let mut ed = 0usize;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
            ed += 1;
        }
        if ed == 0 {
            return false;
        }
    }
    i == bytes.len()
}

/// Count leading spaces.
pub fn leading_spaces(s: &str) -> usize {
    s.chars().take_while(|c| *c == ' ').count()
}

/// Any character that can appear in a plain (unquoted) scalar run.
/// We exclude whitespace and YAML indicator/delimiter characters that
/// we already lex as their own tokens, as well as forbidden control characters.
pub fn is_plain_scalar_char(c: char) -> bool {
    // whitespace or line breaks end the plain token
    if matches!(c, ' ' | '\t' | '\n' | '\r') {
        return false;
    }
    // characters that have dedicated tokens in our lexer:
    // Note: '-' is allowed in plain scalars (e.g., "not-date", "2002-04-28")
    // The lexer handles '- ' (dash-space) as a special list indicator token
    if matches!(
        c,
        ':' | '[' | ']' | '{' | '}' | ',' | '|' | '>' | '+' | '&' | '*' | '#' | '"' | '\''
    ) {
        return false;
    }
    // Forbidden control characters per YAML 1.2.2 Section 5.8
    if crate::char_validator::is_forbidden_control_char(c) {
        return false;
    }
    // everything else, including all Unicode letters/digits/marks/punctuation, is fine.
    true
}
