// crates/forgo_lib_yaml/src/emitter.rs
//! Block-style emitter for `Doc` trees.
//!
//! Every comment is printed exactly once: a map entry's or sequence item's
//! leading comments before its line, trailing comments at the end of the
//! node's first line, and the root's leading comments by `emit_root`.

use crate::ast::{BlockStyle, Elem, Error, MapKey, Meta, Node, Scalar};
use crate::emitter_helpers::{
    KeyForm, emit_indent, emit_key, emit_quoted, emit_scalar_with_ctx, is_block_scalar_candidate,
    write_props,
};

pub struct Emitter;

fn emit_comment_lines(comments: &[String], indent: usize, out: &mut String) {
    for c in comments {
        emit_indent(indent, out);
        out.push('#');
        if !c.is_empty() {
            out.push(' ');
            out.push_str(c);
        }
        out.push('\n');
    }
}

/// Block style for values written that way in the source, and for other
/// multi-line strings unless a line ends in white space (which a block
/// scalar would carry invisibly; libyaml makes the same choice).
fn wants_block(meta: &Meta, s: &str) -> bool {
    if meta.prefer_quoted {
        return false;
    }
    if meta.prefer_block || meta.block_style.is_some() {
        return true;
    }
    is_block_scalar_candidate(s) && !s.contains(" \n") && !s.contains("\t\n")
}

/// Can `s` be written as a block scalar at all?
fn block_representable(s: &str) -> bool {
    s.chars().all(|c| {
        c == '\n'
            || c == '\t'
            || (crate::syntax::chars::is_printable(c)
                && c != '\r'
                && c != '\u{FEFF}'
                && c != '\u{85}'
                && c != '\u{2028}'
                && c != '\u{2029}')
    })
}

/// Write `|`/`>` header and body for `s`, or return false when `s` can't
/// be a block scalar here. `content_indent` is the body's column. An
/// indentation indicator (2, relative to the parent node, or to column 0
/// at the top level) is used when the first non-empty line starts with a
/// space; without `indicator_ok` such values fall back to double quotes.
fn emit_block_scalar(
    meta: &Meta,
    s: &str,
    folded: bool,
    content_indent: usize,
    indicator_ok: bool,
    out: &mut String,
) -> bool {
    if !block_representable(s) {
        return false;
    }
    let needs_indicator = s
        .split('\n')
        .find(|l| !l.is_empty())
        .is_some_and(|l| l.starts_with(' '));
    if needs_indicator && !indicator_ok {
        return false;
    }
    let body = s.trim_end_matches('\n');
    let trailing = s.len() - body.len();
    let computed = if s.is_empty() {
        "-"
    } else if body.is_empty() {
        "+"
    } else {
        match trailing {
            0 => "-",
            1 => "",
            _ => "+",
        }
    };
    // Keep the source's chomping indicator when it yields the same value.
    let recorded = match meta.block_style {
        Some(BlockStyle::Literal(c) | BlockStyle::Folded(c)) => Some(c),
        None => None,
    };
    let chomp = match recorded {
        Some(Some('+')) => "+",
        Some(Some('-')) if trailing == 0 => "-",
        Some(None) if s.is_empty() || (trailing == 1 && !body.is_empty()) => "",
        _ => computed,
    };
    out.push(if folded { '>' } else { '|' });
    if needs_indicator {
        out.push('2');
    }
    out.push_str(chomp);
    if let Some(tc) = &meta.trailing_comment {
        out.push_str(" # ");
        out.push_str(tc);
    }
    out.push('\n');
    if folded {
        write_folded_body(s, content_indent, out);
    } else {
        for piece in s.split_inclusive('\n') {
            if piece != "\n" {
                emit_indent(content_indent, out);
                out.push_str(piece);
            } else {
                out.push('\n');
            }
        }
        if !s.ends_with('\n') && !s.is_empty() {
            out.push('\n');
        }
    }
    true
}

/// Folded body: a line break between two non-indented lines must be
/// written as an empty line, because a single break folds into a space.
fn write_folded_body(s: &str, indent: usize, out: &mut String) {
    let mut breaks = true;
    let mut leading_spaces = true;
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '\n' {
            if !breaks && !leading_spaces {
                let mut k = i;
                while k < chars.len() && chars[k] == '\n' {
                    k += 1;
                }
                if k < chars.len() && chars[k] != ' ' && chars[k] != '\t' {
                    out.push('\n');
                }
            }
            out.push('\n');
            breaks = true;
        } else {
            if breaks {
                emit_indent(indent, out);
                leading_spaces = c == ' ' || c == '\t';
            }
            out.push(c);
            breaks = false;
        }
        i += 1;
    }
    if !s.ends_with('\n') && !s.is_empty() {
        out.push('\n');
    }
}

fn emit_trailing_comment(meta: &Meta, out: &mut String) {
    if let Some(tc) = &meta.trailing_comment {
        out.push_str(" # ");
        out.push_str(tc);
    }
}

impl Emitter {
    /// Emit a document root, including its leading comments and properties.
    pub fn emit_root(elem: &Elem, out: &mut String) -> Result<(), Error> {
        emit_comment_lines(&elem.meta.leading_comments, 0, out);
        let collection = matches!(&elem.node, Node::Map(m) if !m.is_empty())
            || matches!(&elem.node, Node::Seq(s) if !s.is_empty());
        if collection {
            let mut props = String::new();
            write_props(&elem.meta, &mut props);
            let props = props.trim_end();
            if !props.is_empty() || elem.meta.trailing_comment.is_some() {
                out.push_str(props);
                if let Some(tc) = &elem.meta.trailing_comment {
                    if !props.is_empty() {
                        out.push(' ');
                    }
                    out.push_str("# ");
                    out.push_str(tc);
                }
                out.push('\n');
            }
        }
        Self::emit_elem(elem, 0, out)
    }

    /// Emit `elem`'s content at `indent`. Leading comments are the caller's
    /// job (see module docs).
    pub fn emit_elem(elem: &Elem, indent: usize, out: &mut String) -> Result<(), Error> {
        match &elem.node {
            Node::Map(entries) => {
                if entries.is_empty() {
                    write_props(&elem.meta, out);
                    out.push_str("{}");
                    emit_trailing_comment(&elem.meta, out);
                    out.push('\n');
                    return Ok(());
                }
                for (k, v) in entries {
                    Self::emit_entry(k, v, indent, out)?;
                }
                Ok(())
            }
            Node::Seq(items) => {
                if items.is_empty() {
                    write_props(&elem.meta, out);
                    out.push_str("[]");
                    emit_trailing_comment(&elem.meta, out);
                    out.push('\n');
                    return Ok(());
                }
                for it in items {
                    Self::emit_item(it, indent, out)?;
                }
                Ok(())
            }
            Node::Scalar(Scalar::Str(s)) if wants_block(&elem.meta, s) => {
                emit_indent(indent, out);
                write_props(&elem.meta, out);
                let folded = matches!(elem.meta.block_style, Some(BlockStyle::Folded(_)));
                if !emit_block_scalar(&elem.meta, s, folded, indent + 2, true, out) {
                    emit_quoted(s, out);
                    emit_trailing_comment(&elem.meta, out);
                    out.push('\n');
                }
                Ok(())
            }
            Node::Scalar(sc) => {
                emit_indent(indent, out);
                write_props(&elem.meta, out);
                let force = matches!(sc, Scalar::Str(_)) && elem.meta.prefer_quoted;
                emit_scalar_with_ctx(sc, force, false, out);
                emit_trailing_comment(&elem.meta, out);
                out.push('\n');
                Ok(())
            }
            Node::Alias(name) => {
                emit_indent(indent, out);
                write_props(&elem.meta, out);
                out.push('*');
                out.push_str(name);
                emit_trailing_comment(&elem.meta, out);
                out.push('\n');
                Ok(())
            }
        }
    }

    fn emit_entry(k: &MapKey, v: &Elem, indent: usize, out: &mut String) -> Result<(), Error> {
        emit_comment_lines(&v.meta.leading_comments, indent, out);
        emit_indent(indent, out);
        if emit_key(k, out) == KeyForm::Explicit {
            out.push('\n');
            emit_indent(indent, out);
        }
        out.push(':');
        Self::emit_value_after_indicator(v, indent, out)
    }

    fn emit_item(it: &Elem, indent: usize, out: &mut String) -> Result<(), Error> {
        emit_comment_lines(&it.meta.leading_comments, indent, out);
        emit_indent(indent, out);
        out.push('-');
        Self::emit_value_after_indicator(it, indent, out)
    }

    /// Emit a value right after `key:` or `-` at `indent`.
    fn emit_value_after_indicator(v: &Elem, indent: usize, out: &mut String) -> Result<(), Error> {
        match &v.node {
            Node::Scalar(Scalar::Str(s)) if wants_block(&v.meta, s) => {
                out.push(' ');
                write_props(&v.meta, out);
                let folded = matches!(v.meta.block_style, Some(BlockStyle::Folded(_)));
                if !emit_block_scalar(&v.meta, s, folded, indent + 2, true, out) {
                    emit_quoted(s, out);
                    emit_trailing_comment(&v.meta, out);
                    out.push('\n');
                }
            }
            Node::Scalar(sc) => {
                out.push(' ');
                write_props(&v.meta, out);
                let force = matches!(sc, Scalar::Str(_)) && v.meta.prefer_quoted;
                emit_scalar_with_ctx(sc, force, false, out);
                emit_trailing_comment(&v.meta, out);
                out.push('\n');
            }
            Node::Alias(name) => {
                out.push(' ');
                write_props(&v.meta, out);
                out.push('*');
                out.push_str(name);
                emit_trailing_comment(&v.meta, out);
                out.push('\n');
            }
            Node::Seq(items) if items.is_empty() => {
                out.push(' ');
                write_props(&v.meta, out);
                out.push_str("[]");
                emit_trailing_comment(&v.meta, out);
                out.push('\n');
            }
            Node::Map(entries) if entries.is_empty() => {
                out.push(' ');
                write_props(&v.meta, out);
                out.push_str("{}");
                emit_trailing_comment(&v.meta, out);
                out.push('\n');
            }
            Node::Seq(_) | Node::Map(_) => {
                let mut props = String::new();
                write_props(&v.meta, &mut props);
                let props = props.trim_end();
                if !props.is_empty() {
                    out.push(' ');
                    out.push_str(props);
                }
                emit_trailing_comment(&v.meta, out);
                out.push('\n');
                Self::emit_elem(v, indent + 2, out)?;
            }
        }
        Ok(())
    }
}
