// crates/forgo_lib_yaml/src/emitter.rs
//! Emitter for `Doc` trees.
//!
//! Block collections are indented by two spaces per level. Flow
//! collections (recorded `Style::Flow`, and empty collections) stay flow:
//! on one line, or one entry per line when they contain comments. Every
//! comment is printed exactly once: leading comments before their entry,
//! trailing comments at the end of the node's line. Scalar styles are
//! chosen by `emitter_helpers::scalar_repr`.

use crate::ast::{Elem, Error, MapKey, Node, Scalar, Style};
use crate::emitter_helpers::{
    Ctx, Repr, emit_block, emit_indent, emit_quoted, emit_single, plain_ok, scalar_repr,
    scalar_text, write_folded_lines, write_props,
};

pub struct Emitter;

const ROOT: Ctx = Ctx {
    flow: false,
    key: false,
    block_ok: true,
    empty_ok: true,
};
const BLOCK_VALUE: Ctx = ROOT;
const BLOCK_KEY: Ctx = Ctx {
    flow: false,
    key: true,
    block_ok: false,
    empty_ok: true,
};
const FLOW_ITEM: Ctx = Ctx {
    flow: true,
    key: false,
    block_ok: false,
    empty_ok: false,
};
const FLOW_VALUE: Ctx = Ctx {
    flow: true,
    key: false,
    block_ok: false,
    empty_ok: true,
};
const FLOW_KEY: Ctx = Ctx {
    flow: true,
    key: true,
    block_ok: false,
    empty_ok: true,
};

fn comment_lines(comments: &[String], indent: usize, out: &mut String) {
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

fn trailing(e: &Elem, out: &mut String) {
    if let Some(tc) = &e.meta.trailing_comment {
        out.push_str(" # ");
        out.push_str(tc);
    }
}

/// Written as a flow collection.
fn is_flow(e: &Elem) -> bool {
    match &e.node {
        Node::Seq(items) => items.is_empty() || e.meta.style == Some(Style::Flow),
        Node::Map(entries) => entries.is_empty() || e.meta.style == Some(Style::Flow),
        _ => false,
    }
}

/// Does anything inside `e` (not `e` itself) carry a comment?
fn has_inner_comments(e: &Elem) -> bool {
    fn any(e: &Elem) -> bool {
        !e.meta.leading_comments.is_empty()
            || e.meta.trailing_comment.is_some()
            || has_inner_comments(e)
    }
    match &e.node {
        Node::Seq(items) => items.iter().any(any),
        Node::Map(entries) => entries.iter().any(|(k, v)| {
            any(v)
                || match k {
                    MapKey::Complex(ke) => any(ke),
                    MapKey::Str(_) => false,
                }
        }),
        _ => false,
    }
}

fn props_string(e: &Elem) -> String {
    let mut p = String::new();
    write_props(&e.meta, &mut p);
    p
}

/// Write a scalar with the chosen representation (props are the caller's).
/// `indent` is the column for continuation lines; `parent` is the column
/// a block scalar's indentation indicator counts from.
fn write_scalar(
    sc: &Scalar,
    e: &Elem,
    ctx: Ctx,
    indent: usize,
    parent: usize,
    out: &mut String,
) -> Repr {
    let repr = scalar_repr(sc, &e.meta, ctx);
    let text = scalar_text(sc);
    match &repr {
        Repr::Plain(t) => write_folded_lines(t, indent, out),
        Repr::Empty => {}
        Repr::Single => emit_single(&text, indent, out),
        Repr::Double => emit_quoted(&text, out),
        Repr::Block { folded, header } => emit_block(
            &text,
            *folded,
            *header,
            parent,
            e.meta.trailing_comment.as_deref(),
            out,
        ),
    }
    repr
}

fn trim_trailing_spaces(out: &mut String) {
    while out.ends_with(' ') {
        out.pop();
    }
}

impl Emitter {
    /// Emit a document root, including its leading comments and properties.
    pub fn emit_root(elem: &Elem, out: &mut String) -> Result<(), Error> {
        comment_lines(&elem.meta.leading_comments, 0, out);
        match &elem.node {
            Node::Map(_) | Node::Seq(_) if !is_flow(elem) => {
                let props = props_string(elem);
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
                Self::emit_block_collection(elem, 0, out)
            }
            Node::Map(_) | Node::Seq(_) => {
                write_props(&elem.meta, out);
                Self::emit_flow(elem, 2, out);
                trailing(elem, out);
                out.push('\n');
                Ok(())
            }
            Node::Scalar(sc) => {
                out.push_str(&props_string(elem));
                let repr = write_scalar(sc, elem, ROOT, 2, 0, out);
                if !matches!(repr, Repr::Block { .. }) {
                    if repr == Repr::Empty {
                        trim_trailing_spaces(out);
                    }
                    trailing(elem, out);
                    out.push('\n');
                }
                Ok(())
            }
            Node::Alias(name) => {
                write_props(&elem.meta, out);
                out.push('*');
                out.push_str(name);
                trailing(elem, out);
                out.push('\n');
                Ok(())
            }
        }
    }

    fn emit_block_collection(elem: &Elem, indent: usize, out: &mut String) -> Result<(), Error> {
        match &elem.node {
            Node::Map(entries) => {
                for (k, v) in entries {
                    Self::emit_entry(k, v, indent, out)?;
                }
            }
            Node::Seq(items) => {
                for it in items {
                    comment_lines(&it.meta.leading_comments, indent, out);
                    emit_indent(indent, out);
                    out.push('-');
                    Self::emit_value_after_indicator(it, indent, out)?;
                }
            }
            _ => unreachable!("not a collection"),
        }
        Ok(())
    }

    fn emit_entry(k: &MapKey, v: &Elem, indent: usize, out: &mut String) -> Result<(), Error> {
        comment_lines(&v.meta.leading_comments, indent, out);
        emit_indent(indent, out);
        match k {
            MapKey::Str(s) if plain_ok(s, BLOCK_KEY) => out.push_str(s),
            MapKey::Str(s) if !s.contains('\n') && s.chars().count() <= 1000 => emit_quoted(s, out),
            MapKey::Str(s) => {
                out.push_str("? ");
                emit_quoted(s, out);
                out.push('\n');
                emit_indent(indent, out);
            }
            MapKey::Complex(ke) => {
                if !Self::write_inline_key(ke, out) {
                    out.push('?');
                    Self::emit_value_after_indicator(ke, indent, out)?;
                    emit_indent(indent, out);
                }
            }
        }
        out.push(':');
        Self::emit_value_after_indicator(v, indent, out)
    }

    /// Write a complex key as an implicit key if it can be one.
    fn write_inline_key(ke: &Elem, out: &mut String) -> bool {
        let short = |s: &str| !s.contains('\n') && s.chars().count() <= 1000;
        if !ke.meta.leading_comments.is_empty() || ke.meta.trailing_comment.is_some() {
            return false;
        }
        // A key written as a block scalar keeps that form: `? |`.
        if matches!(ke.meta.style, Some(Style::Literal(_) | Style::Folded(_))) {
            return false;
        }
        match &ke.node {
            Node::Scalar(sc) => {
                let props = props_string(ke);
                let mut tmp = props.clone();
                match write_scalar(sc, ke, BLOCK_KEY, 0, 0, &mut tmp) {
                    Repr::Block { .. } => false,
                    Repr::Empty => {
                        // `: value`, or `!tag : value` / `&a : value`.
                        out.push_str(&props);
                        true
                    }
                    _ if short(&tmp) => {
                        out.push_str(&tmp);
                        true
                    }
                    _ => false,
                }
            }
            Node::Alias(name) => {
                write_props(&ke.meta, out);
                out.push('*');
                out.push_str(name);
                // `:` is an anchor character: separate it from the name.
                out.push(' ');
                true
            }
            Node::Seq(_) | Node::Map(_) => {
                if !is_flow(ke) || has_inner_comments(ke) {
                    return false;
                }
                let mut tmp = props_string(ke);
                Self::emit_flow(ke, 2, &mut tmp);
                if short(&tmp) {
                    out.push_str(&tmp);
                    true
                } else {
                    false
                }
            }
        }
    }

    /// Emit a value right after `key:`, `-` or `?` at `indent`, through the
    /// end of its last line.
    fn emit_value_after_indicator(v: &Elem, indent: usize, out: &mut String) -> Result<(), Error> {
        match &v.node {
            Node::Scalar(sc) => {
                out.push(' ');
                out.push_str(&props_string(v));
                let repr = write_scalar(sc, v, BLOCK_VALUE, indent + 2, indent, out);
                if !matches!(repr, Repr::Block { .. }) {
                    if repr == Repr::Empty {
                        trim_trailing_spaces(out);
                    }
                    trailing(v, out);
                    out.push('\n');
                }
            }
            Node::Alias(name) => {
                out.push(' ');
                write_props(&v.meta, out);
                out.push('*');
                out.push_str(name);
                trailing(v, out);
                out.push('\n');
            }
            Node::Seq(_) | Node::Map(_) if is_flow(v) => {
                out.push(' ');
                write_props(&v.meta, out);
                Self::emit_flow(v, indent + 2, out);
                trailing(v, out);
                out.push('\n');
            }
            Node::Seq(_) | Node::Map(_) => {
                let props = props_string(v);
                let props = props.trim_end();
                if !props.is_empty() {
                    out.push(' ');
                    out.push_str(props);
                }
                trailing(v, out);
                out.push('\n');
                Self::emit_block_collection(v, indent + 2, out)?;
            }
        }
        Ok(())
    }

    /// Emit a node inside a flow collection (no line break at the end).
    /// `indent` is the column for continuation lines.
    fn emit_flow_node(e: &Elem, ctx: Ctx, indent: usize, out: &mut String) {
        write_props(&e.meta, out);
        match &e.node {
            Node::Scalar(sc) => {
                if write_scalar(sc, e, ctx, indent, indent, out) == Repr::Empty {
                    trim_trailing_spaces(out);
                }
            }
            Node::Alias(name) => {
                out.push('*');
                out.push_str(name);
            }
            Node::Seq(_) | Node::Map(_) => Self::emit_flow(e, indent + 2, out),
        }
    }

    /// Emit a flow mapping key (without the `:`).
    fn emit_flow_key(k: &MapKey, indent: usize, out: &mut String) {
        match k {
            MapKey::Str(s) if plain_ok(s, FLOW_KEY) => out.push_str(s),
            MapKey::Str(s) => emit_quoted(s, out),
            MapKey::Complex(ke) => {
                let before = out.len();
                Self::emit_flow_node(ke, FLOW_KEY, indent, out);
                let alias = matches!(ke.node, Node::Alias(_));
                let only_props = out.len() == before + props_string(ke).trim_end().len()
                    && (ke.meta.tag.is_some() || ke.meta.anchor.is_some());
                if alias || only_props {
                    // `*a :` and `!tag :` need the space before `:`.
                    out.push(' ');
                }
            }
        }
    }

    /// Emit a collection in flow style (no line break at the end).
    /// `indent` is the column for entries on continuation lines.
    fn emit_flow(e: &Elem, indent: usize, out: &mut String) {
        let multiline = has_inner_comments(e);
        let (open, close, n) = match &e.node {
            Node::Seq(items) => ('[', ']', items.len()),
            Node::Map(entries) => ('{', '}', entries.len()),
            _ => unreachable!("not a collection"),
        };
        out.push(open);
        for i in 0..n {
            let node = match &e.node {
                Node::Seq(items) => &items[i],
                Node::Map(entries) => &entries[i].1,
                _ => unreachable!(),
            };
            if multiline {
                out.push('\n');
                comment_lines(&node.meta.leading_comments, indent, out);
                emit_indent(indent, out);
            } else if i > 0 {
                out.push(' ');
            }
            match &e.node {
                Node::Seq(items) => Self::emit_flow_node(&items[i], FLOW_ITEM, indent, out),
                Node::Map(entries) => {
                    let (k, v) = &entries[i];
                    Self::emit_flow_key(k, indent, out);
                    out.push(':');
                    let mut value = String::from(" ");
                    Self::emit_flow_node(v, FLOW_VALUE, indent, &mut value);
                    if !value.trim().is_empty() {
                        out.push_str(&value);
                    }
                }
                _ => unreachable!(),
            }
            if i + 1 < n {
                out.push(',');
            }
            if multiline && let Some(tc) = &node.meta.trailing_comment {
                out.push_str(" # ");
                out.push_str(tc);
            }
        }
        if multiline {
            out.push('\n');
            emit_indent(indent, out);
        }
        out.push(close);
    }
}
