//! Composer: parse events → documents (`Doc`), with comments attached.
//!
//! Comment placement (comments are scanned on the side and merged here by
//! position):
//! - full-line comments become `leading_comments` of the next map entry
//!   (stored on its value), sequence item, or document root;
//! - an end-of-line comment becomes the `trailing_comment` of the last node
//!   completed before it on its line (in `[a, b] # c`, the sequence); one
//!   that follows only an indicator (`key:` or `-` with the value on a later
//!   line) goes to that value;
//! - comments after a document's last node (and one on its `...` line) go
//!   to the document footer.
//!
//! Each document records its byte range in the stream, and a block
//! collection's span ends at its last token, for format-preserving output.
//!
//! Keys that are plain or quoted strings without properties become
//! `MapKey::Str`; any other key (numbers, booleans, null, tagged or
//! anchored scalars, aliases, collections) becomes `MapKey::Complex`.
//! Duplicate keys are kept in order (lookups use the last one).

use crate::ast::{Doc, Elem, Error, MapKey, Meta, Node, Scalar, SourceSpan, Style};
use crate::schema;
use crate::syntax::{self, Comment, Event, Mark, ScalarStyle, Span, Tag};
use std::borrow::Cow;
use std::collections::HashSet;
use std::sync::Arc;

/// Compose every document in `input`. A non-empty stream without documents
/// gives one placeholder document that keeps its text and comments.
pub(crate) fn compose_stream(input: &str) -> Result<Vec<Doc>, Error> {
    compose_stream_with(input, &crate::ParseOptions::default())
}

/// [`compose_stream`] with explicit limits.
pub(crate) fn compose_stream_with(
    input: &str,
    options: &crate::ParseOptions,
) -> Result<Vec<Doc>, Error> {
    let mut docs = compose(input, options, true)?;
    if input.is_empty() {
        docs.clear();
    }
    Ok(docs)
}

/// Compose the first document, or a placeholder (null root, keeping the
/// comments) when the stream has none.
pub(crate) fn compose_first(input: &str, options: &crate::ParseOptions) -> Result<Doc, Error> {
    Ok(compose(input, options, true)?.swap_remove(0))
}

/// Compose every document, keeping a copy of `input` in each when
/// `keep_source` is set. Always returns at least one document (a
/// placeholder for a stream without documents).
pub(crate) fn compose(
    input: &str,
    options: &crate::ParseOptions,
    keep_source: bool,
) -> Result<Vec<Doc>, Error> {
    let mut events = syntax::events(input);
    let mut c = Composer {
        src: input,
        source: keep_source.then(|| Arc::from(input)),
        doc_start: 0,
        comments: Vec::new(),
        ci: 0,
        leading: Vec::new(),
        carry: None,
        stack: Vec::new(),
        root: None,
        anchors: HashSet::new(),
        doc: None,
        docs: Vec::new(),
        max_depth: options.max_depth,
        last: Last::None,
        saw_cr: input.contains('\r'),
        leading_blank: input
            .trim_start_matches('\u{FEFF}')
            .starts_with(['\n', '\r']),
    };
    // Stream: compose each event as it is parsed. Scanning one token ahead
    // makes the comments up to the next node available (a trailing comment
    // follows the node it belongs to).
    while let Some(r) = events.next() {
        let (ev, span) = r?;
        events.scan_ahead();
        let known = c.comments.len();
        c.comments.extend_from_slice(&events.comments()[known..]);
        c.event(ev, span)?;
    }
    Ok(c.docs)
}

/// The position after the line break of the line holding the byte before
/// `pos` (`pos` itself when it starts a line).
pub(crate) fn line_end_after(src: &str, pos: usize) -> usize {
    if pos == 0 || src.as_bytes().get(pos - 1) == Some(&b'\n') {
        return pos;
    }
    match src.as_bytes()[pos..].iter().position(|&b| b == b'\n') {
        Some(i) => pos + i + 1,
        None => src.len(),
    }
}

/// Where a node's text ends. An empty node written as nothing after `:`,
/// `-` or `?` ends there; one standing for an omitted value (placed at the
/// next token) has no text.
pub(crate) fn text_end(src: &str, span: &SourceSpan) -> Option<usize> {
    let r = span.range()?;
    if r.end > r.start {
        return Some(r.end);
    }
    let before = r.start.checked_sub(1).and_then(|i| src.as_bytes().get(i))?;
    matches!(before, b':' | b'-' | b'?').then_some(r.end)
}

/// A collection being composed.
struct Frame {
    elem: Elem,
    /// Mappings: leading comments of the entry being composed.
    entry_leading: Vec<String>,
    /// Mappings: the entry's key, once complete.
    key: Option<MapKey>,
    /// End of the last token inside the collection so far.
    end: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Role {
    Root,
    Key,
    Value,
    Item,
}

/// The most recently completed node, for end-of-line comments.
#[derive(Clone, Copy)]
enum Last {
    None,
    /// A mapping key ending at this mark: its comments go to the value.
    Key(Mark),
    /// The newest value, item or root (the last element of the top frame).
    Node(Mark),
}

struct DocInfo {
    explicit_start: bool,
    version: Option<(u32, u32)>,
    tags: Vec<(String, String)>,
    first: bool,
}

struct Composer<'a> {
    src: &'a str,
    /// The input, shared by the composed documents (when kept).
    source: Option<Arc<str>>,
    /// Where the current document's text starts.
    doc_start: usize,
    comments: Vec<Comment<'a>>,
    ci: usize,
    /// Full-line comments waiting for the next entry, item or root.
    leading: Vec<String>,
    /// An end-of-line comment that followed only an indicator.
    carry: Option<String>,
    stack: Vec<Frame>,
    root: Option<Elem>,
    anchors: HashSet<String>,
    doc: Option<DocInfo>,
    docs: Vec<Doc>,
    max_depth: usize,
    last: Last,
    /// The input uses CR line breaks (computed once).
    saw_cr: bool,
    /// The input starts with an empty line.
    leading_blank: bool,
}

fn source_tag(t: &Tag<'_>) -> String {
    if t.handle.is_empty() {
        format!("!<{}>", t.suffix)
    } else {
        format!("{}{}", t.handle, t.suffix)
    }
}

fn syntax_err(m: Mark, msg: impl Into<String>) -> Error {
    Error::Syntax {
        line: m.line + 1,
        col: m.col + 1,
        msg: msg.into(),
    }
}

fn append_comment(slot: &mut Option<String>, text: String) {
    *slot = Some(match slot.take() {
        Some(t) => format!("{t} # {text}"),
        None => text,
    });
}

impl<'a> Composer<'a> {
    fn role(&self) -> Role {
        match self.stack.last() {
            None => Role::Root,
            Some(f) if f.elem.node.is_seq() => Role::Item,
            Some(Frame { key: None, .. }) => Role::Key,
            Some(_) => Role::Value,
        }
    }

    /// Move comments that start before `pos` into the pending buffers.
    fn drain_until(&mut self, pos: usize) {
        while let Some(c) = self.comments.get(self.ci) {
            if c.start.index >= pos {
                break;
            }
            let text = c.text.trim().to_string();
            if c.trailing {
                append_comment(&mut self.carry, text);
            } else {
                self.leading.push(text);
            }
            self.ci += 1;
        }
    }

    /// Comments a node takes when it starts.
    fn begin_node(&mut self, start: Mark, collection: bool, meta: &mut Meta) {
        self.drain_until(start.index);
        match self.role() {
            Role::Key => {
                let l = std::mem::take(&mut self.leading);
                if let Some(f) = self.stack.last_mut() {
                    f.entry_leading = l;
                }
            }
            Role::Value => {
                if let Some(f) = self.stack.last_mut() {
                    meta.leading_comments = std::mem::take(&mut f.entry_leading);
                }
                // A nested collection leaves later comments to its first child.
                if !collection {
                    meta.leading_comments.append(&mut self.leading);
                }
                if let Some(c) = self.carry.take() {
                    append_comment(&mut meta.trailing_comment, c);
                }
            }
            Role::Root if collection => {
                // Comments above a root collection belong to its first entry.
                if let Some(c) = self.carry.take() {
                    append_comment(&mut meta.trailing_comment, c);
                }
            }
            Role::Item | Role::Root => {
                meta.leading_comments = std::mem::take(&mut self.leading);
                if let Some(c) = self.carry.take() {
                    append_comment(&mut meta.trailing_comment, c);
                }
            }
        }
    }

    /// The newest value, item or root.
    fn last_meta_mut(&mut self) -> Option<&mut Meta> {
        match self.stack.last_mut() {
            None => self.root.as_mut().map(|e| &mut e.meta),
            Some(f) => match &mut f.elem.node {
                Node::Seq(items) => items.last_mut().map(|e| &mut e.meta),
                Node::Map(entries) => entries.last_mut().map(|(_, v)| &mut v.meta),
                _ => None,
            },
        }
    }

    /// Before the event at `pos`: end-of-line comments on the line where
    /// the last completed node ended, after it and before `pos`, belong to
    /// that node (or, after a key, to its value).
    fn attach_trailing(&mut self, pos: usize) {
        let (end, key) = match std::mem::replace(&mut self.last, Last::None) {
            Last::None => return,
            Last::Key(m) => (m, true),
            Last::Node(m) => (m, false),
        };
        while let Some(c) = self.comments.get(self.ci) {
            let same_line = c.start.line == end.line && c.start.index >= end.index;
            if !(c.trailing && same_line && c.start.index < pos) {
                break;
            }
            let text = c.text.trim().to_string();
            self.ci += 1;
            if key {
                append_comment(&mut self.carry, text);
            } else if let Some(meta) = self.last_meta_mut() {
                append_comment(&mut meta.trailing_comment, text);
            }
        }
    }

    /// A block scalar's header comment (inside its span, or on its first
    /// line when it has no content).
    fn header_comment(&mut self, meta: &mut Meta, span: Span) {
        if let Some(c) = self.comments.get(self.ci)
            && c.trailing
            && (c.start.index < span.end.index || c.start.line == span.end.line)
        {
            let text = c.text.trim().to_string();
            append_comment(&mut meta.trailing_comment, text);
            self.ci += 1;
        }
    }

    /// Record what just completed, for comments on the rest of its line.
    fn completed(&mut self, was_key: bool, end: Mark) {
        self.last = if was_key {
            Last::Key(end)
        } else {
            Last::Node(end)
        };
    }

    fn complete(&mut self, elem: Elem) {
        let Some(f) = self.stack.last_mut() else {
            self.root = Some(elem);
            return;
        };
        if let Some(end) = text_end(self.src, &elem.meta.span) {
            f.end = f.end.max(end);
        }
        match &mut f.elem.node {
            Node::Seq(items) => items.push(elem),
            Node::Map(entries) => match f.key.take() {
                Some(k) => entries.push((k, elem)),
                None => f.key = Some(make_key(elem)),
            },
            _ => {}
        }
    }

    fn props(&mut self, anchor: Option<Cow<'a, str>>, tag: Option<&Tag<'a>>, meta: &mut Meta) {
        if let Some(a) = anchor {
            let a = a.into_owned();
            self.anchors.insert(a.clone());
            meta.anchor = Some(a);
        }
        meta.tag = tag.map(source_tag);
    }

    fn check_depth(&self, span: Span) -> Result<(), Error> {
        if self.stack.len() >= self.max_depth {
            return Err(syntax_err(
                span.start,
                format!(
                    "collections are nested more than {} levels deep (see ParseOptions::max_depth)",
                    self.max_depth
                ),
            ));
        }
        Ok(())
    }

    fn event(&mut self, ev: Event<'a>, span: Span) -> Result<(), Error> {
        let pos = match ev {
            Event::StreamEnd => usize::MAX,
            _ => span.start.index,
        };
        self.attach_trailing(pos);
        match ev {
            Event::StreamStart => {}
            Event::StreamEnd => {
                self.drain_until(usize::MAX);
                let rest: Vec<String> = std::mem::take(&mut self.leading)
                    .into_iter()
                    .chain(self.carry.take())
                    .collect();
                let len = self.src.len();
                match self.docs.last_mut() {
                    Some(d) => {
                        d.footer_comments.extend(rest);
                        // Anything after the last document is its text.
                        d.origin.range.1 = len;
                    }
                    None => {
                        let source = self.source.clone().map(|s| (s, (0, len)));
                        self.docs
                            .push(Doc::empty_with_comments(rest, self.saw_cr, source));
                    }
                }
            }
            Event::DocumentStart {
                explicit,
                version,
                tags,
            } => {
                self.anchors.clear();
                self.root = None;
                self.doc = Some(DocInfo {
                    explicit_start: explicit,
                    version,
                    tags: tags
                        .into_iter()
                        .map(|(h, p)| (h.into_owned(), p.into_owned()))
                        .collect(),
                    first: self.docs.is_empty(),
                });
            }
            Event::DocumentEnd { explicit } => {
                self.drain_until(span.start.index);
                let mut footer: Vec<String> = std::mem::take(&mut self.leading)
                    .into_iter()
                    .chain(self.carry.take())
                    .collect();
                // The document's text runs to the next document's marker,
                // or through the `...` line (with its comment).
                let mut end = span.start.index;
                if explicit {
                    end = span.end.index;
                    if let Some(c) = self.comments.get(self.ci)
                        && c.trailing
                        && c.start.line == span.end.line
                    {
                        footer.push(c.text.trim().to_string());
                        self.ci += 1;
                        end = c.start.index;
                    }
                    end = line_end_after(self.src, (end + 1).min(self.src.len()));
                }
                let range = (self.doc_start, end);
                self.doc_start = end;
                let info = self.doc.take().expect("document start precedes its end");
                let root = self
                    .root
                    .take()
                    .unwrap_or_else(|| Elem::new(Node::Scalar(Scalar::Null)));
                let first = info.first;
                self.docs.push(Doc::from_parts(
                    root,
                    first && self.leading_blank,
                    self.saw_cr,
                    info.version
                        .map(|(a, b)| (a.min(255) as u8, b.min(255) as u8)),
                    info.tags,
                    info.explicit_start,
                    explicit,
                    footer,
                    self.source.clone().map(|s| (s, range)),
                ));
            }
            Event::Scalar {
                anchor,
                tag,
                value,
                style,
                block,
            } => {
                let mut meta = Meta::default();
                self.begin_node(span.start, false, &mut meta);
                self.props(anchor, tag.as_ref(), &mut meta);
                meta.span = SourceSpan::from_span(span);
                meta.style = Some(match style {
                    ScalarStyle::Plain => Style::Plain,
                    ScalarStyle::SingleQuoted => Style::SingleQuoted,
                    ScalarStyle::DoubleQuoted => Style::DoubleQuoted,
                    ScalarStyle::Literal => Style::Literal(block.unwrap_or_default()),
                    ScalarStyle::Folded => Style::Folded(block.unwrap_or_default()),
                });
                let resolved = tag.as_ref().map(|t| t.resolved.as_ref());
                // Remember how a plain null or boolean was spelled.
                let spelled = style == ScalarStyle::Plain
                    && (schema::is_null(&value) || schema::parse_bool(&value).is_some());
                let raw = spelled.then(|| value.to_string());
                let scalar = schema::resolve(value.into_owned(), style, resolved);
                let canonical = match &scalar {
                    Scalar::Null => Some("null"),
                    Scalar::Bool(true) => Some("true"),
                    Scalar::Bool(false) => Some("false"),
                    _ => None,
                };
                if let (Some(raw), Some(canonical)) = (raw, canonical)
                    && raw != canonical
                {
                    meta.spelling = Some(raw);
                }
                let block_style = matches!(style, ScalarStyle::Literal | ScalarStyle::Folded);
                let was_key = self.role() == Role::Key;
                if block_style && !was_key {
                    self.header_comment(&mut meta, span);
                }
                self.complete(Elem {
                    node: Node::Scalar(scalar),
                    meta,
                });
                if !block_style {
                    self.completed(was_key, span.end);
                }
            }
            Event::Alias { name } => {
                if !self.anchors.contains(name.as_ref()) {
                    return Err(syntax_err(
                        span.start,
                        format!(
                            "alias `*{name}` refers to an anchor that is not defined before it"
                        ),
                    ));
                }
                let mut meta = Meta::default();
                self.begin_node(span.start, false, &mut meta);
                meta.span = SourceSpan::from_span(span);
                let was_key = self.role() == Role::Key;
                self.complete(Elem {
                    node: Node::Alias(name.into_owned()),
                    meta,
                });
                self.completed(was_key, span.end);
            }
            Event::SequenceStart { anchor, tag, flow } => {
                self.check_depth(span)?;
                let mut meta = Meta::default();
                self.begin_node(span.start, true, &mut meta);
                self.props(anchor, tag.as_ref(), &mut meta);
                meta.span = SourceSpan::from_span(span);
                meta.style = Some(if flow { Style::Flow } else { Style::Block });
                self.stack.push(Frame {
                    elem: Elem {
                        node: Node::Seq(Vec::new()),
                        meta,
                    },
                    entry_leading: Vec::new(),
                    key: None,
                    end: span.end.index,
                });
            }
            Event::MappingStart { anchor, tag, flow } => {
                self.check_depth(span)?;
                let mut meta = Meta::default();
                self.begin_node(span.start, true, &mut meta);
                self.props(anchor, tag.as_ref(), &mut meta);
                meta.span = SourceSpan::from_span(span);
                meta.style = Some(if flow { Style::Flow } else { Style::Block });
                self.stack.push(Frame {
                    elem: Elem {
                        node: Node::Map(Vec::new()),
                        meta,
                    },
                    entry_leading: Vec::new(),
                    key: None,
                    end: span.end.index,
                });
            }
            Event::SequenceEnd | Event::MappingEnd => {
                let frame = self
                    .stack
                    .pop()
                    .expect("a collection end follows its start");
                let mut elem = frame.elem;
                // Only a flow collection's end is a real position on its
                // line; a block collection's end event sits at the next
                // token, so it ends at its last token instead.
                let is_flow_end = span.end.index > span.start.index;
                elem.meta.span.set_end(if is_flow_end {
                    span.end.index
                } else {
                    frame.end
                });
                let was_key = self.role() == Role::Key;
                self.complete(elem);
                if is_flow_end {
                    self.completed(was_key, span.end);
                }
            }
        }
        Ok(())
    }
}

/// Plain, untagged, unanchored strings become `MapKey::Str`; every other
/// key keeps its node and metadata (so quoting and types survive).
fn make_key(elem: Elem) -> MapKey {
    let plain_string = elem.meta.anchor.is_none()
        && elem.meta.tag.is_none()
        && matches!(elem.meta.style, None | Some(Style::Plain))
        && matches!(elem.node, Node::Scalar(Scalar::Str(_)));
    if plain_string && let Node::Scalar(Scalar::Str(s)) = elem.node {
        return MapKey::Str(s);
    }
    MapKey::Complex(Box::new(elem))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one(input: &str) -> Doc {
        compose_first(input, &crate::ParseOptions::default()).unwrap()
    }

    #[test]
    fn composes_scalars_with_core_schema() {
        let d = one("a: 1\nb: true\nc: ~\nd: \"1\"\ne: x\nf:\n");
        let m = d.root().node().as_map().unwrap();
        assert!(matches!(m[0].1.node, Node::Scalar(Scalar::Num { .. })));
        assert_eq!(m[1].1.node, Node::Scalar(Scalar::Bool(true)));
        assert_eq!(m[2].1.node, Node::Scalar(Scalar::Null));
        assert_eq!(m[3].1.node, Node::Scalar(Scalar::Str("1".into())));
        assert!(m[3].1.meta.is_quoted());
        assert_eq!(m[5].1.node, Node::Scalar(Scalar::Null));
    }

    #[test]
    fn non_string_keys_are_complex() {
        let d = one("200: ok\nname: x\n");
        let m = d.root().node().as_map().unwrap();
        assert!(m[0].0.is_complex());
        assert_eq!(m[1].0, MapKey::Str("name".into()));
    }

    #[test]
    fn places_comments() {
        let d = one("# head\na: 1 # one\n# before b\nb:\n  # inside\n  c: 2\n# tail\n");
        let m = d.root().node().as_map().unwrap();
        assert_eq!(m[0].1.meta.leading_comments, vec!["head"]);
        assert_eq!(m[0].1.meta.trailing_comment.as_deref(), Some("one"));
        assert_eq!(m[1].1.meta.leading_comments, vec!["before b"]);
        let inner = m[1].1.node().as_map().unwrap();
        assert_eq!(inner[0].1.meta.leading_comments, vec!["inside"]);
        assert_eq!(d.footer_comments, vec!["tail"]);
    }

    #[test]
    fn comment_after_indicator_goes_to_value() {
        let d = one("a: # note\n  - x\n");
        let m = d.root().node().as_map().unwrap();
        assert_eq!(m[0].1.meta.trailing_comment.as_deref(), Some("note"));
    }

    #[test]
    fn block_scalar_header_comment() {
        let d = one("a: | # hdr\n  text\nb: 2\n");
        let m = d.root().node().as_map().unwrap();
        assert_eq!(m[0].1.meta.trailing_comment.as_deref(), Some("hdr"));
        assert_eq!(m[0].1.node, Node::Scalar(Scalar::Str("text\n".into())));
    }

    #[test]
    fn undefined_alias_is_an_error() {
        assert!(compose_stream("a: *x\n").is_err());
        assert!(compose_stream("a: *x\nb: &x 1\n").is_err());
        assert!(compose_stream("a: &x 1\nb: *x\n").is_ok());
    }

    #[test]
    fn duplicate_keys_are_kept() {
        let d = one("a: 1\na: 2\n");
        assert_eq!(d.root().node().as_map().unwrap().len(), 2);
    }
}
