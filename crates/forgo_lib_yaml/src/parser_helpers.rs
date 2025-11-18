// crates/forgo_lib_yaml/src/parser_helpers.rs
//! Helper functions for YAML parsing.

use crate::ast::Scalar;
use crate::lexer::StrTok;
use crate::util::is_number_like;

/// Classify a StrTok as a typed Scalar.
pub(crate) fn classify_scalar(s: &StrTok) -> Scalar {
    classify_text_as_scalar(&s.text, s.quoted)
}

/// Classify plain text as a typed Scalar.
/// Per YAML 1.2 Core Schema, boolean and null recognition is case-sensitive.
pub(crate) fn classify_text_as_scalar(text: &str, was_quoted: bool) -> Scalar {
    if was_quoted {
        return Scalar::Str(text.to_string());
    }
    // YAML 1.2 Core Schema: case-sensitive recognition
    // Only lowercase variants are booleans/null
    if text == "true" {
        return Scalar::Bool(true);
    }
    if text == "false" {
        return Scalar::Bool(false);
    }
    if text == "null" || text == "~" {
        return Scalar::Null;
    }
    if is_number_like(text) {
        return Scalar::Num {
            text: text.to_string(),
        };
    }
    Scalar::Str(text.to_string())
}

/// Trim quotes from a string (used for inline arrays).
// #[allow(dead_code)]
// pub(crate) fn trim_quotes_like(s: &str) -> String {
//     let t = s.trim();
//     if t.len() >= 2 && t.starts_with('"') && t.ends_with('"') {
//         t[1..t.len() - 1].to_string()
//     } else {
//         t.to_string()
//     }
// }

/// Strip space before colons (e.g., "key :" -> "key:").
pub(crate) fn strip_space_before_colon(s: &mut String) {
    let mut out = String::with_capacity(s.len());
    let mut prev_space = false;

    for ch in s.chars() {
        if ch == ':' && prev_space {
            out.pop();
            out.push(':');
            prev_space = false;
        } else {
            out.push(ch);
            prev_space = ch == ' ';
        }
    }

    *s = out;
}

/// Apply chomping indicator to final block scalar text.
/// - '-': strip all trailing newlines
/// - '+': keep all trailing newlines
/// - None or other: strip trailing newlines and add exactly one
pub(crate) fn apply_chomping(mut s: String, chomp: Option<char>) -> String {
    match chomp {
        Some('-') => {
            while s.ends_with('\n') {
                s.pop();
            }
            s
        }
        Some('+') => s,
        Some(_) => {
            while s.ends_with('\n') {
                s.pop();
            }
            s.push('\n');
            s
        }
        None => {
            while s.ends_with('\n') {
                s.pop();
            }
            s.push('\n');
            s
        }
    }
}

/// Fold text for folded block scalars (`>`).
/// Preserves "more-indented" lines (leading space) as hard breaks.
/// Blank lines also preserved as hard breaks.
/// Otherwise, single newlines fold to space.
pub(crate) fn fold_text(lines: Vec<String>) -> String {
    let mut out = String::new();

    let is_more = |s: &String| -> bool { s.starts_with(' ') };

    for (i, line) in lines.iter().enumerate() {
        if i > 0 {
            let prev = &lines[i - 1];
            let prev_blank = prev.is_empty();
            let curr_blank = line.is_empty();
            let prev_more = is_more(prev);
            let curr_more = is_more(line);

            if prev_blank || curr_blank || prev_more || curr_more {
                out.push('\n'); // keep a real newline
            } else {
                out.push(' '); // fold single newline to space
            }
        }
        out.push_str(line);
    }
    out
}
