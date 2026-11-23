//! Format-preserving output.
//!
//! A parsed [`Doc`] keeps its source text (`ast::Origin`). Writing it back:
//!
//! - an untouched document is its source text, byte for byte;
//! - an edited document is re-parsed from its source and compared with
//!   the edited tree. In each block collection that changed:
//!   - an entry that didn't change is copied, with the comments and blank
//!     lines above it;
//!   - a changed scalar, alias or flow value whose new text fits on one
//!     line replaces the old text in place, keeping what surrounds it;
//!   - in a flow collection that kept its shape (a JSON-style document,
//!     say), each changed scalar is replaced in place, at any depth;
//!   - a changed nested block collection is handled the same way, inside;
//!   - anything else (new entries, other changes) is rendered by the
//!     emitter, in the indentation style the file uses.
//!
//! The result must parse back to the edited document, comments and styles
//! included. Otherwise the caller renders the whole document instead.

use crate::ast::{Doc, Elem, MapKey, Meta, Node};
use crate::compose::{compose, line_end_after, text_end};
use crate::emitter::Emitter;
use std::collections::HashMap;

/// The document's text keeping its source layout. `None` when it has no
/// source, or the spliced text doesn't read back as the document.
pub(crate) fn doc_text(doc: &Doc) -> Option<String> {
    let o = &doc.origin;
    let src = o.source.as_deref()?.get(o.range.0..o.range.1)?;
    let fields_same = doc.yaml_version == o.yaml_version
        && doc.tag_handles == o.tag_handles
        && doc.explicit_start == o.explicit_start
        && doc.explicit_end == o.explicit_end;
    if !o.touched && fields_same {
        return Some(src.to_string());
    }
    let orig = reparse(src)?;
    if *doc == orig {
        return Some(src.to_string());
    }
    let text = splice_doc(doc, &orig, src, o.range.0)?;
    equivalent_doc(doc, &reparse(&text)?).then_some(text)
}

/// A stream of documents, each keeping its source layout where it has
/// one, with `---` or `...` added where documents now meet differently.
/// `None` when no document has a source, or the result doesn't read back.
pub(crate) fn stream_text(docs: &[Doc]) -> Option<String> {
    if docs.iter().all(|d| d.origin.source.is_none()) {
        return None;
    }
    // Untouched documents that make up a whole source: that source.
    if let Some(text) = whole_source(docs) {
        return Some(text.to_string());
    }
    let mut out = String::new();
    let mut prev_from_source = false;
    for (i, d) in docs.iter().enumerate() {
        let (text, from_source) = match doc_text(d) {
            Some(t) => (t, true),
            None => (d.render().ok()?, false),
        };
        // Documents that follow each other in the same source already
        // carry what separates them.
        let adjacent = i > 0 && from_source && prev_from_source && {
            let (p, o) = (&docs[i - 1].origin, &d.origin);
            matches!((&p.source, &o.source), (Some(a), Some(b)) if std::sync::Arc::ptr_eq(a, b))
                && p.range.1 == o.range.0
        };
        prev_from_source = from_source;
        if i > 0 && !adjacent {
            let nl = if text.contains("\r\n") { "\r\n" } else { "\n" };
            if !out.is_empty() && !out.ends_with(['\n', '\r']) {
                out.push_str(nl);
            }
            if !docs[i - 1].explicit_end {
                match first_line(&text) {
                    Some(l) if l.starts_with('%') => {
                        out.push_str("...");
                        out.push_str(nl);
                    }
                    Some(l) if is_marker(l) => {}
                    _ => {
                        out.push_str("---");
                        out.push_str(nl);
                    }
                }
            }
        }
        out.push_str(&text);
    }
    let back = compose(&out, &crate::ParseOptions::default(), false).ok()?;
    let back: Vec<Doc> = back.into_iter().filter(|d| !d.is_placeholder()).collect();
    let docs: Vec<&Doc> = docs.iter().filter(|d| !d.is_placeholder()).collect();
    let ok = back.len() == docs.len() && docs.iter().zip(&back).all(|(d, b)| equivalent_doc(d, b));
    ok.then_some(out)
}

/// The source of `docs` when they are all of it, untouched and in order.
fn whole_source(docs: &[Doc]) -> Option<&str> {
    let first = docs.first()?.origin.source.as_ref()?;
    let mut at = 0;
    for d in docs {
        let o = &d.origin;
        let untouched = !o.touched
            && d.yaml_version == o.yaml_version
            && d.tag_handles == o.tag_handles
            && d.explicit_start == o.explicit_start
            && d.explicit_end == o.explicit_end;
        let same = o
            .source
            .as_ref()
            .is_some_and(|s| std::sync::Arc::ptr_eq(s, first));
        if !untouched || !same || o.range.0 != at {
            return None;
        }
        at = o.range.1;
    }
    (at == first.len()).then_some(&**first)
}

/// The first line that isn't blank or a comment.
fn first_line(text: &str) -> Option<&str> {
    text.trim_start_matches('\u{FEFF}')
        .lines()
        .find(|l| !trivia(l))
}

/// `---` followed by white space or the end of the line.
fn is_marker(line: &str) -> bool {
    line.starts_with("---") && matches!(line.as_bytes().get(3), None | Some(b' ' | b'\t' | b'\r'))
}

fn reparse(src: &str) -> Option<Doc> {
    let mut docs = compose(src, &crate::ParseOptions::default(), false).ok()?;
    (docs.len() == 1).then(|| docs.swap_remove(0))
}

/// `back` (parsed from what was written for `doc`) reads as `doc`: equal,
/// except where `doc` leaves a choice to the emitter (no recorded style or
/// spelling, or a plain string key).
pub(crate) fn equivalent_doc(doc: &Doc, back: &Doc) -> bool {
    equivalent(&doc.root, &back.root)
        && doc.footer_comments == back.footer_comments
        && doc.yaml_version == back.yaml_version
        && doc.tag_handles == back.tag_handles
        && doc.explicit_start == back.explicit_start
        && doc.explicit_end == back.explicit_end
        && doc.is_placeholder() == back.is_placeholder()
}

fn equivalent(a: &Elem, b: &Elem) -> bool {
    let (m, n) = (&a.meta, &b.meta);
    let meta = m.leading_comments == n.leading_comments
        && m.trailing_comment == n.trailing_comment
        && m.anchor == n.anchor
        && m.tag == n.tag
        && (m.style.is_none() || m.style == n.style)
        && (m.spelling.is_none() || m.spelling == n.spelling);
    meta && match (&a.node, &b.node) {
        (Node::Seq(x), Node::Seq(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(p, q)| equivalent(p, q))
        }
        (Node::Map(x), Node::Map(y)) => {
            x.len() == y.len()
                && x.iter()
                    .zip(y)
                    .all(|((k, v), (l, w))| equivalent_key(k, l) && equivalent(v, w))
        }
        (x, y) => x == y,
    }
}

/// A string key without properties or comments, however it is quoted.
fn bare_string(k: &MapKey) -> Option<&str> {
    match k {
        MapKey::Str(s) => Some(s),
        MapKey::Complex(e) => match &e.node {
            Node::Scalar(crate::ast::Scalar::Str(s))
                if e.meta.anchor.is_none()
                    && e.meta.tag.is_none()
                    && e.meta.leading_comments.is_empty()
                    && e.meta.trailing_comment.is_none() =>
            {
                Some(s)
            }
            _ => None,
        },
    }
}

fn equivalent_key(a: &MapKey, b: &MapKey) -> bool {
    match (a, b) {
        (MapKey::Complex(x), MapKey::Complex(y)) if equivalent(x, y) => true,
        // The emitter picks how a `MapKey::Str` (or an unstyled string
        // key) is quoted.
        (MapKey::Str(_), _) | (MapKey::Complex(_), MapKey::Str(_)) => {
            let unstyled = match a {
                MapKey::Complex(e) => e.meta.style.is_none() || b.is_str(),
                MapKey::Str(_) => true,
            };
            unstyled && bare_string(a).is_some() && bare_string(a) == bare_string(b)
        }
        _ => false,
    }
}

/// A blank line or a comment line.
fn trivia(line: &str) -> bool {
    let t = line.trim_matches([' ', '\t', '\r', '\n']);
    t.is_empty() || t.starts_with('#')
}

fn line_start(src: &str, pos: usize) -> usize {
    src[..pos].rfind('\n').map_or(0, |i| i + 1)
}

fn column(src: &str, pos: usize) -> usize {
    pos - line_start(src, pos)
}

fn indentation(s: &str) -> usize {
    s.len() - s.trim_start_matches(' ').len()
}

fn has_props(e: &Elem) -> bool {
    e.meta.anchor.is_some() || e.meta.tag.is_some()
}

fn block_collection(e: &Elem) -> bool {
    match &e.node {
        Node::Map(m) => !m.is_empty() && !e.meta.is_flow(),
        Node::Seq(s) => !s.is_empty() && !e.meta.is_flow(),
        _ => false,
    }
}

/// A value that may be replaced within its line: not a block scalar and
/// not a block collection.
fn one_line_kind(e: &Elem) -> bool {
    match &e.node {
        Node::Scalar(_) => !matches!(
            e.meta.style,
            Some(crate::ast::Style::Literal(_) | crate::ast::Style::Folded(_))
        ),
        Node::Alias(_) => true,
        Node::Seq(_) | Node::Map(_) => !block_collection(e),
    }
}

/// Everything but the leading comments (which belong to the entry).
fn same_meta(a: &Meta, b: &Meta) -> bool {
    a.trailing_comment == b.trailing_comment
        && a.anchor == b.anchor
        && a.tag == b.tag
        && a.style == b.style
        && a.spelling == b.spelling
}

fn same_value(a: &Elem, b: &Elem) -> bool {
    a.node == b.node && same_meta(&a.meta, &b.meta)
}

/// Pair each current entry with the source entry it came from: the one
/// whose value has the same source position, else (maps) the first
/// unused one with an equal key.
fn pair<T>(
    cur: &[T],
    orig: &[T],
    base: usize,
    value: impl Fn(&T) -> &Elem,
    same_key: impl Fn(&T, &T) -> bool,
) -> Vec<Option<usize>> {
    let span = |e: &Elem, base: usize| {
        let r = e.meta.span.range()?;
        Some((r.start.checked_sub(base)?, r.end.checked_sub(base)?))
    };
    let mut at: HashMap<(usize, usize), usize> = HashMap::new();
    for (j, o) in orig.iter().enumerate() {
        if let Some(s) = span(value(o), 0) {
            at.entry(s).or_insert(j);
        }
    }
    let mut used = vec![false; orig.len()];
    let mut pairs: Vec<Option<usize>> = cur
        .iter()
        .map(|c| {
            let j = *at.get(&span(value(c), base)?)?;
            (!std::mem::replace(&mut used[j], true)).then_some(j)
        })
        .collect();
    for (i, c) in cur.iter().enumerate() {
        if pairs[i].is_none()
            && let Some(j) = (0..orig.len()).find(|&j| !used[j] && same_key(c, &orig[j]))
        {
            used[j] = true;
            pairs[i] = Some(j);
        }
    }
    pairs
}

fn splice_doc(cur: &Doc, orig: &Doc, src: &str, base: usize) -> Option<String> {
    // Bare CR line breaks aren't handled line by line here.
    let bytes = src.as_bytes();
    if (0..bytes.len()).any(|i| bytes[i] == b'\r' && bytes.get(i + 1) != Some(&b'\n')) {
        return None;
    }
    if cur.yaml_version != orig.yaml_version
        || cur.tag_handles != orig.tag_handles
        || cur.explicit_start != orig.explicit_start
        || cur.is_placeholder()
        || orig.is_placeholder()
    {
        return None;
    }
    let (r, o) = (&cur.root, &orig.root);
    let sp = Splicer {
        src,
        base,
        em: infer_layout(o, src),
        crlf: src.contains("\r\n"),
    };
    // A flow root (a JSON-style document) that kept its shape.
    if o.meta.is_flow()
        && cur.footer_comments == orig.footer_comments
        && cur.explicit_end == orig.explicit_end
        && let (Some(range), Some(text)) = (o.meta.span.range(), sp.patch_flow(r, o))
    {
        return Some(format!(
            "{}{text}{}",
            &src[..range.start],
            &src[range.end..]
        ));
    }
    if !block_collection(r)
        || !block_collection(o)
        || r.node.is_map() != o.node.is_map()
        || !same_meta(&r.meta, &o.meta)
        || r.meta.leading_comments != o.meta.leading_comments
    {
        return None;
    }
    let first = sp.first_head(o);
    let b0 = if has_props(o) {
        sp.after_props(o, o.meta.span.range()?.start)
    } else {
        sp.trivia_above(first)
    };
    let mut out = String::from(&src[..b0]);
    let end = sp.block(r, o, column(src, first), b0, false, &mut out)?;
    if cur.footer_comments == orig.footer_comments && cur.explicit_end == orig.explicit_end {
        out.push_str(&src[end..]);
    } else {
        let mut f = String::new();
        if !out.is_empty() && !out.ends_with('\n') {
            f.push('\n');
        }
        for c in &cur.footer_comments {
            f.push('#');
            if !c.is_empty() {
                f.push(' ');
                f.push_str(c);
            }
            f.push('\n');
        }
        if cur.explicit_end {
            f.push_str("...\n");
        }
        sp.push_fresh(&f, &mut out);
    }
    Some(out)
}

/// The indentation the source uses: the most common step for nested
/// mappings, and for sequences under a key (0 when `- ` sits at the key's
/// column).
fn infer_layout(root: &Elem, src: &str) -> Emitter {
    let mut map_steps = [0usize; 9];
    let mut seq_steps = [0usize; 9];
    let sp = Splicer {
        src,
        base: 0,
        em: Emitter::default(),
        crlf: false,
    };
    let mut stack = vec![(root, column(src, sp.first_head(root)))];
    while let Some((e, col)) = stack.pop() {
        let children: Vec<(&Elem, bool)> = match &e.node {
            Node::Map(entries) => entries.iter().map(|(_, v)| (v, true)).collect(),
            Node::Seq(items) => items.iter().map(|v| (v, false)).collect(),
            _ => Vec::new(),
        };
        for (child, in_map) in children {
            if !block_collection(child) || child.meta.span.range().is_none() {
                continue;
            }
            let c = column(src, sp.first_head(child));
            if in_map && c >= col && c - col < 9 {
                if child.node.is_seq() {
                    seq_steps[c - col] += 1;
                } else {
                    map_steps[c - col] += 1;
                }
            }
            stack.push((child, c));
        }
    }
    // The most common step; ties go to the smaller one.
    let mode = |steps: &[usize], from: usize| {
        (from..steps.len())
            .filter(|&i| steps[i] > 0)
            .max_by_key(|&i| (steps[i], std::cmp::Reverse(i)))
    };
    let indent = mode(&map_steps, 1).unwrap_or(2);
    Emitter {
        indent,
        seq_indent: mode(&seq_steps, 0).unwrap_or(indent),
    }
}

struct Splicer<'s> {
    /// The document's source text; spans of the re-parsed tree index it.
    src: &'s str,
    /// Where `src` starts in the edited tree's source (its spans).
    base: usize,
    /// Renders new and rewritten parts.
    em: Emitter,
    /// The source uses CRLF line breaks.
    crlf: bool,
}

impl Splicer<'_> {
    fn push_fresh(&self, s: &str, out: &mut String) {
        if self.crlf {
            out.push_str(&s.replace('\n', "\r\n"));
        } else {
            out.push_str(s);
        }
    }

    /// Render with the emitter.
    fn fresh(
        &self,
        f: impl FnOnce(&Emitter, &mut String) -> Result<(), crate::Error>,
        out: &mut String,
    ) -> Option<()> {
        let mut s = String::new();
        f(&self.em, &mut s).ok()?;
        self.push_fresh(&s, out);
        Some(())
    }

    /// The start of the first line at or after `pos` that isn't blank or
    /// a comment (`end` if there is none before it).
    fn head_line(&self, mut pos: usize, end: usize) -> usize {
        while pos < end {
            let next = line_end_after(self.src, pos + 1);
            if !trivia(&self.src[pos..next]) {
                return pos;
            }
            pos = next;
        }
        end
    }

    /// The start of the run of blank and comment lines right above `pos`'s
    /// line.
    fn trivia_above(&self, pos: usize) -> usize {
        let mut start = line_start(self.src, pos);
        while start > 0 {
            let prev = line_start(self.src, start - 1);
            if !trivia(&self.src[prev..start]) {
                break;
            }
            start = prev;
        }
        start
    }

    /// Where a block collection's first entry (key, `?` or `-`) starts.
    fn first_head(&self, e: &Elem) -> usize {
        let start = e.meta.span.range().map_or(0, |r| r.start);
        if !has_props(e) {
            return start;
        }
        let line = self.head_line(self.after_props(e, start), self.src.len());
        line + indentation(&self.src[line..])
    }

    /// The start of the line after the properties of `e` (`&anchor`,
    /// `!tag`; they may be on separate lines), which start at `start`.
    fn after_props(&self, e: &Elem, start: usize) -> usize {
        let b = self.src.as_bytes();
        let (mut anchor, mut tag) = (e.meta.anchor.is_some(), e.meta.tag.is_some());
        let (mut i, mut last) = (start, start);
        while anchor || tag {
            while i < b.len() && matches!(b[i], b' ' | b'\t' | b'\r' | b'\n') {
                i += 1;
            }
            match b.get(i) {
                Some(b'&') if anchor => anchor = false,
                Some(b'!') if tag => tag = false,
                _ => break,
            }
            if b.get(i + 1) == Some(&b'<') {
                i += self.src[i..].find('>').map_or(b.len() - i, |j| j + 1);
            } else {
                while i < b.len() && !matches!(b[i], b' ' | b'\t' | b'\r' | b'\n') {
                    i += 1;
                }
            }
            last = i;
        }
        line_end_after(self.src, last.max(start + 1))
    }

    /// The source chunks of a block collection's entries: each runs from
    /// the end of the previous one (`b0` for the first) through the line
    /// of its last token, so it holds the comments and blank lines above
    /// the entry. With `inline_first`, the first entry starts at `b0`.
    fn chunks(&self, orig: &Elem, b0: usize, inline_first: bool) -> Vec<(usize, usize)> {
        let parts: Vec<(Option<&Elem>, &Elem)> = match &orig.node {
            Node::Map(entries) => entries
                .iter()
                .map(|(k, v)| {
                    let key = match k {
                        MapKey::Complex(ke) => Some(&**ke),
                        MapKey::Str(_) => None,
                    };
                    (key, v)
                })
                .collect(),
            Node::Seq(items) => items.iter().map(|v| (None, v)).collect(),
            _ => Vec::new(),
        };
        let mut out = Vec::with_capacity(parts.len());
        let mut start = b0;
        for (i, (key, value)) in parts.into_iter().enumerate() {
            let head = if inline_first && i == 0 {
                start
            } else {
                self.head_line(start, self.src.len())
            };
            let mut end = line_end_after(self.src, (head + 1).min(self.src.len()));
            for e in key.into_iter().chain([value]) {
                if let Some(e) = text_end(self.src, &e.meta.span) {
                    end = end.max(line_end_after(self.src, e));
                }
            }
            if keeps_trailing_lines(value) {
                // Blank lines after a `|+` scalar are its content.
                while end < self.src.len() {
                    let next = line_end_after(self.src, end + 1);
                    if !self.src[end..next]
                        .trim_matches([' ', '\t', '\r', '\n'])
                        .is_empty()
                    {
                        break;
                    }
                    end = next;
                }
            }
            out.push((start, end));
            start = end;
        }
        out
    }

    /// Write a changed block collection: `cur`, parsed as `orig` with
    /// entries at column `col` from `b0`. Returns where `orig`'s text ends.
    fn block(
        &self,
        cur: &Elem,
        orig: &Elem,
        col: usize,
        b0: usize,
        inline_first: bool,
        out: &mut String,
    ) -> Option<usize> {
        let chunks = self.chunks(orig, b0, inline_first);
        let end = chunks.last().map_or(b0, |c| c.1);
        match (&cur.node, &orig.node) {
            (Node::Map(ce), Node::Map(oe)) => {
                let pairs = pair(ce, oe, self.base, |e| &e.1, |a, b| a.0 == b.0);
                for (i, (k, v)) in ce.iter().enumerate() {
                    let inline = inline_first && i == 0;
                    line_break_before(inline, self.crlf, out);
                    match pairs[i] {
                        Some(j) => {
                            let (k0, v0) = &oe[j];
                            let at = Pos {
                                chunk: chunks[j],
                                was_inline: inline_first && j == 0,
                                inline,
                                col,
                            };
                            self.entry(k, v, k0, v0, at, out)?;
                        }
                        None => {
                            if inline && !v.meta.leading_comments.is_empty() {
                                return None;
                            }
                            self.fresh(|em, s| em.emit_entry(k, v, col, inline, s), out)?;
                        }
                    }
                }
            }
            (Node::Seq(ci), Node::Seq(oi)) => {
                let pairs = pair(ci, oi, self.base, |e| e, |_, _| false);
                for (i, it) in ci.iter().enumerate() {
                    let inline = inline_first && i == 0;
                    line_break_before(inline, self.crlf, out);
                    match pairs[i] {
                        Some(j) => {
                            let at = Pos {
                                chunk: chunks[j],
                                was_inline: inline_first && j == 0,
                                inline,
                                col,
                            };
                            self.item(it, &oi[j], at, out)?;
                        }
                        None => {
                            if inline && !it.meta.leading_comments.is_empty() {
                                return None;
                            }
                            self.fresh(|em, s| em.emit_item(it, col, inline, s), out)?;
                        }
                    }
                }
            }
            _ => return None,
        }
        Some(end)
    }

    /// The start of an entry's own line (after its leading lines).
    fn head(&self, at: &Pos) -> usize {
        if at.was_inline {
            at.chunk.0
        } else {
            self.head_line(at.chunk.0, at.chunk.1)
        }
    }

    /// Copy an unchanged entry, moving it on or off a `- ` line if needed.
    fn copy(&self, at: &Pos, out: &mut String) -> Option<()> {
        let (cs, ce) = at.chunk;
        let head = self.head(at);
        match (at.was_inline, at.inline) {
            (false, false) | (true, true) => out.push_str(&self.src[cs..ce]),
            (true, false) => {
                out.extend(std::iter::repeat_n(' ', at.col));
                out.push_str(&self.src[cs..ce]);
            }
            (false, true) => {
                if self.src[cs..head].contains('#') {
                    return None;
                }
                let first = head + indentation(&self.src[head..]);
                out.push_str(&self.src[first..ce]);
            }
        }
        Some(())
    }

    /// The lines above an entry: kept when its comments didn't change.
    /// Returns where the entry's own text starts, having written the line
    /// prefix (indentation) unless it goes on a `- ` line.
    fn lead(&self, cur: &[String], orig: &[String], at: &Pos, out: &mut String) -> Option<usize> {
        let (cs, _) = at.chunk;
        let head = self.head(at);
        let above = &self.src[cs..head];
        if at.inline {
            // Nothing can sit between `- ` and the first entry.
            if !cur.is_empty() || above.contains('#') {
                return None;
            }
        } else if cur == orig {
            out.push_str(above);
        } else {
            let blank = above.lines().take_while(|l| l.trim().is_empty()).count();
            let mut s = "\n".repeat(blank);
            for c in cur {
                s.extend(std::iter::repeat_n(' ', at.col));
                s.push('#');
                if !c.is_empty() {
                    s.push(' ');
                    s.push_str(c);
                }
                s.push('\n');
            }
            self.push_fresh(&s, out);
        }
        if at.was_inline {
            if !at.inline {
                out.extend(std::iter::repeat_n(' ', at.col));
            }
            return Some(head);
        }
        let first = head + indentation(&self.src[head..]);
        if !at.inline {
            out.push_str(&self.src[head..first]);
        }
        Some(first)
    }

    fn entry(
        &self,
        k: &MapKey,
        v: &Elem,
        k0: &MapKey,
        v0: &Elem,
        at: Pos,
        out: &mut String,
    ) -> Option<()> {
        if k == k0 && v == v0 {
            return self.copy(&at, out);
        }
        let body = self.lead(
            &v.meta.leading_comments,
            &v0.meta.leading_comments,
            &at,
            out,
        )?;
        let mark = out.len();
        let kept = if k == k0 {
            self.value(v, v0, body, at.chunk.1, at.col, out)
        } else if same_value(v, v0) {
            self.rekey(k, k0, body, at.chunk.1, at.col, out)
        } else {
            None
        };
        if kept.is_some() {
            return Some(());
        }
        out.truncate(mark);
        self.fresh(|em, s| em.emit_entry(k, v, at.col, true, s), out)
    }

    fn item(&self, it: &Elem, it0: &Elem, at: Pos, out: &mut String) -> Option<()> {
        if it == it0 {
            return self.copy(&at, out);
        }
        let body = self.lead(
            &it.meta.leading_comments,
            &it0.meta.leading_comments,
            &at,
            out,
        )?;
        let mark = out.len();
        if self.value(it, it0, body, at.chunk.1, at.col, out).is_some() {
            return Some(());
        }
        out.truncate(mark);
        self.fresh(|em, s| em.emit_item(it, at.col, true, s), out)
    }

    /// Write a changed value whose entry text (key or `-`) starts at
    /// `body` and runs to `end`, keeping what can be kept.
    #[allow(clippy::too_many_arguments)]
    fn value(
        &self,
        v: &Elem,
        v0: &Elem,
        body: usize,
        end: usize,
        col: usize,
        out: &mut String,
    ) -> Option<()> {
        let src = self.src;
        let r = v0.meta.span.range()?;
        let (vs, ve) = (r.start, r.end);
        if vs < body || ve > end {
            return None;
        }
        // A nested block collection: keep its header line, splice inside.
        if block_collection(v)
            && block_collection(v0)
            && v.node.is_map() == v0.node.is_map()
            && same_meta(&v.meta, &v0.meta)
        {
            // Compact: it starts on the line of its indicator (`- a: 1`,
            // `: - b` after `? key`).
            let before = src[line_start(src, vs)..vs].trim_matches([' ', '\t']);
            let compact = !has_props(v0)
                && vs > body
                && !before.is_empty()
                && before
                    .bytes()
                    .all(|c| matches!(c, b'-' | b':' | b'?' | b' ' | b'\t'));
            if compact {
                out.push_str(&src[body..vs]);
                self.block(v, v0, column(src, vs), vs, true, out)?;
            } else {
                let b0 = if has_props(v0) {
                    self.after_props(v0, vs)
                } else {
                    line_end_after(src, body + 1)
                };
                if b0 > end {
                    return None;
                }
                out.push_str(&src[body..b0]);
                let first = self.first_head(v0);
                self.block(v, v0, column(src, first), b0, false, out)?;
            }
            return Some(());
        }
        // A scalar, alias or flow value (on the entry's line or below it):
        // replace its text, keeping what surrounds it. A flow collection
        // that kept its shape keeps its layout too.
        if one_line_kind(v0) && ve > vs && line_end_after(src, ve) == end {
            let text = match self.patch_flow(v, v0) {
                Some(t) => t,
                None => self.em.inline_value(v, col)?,
            };
            out.push_str(&src[body..vs]);
            self.push_fresh(&text, out);
            if v.meta.trailing_comment == v0.meta.trailing_comment {
                out.push_str(&src[ve..end]);
            } else {
                let mut s = String::new();
                if let Some(tc) = &v.meta.trailing_comment {
                    s.push_str(" # ");
                    s.push_str(tc);
                }
                s.push('\n');
                self.push_fresh(&s, out);
            }
            return Some(());
        }
        None
    }

    /// The text of flow collection `orig` with the changes in `cur` made in
    /// place, when its shape (length, keys, properties, comments) didn't
    /// change: changed scalars and aliases are replaced, nested flow
    /// collections patched the same way.
    fn patch_flow(&self, cur: &Elem, orig: &Elem) -> Option<String> {
        let r = orig.meta.span.range()?;
        let mut patches = Vec::new();
        self.flow_patches(cur, orig, &mut patches)?;
        patches.sort_by_key(|p| p.0);
        let mut out = String::new();
        let mut at = r.start;
        for (s, e, text) in patches {
            if s < at || e > r.end {
                return None;
            }
            out.push_str(&self.src[at..s]);
            out.push_str(&text);
            at = e;
        }
        out.push_str(&self.src[at..r.end]);
        Some(out)
    }

    fn flow_patches(
        &self,
        cur: &Elem,
        orig: &Elem,
        patches: &mut Vec<(usize, usize, String)>,
    ) -> Option<()> {
        if !orig.meta.is_flow()
            || !same_meta(&cur.meta, &orig.meta)
            || cur.meta.leading_comments != orig.meta.leading_comments
        {
            return None;
        }
        match (&cur.node, &orig.node) {
            (Node::Seq(a), Node::Seq(b)) if a.len() == b.len() => {
                for (x, y) in a.iter().zip(b) {
                    self.flow_patch(x, y, false, patches)?;
                }
            }
            (Node::Map(a), Node::Map(b)) if a.len() == b.len() => {
                for ((k, x), (l, y)) in a.iter().zip(b) {
                    if k != l {
                        return None;
                    }
                    self.flow_patch(x, y, true, patches)?;
                }
            }
            _ => return None,
        }
        Some(())
    }

    fn flow_patch(
        &self,
        cur: &Elem,
        orig: &Elem,
        in_map: bool,
        patches: &mut Vec<(usize, usize, String)>,
    ) -> Option<()> {
        if cur == orig {
            return Some(());
        }
        let collection = |e: &Elem| matches!(e.node, Node::Seq(_) | Node::Map(_));
        if collection(cur) && collection(orig) {
            return self.flow_patches(cur, orig, patches);
        }
        let r = orig.meta.span.range()?;
        let comments_same = cur.meta.leading_comments == orig.meta.leading_comments
            && cur.meta.trailing_comment == orig.meta.trailing_comment;
        if collection(cur) || collection(orig) || !comments_same || r.end <= r.start {
            return None;
        }
        patches.push((r.start, r.end, self.em.flow_text(cur, in_map)?));
        Some(())
    }

    /// A renamed key whose value didn't change: write the new key, keep
    /// everything after the old one (`:`, the value, comments).
    fn rekey(
        &self,
        k: &MapKey,
        k0: &MapKey,
        body: usize,
        end: usize,
        col: usize,
        out: &mut String,
    ) -> Option<()> {
        let src = self.src;
        let key = self.em.implicit_key(k, col)?;
        // Where the old key's text ends: a plain key is its own text.
        let key_end = match k0 {
            MapKey::Str(t) if !t.is_empty() && src[body..end].starts_with(t.as_str()) => {
                body + t.len()
            }
            MapKey::Complex(ke) => {
                let r = ke.meta.span.range()?;
                (r.start == body && r.end > r.start).then_some(r.end)?
            }
            MapKey::Str(_) => return None,
        };
        // An implicit key: `:` follows on the same line.
        if !src[key_end..end]
            .trim_start_matches([' ', '\t'])
            .starts_with(':')
        {
            return None;
        }
        self.push_fresh(&key, out);
        out.push_str(&src[key_end..end]);
        Some(())
    }
}

/// An entry on its own line needs the line before it ended (the source's
/// last line may lack a line break).
fn line_break_before(inline: bool, crlf: bool, out: &mut String) {
    if !inline && !out.is_empty() && !out.ends_with(['\n', '\r']) {
        out.push_str(if crlf { "\r\n" } else { "\n" });
    }
}

/// Where an entry goes: its source chunk, whether it sat right after
/// `- ` in the source, whether it goes there now, and the entries' column.
struct Pos {
    chunk: (usize, usize),
    was_inline: bool,
    inline: bool,
    col: usize,
}

/// The node's text ends in a `|+`/`>+` block scalar, whose trailing blank
/// lines are content.
fn keeps_trailing_lines(e: &Elem) -> bool {
    let mut e = e;
    loop {
        match &e.node {
            Node::Map(m) if !e.meta.is_flow() => match m.last() {
                Some((_, v)) => e = v,
                None => return false,
            },
            Node::Seq(s) if !e.meta.is_flow() => match s.last() {
                Some(v) => e = v,
                None => return false,
            },
            _ => {
                return matches!(
                    e.meta.style,
                    Some(
                        crate::ast::Style::Literal(h) | crate::ast::Style::Folded(h)
                    ) if h.chomping == crate::ast::Chomping::Keep
                );
            }
        }
    }
}
