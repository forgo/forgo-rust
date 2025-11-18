// crates/forgo_lib_yaml/src/emitter_helpers.rs
//! Helper functions for YAML emission.

use crate::ast::Scalar;
use crate::util::needs_quotes;

/// Emit a mapping key, quoting if necessary.
pub(crate) fn emit_key(k: &str, out: &mut String) {
    if k == "<<" {
        out.push_str("<<");
        return;
    }
    // In block context, keys containing colons must be quoted
    // because they would be misinterpreted as key-value separators
    if k.contains(':') || needs_quotes(k) {
        emit_quoted(k, out);
    } else {
        out.push_str(k);
    }
}

/// Check if a string should be emitted as a block scalar.
pub(crate) fn is_block_scalar_candidate(s: &str) -> bool {
    s.contains('\n')
}

/// Normalize colon spacing in-place (e.g., " :" -> ":" and ": " -> ":").
pub(crate) fn normalize_colon_spacing_inplace(s: &mut String) {
    let mut out = String::with_capacity(s.len());

    let mut chars = s.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == ' ' {
            if let Some(':') = chars.peek().copied() {
                out.push(':');
                chars.next();
                if let Some(' ') = chars.peek().copied() {
                    chars.next();
                }
                continue;
            }
        }
        if ch == ':' {
            out.push(':');
            if let Some(' ') = chars.peek().copied() {
                chars.next();
            }
            continue;
        }
        out.push(ch);
    }
    *s = out;
}

/// Emit a scalar value with context-aware quoting.
pub(crate) fn emit_scalar_with_ctx(
    s: &Scalar,
    force_quotes: bool,
    _in_seq_item: bool,
    out: &mut String,
) {
    match s {
        Scalar::Str(t) => {
            if force_quotes {
                emit_quoted(t, out);
                return;
            }
            let mut normalized = t.clone();
            normalize_colon_spacing_inplace(&mut normalized);

            if t.contains(": ") && !normalized.contains("://") {
                emit_quoted(t, out);
                return;
            }

            if needs_quotes(&normalized) {
                emit_quoted(t, out);
            } else {
                out.push_str(&normalized);
            }
        }
        Scalar::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Scalar::Num { text } => out.push_str(text),
        Scalar::Null => out.push_str("null"),
    }
}

/// Emit a quoted string with proper escaping.
pub(crate) fn emit_quoted(s: &str, out: &mut String) {
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            other => out.push(other),
        }
    }
    out.push('"');
}

/// Emit indentation (spaces).
pub(crate) fn emit_indent(n: usize, out: &mut String) {
    for _ in 0..n {
        out.push(' ');
    }
}
