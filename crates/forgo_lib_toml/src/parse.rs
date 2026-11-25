//! The parser: TOML text to a [`Document`].
//!
//! One pass over the bytes. Values nest through arrays and inline tables,
//! with a depth limit. Strings without escapes are ranges of the source;
//! the others are decoded into the document's side buffer.

use crate::datetime::{self, DtError};
use crate::doc::{
    ArrayData, Document, Kind, NONE, Node, ParseOptions, Spans, TableSpan, Text, TomlVersion,
};
use crate::error::{Error, ErrorKind};

pub(crate) fn parse(src: &str, options: &ParseOptions) -> Result<Document, Error> {
    run(src, options, options.editing)
}

/// Parse and record where each part of the document is (for editing).
pub(crate) fn parse_spans(src: &str, options: &ParseOptions) -> Result<Document, Error> {
    run(src, options, true)
}

/// Parse a document's own text again, recording positions, in its own
/// memory (for the first edit of a document parsed without them).
pub(crate) fn reparse_spans(doc: &mut Document) -> Result<(), Error> {
    let src = std::mem::replace(&mut doc.src, Box::from(""));
    let options = doc.options;
    let mut old = std::mem::replace(doc, Document::placeholder());
    old.clear();
    let mut new = run_in(old, &src, &options, true)?;
    new.src = src;
    *doc = new;
    Ok(())
}

fn run(src: &str, options: &ParseOptions, spans: bool) -> Result<Document, Error> {
    let mut doc = run_in(Document::empty(src), src, options, spans)?;
    doc.src = src.into();
    Ok(doc)
}

/// Parse `src` into `doc` (a root and nothing else), without setting its
/// text.
fn run_in(
    doc: Document,
    src: &str,
    options: &ParseOptions,
    spans: bool,
) -> Result<Document, Error> {
    if src.len() >= u32::MAX as usize {
        return Err(Error::at(
            src,
            0,
            ErrorKind::TooLarge,
            "a document must be smaller than 4 GiB",
        ));
    }
    // A byte order mark is allowed before the document.
    let skip = if src.starts_with('\u{feff}') { 3 } else { 0 };
    let mut p = Parser {
        src,
        b: src.as_bytes(),
        i: skip,
        doc,
        v11: options.version == TomlVersion::V1_1,
        max_depth: options.max_depth,
        keys: Vec::new(),
        scratch: Vec::new(),
        spans: spans.then(|| {
            // As much room as the document's own vectors start with.
            let per = |bytes: usize, min: usize| (src.len() / bytes).clamp(min, 1 << 20);
            let mut table = Vec::with_capacity(per(64, 8));
            table.push(TableSpan::ROOT);
            Box::new(Spans {
                node: Vec::with_capacity(per(16, 64)),
                entry: Vec::with_capacity(per(20, 32)),
                table,
            })
        }),
    };
    p.document()?;
    let mut doc = p.doc;
    doc.finish();
    if let Some(mut spans) = p.spans {
        spans.node.resize(doc.nodes.len(), (NONE, NONE));
        spans.entry.resize(doc.entries.len(), (NONE, NONE));
        spans.table.resize(doc.tables.len(), TableSpan::NONE);
        doc.spans = Some(spans);
    }
    doc.options = *options;
    Ok(doc)
}

/// Parse a value at `at` of `text` (an edit of `doc`'s text, with `doc`'s
/// positions already moved to match) into `doc`: its node, and where it
/// ends. `depth` is how deep in arrays and inline tables it sits.
pub(crate) fn resume_value(
    doc: &mut Document,
    text: &str,
    at: usize,
    depth: usize,
) -> Result<(u32, usize), Error> {
    with_parser(doc, text, at, |p| {
        let node = p.value(depth)?;
        Ok((node, p.i))
    })
}

/// Parse the whole lines at `at..limit` of `text` (as [`resume_value`])
/// into `doc`, starting in the section of table `section`.
pub(crate) fn resume_lines(
    doc: &mut Document,
    text: &str,
    at: usize,
    section: u32,
    limit: usize,
) -> Result<(), Error> {
    with_parser(doc, text, at, |p| p.lines(section, limit))
}

fn with_parser<T>(
    doc: &mut Document,
    text: &str,
    at: usize,
    f: impl FnOnce(&mut Parser) -> Result<T, Error>,
) -> Result<T, Error> {
    let options = doc.options;
    let spans = doc.spans.take();
    let mut p = Parser {
        src: text,
        b: text.as_bytes(),
        i: at,
        doc: std::mem::replace(doc, Document::placeholder()),
        v11: options.version == TomlVersion::V1_1,
        max_depth: options.max_depth,
        keys: Vec::new(),
        scratch: Vec::new(),
        spans,
    };
    let r = f(&mut p);
    *doc = p.doc;
    if let Some(mut spans) = p.spans {
        spans.node.resize(doc.nodes.len(), (NONE, NONE));
        spans.entry.resize(doc.entries.len(), (NONE, NONE));
        spans.table.resize(doc.tables.len(), TableSpan::NONE);
        doc.spans = Some(spans);
    }
    r
}

/// Set `v[i]`, growing `v` with `fill` as needed.
fn put<T: Copy>(v: &mut Vec<T>, i: u32, x: T, fill: T) {
    let i = i as usize;
    if v.len() <= i {
        v.resize(i + 1, fill);
    }
    v[i] = x;
}

struct Parser<'s> {
    src: &'s str,
    b: &'s [u8],
    i: usize,
    doc: Document,
    v11: bool,
    max_depth: usize,
    /// The parts of the keys being parsed (an inline table's keys above
    /// the key it is the value of).
    keys: Vec<Text>,
    /// Items of the arrays being parsed (inner arrays above outer ones).
    scratch: Vec<u32>,
    /// Where things are, when parsing for an edit.
    spans: Option<Box<Spans>>,
}

/// The bytes of a bare key.
const BARE: [bool; 256] = {
    let mut t = [false; 256];
    let mut c = 0;
    while c < 256 {
        let b = c as u8;
        t[c] = b.is_ascii_alphanumeric() || b == b'_' || b == b'-';
        c += 1;
    }
    t
};

fn bare(c: u8) -> bool {
    BARE[c as usize]
}

const LO: u64 = 0x0101_0101_0101_0101;
const HI: u64 = 0x8080_8080_8080_8080;

/// The bytes of `x` equal to `c`, flagged in their high bits. Only the
/// lowest flag is exact (a borrow can flag bytes above a match), which
/// is all [`scan`] uses.
#[inline(always)]
const fn eq(x: u64, c: u8) -> u64 {
    let y = x ^ (LO * c as u64);
    y.wrapping_sub(LO) & !y & HI
}

/// Control characters and DEL in `x` (the lowest flag is exact).
#[inline(always)]
const fn control(x: u64) -> u64 {
    (x.wrapping_sub(LO * 0x20) & !x & HI) | eq(x, 0x7f)
}

/// The first index from `i` with byte `a`, byte `b`, a control
/// character (tab and newlines included) or DEL; else `s.len()`. Eight
/// bytes at a time: string and comment bodies are most of a document.
#[inline(always)]
fn scan(s: &[u8], mut i: usize, a: u8, b: u8) -> usize {
    while let Some(chunk) = s.get(i..i + 8) {
        let x = u64::from_le_bytes(chunk.try_into().expect("8 bytes"));
        let m = eq(x, a) | eq(x, b) | control(x);
        if m != 0 {
            return i + (m.trailing_zeros() / 8) as usize;
        }
        i += 8;
    }
    while let Some(&c) = s.get(i) {
        if c == a || c == b || c < 0x20 || c == 0x7f {
            return i;
        }
        i += 1;
    }
    i
}

impl<'s> Parser<'s> {
    fn err<T>(&self, at: usize, kind: ErrorKind, msg: impl Into<String>) -> Result<T, Error> {
        Err(Error::at(self.src, at, kind, msg))
    }

    fn peek(&self) -> Option<u8> {
        self.b.get(self.i).copied()
    }

    fn ws(&mut self) {
        let mut i = self.i;
        while let Some(b' ' | b'\t') = self.b.get(i) {
            i += 1;
        }
        self.i = i;
    }

    /// A newline at `i` (LF, or CRLF); a bare CR is an error.
    fn newline(&mut self) -> Result<bool, Error> {
        match self.peek() {
            Some(b'\n') => {
                self.i += 1;
                Ok(true)
            }
            Some(b'\r') => {
                if self.b.get(self.i + 1) == Some(&b'\n') {
                    self.i += 2;
                    Ok(true)
                } else {
                    self.err(
                        self.i,
                        ErrorKind::Syntax,
                        "a carriage return must be followed by a newline",
                    )
                }
            }
            _ => Ok(false),
        }
    }

    /// A comment from `#` to the end of the line (not including it).
    fn comment(&mut self) -> Result<(), Error> {
        debug_assert_eq!(self.peek(), Some(b'#'));
        let mut i = self.i + 1;
        loop {
            i = scan(self.b, i, 0x7f, 0x7f);
            match self.b.get(i) {
                Some(b'\t') => i += 1,
                Some(b'\n') | None => break,
                Some(b'\r') if self.b.get(i + 1) == Some(&b'\n') => break,
                Some(_) => {
                    return self.err(
                        i,
                        ErrorKind::Syntax,
                        "control characters aren't allowed in comments",
                    );
                }
            }
        }
        self.i = i;
        Ok(())
    }

    /// Spaces, an optional comment, then a newline or the end.
    fn end_of_line(&mut self) -> Result<(), Error> {
        self.ws();
        if self.peek() == Some(b'#') {
            self.comment()?;
        }
        if self.peek().is_none() || self.newline()? {
            return Ok(());
        }
        let c = self.src[self.i..].chars().next().unwrap_or('\0');
        self.err(
            self.i,
            ErrorKind::Syntax,
            format!("expected the end of the line, found {c:?}"),
        )
    }

    /// Whitespace, comments and newlines (between array items, and in
    /// TOML 1.1 inline tables).
    fn blank(&mut self) -> Result<(), Error> {
        let mut i = self.i;
        loop {
            match self.b.get(i) {
                Some(b' ' | b'\t' | b'\n') => i += 1,
                Some(b'\r') if self.b.get(i + 1) == Some(&b'\n') => i += 2,
                Some(b'#') => {
                    self.i = i;
                    self.comment()?;
                    i = self.i;
                }
                Some(b'\r') => {
                    self.i = i;
                    return self.err(
                        i,
                        ErrorKind::Syntax,
                        "a carriage return must be followed by a newline",
                    );
                }
                _ => {
                    self.i = i;
                    return Ok(());
                }
            }
        }
    }

    fn document(&mut self) -> Result<(), Error> {
        self.lines(0, usize::MAX)
    }

    /// Lines from the cursor up to `limit` (or the end), starting in the
    /// section of table `section`.
    fn lines(&mut self, mut section: u32, limit: usize) -> Result<(), Error> {
        while self.i < limit {
            self.ws();
            match self.peek() {
                None => return Ok(()),
                Some(b'#') => {
                    self.comment()?;
                    self.end_of_line()?;
                }
                Some(b'\n' | b'\r') => {
                    self.newline()?;
                }
                Some(b'[') => section = self.header()?,
                Some(_) => {
                    let key = self.i;
                    self.keyval(section, section, 0)?;
                    self.end_of_line()?;
                    if let Some(spans) = &mut self.spans {
                        let mut t = spans
                            .table
                            .get(section as usize)
                            .copied()
                            .unwrap_or(TableSpan::NONE);
                        t.last_key = key as u32;
                        t.end = self.i as u32;
                        put(&mut spans.table, section, t, TableSpan::NONE);
                    }
                }
            }
        }
        Ok(())
    }

    /// Record where the entry just made was defined (for editing).
    fn defined(&mut self, at: usize) {
        if let Some(spans) = &mut self.spans {
            let e = (self.doc.entries.len() - 1) as u32;
            put(&mut spans.entry, e, (at as u32, NONE), (NONE, NONE));
        }
    }

    // -----------------------------------------------------------------
    // Keys
    // -----------------------------------------------------------------

    /// A dotted key, pushed onto `self.keys`: returns where it starts.
    fn key(&mut self) -> Result<usize, Error> {
        let mark = self.keys.len();
        self.key_parts()?;
        Ok(mark)
    }

    /// The parts of a dotted key, pushed onto `self.keys`.
    fn key_parts(&mut self) -> Result<(), Error> {
        loop {
            self.ws();
            let k = self.simple_key()?;
            self.keys.push(k);
            self.ws();
            if self.peek() == Some(b'.') {
                self.i += 1;
            } else {
                return Ok(());
            }
        }
    }

    fn simple_key(&mut self) -> Result<Text, Error> {
        match self.peek() {
            Some(b'"') => {
                if self.b[self.i..].starts_with(b"\"\"\"") {
                    return self.err(
                        self.i,
                        ErrorKind::Syntax,
                        "a key can't be a multi-line string",
                    );
                }
                self.basic_string()
            }
            Some(b'\'') => {
                if self.b[self.i..].starts_with(b"'''") {
                    return self.err(
                        self.i,
                        ErrorKind::Syntax,
                        "a key can't be a multi-line string",
                    );
                }
                self.literal_string()
            }
            Some(c) if bare(c) => {
                let start = self.i;
                let mut i = start + 1;
                while self.b.get(i).is_some_and(|&c| bare(c)) {
                    i += 1;
                }
                self.i = i;
                Ok(Text {
                    start: start as u32,
                    end: self.i as u32,
                    side: false,
                })
            }
            Some(_) => {
                let c = self.src[self.i..].chars().next().unwrap_or('\0');
                self.err(
                    self.i,
                    ErrorKind::Syntax,
                    format!("expected a key, found {c:?}"),
                )
            }
            None => self.err(
                self.i,
                ErrorKind::Syntax,
                "expected a key, found the end of input",
            ),
        }
    }

    fn key_text(&self, t: Text) -> &str {
        let range = t.start as usize..t.end as usize;
        if t.side {
            &self.doc.side[range]
        } else {
            &self.src[range]
        }
    }

    fn key_path(&self, keys: &[Text]) -> String {
        keys.iter()
            .map(|&k| {
                let s = self.key_text(k);
                if !s.is_empty() && s.bytes().all(bare) {
                    s.to_string()
                } else {
                    format!("{s:?}")
                }
            })
            .collect::<Vec<_>>()
            .join(".")
    }

    // -----------------------------------------------------------------
    // Headers and key/value pairs
    // -----------------------------------------------------------------

    /// `[table]` or `[[array]]`: the table that follows (the new
    /// section).
    fn header(&mut self) -> Result<u32, Error> {
        let start = self.i;
        self.i += 1;
        let array = self.peek() == Some(b'[');
        if array {
            self.i += 1;
        }
        let mark = self.key()?;
        if self.peek() != Some(b']') {
            return self.err(
                self.i,
                ErrorKind::Syntax,
                "expected `]` after the table name",
            );
        }
        self.i += 1;
        if array {
            if self.peek() != Some(b']') {
                return self.err(
                    self.i,
                    ErrorKind::Syntax,
                    "expected `]]` after the array name",
                );
            }
            self.i += 1;
        }
        self.end_of_line()?;
        let r = self.open(mark, array, start);
        self.keys.truncate(mark);
        if let (Ok(t), Some(spans)) = (&r, &mut self.spans) {
            let span = TableSpan {
                header: start as u32,
                head_end: self.i as u32,
                last_key: NONE,
                end: self.i as u32,
            };
            put(&mut spans.table, *t, span, TableSpan::NONE);
        }
        r
    }

    /// Find or create the table a header names (its key is
    /// `self.keys[mark..]`).
    fn open(&mut self, mark: usize, array: bool, at: usize) -> Result<u32, Error> {
        let mut t = 0u32;
        let end = self.keys.len() - 1;
        for n in mark..end {
            let k = self.keys[n];
            t = match self.doc.find_key(self.src, t, k) {
                None => {
                    let (id, node) = self.doc.new_table(Kind::Implicit);
                    self.doc.insert(self.src, t, k, node);
                    self.defined(at);
                    id
                }
                Some(node) => match self.doc.nodes[node as usize] {
                    Node::Table(id) if self.doc.tables[id as usize].kind != Kind::Inline => id,
                    Node::Array(a) if self.doc.arrays[a as usize].is_tables() => {
                        match self.doc.nodes[self.doc.arrays[a as usize].last as usize] {
                            Node::Table(id) => id,
                            _ => unreachable!("arrays of tables hold tables"),
                        }
                    }
                    _ => {
                        let path = self.key_path(&self.keys[mark..=n]);
                        return self.err(
                            at,
                            ErrorKind::Table,
                            format!("`{path}` isn't a table that can be extended"),
                        );
                    }
                },
            };
        }
        let last = self.keys[end];
        let existing = self.doc.find_key(self.src, t, last);
        if array {
            return match existing {
                None => {
                    let (el, el_node) = self.doc.new_table(Kind::Element);
                    self.doc.arrays.push(ArrayData {
                        start: el_node,
                        len: 1,
                        last: el_node,
                    });
                    let a = (self.doc.arrays.len() - 1) as u32;
                    let node = self.doc.push_node(Node::Array(a));
                    self.doc.insert(self.src, t, last, node);
                    self.defined(at);
                    Ok(el)
                }
                Some(node) => match self.doc.nodes[node as usize] {
                    Node::Array(a) if self.doc.arrays[a as usize].is_tables() => {
                        let (el, el_node) = self.doc.new_table(Kind::Element);
                        let array = &mut self.doc.arrays[a as usize];
                        let prev = std::mem::replace(&mut array.last, el_node);
                        array.len += 1;
                        let Node::Table(prev) = self.doc.nodes[prev as usize] else {
                            unreachable!("arrays of tables hold tables")
                        };
                        self.doc.tables[prev as usize].next = el_node;
                        Ok(el)
                    }
                    _ => {
                        let path = self.key_path(&self.keys[mark..]);
                        self.err(
                            at,
                            ErrorKind::Table,
                            format!("`{path}` is already defined, not as an array of tables"),
                        )
                    }
                },
            };
        }
        match existing {
            None => {
                let (id, node) = self.doc.new_table(Kind::Header);
                self.doc.insert(self.src, t, last, node);
                self.defined(at);
                Ok(id)
            }
            Some(node) => match self.doc.nodes[node as usize] {
                Node::Table(id) if self.doc.tables[id as usize].kind == Kind::Implicit => {
                    self.doc.tables[id as usize].kind = Kind::Header;
                    Ok(id)
                }
                _ => {
                    let path = self.key_path(&self.keys[mark..]);
                    self.err(
                        at,
                        ErrorKind::Table,
                        format!("the table `{path}` is already defined"),
                    )
                }
            },
        }
    }

    /// `key = value` into table `t` (in `section`, for dotted keys).
    fn keyval(&mut self, t: u32, section: u32, depth: usize) -> Result<(), Error> {
        let start = self.i;
        self.ws();
        let key_start = self.i;
        self.keyval_inner(t, section, depth, start)?;
        if let Some(spans) = &mut self.spans {
            let e = (self.doc.entries.len() - 1) as u32;
            put(
                &mut spans.entry,
                e,
                (key_start as u32, self.i as u32),
                (NONE, NONE),
            );
        }
        Ok(())
    }

    fn keyval_inner(
        &mut self,
        t: u32,
        section: u32,
        depth: usize,
        start: usize,
    ) -> Result<(), Error> {
        let first = self.simple_key()?;
        self.ws();
        if self.peek() == Some(b'.') {
            self.i += 1;
            let mark = self.keys.len();
            self.keys.push(first);
            self.key_parts()?;
            self.equals()?;
            let value = self.value(depth)?;
            let r = self.assign(t, section, mark, value, start);
            self.keys.truncate(mark);
            return r;
        }
        // Most keys have one part.
        self.equals()?;
        let value = self.value(depth)?;
        if self.doc.find_key(self.src, t, first).is_some() {
            let path = self.key_path(&[first]);
            return self.err(
                start,
                ErrorKind::Duplicate,
                format!("the key `{path}` is already defined"),
            );
        }
        self.doc.insert(self.src, t, first, value);
        Ok(())
    }

    /// `=` and the spaces after it.
    fn equals(&mut self) -> Result<(), Error> {
        match self.peek() {
            Some(b'=') => {
                self.i += 1;
                self.ws();
                Ok(())
            }
            None => self.err(
                self.i,
                ErrorKind::Syntax,
                "expected `=` after the key, found the end of input",
            ),
            Some(_) => {
                let c = self.src[self.i..].chars().next().unwrap_or('\0');
                self.err(
                    self.i,
                    ErrorKind::Syntax,
                    format!("expected `=` after the key, found {c:?}"),
                )
            }
        }
    }

    /// Put `value` at the dotted key `self.keys[mark..]` in table `t`.
    fn assign(
        &mut self,
        t: u32,
        section: u32,
        mark: usize,
        value: u32,
        at: usize,
    ) -> Result<(), Error> {
        let mut t = t;
        let end = self.keys.len() - 1;
        for n in mark..end {
            let k = self.keys[n];
            t = match self.doc.find_key(self.src, t, k) {
                None => {
                    let (id, node) = self.doc.new_table(Kind::Dotted(section));
                    self.doc.insert(self.src, t, k, node);
                    self.defined(at);
                    id
                }
                Some(node) => match self.doc.nodes[node as usize] {
                    Node::Table(id)
                        if self.doc.tables[id as usize].kind == Kind::Dotted(section) =>
                    {
                        id
                    }
                    _ => {
                        let path = self.key_path(&self.keys[mark..=n]);
                        return self.err(
                            at,
                            ErrorKind::Table,
                            format!("`{path}` can't be extended with dotted keys here"),
                        );
                    }
                },
            };
        }
        let last = self.keys[end];
        if self.doc.find_key(self.src, t, last).is_some() {
            let path = self.key_path(&self.keys[mark..]);
            return self.err(
                at,
                ErrorKind::Duplicate,
                format!("the key `{path}` is already defined"),
            );
        }
        self.doc.insert(self.src, t, last, value);
        Ok(())
    }

    // -----------------------------------------------------------------
    // Values
    // -----------------------------------------------------------------

    fn value(&mut self, depth: usize) -> Result<u32, Error> {
        let start = self.i;
        let id = self.value_inner(depth)?;
        if let Some(spans) = &mut self.spans {
            put(
                &mut spans.node,
                id,
                (start as u32, self.i as u32),
                (NONE, NONE),
            );
        }
        Ok(id)
    }

    fn value_inner(&mut self, depth: usize) -> Result<u32, Error> {
        let node = match self.peek() {
            Some(b'"') => {
                let t = if self.b[self.i..].starts_with(b"\"\"\"") {
                    self.multiline_basic()?
                } else if let Some(t) = self.plain_basic_string() {
                    t
                } else {
                    self.basic_string()?
                };
                Node::String(t)
            }
            Some(b'\'') => {
                let t = if self.b[self.i..].starts_with(b"'''") {
                    self.multiline_literal()?
                } else {
                    self.literal_string()?
                };
                Node::String(t)
            }
            Some(b't') if self.b[self.i..].starts_with(b"true") => {
                self.i += 4;
                self.after_scalar()?;
                Node::Boolean(true)
            }
            Some(b'f') if self.b[self.i..].starts_with(b"false") => {
                self.i += 5;
                self.after_scalar()?;
                Node::Boolean(false)
            }
            Some(b'[') => return self.array(depth + 1),
            Some(b'{') => return self.inline_table(depth + 1),
            Some(b'+' | b'-' | b'0'..=b'9' | b'i' | b'n') => self.number_or_datetime()?,
            None => {
                return self.err(
                    self.i,
                    ErrorKind::Syntax,
                    "expected a value, found the end of input",
                );
            }
            Some(_) => {
                let c = self.src[self.i..].chars().next().unwrap_or('\0');
                return self.err(
                    self.i,
                    ErrorKind::Syntax,
                    format!("expected a value, found {c:?}"),
                );
            }
        };
        Ok(self.doc.push_node(node))
    }

    /// After a bare value: a character that can end it.
    fn after_scalar(&self) -> Result<(), Error> {
        match self.peek() {
            None | Some(b' ' | b'\t' | b'\n' | b'\r' | b'#' | b',' | b']' | b'}') => Ok(()),
            Some(_) => {
                let c = self.src[self.i..].chars().next().unwrap_or('\0');
                self.err(
                    self.i,
                    ErrorKind::Syntax,
                    format!("unexpected {c:?} after the value"),
                )
            }
        }
    }

    fn array(&mut self, depth: usize) -> Result<u32, Error> {
        if depth > self.max_depth {
            return self.err(
                self.i,
                ErrorKind::Depth,
                format!("nested deeper than {} levels", self.max_depth),
            );
        }
        self.i += 1;
        let mark = self.scratch.len();
        loop {
            self.blank()?;
            if self.peek() == Some(b']') {
                self.i += 1;
                break;
            }
            let v = self.value(depth)?;
            self.scratch.push(v);
            self.blank()?;
            match self.peek() {
                Some(b',') => self.i += 1,
                Some(b']') => {
                    self.i += 1;
                    break;
                }
                None => return self.err(self.i, ErrorKind::Syntax, "unclosed array: expected `]`"),
                Some(_) => {
                    let c = self.src[self.i..].chars().next().unwrap_or('\0');
                    return self.err(
                        self.i,
                        ErrorKind::Syntax,
                        format!("expected `,` or `]` in the array, found {c:?}"),
                    );
                }
            }
        }
        let start = self.doc.items.len() as u32;
        self.doc.items.extend_from_slice(&self.scratch[mark..]);
        self.scratch.truncate(mark);
        let len = self.doc.items.len() as u32 - start;
        self.doc.arrays.push(ArrayData {
            start,
            len,
            last: NONE,
        });
        let a = (self.doc.arrays.len() - 1) as u32;
        Ok(self.doc.push_node(Node::Array(a)))
    }

    fn inline_table(&mut self, depth: usize) -> Result<u32, Error> {
        if depth > self.max_depth {
            return self.err(
                self.i,
                ErrorKind::Depth,
                format!("nested deeper than {} levels", self.max_depth),
            );
        }
        let open = self.i;
        self.i += 1;
        let (id, node) = self.doc.new_table(Kind::Dotted(u32::MAX));
        // Its own section: dotted keys inside create tables only for it.
        self.doc.tables[id as usize].kind = Kind::Dotted(id);
        let mut after_comma = false;
        loop {
            if self.v11 {
                self.blank()?;
            } else {
                self.ws();
                if matches!(self.peek(), Some(b'\n' | b'\r')) {
                    return self.err(
                        self.i,
                        ErrorKind::Syntax,
                        "newlines aren't allowed in an inline table in TOML 1.0",
                    );
                }
            }
            if self.peek() == Some(b'}') {
                if after_comma && !self.v11 {
                    return self.err(
                        self.i,
                        ErrorKind::Syntax,
                        "a trailing comma isn't allowed in an inline table in TOML 1.0",
                    );
                }
                self.i += 1;
                break;
            }
            if self.peek().is_none() {
                return self.err(
                    open,
                    ErrorKind::Syntax,
                    "unclosed inline table: expected `}`",
                );
            }
            self.keyval(id, id, depth)?;
            if self.v11 {
                self.blank()?;
            } else {
                self.ws();
            }
            match self.peek() {
                Some(b',') => {
                    self.i += 1;
                    after_comma = true;
                }
                Some(b'}') => {
                    self.i += 1;
                    break;
                }
                None => {
                    return self.err(
                        open,
                        ErrorKind::Syntax,
                        "unclosed inline table: expected `}`",
                    );
                }
                Some(_) => {
                    let c = self.src[self.i..].chars().next().unwrap_or('\0');
                    return self.err(
                        self.i,
                        ErrorKind::Syntax,
                        format!("expected `,` or `}}` in the inline table, found {c:?}"),
                    );
                }
            }
        }
        self.doc.tables[id as usize].kind = Kind::Inline;
        Ok(node)
    }

    // -----------------------------------------------------------------
    // Strings
    // -----------------------------------------------------------------

    fn side_start(&self) -> u32 {
        self.doc.side.len() as u32
    }

    fn side_text(&self, start: u32) -> Text {
        Text {
            start,
            end: self.doc.side.len() as u32,
            side: true,
        }
    }

    fn src_text(start: usize, end: usize) -> Text {
        Text {
            start: start as u32,
            end: end as u32,
            side: false,
        }
    }

    /// A basic string with nothing to decode (most strings), or `None`
    /// to leave it to [`basic_string`](Self::basic_string).
    #[inline(always)]
    fn plain_basic_string(&mut self) -> Option<Text> {
        let start = self.i + 1;
        let end = scan(self.b, start, b'"', b'\\');
        (self.b.get(end) == Some(&b'"')).then(|| {
            self.i = end + 1;
            Self::src_text(start, end)
        })
    }

    /// `"…"` (the cursor at the opening quote).
    fn basic_string(&mut self) -> Result<Text, Error> {
        let open = self.i;
        let start = open + 1;
        // Fast path: no escapes, so the value is the source's text.
        let mut i = start;
        loop {
            i = scan(self.b, i, b'"', b'\\');
            match self.b.get(i) {
                Some(b'"') => {
                    self.i = i + 1;
                    return Ok(Self::src_text(start, i));
                }
                Some(b'\t') => i += 1,
                Some(b'\\') => break,
                Some(b'\n' | b'\r') | None => {
                    return self.err(open, ErrorKind::String, "unterminated string");
                }
                Some(_) => {
                    return self.err(
                        i,
                        ErrorKind::String,
                        "control characters must be escaped in strings",
                    );
                }
            }
        }
        let side = self.side_start();
        self.doc.side.push_str(&self.src[start..i]);
        self.i = i;
        loop {
            let end = scan(self.b, self.i, b'"', b'\\');
            self.doc.side.push_str(&self.src[self.i..end]);
            self.i = end;
            match self.peek() {
                Some(b'"') => {
                    self.i += 1;
                    return Ok(self.side_text(side));
                }
                Some(b'\\') => self.escape()?,
                Some(b'\t') => {
                    self.i += 1;
                    self.doc.side.push('\t');
                }
                Some(b'\n' | b'\r') | None => {
                    return self.err(open, ErrorKind::String, "unterminated string");
                }
                Some(_) => {
                    return self.err(
                        self.i,
                        ErrorKind::String,
                        "control characters must be escaped in strings",
                    );
                }
            }
        }
    }

    /// An escape at `\` into the side buffer.
    fn escape(&mut self) -> Result<(), Error> {
        let at = self.i;
        self.i += 1;
        let c = match self.peek() {
            Some(b'b') => '\u{8}',
            Some(b't') => '\t',
            Some(b'n') => '\n',
            Some(b'f') => '\u{c}',
            Some(b'r') => '\r',
            Some(b'"') => '"',
            Some(b'\\') => '\\',
            Some(b'e') if self.v11 => '\u{1b}',
            Some(b'x') if self.v11 => return self.hex_escape(at, 2),
            Some(b'u') => return self.hex_escape(at, 4),
            Some(b'U') => return self.hex_escape(at, 8),
            _ => {
                let c = self.src[self.i..].chars().next().unwrap_or('\0');
                return self.err(at, ErrorKind::String, format!("invalid escape `\\{c}`"));
            }
        };
        self.i += 1;
        self.doc.side.push(c);
        Ok(())
    }

    fn hex_escape(&mut self, at: usize, n: usize) -> Result<(), Error> {
        let digits = self.b.get(self.i + 1..self.i + 1 + n);
        let value = digits
            .filter(|d| d.iter().all(u8::is_ascii_hexdigit))
            .and_then(|d| u32::from_str_radix(std::str::from_utf8(d).ok()?, 16).ok());
        let Some(value) = value else {
            return self.err(
                at,
                ErrorKind::String,
                format!("an escape needs {n} hex digits"),
            );
        };
        let Some(c) = char::from_u32(value) else {
            return self.err(
                at,
                ErrorKind::String,
                format!("U+{value:X} isn't a Unicode scalar value"),
            );
        };
        self.i += 1 + n;
        self.doc.side.push(c);
        Ok(())
    }

    /// `"""…"""`.
    fn multiline_basic(&mut self) -> Result<Text, Error> {
        let open = self.i;
        self.i += 3;
        // A newline right after the opening is trimmed.
        self.newline()?;
        let start = self.i;
        // Fast path: no escapes and no CRLF, so the value is the source's
        // text.
        let mut i = start;
        loop {
            i = scan(self.b, i, b'"', b'\\');
            match self.b.get(i) {
                Some(b'"') => {
                    let run = self.b[i..].iter().take_while(|&&c| c == b'"').count();
                    if run >= 3 {
                        if run > 5 {
                            return self.err(
                                i,
                                ErrorKind::String,
                                "too many quotes before the end of the string",
                            );
                        }
                        self.i = i + run;
                        return Ok(Self::src_text(start, i + run - 3));
                    }
                    i += run;
                }
                Some(b'\n' | b'\t') => i += 1,
                Some(b'\\' | b'\r') => break,
                None => return self.err(open, ErrorKind::String, "unterminated multi-line string"),
                Some(_) => {
                    return self.err(
                        i,
                        ErrorKind::String,
                        "control characters must be escaped in strings",
                    );
                }
            }
        }
        let side = self.side_start();
        self.doc.side.push_str(&self.src[start..i]);
        self.i = i;
        loop {
            let end = scan(self.b, self.i, b'"', b'\\');
            self.doc.side.push_str(&self.src[self.i..end]);
            self.i = end;
            match self.peek() {
                None => return self.err(open, ErrorKind::String, "unterminated multi-line string"),
                Some(b'"') => {
                    // Up to two quotes may precede the closing three.
                    let run = self.b[self.i..].iter().take_while(|&&c| c == b'"').count();
                    if run >= 3 {
                        if run > 5 {
                            return self.err(
                                self.i,
                                ErrorKind::String,
                                "too many quotes before the end of the string",
                            );
                        }
                        for _ in 0..run - 3 {
                            self.doc.side.push('"');
                        }
                        self.i += run;
                        return Ok(self.side_text(side));
                    }
                    for _ in 0..run {
                        self.doc.side.push('"');
                    }
                    self.i += run;
                }
                Some(b'\\') => {
                    // A line-ending backslash: skip whitespace up to and
                    // across newlines.
                    let mut j = self.i + 1;
                    while matches!(self.b.get(j), Some(b' ' | b'\t')) {
                        j += 1;
                    }
                    if matches!(self.b.get(j), Some(b'\n')) || self.b[j..].starts_with(b"\r\n") {
                        self.i = j;
                        loop {
                            match self.peek() {
                                Some(b' ' | b'\t') => self.i += 1,
                                Some(b'\n' | b'\r') => {
                                    self.newline()?;
                                }
                                _ => break,
                            }
                        }
                    } else {
                        self.escape()?;
                    }
                }
                Some(b'\n') => {
                    self.i += 1;
                    self.doc.side.push('\n');
                }
                Some(b'\t') => {
                    self.i += 1;
                    self.doc.side.push('\t');
                }
                Some(b'\r') => {
                    if self.b.get(self.i + 1) == Some(&b'\n') {
                        self.i += 2;
                        self.doc.side.push('\n');
                    } else {
                        return self.err(
                            self.i,
                            ErrorKind::String,
                            "a carriage return must be followed by a newline",
                        );
                    }
                }
                Some(_) => {
                    return self.err(
                        self.i,
                        ErrorKind::String,
                        "control characters must be escaped in strings",
                    );
                }
            }
        }
    }

    /// `'…'`.
    fn literal_string(&mut self) -> Result<Text, Error> {
        let open = self.i;
        let start = open + 1;
        let mut i = start;
        loop {
            i = scan(self.b, i, b'\'', b'\'');
            match self.b.get(i) {
                Some(b'\'') => {
                    self.i = i + 1;
                    return Ok(Self::src_text(start, i));
                }
                Some(b'\t') => i += 1,
                Some(b'\n' | b'\r') | None => {
                    return self.err(open, ErrorKind::String, "unterminated string");
                }
                Some(_) => {
                    return self.err(
                        i,
                        ErrorKind::String,
                        "control characters aren't allowed in literal strings",
                    );
                }
            }
        }
    }

    /// `'''…'''`.
    fn multiline_literal(&mut self) -> Result<Text, Error> {
        let open = self.i;
        self.i += 3;
        self.newline()?;
        let start = self.i;
        let mut crlf = false;
        let mut i = start;
        loop {
            i = scan(self.b, i, b'\'', b'\'');
            match self.b.get(i) {
                None => return self.err(open, ErrorKind::String, "unterminated multi-line string"),
                Some(b'\'') => {
                    let run = self.b[i..].iter().take_while(|&&c| c == b'\'').count();
                    if run >= 3 {
                        if run > 5 {
                            return self.err(
                                i,
                                ErrorKind::String,
                                "too many quotes before the end of the string",
                            );
                        }
                        let end = i + run - 3;
                        self.i = i + run;
                        if !crlf {
                            return Ok(Self::src_text(start, end));
                        }
                        // Newlines are LF in the value.
                        let side = self.side_start();
                        let text = self.src[start..end].replace("\r\n", "\n");
                        self.doc.side.push_str(&text);
                        return Ok(self.side_text(side));
                    }
                    i += run;
                }
                Some(b'\n' | b'\t') => i += 1,
                Some(b'\r') => {
                    if self.b.get(i + 1) == Some(&b'\n') {
                        crlf = true;
                        i += 2;
                    } else {
                        return self.err(
                            i,
                            ErrorKind::String,
                            "a carriage return must be followed by a newline",
                        );
                    }
                }
                Some(_) => {
                    return self.err(
                        i,
                        ErrorKind::String,
                        "control characters aren't allowed in literal strings",
                    );
                }
            }
        }
    }

    // -----------------------------------------------------------------
    // Numbers and date-times
    // -----------------------------------------------------------------

    fn number_or_datetime(&mut self) -> Result<Node, Error> {
        let b = self.b;
        let s = self.i;
        let digit = |k: usize| b.get(s + k).is_some_and(u8::is_ascii_digit);
        let date_like = (0..4).all(digit) && b.get(s + 4) == Some(&b'-');
        let time_like = (0..2).all(digit) && b.get(s + 2) == Some(&b':');
        if date_like || time_like {
            return match datetime::parse(b, s, self.v11) {
                Ok((dt, end)) => {
                    self.i = end;
                    self.after_scalar()?;
                    self.doc.datetimes.push(dt);
                    Ok(Node::Datetime((self.doc.datetimes.len() - 1) as u32))
                }
                Err(DtError::Syntax(at)) => {
                    self.err(at, ErrorKind::Datetime, "malformed date or time")
                }
                Err(DtError::Range(at, why)) => self.err(at, ErrorKind::Datetime, why),
            };
        }
        let mut end = s;
        while b
            .get(end)
            .is_some_and(|&c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'.' | b'+' | b'-'))
        {
            end += 1;
        }
        let tok = &self.src[s..end];
        self.i = end;
        let node = number(tok).map_err(|why| Error::at(self.src, s, ErrorKind::Number, why))?;
        self.after_scalar()?;
        Ok(node)
    }
}

/// Digits with single underscores between them (`1_000`), in a base.
fn digits_ok(s: &str, radix: u32) -> bool {
    let mut after_digit = false;
    for c in s.bytes() {
        if c == b'_' {
            if !after_digit {
                return false;
            }
            after_digit = false;
        } else if (c as char).is_digit(radix) {
            after_digit = true;
        } else {
            return false;
        }
    }
    after_digit
}

/// The value of validated digits (underscores skipped) in a base, with
/// the sign applied while accumulating (so `i64::MIN` fits).
fn integer(digits: &str, radix: u32, negative: bool) -> Result<i64, String> {
    let mut v: i64 = 0;
    for c in digits.bytes() {
        if c == b'_' {
            continue;
        }
        let d = i64::from((c as char).to_digit(radix).expect("validated"));
        v = v
            .checked_mul(i64::from(radix))
            .and_then(|v| {
                if negative {
                    v.checked_sub(d)
                } else {
                    v.checked_add(d)
                }
            })
            .ok_or_else(|| "the integer doesn't fit in 64 bits".to_string())?;
    }
    Ok(v)
}

/// An integer or a float.
fn number(tok: &str) -> Result<Node, String> {
    let (sign, body) = match tok.as_bytes().first() {
        Some(b'+') => (1, &tok[1..]),
        Some(b'-') => (-1, &tok[1..]),
        _ => (0, tok),
    };
    match body {
        "inf" => {
            return Ok(Node::Float(if sign < 0 {
                f64::NEG_INFINITY
            } else {
                f64::INFINITY
            }));
        }
        "nan" => {
            return Ok(Node::Float(if sign < 0 { -f64::NAN } else { f64::NAN }));
        }
        _ => {}
    }
    // Hex, octal, binary: no sign.
    for (prefix, radix) in [("0x", 16), ("0o", 8), ("0b", 2)] {
        if let Some(digits) = body.strip_prefix(prefix) {
            if sign != 0 {
                return Err(format!("a {prefix} integer can't have a sign"));
            }
            if !digits_ok(digits, radix) {
                return Err(format!("malformed {prefix} integer"));
            }
            return integer(digits, radix, false).map(Node::Integer);
        }
    }
    let is_float = body.contains(['.', 'e', 'E']);
    if !is_float {
        if !digits_ok(body, 10) {
            return Err("malformed number".into());
        }
        if body.len() > 1 && body.starts_with('0') {
            return Err("leading zeros aren't allowed".into());
        }
        return integer(body, 10, sign < 0).map(Node::Integer);
    }
    // A float: int part, then a fraction and/or an exponent.
    let (mantissa, exp) = match body.find(['e', 'E']) {
        Some(e) => (&body[..e], Some(&body[e + 1..])),
        None => (body, None),
    };
    let (int, frac) = match mantissa.split_once('.') {
        Some((i, f)) => (i, Some(f)),
        None => (mantissa, None),
    };
    if !digits_ok(int, 10) || (int.len() > 1 && int.starts_with('0')) {
        return Err("malformed float".into());
    }
    if let Some(f) = frac
        && !digits_ok(f, 10)
    {
        return Err("malformed float".into());
    }
    if let Some(e) = exp {
        let e = e.strip_prefix(['+', '-']).unwrap_or(e);
        if !digits_ok(e, 10) {
            return Err("malformed float exponent".into());
        }
    }
    let parsed = if tok.contains('_') {
        // Without the underscores, on the stack when short.
        let mut buf = [0u8; 64];
        let mut n = 0;
        let mut long = String::new();
        for c in tok.bytes().filter(|&c| c != b'_') {
            if n < buf.len() {
                buf[n] = c;
                n += 1;
            } else {
                if long.is_empty() {
                    long.push_str(std::str::from_utf8(&buf).expect("ASCII"));
                }
                long.push(c as char);
            }
        }
        if long.is_empty() {
            std::str::from_utf8(&buf[..n])
                .expect("ASCII")
                .parse::<f64>()
        } else {
            long.parse::<f64>()
        }
    } else {
        tok.parse::<f64>()
    };
    parsed
        .map(Node::Float)
        .map_err(|_| "malformed float".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn swar_scan_matches_a_byte_loop() {
        // Every byte value at every position, among neighbours that
        // provoke borrows and carries (0x00, 0x01, 0x7f, 0x80, 0xff, and
        // matches of each class).
        let fills = [
            0x00u8, 0x01, b'_', b'^', b'-', b'a', b'Z', b'9', b'"', 0x7f, 0x80, 0xff, b' ',
        ];
        for c in 0..=255u8 {
            for pos in 0..8 {
                for &fill in &fills {
                    let mut bytes = [fill; 8];
                    bytes[pos] = c;
                    let x = u64::from_le_bytes(bytes);
                    let want = bytes
                        .iter()
                        .position(|&b| b == b'"' || b == b'\\' || b < 0x20 || b == 0x7f);
                    let m = eq(x, b'"') | eq(x, b'\\') | control(x);
                    let got = (m != 0).then(|| (m.trailing_zeros() / 8) as usize);
                    assert_eq!(got, want, "string {bytes:02x?}");
                }
            }
        }
    }
}
