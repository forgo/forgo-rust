//! Emitter for `Doc` trees.
//!
//! Block collections are indented by `Emitter::indent` spaces per level
//! (sequences under a key by `seq_indent`), and a collection inside a
//! sequence item starts on the dash's line when it can (`- a: 1`). Flow
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

/// How block content is laid out. The default matches common style: two
/// spaces per level, sequences under a key indented like mappings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Emitter {
    /// Spaces per nesting level: block mappings, block scalar content and
    /// continuation lines.
    pub indent: usize,
    /// How far a block sequence under a mapping key is indented (0 puts
    /// each `- ` at the key's column).
    pub seq_indent: usize,
}

impl Default for Emitter {
    fn default() -> Self {
        Emitter {
            indent: 2,
            seq_indent: 2,
        }
    }
}

const ROOT: Ctx = Ctx {
    flow: false,
    key: false,
    block_ok: true,
    empty_ok: true,
    // Root plain text that looks like a marker is written indented instead.
    line_start: false,
};
const BLOCK_VALUE: Ctx = ROOT;
const BLOCK_KEY: Ctx = Ctx {
    flow: false,
    key: true,
    block_ok: false,
    empty_ok: true,
    line_start: false,
};
const FLOW_ITEM: Ctx = Ctx {
    flow: true,
    key: false,
    block_ok: false,
    empty_ok: false,
    line_start: false,
};
const FLOW_VALUE: Ctx = Ctx {
    flow: true,
    key: false,
    block_ok: false,
    empty_ok: true,
    line_start: false,
};
const FLOW_KEY: Ctx = Ctx {
    flow: true,
    key: true,
    block_ok: false,
    empty_ok: true,
    line_start: false,
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

/// A block collection item that starts on its dash's line (`- a: 1`,
/// `- - x`): anything that would have to sit between the dash and the
/// first entry (properties, comments) rules it out.
fn compact(it: &Elem) -> bool {
    let first_has_comments = match &it.node {
        Node::Map(entries) => entries
            .first()
            .map(|(_, v)| !v.meta.leading_comments.is_empty()),
        Node::Seq(items) => items.first().map(|v| !v.meta.leading_comments.is_empty()),
        _ => None,
    };
    first_has_comments == Some(false)
        && !is_flow(it)
        && it.meta.anchor.is_none()
        && it.meta.tag.is_none()
        && it.meta.trailing_comment.is_none()
}

/// Does `s` start with a document marker (`---`/`...` + space or end)?
fn starts_with_marker(s: &str) -> bool {
    (s.starts_with("---") || s.starts_with("..."))
        && matches!(s.as_bytes().get(3), None | Some(b' ' | b'\t' | b'\n'))
}

fn no_content_line(s: &str) -> bool {
    s.split('\n').all(str::is_empty)
}

/// The text a document root puts on the `---` line, if any: a block
/// collection's properties and trailing comment, or an empty scalar's
/// trailing comment (so that it reads back as trailing `---`).
pub fn root_header(elem: &Elem) -> Option<String> {
    let mut s = String::new();
    match &elem.node {
        Node::Map(_) | Node::Seq(_) if !is_flow(elem) => {
            s.push_str(props_string(elem).trim_end());
        }
        Node::Scalar(sc)
            if elem.meta.tag.is_none()
                && elem.meta.anchor.is_none()
                && !elem.meta.leading_comments.is_empty()
                && scalar_repr(sc, &elem.meta, ROOT) == Repr::Empty => {}
        _ => return None,
    }
    if let Some(tc) = &elem.meta.trailing_comment {
        if !s.is_empty() {
            s.push(' ');
        }
        s.push_str("# ");
        s.push_str(tc);
    }
    (!s.is_empty()).then_some(s)
}

fn trim_trailing_spaces(out: &mut String) {
    while out.ends_with(' ') {
        out.pop();
    }
}

impl Emitter {
    /// Write a scalar with the chosen representation (props are the caller's).
    /// `indent` is the column for continuation lines; `parent` is the column
    /// a block scalar's indentation indicator counts from.
    fn write_scalar(
        &self,
        sc: &Scalar,
        e: &Elem,
        ctx: Ctx,
        indent: usize,
        parent: usize,
        out: &mut String,
    ) -> Repr {
        let repr = scalar_repr(sc, &e.meta, ctx);
        self.write_repr(&repr, sc, e, indent, parent, out);
        repr
    }

    fn write_repr(
        &self,
        repr: &Repr,
        sc: &Scalar,
        e: &Elem,
        indent: usize,
        parent: usize,
        out: &mut String,
    ) {
        let text = scalar_text(sc);
        match repr {
            Repr::Plain(t) => write_folded_lines(t, indent, out),
            Repr::Empty => {}
            Repr::Single => emit_single(&text, indent, out),
            Repr::Double => emit_quoted(&text, out),
            Repr::Block { folded, header } => emit_block(
                &text,
                *folded,
                *header,
                parent,
                self.indent,
                e.meta.trailing_comment.as_deref(),
                out,
            ),
        }
    }

    /// Emit a document root, including its leading comments and properties.
    /// `followed_by_comments`: footer comments come next.
    pub fn emit_root(
        &self,
        elem: &Elem,
        followed_by_comments: bool,
        header_written: bool,
        out: &mut String,
    ) -> Result<(), Error> {
        comment_lines(&elem.meta.leading_comments, 0, out);
        match &elem.node {
            Node::Map(_) | Node::Seq(_) if !is_flow(elem) => {
                let props = props_string(elem);
                let props = props.trim_end();
                if !header_written && (!props.is_empty() || elem.meta.trailing_comment.is_some()) {
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
                self.emit_block_collection(elem, 0, false, out)
            }
            Node::Map(_) | Node::Seq(_) => {
                write_props(&elem.meta, out);
                self.emit_flow(elem, self.indent, out);
                trailing(elem, out);
                out.push('\n');
                Ok(())
            }
            Node::Scalar(sc) => {
                let props = props_string(elem);
                let mut repr = scalar_repr(sc, &elem.meta, ROOT);
                if let Repr::Plain(t) = &repr
                    && props.is_empty()
                    && starts_with_marker(t)
                {
                    // At column 0 this would read as a document marker.
                    out.push(' ');
                }
                out.push_str(&props);
                // An empty root block scalar would read the comments after
                // it as content without an indentation indicator.
                if let Repr::Block { header, .. } = &mut repr
                    && followed_by_comments
                    && no_content_line(&scalar_text(sc))
                {
                    header.indent.get_or_insert(2);
                }
                self.write_repr(&repr, sc, elem, self.indent, 0, out);
                if !matches!(repr, Repr::Block { .. }) {
                    if repr == Repr::Empty {
                        trim_trailing_spaces(out);
                    }
                    if !header_written {
                        trailing(elem, out);
                    }
                    if !(header_written && repr == Repr::Empty) {
                        out.push('\n');
                    }
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

    /// Emit the entries of a block collection whose entries sit at
    /// `indent`. With `inline_first`, the caller has already written the
    /// first entry's line prefix (`- ` of a compact item).
    pub(crate) fn emit_block_collection(
        &self,
        elem: &Elem,
        indent: usize,
        inline_first: bool,
        out: &mut String,
    ) -> Result<(), Error> {
        match &elem.node {
            Node::Map(entries) => {
                for (i, (k, v)) in entries.iter().enumerate() {
                    self.emit_entry(k, v, indent, inline_first && i == 0, out)?;
                }
            }
            Node::Seq(items) => {
                for (i, it) in items.iter().enumerate() {
                    self.emit_item(it, indent, inline_first && i == 0, out)?;
                }
            }
            _ => unreachable!("not a collection"),
        }
        Ok(())
    }

    /// A sequence item with its dash at `indent`, through its last line.
    pub(crate) fn emit_item(
        &self,
        it: &Elem,
        indent: usize,
        inline: bool,
        out: &mut String,
    ) -> Result<(), Error> {
        if !inline {
            comment_lines(&it.meta.leading_comments, indent, out);
            emit_indent(indent, out);
        }
        out.push('-');
        if compact(it) {
            out.push(' ');
            self.emit_block_collection(it, indent + 2, true, out)
        } else {
            self.emit_value_after_indicator(it, indent, self.indent, out)
        }
    }

    /// A mapping entry with its key at `indent`, through its last line.
    /// With `inline`, the line prefix is already written.
    pub(crate) fn emit_entry(
        &self,
        k: &MapKey,
        v: &Elem,
        indent: usize,
        inline: bool,
        out: &mut String,
    ) -> Result<(), Error> {
        if !inline {
            comment_lines(&v.meta.leading_comments, indent, out);
            emit_indent(indent, out);
        }
        let key_ctx = Ctx {
            line_start: indent == 0,
            ..BLOCK_KEY
        };
        // A sequence value may sit at the key's column after an implicit
        // key; after `? key` it is always indented.
        let mut seq_step = self.seq_indent;
        match k {
            MapKey::Str(s) if plain_ok(s, key_ctx) => out.push_str(s),
            MapKey::Str(s) if !s.contains('\n') && s.chars().count() <= 1000 => emit_quoted(s, out),
            // A multi-line plain key: `? a`, an empty line, `b`.
            MapKey::Str(s) if plain_ok(s, BLOCK_VALUE) => {
                out.push_str("? ");
                write_folded_lines(s, indent + self.indent, out);
                out.push('\n');
                emit_indent(indent, out);
                seq_step = self.indent;
            }
            MapKey::Str(s) => {
                out.push_str("? ");
                emit_quoted(s, out);
                out.push('\n');
                emit_indent(indent, out);
                seq_step = self.indent;
            }
            MapKey::Complex(ke) => {
                if !self.write_inline_key(ke, key_ctx, out) {
                    out.push('?');
                    self.emit_value_after_indicator(ke, indent, self.indent, out)?;
                    emit_indent(indent, out);
                    seq_step = self.indent;
                }
            }
        }
        out.push(':');
        self.emit_value_after_indicator(v, indent, seq_step, out)
    }

    /// A key written before `:` on its entry's line, if it can be one.
    pub(crate) fn implicit_key(&self, k: &MapKey, indent: usize) -> Option<String> {
        let key_ctx = Ctx {
            line_start: indent == 0,
            ..BLOCK_KEY
        };
        let mut s = String::new();
        match k {
            MapKey::Str(t) if plain_ok(t, key_ctx) => s.push_str(t),
            MapKey::Str(t) if !t.contains('\n') && t.chars().count() <= 1000 => {
                emit_quoted(t, &mut s)
            }
            MapKey::Str(_) => return None,
            MapKey::Complex(ke) => {
                if !self.write_inline_key(ke, key_ctx, &mut s) {
                    return None;
                }
            }
        }
        Some(s)
    }

    /// A value written on its key's (or dash's) line, after the indicator
    /// and a space: properties and a one-line scalar, alias or flow
    /// collection, without the trailing comment. `None` when it needs
    /// more lines or is empty. `indent` is the entry's column.
    pub(crate) fn inline_value(&self, v: &Elem, indent: usize) -> Option<String> {
        let mut s = props_string(v);
        match &v.node {
            Node::Scalar(sc) => {
                let repr = scalar_repr(sc, &v.meta, BLOCK_VALUE);
                if matches!(repr, Repr::Block { .. } | Repr::Empty) {
                    return None;
                }
                self.write_repr(&repr, sc, v, indent + self.indent, indent, &mut s);
            }
            Node::Alias(name) => {
                s.push('*');
                s.push_str(name);
            }
            Node::Seq(_) | Node::Map(_) if is_flow(v) => {
                self.emit_flow(v, indent + self.indent, &mut s)
            }
            Node::Seq(_) | Node::Map(_) => return None,
        }
        (!s.contains('\n')).then_some(s)
    }

    /// Write a complex key as an implicit key if it can be one.
    fn write_inline_key(&self, ke: &Elem, key_ctx: Ctx, out: &mut String) -> bool {
        let short = |s: &str| !s.contains('\n') && s.chars().count() <= 1000;
        if !ke.meta.leading_comments.is_empty() || ke.meta.trailing_comment.is_some() {
            return false;
        }
        // A key written as a block scalar keeps that form (`? |`), and so
        // does a multi-line plain or single-quoted key (`? a` ...).
        if matches!(ke.meta.style, Some(Style::Literal(_) | Style::Folded(_))) {
            return false;
        }
        if let Node::Scalar(Scalar::Str(s)) = &ke.node
            && s.contains('\n')
            && ke.meta.style != Some(Style::DoubleQuoted)
        {
            return false;
        }
        match &ke.node {
            Node::Scalar(sc) => {
                let props = props_string(ke);
                let mut tmp = props.clone();
                match self.write_scalar(sc, ke, key_ctx, 0, 0, &mut tmp) {
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
                self.emit_flow(ke, self.indent, &mut tmp);
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
    /// end of its last line. A block sequence value is indented by
    /// `seq_step`, a block mapping by `self.indent`.
    fn emit_value_after_indicator(
        &self,
        v: &Elem,
        indent: usize,
        seq_step: usize,
        out: &mut String,
    ) -> Result<(), Error> {
        match &v.node {
            Node::Scalar(sc) => {
                out.push(' ');
                out.push_str(&props_string(v));
                let repr = self.write_scalar(sc, v, BLOCK_VALUE, indent + self.indent, indent, out);
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
                self.emit_flow(v, indent + self.indent, out);
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
                let step = match v.node {
                    Node::Seq(_) => seq_step,
                    _ => self.indent,
                };
                self.emit_block_collection(v, indent + step, false, out)?;
            }
        }
        Ok(())
    }

    /// Emit a node inside a flow collection (no line break at the end).
    /// `indent` is the column for continuation lines.
    fn emit_flow_node(&self, e: &Elem, ctx: Ctx, indent: usize, out: &mut String) {
        let mut node = String::new();
        write_props(&e.meta, &mut node);
        match &e.node {
            Node::Scalar(sc) => {
                if self.write_scalar(sc, e, ctx, indent, indent, &mut node) == Repr::Empty {
                    trim_trailing_spaces(&mut node);
                }
            }
            Node::Alias(name) => {
                node.push('*');
                node.push_str(name);
            }
            Node::Seq(_) | Node::Map(_) => self.emit_flow(e, indent + self.indent, &mut node),
        }
        out.push_str(&node);
    }

    /// Emit a flow mapping key (without the `:`).
    fn emit_flow_key(&self, k: &MapKey, indent: usize, out: &mut String) {
        match k {
            MapKey::Str(s) if plain_ok(s, FLOW_KEY) => out.push_str(s),
            // Keys inside flow mappings may span lines.
            MapKey::Str(s) if s.contains('\n') && plain_ok(s, FLOW_VALUE) => {
                write_folded_lines(s, indent, out)
            }
            MapKey::Str(s) => emit_quoted(s, out),
            MapKey::Complex(ke) => {
                let before = out.len();
                self.emit_flow_node(ke, FLOW_KEY, indent, out);
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
    pub(crate) fn emit_flow(&self, e: &Elem, indent: usize, out: &mut String) {
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
                Node::Seq(items) => self.emit_flow_node(&items[i], FLOW_ITEM, indent, out),
                Node::Map(entries) => {
                    let (k, v) = &entries[i];
                    self.emit_flow_key(k, indent, out);
                    out.push(':');
                    let mut value = String::from(" ");
                    self.emit_flow_node(v, FLOW_VALUE, indent, &mut value);
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
