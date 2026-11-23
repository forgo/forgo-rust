//! Tokenizer for YAML 1.2.2.
//!
//! The structure follows libyaml's scanner: an indentation stack, one
//! simple-key candidate per flow level, and a token queue into which `KEY`
//! and `BLOCK-MAPPING-START` tokens are inserted once a `:` confirms a
//! candidate. On top of that it enforces YAML 1.2.2 rules libyaml lacks:
//!
//! - indentation counts spaces only; a tab may separate tokens but can't
//!   precede a block collection indicator or an implicit key on its line;
//! - every continuation line inside a flow collection or a multi-line
//!   scalar must be indented more than the enclosing block;
//! - `#` starts a comment only at the start of a line or after white space;
//! - keys in flow mappings may span lines, and `:` may follow a JSON-like
//!   key without a space;
//! - block scalars at the top level may start at column 0;
//! - nodes must be separated by white space or an indicator.

use super::chars::*;
use super::event::{BlockHeader, Chomping, Mark, ScalarStyle};
use crate::ast::Error;
use std::borrow::Cow;
use std::collections::{BTreeSet, VecDeque};

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum TokenKind<'a> {
    StreamStart,
    StreamEnd,
    VersionDirective(u32, u32),
    TagDirective(Cow<'a, str>, Cow<'a, str>),
    ReservedDirective,
    DocumentStart,
    DocumentEnd,
    BlockSequenceStart,
    BlockMappingStart,
    BlockEnd,
    FlowSequenceStart,
    FlowSequenceEnd,
    FlowMappingStart,
    FlowMappingEnd,
    BlockEntry,
    FlowEntry,
    Key,
    Value,
    Alias(Cow<'a, str>),
    Anchor(Cow<'a, str>),
    /// (handle, suffix). A verbatim tag has an empty handle.
    Tag(Cow<'a, str>, Cow<'a, str>),
    /// Value, style, and the header of a block scalar.
    Scalar(Cow<'a, str>, ScalarStyle, Option<BlockHeader>),
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Token<'a> {
    pub kind: TokenKind<'a>,
    pub start: Mark,
    pub end: Mark,
}

/// A comment, recorded on the side; the parser never sees it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Comment<'a> {
    /// From `#` to the end of the line (line break excluded).
    pub start: Mark,
    pub end: Mark,
    /// The text after `#`, untrimmed.
    pub text: &'a str,
    /// Other content precedes the comment on its line.
    pub trailing: bool,
}

#[derive(Clone, Copy, Debug)]
struct SimpleKey {
    possible: bool,
    required: bool,
    token_number: usize,
    mark: Mark,
    /// A tab separates this candidate from the start of its line or the
    /// previous token.
    after_tab: bool,
}

impl SimpleKey {
    const NONE: SimpleKey = SimpleKey {
        possible: false,
        required: false,
        token_number: 0,
        mark: Mark {
            index: 0,
            line: 0,
            col: 0,
        },
        after_tab: false,
    };
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FlowKind {
    Sequence,
    Mapping,
}

pub(crate) struct Scanner<'a> {
    src: &'a str,
    bytes: &'a [u8],
    mark: Mark,
    tokens: VecDeque<Token<'a>>,
    tokens_parsed: usize,
    stream_start_produced: bool,
    stream_end_produced: bool,
    /// Current block indentation; -1 outside any block collection.
    indent: i64,
    indents: Vec<i64>,
    simple_key_allowed: bool,
    /// Index 0 is the block context, index i the flow level i.
    simple_keys: Vec<SimpleKey>,
    /// Levels whose simple key is possible, and the subset that can go
    /// stale (not in a flow mapping). Possible keys sit at increasing
    /// positions and token numbers by level, so only the lowest ones ever
    /// need checking: O(1) amortized work per candidate however deep the
    /// nesting (a linear scan per token would be quadratic).
    possible_levels: BTreeSet<usize>,
    stale_levels: BTreeSet<usize>,
    flows: Vec<FlowKind>,
    /// Kind and end of the last token produced (for separation checks).
    last_kind: LastToken,
    last_end: usize,
    /// Line of the last `...`; nothing else may follow it on that line.
    doc_end_line: Option<usize>,
    pub comments: Vec<Comment<'a>>,
    /// Sticky error: (line, col, message).
    error: Option<(usize, usize, String)>,
}

/// What the last produced token was, as far as later checks care.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LastToken {
    /// Stream start, a directive, `---` or `...`.
    Boundary,
    /// A node or node property: content may not follow without white space.
    Node,
    /// A quoted scalar or flow collection end: in flow context a `:` right
    /// after it is a value indicator even without a following space.
    JsonNode,
    /// Any indicator.
    Indicator,
}

type Res<T> = Result<T, Error>;

impl<'a> Scanner<'a> {
    pub fn new(src: &'a str) -> Self {
        Scanner {
            src,
            bytes: src.as_bytes(),
            mark: Mark::default(),
            tokens: VecDeque::new(),
            tokens_parsed: 0,
            stream_start_produced: false,
            stream_end_produced: false,
            indent: -1,
            indents: Vec::new(),
            simple_key_allowed: false,
            simple_keys: vec![SimpleKey::NONE],
            possible_levels: BTreeSet::new(),
            stale_levels: BTreeSet::new(),
            flows: Vec::new(),
            last_kind: LastToken::Boundary,
            last_end: 0,
            doc_end_line: None,
            comments: Vec::new(),
            error: None,
        }
    }

    // ------------------------------------------------------------------
    // Public interface
    // ------------------------------------------------------------------

    pub fn peek(&mut self) -> Res<&Token<'a>> {
        if let Some((line, col, msg)) = &self.error {
            return Err(Error::Syntax {
                line: *line,
                col: *col,
                msg: msg.clone(),
            });
        }
        if let Err(e) = self.fetch_more_tokens() {
            if let Error::Syntax { line, col, msg } = &e {
                self.error = Some((*line, *col, msg.clone()));
            }
            return Err(e);
        }
        Ok(self.tokens.front().expect("token queue is non-empty"))
    }

    pub fn next_token(&mut self) -> Res<Token<'a>> {
        self.peek()?;
        self.tokens_parsed += 1;
        let t = self.tokens.pop_front().expect("token queue is non-empty");
        if t.kind == TokenKind::StreamEnd {
            self.stream_end_produced = true;
        }
        Ok(t)
    }

    // ------------------------------------------------------------------
    // Character helpers
    // ------------------------------------------------------------------

    #[inline]
    fn byte_at(&self, off: usize) -> Option<u8> {
        self.bytes.get(self.mark.index + off).copied()
    }

    #[inline]
    fn ch(&self) -> Option<char> {
        let b = *self.bytes.get(self.mark.index)?;
        if b < 0x80 {
            Some(b as char)
        } else {
            self.src[self.mark.index..].chars().next()
        }
    }

    /// The character `n` characters ahead.
    #[inline]
    fn ch_at(&self, n: usize) -> Option<char> {
        self.src[self.mark.index..].chars().nth(n)
    }

    #[inline]
    fn is_z(&self) -> bool {
        self.mark.index >= self.bytes.len()
    }

    #[inline]
    fn at_break(&self) -> bool {
        matches!(self.byte_at(0), Some(b'\n' | b'\r'))
    }

    #[inline]
    fn at_breakz(&self) -> bool {
        matches!(self.byte_at(0), None | Some(b'\n' | b'\r'))
    }

    #[inline]
    fn at_blank(&self) -> bool {
        matches!(self.byte_at(0), Some(b' ' | b'\t'))
    }

    /// Blank, line break or end of input at byte offset `off` (ASCII lookahead).
    #[inline]
    fn blankz_at(&self, off: usize) -> bool {
        matches!(self.byte_at(off), None | Some(b' ' | b'\t' | b'\n' | b'\r'))
    }

    /// Advance over one non-break character.
    #[inline]
    fn advance(&mut self) {
        let b = self.bytes[self.mark.index];
        let len = if b < 0x80 {
            1
        } else {
            self.src[self.mark.index..]
                .chars()
                .next()
                .map_or(1, char::len_utf8)
        };
        self.mark.index += len;
        self.mark.col += 1;
    }

    /// Consume one line break (`\r\n`, `\r` or `\n`).
    #[inline]
    fn skip_break(&mut self) {
        match self.byte_at(0) {
            Some(b'\r') => {
                self.mark.index += if self.byte_at(1) == Some(b'\n') { 2 } else { 1 };
            }
            Some(b'\n') => self.mark.index += 1,
            _ => return,
        }
        self.mark.line += 1;
        self.mark.col = 0;
    }

    /// `---` or `...` at the start of a line, followed by white space or the end.
    fn at_document_indicator(&self) -> bool {
        if self.mark.col != 0 {
            return false;
        }
        let mut rest = &self.bytes[self.mark.index..];
        // A document prefix may start with a byte order mark.
        if rest.starts_with("\u{FEFF}".as_bytes()) {
            rest = &rest[3..];
        }
        (rest.starts_with(b"---") || rest.starts_with(b"..."))
            && matches!(rest.get(3), None | Some(b' ' | b'\t' | b'\n' | b'\r'))
    }

    /// Does a tab separate byte offset `index` from the start of its line
    /// or the preceding non-white character?
    fn tab_before(&self, index: usize) -> bool {
        let mut i = index;
        while i > 0 {
            match self.bytes[i - 1] {
                b' ' => i -= 1,
                b'\t' => return true,
                _ => return false,
            }
        }
        false
    }

    /// Is byte offset `index` preceded only by white space on its line?
    fn first_on_line(&self, index: usize) -> bool {
        let mut i = index;
        while i > 0 {
            match self.bytes[i - 1] {
                b' ' | b'\t' => i -= 1,
                b'\n' | b'\r' => return true,
                0xBF if i >= 3 && &self.bytes[i - 3..i] == "\u{FEFF}".as_bytes() => i -= 3,
                _ => return false,
            }
        }
        true
    }

    fn err(&self, mark: Mark, msg: impl Into<String>) -> Error {
        Error::Syntax {
            line: mark.line + 1,
            col: mark.col + 1,
            msg: msg.into(),
        }
    }

    fn err_here(&self, msg: impl Into<String>) -> Error {
        self.err(self.mark, msg)
    }

    // ------------------------------------------------------------------
    // Token queue management
    // ------------------------------------------------------------------

    fn fetch_more_tokens(&mut self) -> Res<()> {
        loop {
            let mut need_more = self.tokens.is_empty();
            if !need_more {
                self.stale_simple_keys()?;
                // The lowest possible key has the smallest token number.
                need_more = self
                    .possible_levels
                    .first()
                    .is_some_and(|&l| self.simple_keys[l].token_number == self.tokens_parsed);
            }
            if !need_more {
                return Ok(());
            }
            if self.stream_end_produced
                || self
                    .tokens
                    .back()
                    .is_some_and(|t| t.kind == TokenKind::StreamEnd)
            {
                return Ok(());
            }
            self.fetch_next_token()?;
        }
    }

    fn push(&mut self, kind: TokenKind<'a>, start: Mark, end: Mark) {
        self.last_kind = match kind {
            TokenKind::Scalar(_, ScalarStyle::SingleQuoted | ScalarStyle::DoubleQuoted, _)
            | TokenKind::FlowSequenceEnd
            | TokenKind::FlowMappingEnd => LastToken::JsonNode,
            TokenKind::Scalar(..)
            | TokenKind::Alias(_)
            | TokenKind::Anchor(_)
            | TokenKind::Tag(..) => LastToken::Node,
            TokenKind::StreamStart
            | TokenKind::DocumentStart
            | TokenKind::DocumentEnd
            | TokenKind::VersionDirective(..)
            | TokenKind::TagDirective(..)
            | TokenKind::ReservedDirective => LastToken::Boundary,
            _ => LastToken::Indicator,
        };
        self.last_end = end.index;
        self.tokens.push_back(Token { kind, start, end });
    }

    fn insert(&mut self, pos: usize, kind: TokenKind<'a>, mark: Mark) {
        self.tokens.insert(
            pos,
            Token {
                kind,
                start: mark,
                end: mark,
            },
        );
    }

    fn fetch_next_token(&mut self) -> Res<()> {
        if !self.stream_start_produced {
            self.stream_start_produced = true;
            self.simple_key_allowed = true;
            let m = self.mark;
            self.push(TokenKind::StreamStart, m, m);
            return Ok(());
        }

        self.scan_to_next_token()?;
        self.stale_simple_keys()?;
        self.unroll_indent(self.mark.col as i64);

        let Some(c) = self.ch() else {
            return self.fetch_stream_end();
        };

        if self.doc_end_line == Some(self.mark.line) {
            return Err(self.err_here("only a comment may follow `...` on its line"));
        }

        if self.mark.col == 0 {
            if c == '%' {
                return self.fetch_directive();
            }
            if self.at_document_indicator() {
                let kind = if c == '-' {
                    TokenKind::DocumentStart
                } else {
                    TokenKind::DocumentEnd
                };
                return self.fetch_document_indicator(kind);
            }
        }

        let flow = !self.flows.is_empty();
        let adjacent = matches!(self.last_kind, LastToken::Node | LastToken::JsonNode)
            && self.mark.index == self.last_end;
        let is_value = c == ':'
            && (self.blankz_at(1)
                || (flow
                    && (self.ch_at(1).is_some_and(is_flow_indicator)
                        || self.last_kind == LastToken::JsonNode)));

        let separator_ok = matches!(c, ',' | ']' | '}') || is_value;
        if adjacent && !separator_ok {
            return Err(self.err_here("expected white space or an indicator between nodes"));
        }

        match c {
            '[' => self.fetch_flow_collection_start(FlowKind::Sequence),
            '{' => self.fetch_flow_collection_start(FlowKind::Mapping),
            ']' => self.fetch_flow_collection_end(FlowKind::Sequence),
            '}' => self.fetch_flow_collection_end(FlowKind::Mapping),
            ',' => self.fetch_flow_entry(),
            '-' if self.blankz_at(1) => self.fetch_block_entry(),
            '?' if self.blankz_at(1) => self.fetch_key(),
            ':' if is_value => self.fetch_value(),
            '*' => self.fetch_anchor(true),
            '&' => self.fetch_anchor(false),
            '!' => self.fetch_tag(),
            '|' if !flow => self.fetch_block_scalar(true),
            '>' if !flow => self.fetch_block_scalar(false),
            '\'' => self.fetch_flow_scalar(true),
            '"' => self.fetch_flow_scalar(false),
            _ if self.can_start_plain(c, flow) => self.fetch_plain_scalar(),
            '\u{FEFF}' => Err(self.err_here("a byte order mark may only appear before a document")),
            _ if !is_printable(c) => {
                Err(self.err_here(format!("invalid character U+{:04X}", c as u32)))
            }
            _ => Err(self.err_here(format!("`{c}` cannot start any token here"))),
        }
    }

    fn can_start_plain(&self, c: char, flow: bool) -> bool {
        if matches!(c, '-' | '?' | ':') {
            self.ch_at(1).is_some_and(|n| is_plain_safe(n, flow))
        } else {
            is_ns_char(c) && !is_indicator(c)
        }
    }

    /// A byte order mark may start the stream or a document.
    fn bom_allowed(&self) -> bool {
        if self.mark.index == 0 || matches!(self.last_kind, LastToken::Boundary) {
            return true;
        }
        let rest = &self.bytes[self.mark.index + 3..];
        rest.starts_with(b"---") && matches!(rest.get(3), None | Some(b' ' | b'\t' | b'\n' | b'\r'))
    }

    fn scan_to_next_token(&mut self) -> Res<()> {
        loop {
            if self.mark.col == 0 && self.src[self.mark.index..].starts_with('\u{FEFF}') {
                if !self.bom_allowed() {
                    return Err(
                        self.err_here("a byte order mark may only appear before a document")
                    );
                }
                // Zero-width for column purposes.
                self.mark.index += 3;
                continue;
            }
            let line_start = self.mark.col == 0;
            let mut spaces = 0usize;
            while let Some(b) = self.byte_at(0) {
                match b {
                    b' ' => {
                        if line_start && spaces == self.mark.col {
                            spaces += 1;
                        }
                        self.mark.index += 1;
                        self.mark.col += 1;
                    }
                    b'\t' => {
                        self.mark.index += 1;
                        self.mark.col += 1;
                    }
                    _ => break,
                }
            }
            // Inside flow collections, every content line must be indented
            // more than the enclosing block (spaces only). A line that only
            // closes the collection is exempt, as in yaml-rust2 and saphyr
            // (the strict grammar would require its indentation too).
            if line_start
                && !self.flows.is_empty()
                && !matches!(
                    self.byte_at(0),
                    None | Some(b'\n' | b'\r' | b'#' | b']' | b'}')
                )
                && (spaces as i64) <= self.indent
            {
                return Err(self.err_here(
                    "flow content must be indented more than the enclosing block (tabs don't count)",
                ));
            }
            if self.byte_at(0) == Some(b'#') {
                self.scan_comment()?;
            }
            if self.at_break() {
                self.skip_break();
                if self.flows.is_empty() {
                    self.simple_key_allowed = true;
                }
                continue;
            }
            return Ok(());
        }
    }

    /// Record a comment starting at `#`. It must start a line or follow
    /// white space.
    fn scan_comment(&mut self) -> Res<()> {
        let start = self.mark;
        let i = start.index;
        if i > 0
            && !matches!(self.bytes[i - 1], b' ' | b'\t' | b'\n' | b'\r')
            && !self.first_on_line(i)
        {
            return Err(
                self.err_here("a comment must be separated from other tokens by white space")
            );
        }
        self.advance();
        let text_start = self.mark.index;
        while let Some(c) = self.ch() {
            if is_break(c) {
                break;
            }
            if !is_nb_char(c) {
                return Err(
                    self.err_here(format!("invalid character U+{:04X} in comment", c as u32))
                );
            }
            self.advance();
        }
        self.comments.push(Comment {
            start,
            end: self.mark,
            text: &self.src[text_start..self.mark.index],
            trailing: !self.first_on_line(i),
        });
        Ok(())
    }

    // ------------------------------------------------------------------
    // Simple keys and indentation
    // ------------------------------------------------------------------

    fn in_flow_mapping(&self, level: usize) -> bool {
        level > 0 && self.flows.get(level - 1) == Some(&FlowKind::Mapping)
    }

    /// Drop candidates that can no longer become implicit keys: in block
    /// context and flow sequences a key must be on one line and at most
    /// 1024 characters long. Keys in flow mappings have neither limit.
    fn stale_simple_keys(&mut self) -> Res<()> {
        while let Some(&level) = self.stale_levels.first() {
            let k = self.simple_keys[level];
            if k.mark.line < self.mark.line || self.mark.col > k.mark.col + 1024 {
                if k.required {
                    return Err(self.err(
                        k.mark,
                        "could not find expected `:` after this implicit key",
                    ));
                }
                self.clear_key(level);
            } else {
                // Keys at higher levels come later on this line: not stale.
                break;
            }
        }
        Ok(())
    }

    fn set_key(&mut self, level: usize, key: SimpleKey) {
        self.clear_key(level);
        self.simple_keys[level] = key;
        if key.possible {
            self.possible_levels.insert(level);
            if !self.in_flow_mapping(level) {
                self.stale_levels.insert(level);
            }
        }
    }

    fn clear_key(&mut self, level: usize) {
        if self.simple_keys[level].possible {
            self.simple_keys[level].possible = false;
            self.possible_levels.remove(&level);
            self.stale_levels.remove(&level);
        }
    }

    fn save_simple_key(&mut self) -> Res<()> {
        let required = self.flows.is_empty() && self.indent == self.mark.col as i64;
        if self.simple_key_allowed {
            self.remove_simple_key()?;
            let key = SimpleKey {
                possible: true,
                required,
                token_number: self.tokens_parsed + self.tokens.len(),
                mark: self.mark,
                after_tab: self.tab_before(self.mark.index),
            };
            let top = self.simple_keys.len() - 1;
            self.set_key(top, key);
        }
        Ok(())
    }

    fn remove_simple_key(&mut self) -> Res<()> {
        let k = *self.simple_keys.last().unwrap();
        if k.possible && k.required {
            return Err(self.err(
                k.mark,
                "could not find expected `:` after this implicit key",
            ));
        }
        let top = self.simple_keys.len() - 1;
        self.clear_key(top);
        Ok(())
    }

    fn roll_indent(&mut self, col: usize, number: Option<usize>, kind: TokenKind<'a>, mark: Mark) {
        if !self.flows.is_empty() {
            return;
        }
        if self.indent < col as i64 {
            self.indents.push(self.indent);
            self.indent = col as i64;
            match number {
                Some(n) => self.insert(n - self.tokens_parsed, kind, mark),
                None => self.push(kind, mark, mark),
            }
        }
    }

    fn unroll_indent(&mut self, col: i64) {
        if !self.flows.is_empty() {
            return;
        }
        while self.indent > col {
            let m = self.mark;
            self.push(TokenKind::BlockEnd, m, m);
            self.indent = self.indents.pop().unwrap_or(-1);
        }
    }

    // ------------------------------------------------------------------
    // Fetchers
    // ------------------------------------------------------------------

    fn fetch_stream_end(&mut self) -> Res<()> {
        if self.mark.col != 0 {
            self.mark.col = 0;
            self.mark.line += 1;
        }
        self.unroll_indent(-1);
        self.remove_simple_key()?;
        self.simple_key_allowed = false;
        let m = self.mark;
        self.push(TokenKind::StreamEnd, m, m);
        Ok(())
    }

    fn fetch_document_indicator(&mut self, kind: TokenKind<'a>) -> Res<()> {
        if !self.flows.is_empty() {
            return Err(self.err_here("document markers are not allowed inside flow collections"));
        }
        self.unroll_indent(-1);
        self.remove_simple_key()?;
        self.simple_key_allowed = false;
        let start = self.mark;
        self.mark.index += 3;
        self.mark.col += 3;
        let end = self.mark;
        if kind == TokenKind::DocumentEnd {
            self.doc_end_line = Some(start.line);
        }
        self.push(kind, start, end);
        Ok(())
    }

    fn fetch_flow_collection_start(&mut self, kind: FlowKind) -> Res<()> {
        self.save_simple_key()?;
        self.simple_keys.push(SimpleKey::NONE);
        self.flows.push(kind);
        self.simple_key_allowed = true;
        let start = self.mark;
        self.advance();
        let tok = match kind {
            FlowKind::Sequence => TokenKind::FlowSequenceStart,
            FlowKind::Mapping => TokenKind::FlowMappingStart,
        };
        self.push(tok, start, self.mark);
        Ok(())
    }

    fn fetch_flow_collection_end(&mut self, kind: FlowKind) -> Res<()> {
        match self.flows.last() {
            None => return Err(self.err_here("unexpected end of a flow collection")),
            Some(&k) if k != kind => {
                return Err(self.err_here(match kind {
                    FlowKind::Sequence => "found `]` where `}` was expected",
                    FlowKind::Mapping => "found `}` where `]` was expected",
                }));
            }
            _ => {}
        }
        self.remove_simple_key()?;
        self.simple_keys.pop();
        self.flows.pop();
        self.simple_key_allowed = false;
        let start = self.mark;
        self.advance();
        let tok = match kind {
            FlowKind::Sequence => TokenKind::FlowSequenceEnd,
            FlowKind::Mapping => TokenKind::FlowMappingEnd,
        };
        self.push(tok, start, self.mark);
        Ok(())
    }

    fn fetch_flow_entry(&mut self) -> Res<()> {
        if self.flows.is_empty() {
            return Err(self.err_here("`,` is only allowed inside flow collections"));
        }
        self.remove_simple_key()?;
        self.simple_key_allowed = true;
        let start = self.mark;
        self.advance();
        self.push(TokenKind::FlowEntry, start, self.mark);
        Ok(())
    }

    fn fetch_block_entry(&mut self) -> Res<()> {
        if !self.flows.is_empty() {
            return Err(self.err_here("block sequence entries are not allowed in flow collections"));
        }
        if !self.simple_key_allowed {
            return Err(self.err_here("block sequence entries are not allowed here"));
        }
        if self.tab_before(self.mark.index) {
            return Err(
                self.err_here("a tab can't separate a block sequence entry from what precedes it")
            );
        }
        let m = self.mark;
        self.roll_indent(m.col, None, TokenKind::BlockSequenceStart, m);
        self.remove_simple_key()?;
        self.simple_key_allowed = true;
        let start = self.mark;
        self.advance();
        self.push(TokenKind::BlockEntry, start, self.mark);
        Ok(())
    }

    fn fetch_key(&mut self) -> Res<()> {
        if self.flows.is_empty() {
            if !self.simple_key_allowed {
                return Err(self.err_here("mapping keys are not allowed here"));
            }
            if self.tab_before(self.mark.index) {
                return Err(
                    self.err_here("a tab can't separate an explicit key from what precedes it")
                );
            }
            let m = self.mark;
            self.roll_indent(m.col, None, TokenKind::BlockMappingStart, m);
        }
        self.remove_simple_key()?;
        self.simple_key_allowed = self.flows.is_empty();
        let start = self.mark;
        self.advance();
        self.push(TokenKind::Key, start, self.mark);
        Ok(())
    }

    fn fetch_value(&mut self) -> Res<()> {
        let k = *self.simple_keys.last().unwrap();
        if k.possible {
            if self.flows.is_empty() && k.after_tab {
                return Err(self.err(
                    k.mark,
                    "a tab can't separate an implicit key from what precedes it",
                ));
            }
            self.insert(k.token_number - self.tokens_parsed, TokenKind::Key, k.mark);
            self.roll_indent(
                k.mark.col,
                Some(k.token_number),
                TokenKind::BlockMappingStart,
                k.mark,
            );
            let top = self.simple_keys.len() - 1;
            self.clear_key(top);
            self.simple_key_allowed = false;
        } else {
            if self.flows.is_empty() {
                if !self.simple_key_allowed {
                    return Err(self.err_here("mapping values are not allowed here"));
                }
                if self.tab_before(self.mark.index) && self.first_on_line(self.mark.index) {
                    return Err(self.err_here("a tab can't be used to indent `:`"));
                }
                let m = self.mark;
                self.roll_indent(m.col, None, TokenKind::BlockMappingStart, m);
            }
            self.simple_key_allowed = self.flows.is_empty();
        }
        let start = self.mark;
        self.advance();
        self.push(TokenKind::Value, start, self.mark);
        Ok(())
    }

    fn fetch_anchor(&mut self, alias: bool) -> Res<()> {
        self.save_simple_key()?;
        self.simple_key_allowed = false;
        let start = self.mark;
        self.advance();
        let name_start = self.mark.index;
        while let Some(c) = self.ch() {
            if !is_anchor_char(c) {
                break;
            }
            self.advance();
        }
        if self.mark.index == name_start {
            return Err(self.err(
                start,
                if alias {
                    "an alias needs a name"
                } else {
                    "an anchor needs a name"
                },
            ));
        }
        self.expect_separator_after_property(start)?;
        let name = Cow::Borrowed(&self.src[name_start..self.mark.index]);
        let kind = if alias {
            TokenKind::Alias(name)
        } else {
            TokenKind::Anchor(name)
        };
        self.push(kind, start, self.mark);
        Ok(())
    }

    /// Anchors, aliases and tags end at white space, the end of input, or
    /// (in flow context) a flow indicator.
    fn expect_separator_after_property(&self, start: Mark) -> Res<()> {
        let flow = !self.flows.is_empty();
        match self.ch() {
            None => Ok(()),
            Some(c) if is_blank(c) || is_break(c) => Ok(()),
            Some(',' | ']' | '}') if flow => Ok(()),
            Some(c) => Err(self.err(
                start,
                format!("unexpected `{c}` right after an anchor, alias or tag"),
            )),
        }
    }

    fn fetch_tag(&mut self) -> Res<()> {
        self.save_simple_key()?;
        self.simple_key_allowed = false;
        let start = self.mark;
        let (handle, suffix) = self.scan_tag(start)?;
        self.push(TokenKind::Tag(handle, suffix), start, self.mark);
        Ok(())
    }

    fn fetch_block_scalar(&mut self, literal: bool) -> Res<()> {
        self.remove_simple_key()?;
        self.simple_key_allowed = true;
        let tok = self.scan_block_scalar(literal)?;
        self.push(tok.kind, tok.start, tok.end);
        Ok(())
    }

    fn fetch_flow_scalar(&mut self, single: bool) -> Res<()> {
        self.save_simple_key()?;
        self.simple_key_allowed = false;
        let tok = self.scan_flow_scalar(single)?;
        self.push(tok.kind, tok.start, tok.end);
        Ok(())
    }

    fn fetch_plain_scalar(&mut self) -> Res<()> {
        self.save_simple_key()?;
        self.simple_key_allowed = false;
        let tok = self.scan_plain_scalar()?;
        self.push(tok.kind, tok.start, tok.end);
        Ok(())
    }

    fn fetch_directive(&mut self) -> Res<()> {
        self.unroll_indent(-1);
        self.remove_simple_key()?;
        self.simple_key_allowed = false;
        let tok = self.scan_directive()?;
        self.push(tok.kind, tok.start, tok.end);
        Ok(())
    }

    // ------------------------------------------------------------------
    // Directives
    // ------------------------------------------------------------------

    fn skip_blanks(&mut self) -> bool {
        let start = self.mark.index;
        while self.at_blank() {
            self.advance();
        }
        self.mark.index > start
    }

    fn scan_directive(&mut self) -> Res<Token<'a>> {
        let start = self.mark;
        self.advance(); // '%'
        let name_start = self.mark.index;
        while let Some(c) = self.ch() {
            if !is_ns_char(c) {
                break;
            }
            self.advance();
        }
        let name = &self.src[name_start..self.mark.index];
        if name.is_empty() {
            return Err(self.err(start, "expected a directive name after `%`"));
        }
        let kind = match name {
            "YAML" => {
                if !self.skip_blanks() {
                    return Err(self.err_here("expected white space after `%YAML`"));
                }
                let major = self.scan_version_number()?;
                if self.ch() != Some('.') {
                    return Err(self.err_here("expected `.` in the %YAML version"));
                }
                self.advance();
                let minor = self.scan_version_number()?;
                TokenKind::VersionDirective(major, minor)
            }
            "TAG" => {
                if !self.skip_blanks() {
                    return Err(self.err_here("expected white space after `%TAG`"));
                }
                let handle = self.scan_tag_handle_for_directive()?;
                if !self.skip_blanks() {
                    return Err(self.err_here("expected white space after the %TAG handle"));
                }
                let prefix = self.scan_tag_prefix()?;
                TokenKind::TagDirective(handle, prefix)
            }
            _ => {
                // Reserved directive: parameters are ns-char runs.
                loop {
                    let had_ws = self.skip_blanks();
                    match self.ch() {
                        None | Some('\n' | '\r') => break,
                        Some('#') if had_ws => break,
                        Some(c) if is_ns_char(c) && had_ws => {
                            while self.ch().is_some_and(is_ns_char) {
                                self.advance();
                            }
                        }
                        Some(_) => return Err(self.err_here("invalid directive parameter")),
                    }
                }
                TokenKind::ReservedDirective
            }
        };
        let end = self.mark;
        // Only white space and a comment may follow on the line.
        let had_ws = self.skip_blanks()
            || matches!(
                self.bytes.get(self.mark.index.wrapping_sub(1)),
                Some(b' ' | b'\t')
            );
        match self.ch() {
            None | Some('\n' | '\r') => {}
            Some('#') if had_ws => {}
            Some(_) => return Err(self.err_here("unexpected content after the directive")),
        }
        Ok(Token { kind, start, end })
    }

    fn scan_version_number(&mut self) -> Res<u32> {
        let start = self.mark.index;
        while self.ch().is_some_and(|c| c.is_ascii_digit()) {
            self.advance();
        }
        let digits = &self.src[start..self.mark.index];
        if digits.is_empty() || digits.len() > 9 {
            return Err(self.err_here("expected a version number"));
        }
        Ok(digits.parse().unwrap())
    }

    /// `!`, `!!` or `!word!`.
    fn scan_tag_handle_for_directive(&mut self) -> Res<Cow<'a, str>> {
        let start = self.mark;
        if self.ch() != Some('!') {
            return Err(self.err_here("expected a tag handle starting with `!`"));
        }
        self.advance();
        while self.ch().is_some_and(is_word_char) {
            self.advance();
        }
        if self.ch() == Some('!') {
            self.advance();
        } else if self.mark.index - start.index > 1 {
            return Err(self.err_here("a named tag handle must end with `!`"));
        }
        Ok(Cow::Borrowed(&self.src[start.index..self.mark.index]))
    }

    fn scan_tag_prefix(&mut self) -> Res<Cow<'a, str>> {
        let start = self.mark.index;
        match self.ch() {
            Some('!') => self.advance(),
            Some(c) if is_tag_char(c) => {}
            _ => return Err(self.err_here("expected a tag prefix")),
        }
        self.scan_uri_chars(true)?;
        if self.mark.index == start {
            return Err(self.err_here("expected a tag prefix"));
        }
        Ok(Cow::Borrowed(&self.src[start..self.mark.index]))
    }

    /// Consume URI characters (`allow_all`) or tag characters, validating
    /// `%XX` escapes.
    fn scan_uri_chars(&mut self, allow_all: bool) -> Res<()> {
        while let Some(c) = self.ch() {
            let ok = if allow_all {
                is_uri_char(c)
            } else {
                is_tag_char(c)
            };
            if !ok {
                break;
            }
            if c == '%' {
                let hex = |o: usize| self.byte_at(o).is_some_and(|b| b.is_ascii_hexdigit());
                if !(hex(1) && hex(2)) {
                    return Err(self.err_here("`%` in a tag must be followed by two hex digits"));
                }
                self.mark.index += 3;
                self.mark.col += 3;
            } else {
                self.advance();
            }
        }
        Ok(())
    }

    // ------------------------------------------------------------------
    // Tags
    // ------------------------------------------------------------------

    fn scan_tag(&mut self, start: Mark) -> Res<(Cow<'a, str>, Cow<'a, str>)> {
        self.advance(); // '!'
        if self.ch() == Some('<') {
            self.advance();
            let s = self.mark.index;
            self.scan_uri_chars(true)?;
            let content = &self.src[s..self.mark.index];
            if self.ch() != Some('>') {
                return Err(self.err(start, "a verbatim tag must end with `>`"));
            }
            self.advance();
            let local_ok = content.len() > 1 && content.starts_with('!');
            let global_ok = content
                .as_bytes()
                .first()
                .is_some_and(u8::is_ascii_alphabetic)
                && content.contains(':');
            if !(local_ok || global_ok) {
                return Err(self.err(
                    start,
                    "a verbatim tag must be a local tag (`!x`) or a global URI",
                ));
            }
            self.expect_separator_after_property(start)?;
            return Ok((Cow::Borrowed(""), Cow::Borrowed(content)));
        }
        // Shorthand: `!suffix`, `!!suffix`, `!word!suffix`, or `!` alone.
        let after_bang = self.mark.index;
        let mut handle_end = start.index + 1;
        let save = self.mark;
        while self.ch().is_some_and(is_word_char) {
            self.advance();
        }
        if self.ch() == Some('!') {
            self.advance();
            handle_end = self.mark.index;
        } else {
            self.mark = save;
        }
        let handle = &self.src[start.index..handle_end];
        let suffix_start = self.mark.index;
        debug_assert!(suffix_start >= after_bang);
        self.scan_uri_chars(false)?;
        let suffix = &self.src[suffix_start..self.mark.index];
        if suffix.is_empty() && handle != "!" {
            return Err(self.err(start, format!("the tag handle `{handle}` needs a suffix")));
        }
        self.expect_separator_after_property(start)?;
        Ok((Cow::Borrowed(handle), Cow::Borrowed(suffix)))
    }

    // ------------------------------------------------------------------
    // Block scalars
    // ------------------------------------------------------------------

    fn scan_block_scalar(&mut self, literal: bool) -> Res<Token<'a>> {
        let start = self.mark;
        self.advance(); // '|' or '>'
        let mut chomping = 0i8;
        let mut increment = 0i64;
        let indicator = |s: &mut Self, chomping: &mut i8, increment: &mut i64| -> Res<bool> {
            match s.ch() {
                Some('+') | Some('-') if *chomping == 0 => {
                    *chomping = if s.ch() == Some('+') { 1 } else { -1 };
                    s.advance();
                    Ok(true)
                }
                Some('0') if *increment == 0 => {
                    Err(s.err_here("the indentation indicator must be between 1 and 9"))
                }
                Some(c @ '1'..='9') if *increment == 0 => {
                    *increment = c as i64 - '0' as i64;
                    s.advance();
                    Ok(true)
                }
                _ => Ok(false),
            }
        };
        if indicator(self, &mut chomping, &mut increment)? {
            indicator(self, &mut chomping, &mut increment)?;
        }
        let mut end = self.mark;
        let had_ws = self.skip_blanks();
        if self.ch() == Some('#') {
            if !had_ws {
                return Err(
                    self.err_here("a comment must be separated from other tokens by white space")
                );
            }
            self.scan_comment()?;
        }
        if !self.at_breakz() {
            return Err(
                self.err_here("expected a comment or a line break after the block scalar header")
            );
        }
        self.skip_break();

        // Content indentation: the parent indent plus the indicator, or
        // auto-detected (at least parent + 1, so column 0 at the top level).
        // For an explicit indicator the top level counts as indent 0, as in
        // libyaml, yaml-rust2 and saphyr; the grammar read literally
        // (n = -1) would give one less, a case the test suite doesn't cover.
        let parent = self.indent;
        let mut indent: i64 = if increment > 0 {
            parent.max(0) + increment
        } else {
            -1
        };

        let mut string = String::new();
        let mut leading_break = String::new();
        let mut trailing_breaks = String::new();

        if indent < 0 {
            indent = self.detect_block_indent(parent, &mut trailing_breaks)?;
        } else {
            self.scan_block_breaks(indent, &mut trailing_breaks)?;
        }

        let mut leading_blank = false;
        while self.mark.col as i64 == indent && !self.is_z() {
            if self.at_document_indicator() {
                break;
            }
            let trailing_blank = self.at_blank();
            if !literal && leading_break.starts_with('\n') && !leading_blank && !trailing_blank {
                if trailing_breaks.is_empty() {
                    string.push(' ');
                }
                leading_break.clear();
            } else {
                string.push_str(&leading_break);
                leading_break.clear();
            }
            string.push_str(&trailing_breaks);
            trailing_breaks.clear();
            leading_blank = self.at_blank();
            let line_start = self.mark.index;
            while let Some(c) = self.ch() {
                if is_break(c) {
                    break;
                }
                if !is_nb_char(c) {
                    return Err(self.err_here(format!("invalid character U+{:04X}", c as u32)));
                }
                self.advance();
            }
            string.push_str(&self.src[line_start..self.mark.index]);
            end = self.mark;
            if self.is_z() {
                // The final line break of the stream may be omitted.
                leading_break.clear();
                leading_break.push('\n');
                break;
            }
            self.skip_break();
            leading_break.clear();
            leading_break.push('\n');
            self.scan_block_breaks(indent, &mut trailing_breaks)?;
        }

        if chomping != -1 {
            string.push_str(&leading_break);
        }
        if chomping == 1 {
            string.push_str(&trailing_breaks);
        }
        let style = if literal {
            ScalarStyle::Literal
        } else {
            ScalarStyle::Folded
        };
        Ok(Token {
            kind: TokenKind::Scalar(
                Cow::Owned(string),
                style,
                Some(BlockHeader {
                    chomping: match chomping {
                        -1 => Chomping::Strip,
                        1 => Chomping::Keep,
                        _ => Chomping::Clip,
                    },
                    indent: (increment > 0).then_some(increment as u8),
                }),
            ),
            start,
            end,
        })
    }

    /// Auto-detect the content indentation from the first non-empty line,
    /// collecting the leading empty lines into `breaks`.
    fn detect_block_indent(&mut self, parent: i64, breaks: &mut String) -> Res<i64> {
        let min = parent + 1;
        let mut max_empty = 0usize;
        loop {
            let line = self.mark;
            while self.byte_at(0) == Some(b' ') {
                self.advance();
            }
            let spaces = self.mark.col;
            if self.at_break() {
                max_empty = max_empty.max(spaces);
                breaks.push('\n');
                self.skip_break();
                continue;
            }
            if self.is_z() {
                max_empty = max_empty.max(spaces);
                if spaces > 0 {
                    // A last line of spaces without a line break.
                    breaks.push('\n');
                }
                return Ok(min.max(max_empty as i64));
            }
            if spaces as i64 >= min {
                if max_empty > spaces {
                    return Err(self.err(
                        line,
                        "a leading empty line has more spaces than the first line of the block scalar",
                    ));
                }
                return Ok(spaces as i64);
            }
            // Less indented: the scalar has no content.
            self.check_block_scalar_trailing_line()?;
            return Ok(min.max(max_empty as i64));
        }
    }

    /// Consume empty lines after block scalar content (or before it, with
    /// an explicit indentation indicator), stopping at the next line that
    /// has `indent` spaces or is less indented.
    fn scan_block_breaks(&mut self, indent: i64, breaks: &mut String) -> Res<()> {
        loop {
            while (self.mark.col as i64) < indent && self.byte_at(0) == Some(b' ') {
                self.advance();
            }
            if self.is_z() && self.mark.col > 0 && (self.mark.col as i64) < indent {
                // A last line of spaces without a line break.
                breaks.push('\n');
                return Ok(());
            }
            if self.at_break() {
                breaks.push('\n');
                self.skip_break();
                continue;
            }
            if (self.mark.col as i64) < indent && !self.is_z() {
                self.check_block_scalar_trailing_line()?;
            }
            return Ok(());
        }
    }

    /// The scalar ended at a less indented line. Before any trailing
    /// comment, a tab after the indentation is an error (YAML 1.2.2
    /// `l-chomped-empty`): such a line is neither empty nor a comment.
    fn check_block_scalar_trailing_line(&self) -> Res<()> {
        if self.byte_at(0) == Some(b'\t') {
            return Err(
                self.err_here("a tab can't follow the indentation of a line after a block scalar")
            );
        }
        Ok(())
    }

    // ------------------------------------------------------------------
    // Quoted scalars
    // ------------------------------------------------------------------

    /// After a line break inside a multi-line scalar or flow collection:
    /// the line must be indented more than the enclosing block, counting
    /// spaces only. A shorter line may only be empty, without tabs.
    fn check_continuation_line(&self) -> Res<()> {
        let mut i = self.mark.index;
        let mut spaces = 0usize;
        while self.bytes.get(i) == Some(&b' ') {
            i += 1;
            spaces += 1;
        }
        if spaces as i64 > self.indent {
            return Ok(());
        }
        match self.bytes.get(i) {
            None | Some(b'\n' | b'\r') => Ok(()),
            Some(b'\t') => {
                let mut j = i;
                while matches!(self.bytes.get(j), Some(b' ' | b'\t')) {
                    j += 1;
                }
                if matches!(self.bytes.get(j), None | Some(b'\n' | b'\r')) {
                    Err(self.err_here("a tab can't indent an empty line inside a quoted scalar"))
                } else {
                    Err(self
                        .err_here("continuation line is not indented enough (tabs don't count)"))
                }
            }
            Some(_) => Err(self.err_here("continuation line is not indented enough")),
        }
    }

    fn scan_flow_scalar(&mut self, single: bool) -> Res<Token<'a>> {
        let start = self.mark;
        self.advance(); // opening quote
        let quote = if single { '\'' } else { '"' };
        let content_start = self.mark.index;
        // Stay borrowed until an escape or a line break forces a copy.
        let mut owned: Option<String> = None;
        let mut leading_break = false;
        let mut trailing_breaks = 0usize;
        let mut whitespaces = String::new();

        loop {
            if self.at_document_indicator() {
                return Err(self.err_here("a document marker can't appear inside a quoted scalar"));
            }
            if self.is_z() {
                return Err(self.err(start, "unterminated quoted scalar"));
            }
            let mut leading_blanks = false;
            while let Some(c) = self.ch() {
                if is_blank(c) || is_break(c) {
                    break;
                }
                if single && c == '\'' {
                    if self.byte_at(1) == Some(b'\'') {
                        owned
                            .get_or_insert_with(|| {
                                self.src[content_start..self.mark.index].to_string()
                            })
                            .push('\'');
                        self.mark.index += 2;
                        self.mark.col += 2;
                        continue;
                    }
                    break;
                }
                if !single && c == '"' {
                    break;
                }
                if !single && c == '\\' {
                    let buf = owned.get_or_insert_with(|| {
                        self.src[content_start..self.mark.index].to_string()
                    });
                    if matches!(self.byte_at(1), Some(b'\n' | b'\r')) {
                        // Escaped line break: joins lines without a space.
                        self.advance();
                        self.skip_break();
                        self.check_continuation_line()?;
                        leading_blanks = true;
                        break;
                    }
                    let esc_mark = self.mark;
                    self.advance();
                    let Some(e) = self.ch() else {
                        return Err(self.err(start, "unterminated quoted scalar"));
                    };
                    let simple = match e {
                        '0' => Some('\0'),
                        'a' => Some('\u{7}'),
                        'b' => Some('\u{8}'),
                        't' | '\t' => Some('\t'),
                        'n' => Some('\n'),
                        'v' => Some('\u{B}'),
                        'f' => Some('\u{C}'),
                        'r' => Some('\r'),
                        'e' => Some('\u{1B}'),
                        ' ' => Some(' '),
                        '"' => Some('"'),
                        '/' => Some('/'),
                        '\\' => Some('\\'),
                        'N' => Some('\u{85}'),
                        '_' => Some('\u{A0}'),
                        'L' => Some('\u{2028}'),
                        'P' => Some('\u{2029}'),
                        _ => None,
                    };
                    if let Some(ch) = simple {
                        buf.push(ch);
                        self.advance();
                        continue;
                    }
                    let digits = match e {
                        'x' => 2,
                        'u' => 4,
                        'U' => 8,
                        _ => {
                            return Err(
                                self.err(esc_mark, format!("invalid escape sequence `\\{e}`"))
                            );
                        }
                    };
                    self.advance();
                    let hex_start = self.mark.index;
                    for _ in 0..digits {
                        if !self.byte_at(0).is_some_and(|b| b.is_ascii_hexdigit()) {
                            return Err(
                                self.err(esc_mark, "expected hex digits in escape sequence")
                            );
                        }
                        self.advance();
                    }
                    let mut cp =
                        u32::from_str_radix(&self.src[hex_start..self.mark.index], 16).unwrap();
                    // JSON-style surrogate pair: \uD83D\uDE00.
                    if (0xD800..0xDC00).contains(&cp) {
                        let i = self.mark.index;
                        let lo = self
                            .bytes
                            .get(i..i + 6)
                            .filter(|b| {
                                b.starts_with(b"\\u") && b[2..].iter().all(u8::is_ascii_hexdigit)
                            })
                            .and_then(|b| {
                                u32::from_str_radix(std::str::from_utf8(&b[2..]).ok()?, 16).ok()
                            })
                            .filter(|lo| (0xDC00..0xE000).contains(lo));
                        if let Some(lo) = lo {
                            cp = 0x10000 + ((cp - 0xD800) << 10) + (lo - 0xDC00);
                            self.mark.index += 6;
                            self.mark.col += 6;
                        }
                    }
                    match char::from_u32(cp) {
                        Some(ch) => buf.push(ch),
                        None => {
                            return Err(
                                self.err(esc_mark, "escape sequence is not a valid character")
                            );
                        }
                    }
                    continue;
                }
                if !is_json_char(c) {
                    return Err(self.err_here(format!("invalid character U+{:04X}", c as u32)));
                }
                if let Some(buf) = owned.as_mut() {
                    buf.push(c);
                }
                self.advance();
            }

            if self.ch() == Some(quote) {
                break;
            }

            // White space and line breaks.
            let ws_from = self.mark.index;
            while let Some(c) = self.ch() {
                if is_blank(c) {
                    if !leading_blanks {
                        whitespaces.push(c);
                    }
                    self.advance();
                } else if is_break(c) {
                    // White space before a line break is dropped.
                    owned.get_or_insert_with(|| self.src[content_start..ws_from].to_string());
                    if !leading_blanks {
                        whitespaces.clear();
                        leading_break = true;
                        leading_blanks = true;
                    } else {
                        trailing_breaks += 1;
                    }
                    self.skip_break();
                    self.check_continuation_line()?;
                } else {
                    break;
                }
            }

            if leading_blanks {
                let buf = owned
                    .get_or_insert_with(|| self.src[content_start..self.mark.index].to_string());
                if leading_break {
                    if trailing_breaks == 0 {
                        buf.push(' ');
                    } else {
                        buf.extend(std::iter::repeat_n('\n', trailing_breaks));
                    }
                } else {
                    buf.extend(std::iter::repeat_n('\n', trailing_breaks));
                }
                leading_break = false;
                trailing_breaks = 0;
                whitespaces.clear();
            } else if !whitespaces.is_empty() {
                if let Some(buf) = owned.as_mut() {
                    buf.push_str(&whitespaces);
                }
                whitespaces.clear();
            }
        }

        let content_end = self.mark.index;
        self.advance(); // closing quote
        let value = match owned {
            Some(s) => Cow::Owned(s),
            None => Cow::Borrowed(&self.src[content_start..content_end]),
        };
        let style = if single {
            ScalarStyle::SingleQuoted
        } else {
            ScalarStyle::DoubleQuoted
        };
        Ok(Token {
            kind: TokenKind::Scalar(value, style, None),
            start,
            end: self.mark,
        })
    }

    // ------------------------------------------------------------------
    // Plain scalars
    // ------------------------------------------------------------------

    fn scan_plain_scalar(&mut self) -> Res<Token<'a>> {
        let start = self.mark;
        let mut end = self.mark;
        let flow = !self.flows.is_empty();
        let min_indent = self.indent + 1;
        // Borrowed while the scalar is a single run of the input.
        let mut owned: Option<String> = None;
        let mut whitespaces_start: Option<usize> = None;
        let mut leading_blanks = false;
        let mut leading_break = false;
        let mut trailing_breaks = 0usize;

        loop {
            if self.at_document_indicator() {
                break;
            }
            if self.byte_at(0) == Some(b'#') {
                break;
            }
            while let Some(c) = self.ch() {
                if is_blank(c) || is_break(c) {
                    break;
                }
                if c == ':' {
                    let ends = match self.ch_at(1) {
                        None => true,
                        Some(n) => is_blank(n) || is_break(n) || (flow && is_flow_indicator(n)),
                    };
                    if ends {
                        break;
                    }
                } else if flow && is_flow_indicator(c) {
                    break;
                }
                if !is_ns_char(c) {
                    if c == '\u{FEFF}' {
                        return Err(
                            self.err_here("a byte order mark can't appear in a plain scalar")
                        );
                    }
                    return Err(self.err_here(format!("invalid character U+{:04X}", c as u32)));
                }
                if leading_blanks {
                    let buf =
                        owned.get_or_insert_with(|| self.src[start.index..end.index].to_string());
                    if leading_break {
                        if trailing_breaks == 0 {
                            buf.push(' ');
                        } else {
                            buf.extend(std::iter::repeat_n('\n', trailing_breaks));
                        }
                    }
                    leading_blanks = false;
                    leading_break = false;
                    trailing_breaks = 0;
                    whitespaces_start = None;
                } else if let Some(ws) = whitespaces_start.take()
                    && let Some(buf) = owned.as_mut()
                {
                    buf.push_str(&self.src[ws..self.mark.index]);
                }
                let c_start = self.mark.index;
                self.advance();
                if let Some(buf) = owned.as_mut() {
                    buf.push_str(&self.src[c_start..self.mark.index]);
                }
                end = self.mark;
            }

            if !(self.at_blank() || self.at_break()) {
                break;
            }

            while let Some(c) = self.ch() {
                if is_blank(c) {
                    if !leading_blanks && whitespaces_start.is_none() {
                        whitespaces_start = Some(self.mark.index);
                    }
                    self.advance();
                } else if is_break(c) {
                    if !leading_blanks {
                        whitespaces_start = None;
                        leading_break = true;
                        leading_blanks = true;
                    } else {
                        trailing_breaks += 1;
                    }
                    self.skip_break();
                } else {
                    break;
                }
            }

            if leading_blanks {
                let spaces = {
                    let mut i = self.mark.index;
                    while i > 0 && self.bytes[i - 1] != b'\n' && self.bytes[i - 1] != b'\r' {
                        i -= 1;
                    }
                    self.bytes[i..self.mark.index]
                        .iter()
                        .take_while(|&&b| b == b' ')
                        .count()
                };
                if flow {
                    if (spaces as i64) <= self.indent
                        && !matches!(
                            self.byte_at(0),
                            None | Some(b'#' | b'\n' | b'\r' | b']' | b'}')
                        )
                    {
                        return Err(self.err_here(
                            "flow content must be indented more than the enclosing block (tabs don't count)",
                        ));
                    }
                } else if (spaces as i64) < min_indent {
                    break;
                }
            }
        }

        if leading_blanks {
            self.simple_key_allowed = true;
        }
        let value = match owned {
            Some(s) => Cow::Owned(s),
            None => Cow::Borrowed(&self.src[start.index..end.index]),
        };
        Ok(Token {
            kind: TokenKind::Scalar(value, ScalarStyle::Plain, None),
            start,
            end,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(src: &str) -> Vec<String> {
        let mut s = Scanner::new(src);
        let mut out = Vec::new();
        loop {
            let t = s.next_token().unwrap();
            let done = t.kind == TokenKind::StreamEnd;
            out.push(match t.kind {
                TokenKind::Scalar(v, ..) => format!("S({v})"),
                k => format!("{k:?}"),
            });
            if done {
                return out;
            }
        }
    }

    #[test]
    fn simple_mapping_tokens() {
        assert_eq!(
            kinds("a: b\n"),
            [
                "StreamStart",
                "BlockMappingStart",
                "Key",
                "S(a)",
                "Value",
                "S(b)",
                "BlockEnd",
                "StreamEnd"
            ]
        );
    }

    #[test]
    fn comments_go_to_the_side_buffer() {
        let mut s = Scanner::new("# head\na: b # tail\n");
        while s.next_token().unwrap().kind != TokenKind::StreamEnd {}
        assert_eq!(s.comments.len(), 2);
        assert_eq!(s.comments[0].text, " head");
        assert!(!s.comments[0].trailing);
        assert_eq!(s.comments[1].text, " tail");
        assert!(s.comments[1].trailing);
        assert_eq!((s.comments[1].start.line, s.comments[1].start.col), (1, 5));
    }

    #[test]
    fn plain_scalars_borrow_unless_folded() {
        let mut s = Scanner::new("a b\n");
        s.next_token().unwrap();
        let t = s.next_token().unwrap();
        assert!(matches!(
            t.kind,
            TokenKind::Scalar(Cow::Borrowed("a b"), ..)
        ));
        let mut s = Scanner::new("a\n b\n");
        s.next_token().unwrap();
        let t = s.next_token().unwrap();
        assert!(matches!(t.kind, TokenKind::Scalar(Cow::Owned(ref v), ..) if v == "a b"));
    }
}
