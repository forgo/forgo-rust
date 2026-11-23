// crates/forgo_lib_yaml/src/emitter_helpers.rs
//! Helper functions for YAML emission.

use crate::ast::{Elem, MapKey, Meta, Node, Scalar};
use crate::syntax::chars::is_printable;
use crate::util::needs_quotes;

/// How a mapping key was written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum KeyForm {
    /// `key:` on one line.
    Inline,
    /// `? key` — the caller must start `:` on a new line.
    Explicit,
}

/// Write a node's tag and anchor, each followed by a space.
pub(crate) fn write_props(meta: &Meta, out: &mut String) {
    if let Some(t) = &meta.tag {
        out.push_str(t);
        out.push(' ');
    }
    if let Some(a) = &meta.anchor {
        out.push('&');
        out.push_str(a);
        out.push(' ');
    }
}

fn emit_str_key(s: &str, out: &mut String) {
    if s == "<<" {
        out.push_str("<<");
    } else if s.contains(':') || needs_quotes(s) {
        emit_quoted(s, out);
    } else {
        out.push_str(s);
    }
}

/// Emit a mapping key. Scalars and aliases are written inline; collection
/// keys and multi-line strings use the explicit `? ` form.
pub(crate) fn emit_key(k: &MapKey, out: &mut String) -> KeyForm {
    match k {
        MapKey::Str(s) if !s.contains('\n') => {
            emit_str_key(s, out);
            KeyForm::Inline
        }
        MapKey::Str(s) => {
            out.push_str("? ");
            emit_quoted(s, out);
            KeyForm::Explicit
        }
        MapKey::Complex(elem) => match &elem.node {
            Node::Scalar(Scalar::Str(s)) if !s.contains('\n') => {
                write_props(&elem.meta, out);
                emit_str_key(s, out);
                KeyForm::Inline
            }
            Node::Scalar(Scalar::Str(s)) => {
                out.push_str("? ");
                write_props(&elem.meta, out);
                emit_quoted(s, out);
                KeyForm::Explicit
            }
            Node::Scalar(sc) => {
                write_props(&elem.meta, out);
                emit_scalar_with_ctx(sc, false, false, out);
                KeyForm::Inline
            }
            Node::Alias(name) => {
                write_props(&elem.meta, out);
                out.push('*');
                out.push_str(name);
                // `:` is an anchor character: separate it from the name.
                out.push(' ');
                KeyForm::Inline
            }
            Node::Seq(_) | Node::Map(_) => {
                out.push_str("? ");
                emit_elem_flow(elem, out);
                KeyForm::Explicit
            }
        },
    }
}

fn flow_str(s: &str, out: &mut String) {
    let flow_unsafe = s.contains([':', ',', '[', ']', '{', '}', '#']);
    if flow_unsafe || needs_quotes(s) {
        emit_quoted(s, out);
    } else {
        out.push_str(s);
    }
}

/// Emit an element in flow style (used for collection keys).
fn emit_elem_flow(elem: &Elem, out: &mut String) {
    write_props(&elem.meta, out);
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
                match k {
                    MapKey::Str(s) => flow_str(s, out),
                    MapKey::Complex(key) => {
                        let collection = matches!(key.node, Node::Seq(_) | Node::Map(_));
                        if collection {
                            out.push_str("? ");
                        }
                        emit_elem_flow(key, out);
                        if matches!(key.node, Node::Alias(_)) || collection {
                            out.push(' ');
                        }
                    }
                }
                out.push_str(": ");
                emit_elem_flow(v, out);
            }
            out.push('}');
        }
        Node::Scalar(Scalar::Str(s)) => flow_str(s, out),
        Node::Scalar(s) => emit_scalar_with_ctx(s, false, false, out),
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

            // Per YAML 1.2.2 §7.3.3, colons in plain scalars are allowed when
            // NOT followed by whitespace (e.g., "k:v", "http://example.com"),
            // so only ": " (colon-space) needs quotes.
            let has_colon_space = t.contains(": ");
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

/// Emit a double-quoted string, escaping every character that isn't
/// printable (plus `"`, `\`, line breaks, tabs, the BOM and the Unicode
/// line/paragraph separators).
pub(crate) fn emit_quoted(s: &str, out: &mut String) {
    use std::fmt::Write as _;
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            '\0' => out.push_str("\\0"),
            '\u{7}' => out.push_str("\\a"),
            '\u{8}' => out.push_str("\\b"),
            '\u{B}' => out.push_str("\\v"),
            '\u{C}' => out.push_str("\\f"),
            '\u{1B}' => out.push_str("\\e"),
            '\u{85}' => out.push_str("\\N"),
            '\u{2028}' => out.push_str("\\L"),
            '\u{2029}' => out.push_str("\\P"),
            '\u{FEFF}' => out.push_str("\\uFEFF"),
            c if !is_printable(c) => {
                let v = c as u32;
                let _ = if v <= 0xFF {
                    write!(out, "\\x{v:02X}")
                } else {
                    write!(out, "\\u{v:04X}")
                };
            }
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
