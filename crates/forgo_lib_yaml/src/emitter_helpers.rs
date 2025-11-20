// crates/forgo_lib_yaml/src/emitter_helpers.rs
//! Helper functions for YAML emission.

use crate::ast::{Elem, MapKey, Node, Scalar};
use crate::util::needs_quotes;

/// Emit a mapping key, quoting if necessary.
/// For complex keys (sequences/maps), emits using explicit `?` syntax in flow style.
pub(crate) fn emit_key(k: &MapKey, out: &mut String) {
    if let Some(s) = k.as_str() {
        // String key
        if s == "<<" {
            out.push_str("<<");
            return;
        }
        // In block context, keys containing colons must be quoted
        // because they would be misinterpreted as key-value separators
        if s.contains(':') || needs_quotes(s) {
            emit_quoted(s, out);
        } else {
            out.push_str(s);
        }
    } else if let Some(elem) = k.as_elem() {
        // Complex key - emit using explicit `?` syntax in flow style
        out.push_str("? ");
        emit_elem_flow(elem, out);
    }
}

/// Emit an element in flow style (for complex keys)
fn emit_elem_flow(elem: &Elem, out: &mut String) {
    match &elem.node {
        Node::Seq(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                emit_elem_flow(item, out);
            }
            out.push(']');
        }
        Node::Map(entries) => {
            out.push('{');
            for (i, (k, v)) in entries.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                // Recursively emit key in flow style
                if let Some(s) = k.as_str() {
                    if s.contains(':') || s.contains(',') || needs_quotes(s) {
                        emit_quoted(s, out);
                    } else {
                        out.push_str(s);
                    }
                } else if let Some(key_elem) = k.as_elem() {
                    out.push_str("? ");
                    emit_elem_flow(key_elem, out);
                    out.push_str(" :");
                    out.push(' ');
                    emit_elem_flow(v, out);
                    continue;
                }
                out.push_str(": ");
                emit_elem_flow(v, out);
            }
            out.push('}');
        }
        Node::Scalar(Scalar::Str(s)) => {
            if s.contains(':') || s.contains(',') || s.contains('[') || s.contains(']') || s.contains('{') || s.contains('}') || needs_quotes(s) {
                emit_quoted(s, out);
            } else {
                out.push_str(s);
            }
        }
        Node::Scalar(s) => {
            out.push_str(&s.to_string());
        }
        Node::Alias(a) => {
            out.push('*');
            out.push_str(a);
        }
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

            // Check if string needs quoting to avoid ambiguity
            // Per YAML 1.2.2 §7.3.3, colons in plain scalars are allowed when
            // NOT followed by whitespace (e.g., "k:v", "http://example.com")
            //
            // Only quote if contains ": " (colon-space) which could be interpreted
            // as a key-value separator, UNLESS it's clearly a URL pattern

            let has_colon_space = t.contains(": ");

            // Quote if we have ": " (colon-space), unless it's clearly a URL
            if has_colon_space && !t.contains("://") {
                emit_quoted(t, out);
                return;
            }

            if needs_quotes(t) {
                emit_quoted(t, out);
            } else {
                out.push_str(t);
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
