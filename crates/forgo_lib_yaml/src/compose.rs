//! Composer: parse events → documents (`Doc`), with comments attached.
//!
//! Comment placement (comments are scanned on the side and merged here by
//! position):
//! - full-line comments become `leading_comments` of the next map entry
//!   (stored on its value), sequence item, or document root;
//! - an end-of-line comment becomes the `trailing_comment` of the node that
//!   ends on that line; one that follows only an indicator (`key:` or `-`
//!   with the value on a later line) goes to that value;
//! - comments after a document's last node go to the document footer.
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

/// Compose every document in `input`.
pub(crate) fn compose_stream(input: &str) -> Result<Vec<Doc>, Error> {
    let mut events = syntax::events(input);
    let mut evs = Vec::new();
    for r in &mut events {
        evs.push(r?);
    }
    let comments = events.comments().to_vec();
    let mut c = Composer {
        input,
        comments,
        ci: 0,
        leading: Vec::new(),
        carry: None,
        stack: Vec::new(),
        root: None,
        anchors: HashSet::new(),
        doc: None,
        docs: Vec::new(),
    };
    for (ev, span) in evs {
        c.event(ev, span)?;
    }
    Ok(c.docs)
}

/// Compose the first document, or an empty one (keeping any comments) when
/// the stream has none.
pub(crate) fn compose_first(input: &str) -> Result<Doc, Error> {
    let mut docs = compose_stream(input)?;
    if docs.is_empty() {
        let comments = syntax_comments(input)?;
        return Ok(Doc::empty_with_comments(comments));
    }
    Ok(docs.swap_remove(0))
}

fn syntax_comments(input: &str) -> Result<Vec<String>, Error> {
    let mut events = syntax::events(input);
    for r in &mut events {
        r?;
    }
    Ok(events
        .comments()
        .iter()
        .map(|c| c.text.trim().to_string())
        .collect())
}

enum Frame {
    Seq(Elem),
    Map {
        elem: Elem,
        /// Leading comments of the entry being composed.
        entry_leading: Vec<String>,
        /// The entry's key, once complete.
        key: Option<MapKey>,
    },
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Role {
    Root,
    Key,
    Value,
    Item,
}

struct DocInfo {
    explicit_start: bool,
    version: Option<(u32, u32)>,
    tags: Vec<(String, String)>,
    first: bool,
}

struct Composer<'a> {
    input: &'a str,
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
            Some(Frame::Seq(_)) => Role::Item,
            Some(Frame::Map { key: None, .. }) => Role::Key,
            Some(Frame::Map { .. }) => Role::Value,
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
                if let Some(Frame::Map { entry_leading, .. }) = self.stack.last_mut() {
                    *entry_leading = l;
                }
            }
            Role::Value => {
                if let Some(Frame::Map { entry_leading, .. }) = self.stack.last_mut() {
                    meta.leading_comments = std::mem::take(entry_leading);
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

    /// Attach an end-of-line comment that follows a node ending at `end`.
    /// With `within`, a trailing comment before that offset (a block
    /// scalar's header comment) also qualifies.
    fn grab_trailing(&mut self, meta: &mut Meta, end: Mark, within: Option<usize>) {
        let Some(c) = self.comments.get(self.ci) else {
            return;
        };
        if !c.trailing {
            return;
        }
        let ok = match within {
            // A block scalar's header comment: inside its span, or on its
            // first line when the scalar has no content.
            Some(limit) => c.start.index < limit || c.start.line == end.line,
            None => c.start.line == end.line && c.start.index >= end.index,
        };
        if ok {
            let text = c.text.trim().to_string();
            append_comment(&mut meta.trailing_comment, text);
            self.ci += 1;
        }
    }

    fn complete(&mut self, elem: Elem) {
        match self.stack.last_mut() {
            None => self.root = Some(elem),
            Some(Frame::Seq(seq)) => {
                if let Node::Seq(items) = &mut seq.node {
                    items.push(elem);
                }
            }
            Some(Frame::Map { elem: map, key, .. }) => match key.take() {
                Some(k) => {
                    if let Node::Map(entries) = &mut map.node {
                        entries.push((k, elem));
                    }
                }
                None => *key = Some(make_key(elem)),
            },
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

    fn event(&mut self, ev: Event<'a>, span: Span) -> Result<(), Error> {
        match ev {
            Event::StreamStart => {}
            Event::StreamEnd => {
                self.drain_until(usize::MAX);
                let rest: Vec<String> = std::mem::take(&mut self.leading)
                    .into_iter()
                    .chain(self.carry.take())
                    .collect();
                if let Some(d) = self.docs.last_mut() {
                    d.footer_comments.extend(rest);
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
                let footer: Vec<String> = std::mem::take(&mut self.leading)
                    .into_iter()
                    .chain(self.carry.take())
                    .collect();
                let info = self.doc.take().expect("document start precedes its end");
                let root = self
                    .root
                    .take()
                    .unwrap_or_else(|| Elem::new(Node::Scalar(Scalar::Null)));
                let first = info.first;
                let input = self.input.trim_start_matches('\u{FEFF}');
                self.docs.push(Doc::from_parts(
                    root,
                    first && input.starts_with(['\n', '\r']),
                    self.input.contains('\r'),
                    info.version
                        .map(|(a, b)| (a.min(255) as u8, b.min(255) as u8)),
                    info.tags,
                    info.explicit_start,
                    explicit,
                    footer,
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
                meta.span = SourceSpan(Some(span));
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
                if self.role() != Role::Key {
                    let within = block_style.then_some(span.end.index);
                    self.grab_trailing(&mut meta, span.end, within);
                }
                self.complete(Elem {
                    node: Node::Scalar(scalar),
                    meta,
                });
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
                meta.span = SourceSpan(Some(span));
                if self.role() != Role::Key {
                    self.grab_trailing(&mut meta, span.end, None);
                }
                self.complete(Elem {
                    node: Node::Alias(name.into_owned()),
                    meta,
                });
            }
            Event::SequenceStart { anchor, tag, flow } => {
                let mut meta = Meta::default();
                self.begin_node(span.start, true, &mut meta);
                self.props(anchor, tag.as_ref(), &mut meta);
                meta.span = SourceSpan(Some(span));
                meta.style = Some(if flow { Style::Flow } else { Style::Block });
                self.stack.push(Frame::Seq(Elem {
                    node: Node::Seq(Vec::new()),
                    meta,
                }));
            }
            Event::MappingStart { anchor, tag, flow } => {
                let mut meta = Meta::default();
                self.begin_node(span.start, true, &mut meta);
                self.props(anchor, tag.as_ref(), &mut meta);
                meta.span = SourceSpan(Some(span));
                meta.style = Some(if flow { Style::Flow } else { Style::Block });
                self.stack.push(Frame::Map {
                    elem: Elem {
                        node: Node::Map(Vec::new()),
                        meta,
                    },
                    entry_leading: Vec::new(),
                    key: None,
                });
            }
            Event::SequenceEnd | Event::MappingEnd => {
                let mut elem = match self.stack.pop() {
                    Some(Frame::Seq(e)) => e,
                    Some(Frame::Map { elem, .. }) => elem,
                    None => unreachable!("collection end without start"),
                };
                // Only a flow collection's end is a real position on its line.
                let is_flow_end = span.end.index > span.start.index;
                if let SourceSpan(Some(s)) = &mut elem.meta.span {
                    s.end = span.end;
                }
                if is_flow_end && self.role() != Role::Key {
                    self.grab_trailing(&mut elem.meta, span.end, None);
                }
                self.complete(elem);
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
        compose_first(input).unwrap()
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
