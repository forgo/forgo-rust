//! Event parser: an iterative state machine over the scanner's tokens
//! (the libyaml state set), plus YAML 1.2.2 document rules.

use super::event::{Event, Mark, ScalarStyle, Span, Tag};
use super::scanner::{Comment, Scanner, Token, TokenKind};
use crate::ast::Error;
use std::borrow::Cow;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    StreamStart,
    ImplicitDocumentStart,
    DocumentStart,
    DocumentContent,
    DocumentEnd,
    BlockNode,
    BlockSequenceFirstEntry,
    BlockSequenceEntry,
    IndentlessSequenceEntry,
    BlockMappingFirstKey,
    BlockMappingKey,
    BlockMappingValue,
    FlowSequenceFirstEntry,
    FlowSequenceEntry,
    FlowSequenceEntryMappingKey,
    FlowSequenceEntryMappingValue,
    FlowSequenceEntryMappingEnd,
    FlowMappingFirstKey,
    FlowMappingKey,
    FlowMappingValue,
    FlowMappingEmptyValue,
    End,
}

type Res<T> = Result<T, Error>;
type Ev<'a> = (Event<'a>, Span);

pub struct Parser<'a> {
    scanner: Scanner<'a>,
    state: State,
    states: Vec<State>,
    /// `%TAG` directives of the current document.
    tag_handles: Vec<(Cow<'a, str>, Cow<'a, str>)>,
    /// The previous document ended with `...` (true before the first one).
    prev_doc_end_explicit: bool,
    failed: bool,
}

fn err(mark: Mark, msg: impl Into<String>) -> Error {
    Error::Syntax {
        line: mark.line + 1,
        col: mark.col + 1,
        msg: msg.into(),
    }
}

fn empty_scalar<'a>(mark: Mark) -> Ev<'a> {
    (
        Event::Scalar {
            anchor: None,
            tag: None,
            value: Cow::Borrowed(""),
            style: ScalarStyle::Plain,
            block: None,
        },
        Span::new(mark, mark),
    )
}

/// Decode `%XX` escapes in a tag.
fn percent_decode<'a>(s: Cow<'a, str>, mark: Mark) -> Res<Cow<'a, str>> {
    if !s.contains('%') {
        return Ok(s);
    }
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' {
            let v = b
                .get(i + 1..i + 3)
                .and_then(|h| std::str::from_utf8(h).ok())
                .and_then(|h| u8::from_str_radix(h, 16).ok())
                .ok_or_else(|| err(mark, "invalid `%` escape in tag"))?;
            out.push(v);
            i += 3;
            continue;
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8(out)
        .map(Cow::Owned)
        .map_err(|_| err(mark, "percent-escapes in tag are not valid UTF-8"))
}

impl<'a> Parser<'a> {
    pub fn new(src: &'a str) -> Self {
        Parser {
            scanner: Scanner::new(src),
            state: State::StreamStart,
            states: Vec::new(),
            tag_handles: Vec::new(),
            prev_doc_end_explicit: true,
            failed: false,
        }
    }

    /// Comments seen so far (all of them once the stream has been parsed).
    pub fn comments(&self) -> &[Comment<'a>] {
        &self.scanner.comments
    }

    /// Scan up to the next token, so that every comment before it is in
    /// [`Parser::comments`]. Errors are left for `next_event` to report.
    pub(crate) fn scan_ahead(&mut self) {
        if self.state != State::End && !self.failed {
            let _ = self.scanner.peek();
        }
    }

    /// The next event, or `None` after `StreamEnd` or an error.
    pub fn next_event(&mut self) -> Res<Option<Ev<'a>>> {
        if self.state == State::End || self.failed {
            return Ok(None);
        }
        match self.dispatch() {
            Ok(ev) => Ok(Some(ev)),
            Err(e) => {
                self.failed = true;
                Err(e)
            }
        }
    }

    fn dispatch(&mut self) -> Res<Ev<'a>> {
        match self.state {
            State::StreamStart => self.parse_stream_start(),
            State::ImplicitDocumentStart => self.parse_document_start(true),
            State::DocumentStart => self.parse_document_start(false),
            State::DocumentContent => self.parse_document_content(),
            State::DocumentEnd => self.parse_document_end(),
            State::BlockNode => self.parse_node(true, false),
            State::BlockSequenceFirstEntry => self.parse_block_sequence_entry(true),
            State::BlockSequenceEntry => self.parse_block_sequence_entry(false),
            State::IndentlessSequenceEntry => self.parse_indentless_sequence_entry(),
            State::BlockMappingFirstKey => self.parse_block_mapping_key(true),
            State::BlockMappingKey => self.parse_block_mapping_key(false),
            State::BlockMappingValue => self.parse_block_mapping_value(),
            State::FlowSequenceFirstEntry => self.parse_flow_sequence_entry(true),
            State::FlowSequenceEntry => self.parse_flow_sequence_entry(false),
            State::FlowSequenceEntryMappingKey => self.parse_flow_sequence_entry_mapping_key(),
            State::FlowSequenceEntryMappingValue => self.parse_flow_sequence_entry_mapping_value(),
            State::FlowSequenceEntryMappingEnd => {
                self.state = State::FlowSequenceEntry;
                let m = self.peek()?.start;
                Ok((Event::MappingEnd, Span::new(m, m)))
            }
            State::FlowMappingFirstKey => self.parse_flow_mapping_key(true),
            State::FlowMappingKey => self.parse_flow_mapping_key(false),
            State::FlowMappingValue => self.parse_flow_mapping_value(false),
            State::FlowMappingEmptyValue => self.parse_flow_mapping_value(true),
            State::End => unreachable!("no events after the stream end"),
        }
    }

    // ------------------------------------------------------------------

    fn peek(&mut self) -> Res<&Token<'a>> {
        self.scanner.peek()
    }

    fn peek_kind(&mut self) -> Res<&TokenKind<'a>> {
        Ok(&self.scanner.peek()?.kind)
    }

    fn skip(&mut self) -> Res<Token<'a>> {
        self.scanner.next_token()
    }

    fn pop_state(&mut self) -> State {
        self.states.pop().unwrap_or(State::End)
    }

    // ------------------------------------------------------------------
    // Stream and documents
    // ------------------------------------------------------------------

    fn parse_stream_start(&mut self) -> Res<Ev<'a>> {
        let t = self.skip()?;
        debug_assert_eq!(t.kind, TokenKind::StreamStart);
        self.state = State::ImplicitDocumentStart;
        Ok((Event::StreamStart, Span::new(t.start, t.end)))
    }

    fn parse_document_start(&mut self, first: bool) -> Res<Ev<'a>> {
        // `...` may repeat between documents, and may even start the stream.
        while *self.peek_kind()? == TokenKind::DocumentEnd {
            self.skip()?;
            self.prev_doc_end_explicit = true;
        }
        let tok = self.peek()?;
        let start = tok.start;
        match tok.kind {
            TokenKind::StreamEnd => {
                let t = self.skip()?;
                self.state = State::End;
                Ok((Event::StreamEnd, Span::new(t.start, t.end)))
            }
            TokenKind::VersionDirective(..)
            | TokenKind::TagDirective(..)
            | TokenKind::ReservedDirective
            | TokenKind::DocumentStart => {
                let has_directives = tok.kind != TokenKind::DocumentStart;
                if has_directives && !self.prev_doc_end_explicit {
                    return Err(err(
                        start,
                        "a directive must be preceded by `...` ending the previous document",
                    ));
                }
                let (version, tags) = self.process_directives()?;
                let tok = self.peek()?;
                if tok.kind != TokenKind::DocumentStart {
                    return Err(err(tok.start, "expected `---` after the directives"));
                }
                let t = self.skip()?;
                self.states.push(State::DocumentEnd);
                self.state = State::DocumentContent;
                Ok((
                    Event::DocumentStart {
                        explicit: true,
                        version,
                        tags,
                    },
                    Span::new(start, t.end),
                ))
            }
            _ => {
                if !first && !self.prev_doc_end_explicit {
                    return Err(err(
                        start,
                        "expected a single document node; use `---` to start another document",
                    ));
                }
                self.tag_handles.clear();
                self.states.push(State::DocumentEnd);
                self.state = State::BlockNode;
                Ok((
                    Event::DocumentStart {
                        explicit: false,
                        version: None,
                        tags: Vec::new(),
                    },
                    Span::new(start, start),
                ))
            }
        }
    }

    #[allow(clippy::type_complexity)]
    fn process_directives(
        &mut self,
    ) -> Res<(Option<(u32, u32)>, Vec<(Cow<'a, str>, Cow<'a, str>)>)> {
        let mut version = None;
        let mut tags: Vec<(Cow<'a, str>, Cow<'a, str>)> = Vec::new();
        loop {
            match self.peek_kind()? {
                TokenKind::VersionDirective(..) => {
                    let t = self.skip()?;
                    let TokenKind::VersionDirective(major, minor) = t.kind else {
                        unreachable!()
                    };
                    if version.is_some() {
                        return Err(err(t.start, "duplicate %YAML directive"));
                    }
                    if major != 1 {
                        return Err(err(
                            t.start,
                            format!("unsupported YAML version {major}.{minor}"),
                        ));
                    }
                    version = Some((major, minor));
                }
                TokenKind::TagDirective(..) => {
                    let t = self.skip()?;
                    let TokenKind::TagDirective(handle, prefix) = t.kind else {
                        unreachable!()
                    };
                    if tags.iter().any(|(h, _)| *h == handle) {
                        return Err(err(
                            t.start,
                            format!("duplicate %TAG directive for `{handle}`"),
                        ));
                    }
                    tags.push((handle, prefix));
                }
                TokenKind::ReservedDirective => {
                    self.skip()?;
                }
                _ => break,
            }
        }
        self.tag_handles = tags.clone();
        Ok((version, tags))
    }

    fn parse_document_content(&mut self) -> Res<Ev<'a>> {
        let tok = self.peek()?;
        match tok.kind {
            TokenKind::VersionDirective(..)
            | TokenKind::TagDirective(..)
            | TokenKind::ReservedDirective
            | TokenKind::DocumentStart
            | TokenKind::DocumentEnd
            | TokenKind::StreamEnd => {
                let m = tok.start;
                self.state = self.pop_state();
                Ok(empty_scalar(m))
            }
            _ => self.parse_node(true, false),
        }
    }

    fn parse_document_end(&mut self) -> Res<Ev<'a>> {
        let tok = self.peek()?;
        let start = tok.start;
        let mut end = start;
        let explicit = tok.kind == TokenKind::DocumentEnd;
        if explicit {
            end = self.skip()?.end;
        }
        self.prev_doc_end_explicit = explicit;
        self.tag_handles.clear();
        self.state = State::DocumentStart;
        Ok((Event::DocumentEnd { explicit }, Span::new(start, end)))
    }

    // ------------------------------------------------------------------
    // Nodes
    // ------------------------------------------------------------------

    fn resolve_tag(&self, handle: Cow<'a, str>, suffix: Cow<'a, str>, mark: Mark) -> Res<Tag<'a>> {
        if handle.is_empty() {
            let resolved = percent_decode(suffix.clone(), mark)?;
            return Ok(Tag {
                handle,
                suffix,
                resolved,
            });
        }
        if handle == "!" && suffix.is_empty() {
            return Ok(Tag {
                handle,
                suffix,
                resolved: Cow::Borrowed("!"),
            });
        }
        let prefix: Cow<'a, str> = match self.tag_handles.iter().find(|(h, _)| *h == handle) {
            Some((_, p)) => p.clone(),
            None if handle == "!" => Cow::Borrowed("!"),
            None if handle == "!!" => Cow::Borrowed("tag:yaml.org,2002:"),
            None => {
                return Err(err(
                    mark,
                    format!(
                        "undefined tag handle `{handle}` (declare it with %TAG in this document)"
                    ),
                ));
            }
        };
        let full = Cow::Owned(format!("{prefix}{suffix}"));
        let resolved = percent_decode(full, mark)?;
        Ok(Tag {
            handle,
            suffix,
            resolved,
        })
    }

    fn parse_node(&mut self, block: bool, indentless_sequence: bool) -> Res<Ev<'a>> {
        if matches!(self.peek_kind()?, TokenKind::Alias(_)) {
            let t = self.skip()?;
            let TokenKind::Alias(name) = t.kind else {
                unreachable!()
            };
            self.state = self.pop_state();
            return Ok((Event::Alias { name }, Span::new(t.start, t.end)));
        }

        let start = self.peek()?.start;
        let mut end = start;
        let mut anchor: Option<Cow<'a, str>> = None;
        let mut raw_tag: Option<(Cow<'a, str>, Cow<'a, str>, Mark)> = None;
        loop {
            match self.peek_kind()? {
                TokenKind::Anchor(_) => {
                    let t = self.skip()?;
                    if anchor.is_some() {
                        return Err(err(t.start, "a node can have only one anchor"));
                    }
                    let TokenKind::Anchor(name) = t.kind else {
                        unreachable!()
                    };
                    anchor = Some(name);
                    end = t.end;
                }
                TokenKind::Tag(..) => {
                    let t = self.skip()?;
                    if raw_tag.is_some() {
                        return Err(err(t.start, "a node can have only one tag"));
                    }
                    let TokenKind::Tag(h, s) = t.kind else {
                        unreachable!()
                    };
                    raw_tag = Some((h, s, t.start));
                    end = t.end;
                }
                _ => break,
            }
        }
        let tag = match raw_tag {
            Some((h, s, m)) => Some(self.resolve_tag(h, s, m)?),
            None => None,
        };
        let has_props = anchor.is_some() || tag.is_some();

        let tok = self.peek()?;
        let tok_start = tok.start;
        let tok_end = tok.end;
        match tok.kind {
            TokenKind::Alias(_) => Err(err(tok_start, "an alias can't have an anchor or a tag")),
            TokenKind::BlockEntry if indentless_sequence => {
                self.state = State::IndentlessSequenceEntry;
                Ok((
                    Event::SequenceStart {
                        anchor,
                        tag,
                        flow: false,
                    },
                    Span::new(start, tok_end),
                ))
            }
            TokenKind::Scalar(..) => {
                let t = self.skip()?;
                let TokenKind::Scalar(value, style, block) = t.kind else {
                    unreachable!()
                };
                self.state = self.pop_state();
                Ok((
                    Event::Scalar {
                        anchor,
                        tag,
                        value,
                        style,
                        block,
                    },
                    Span::new(start, t.end),
                ))
            }
            TokenKind::FlowSequenceStart => {
                self.state = State::FlowSequenceFirstEntry;
                Ok((
                    Event::SequenceStart {
                        anchor,
                        tag,
                        flow: true,
                    },
                    Span::new(start, tok_end),
                ))
            }
            TokenKind::FlowMappingStart => {
                self.state = State::FlowMappingFirstKey;
                Ok((
                    Event::MappingStart {
                        anchor,
                        tag,
                        flow: true,
                    },
                    Span::new(start, tok_end),
                ))
            }
            TokenKind::BlockSequenceStart if block => {
                self.state = State::BlockSequenceFirstEntry;
                Ok((
                    Event::SequenceStart {
                        anchor,
                        tag,
                        flow: false,
                    },
                    Span::new(start, tok_end),
                ))
            }
            TokenKind::BlockMappingStart if block => {
                self.state = State::BlockMappingFirstKey;
                Ok((
                    Event::MappingStart {
                        anchor,
                        tag,
                        flow: false,
                    },
                    Span::new(start, tok_end),
                ))
            }
            _ if has_props => {
                self.state = self.pop_state();
                Ok((
                    Event::Scalar {
                        anchor,
                        tag,
                        value: Cow::Borrowed(""),
                        style: ScalarStyle::Plain,
                        block: None,
                    },
                    Span::new(start, end),
                ))
            }
            _ => Err(err(tok_start, "expected a node here")),
        }
    }

    // ------------------------------------------------------------------
    // Block collections
    // ------------------------------------------------------------------

    fn parse_block_sequence_entry(&mut self, first: bool) -> Res<Ev<'a>> {
        if first {
            self.skip()?; // BLOCK-SEQUENCE-START
        }
        let tok = self.peek()?;
        match tok.kind {
            TokenKind::BlockEntry => {
                let mark = self.skip()?.end;
                if matches!(
                    self.peek_kind()?,
                    TokenKind::BlockEntry | TokenKind::BlockEnd
                ) {
                    self.state = State::BlockSequenceEntry;
                    Ok(empty_scalar(mark))
                } else {
                    self.states.push(State::BlockSequenceEntry);
                    self.parse_node(true, false)
                }
            }
            TokenKind::BlockEnd => {
                let t = self.skip()?;
                self.state = self.pop_state();
                Ok((Event::SequenceEnd, Span::new(t.start, t.end)))
            }
            _ => Err(err(
                tok.start,
                "expected `-` for the next block sequence entry",
            )),
        }
    }

    fn parse_indentless_sequence_entry(&mut self) -> Res<Ev<'a>> {
        let tok = self.peek()?;
        if tok.kind == TokenKind::BlockEntry {
            let mark = self.skip()?.end;
            if matches!(
                self.peek_kind()?,
                TokenKind::BlockEntry | TokenKind::Key | TokenKind::Value | TokenKind::BlockEnd
            ) {
                self.state = State::IndentlessSequenceEntry;
                Ok(empty_scalar(mark))
            } else {
                self.states.push(State::IndentlessSequenceEntry);
                self.parse_node(true, false)
            }
        } else {
            let m = tok.start;
            self.state = self.pop_state();
            Ok((Event::SequenceEnd, Span::new(m, m)))
        }
    }

    fn parse_block_mapping_key(&mut self, first: bool) -> Res<Ev<'a>> {
        if first {
            self.skip()?; // BLOCK-MAPPING-START
        }
        let tok = self.peek()?;
        match tok.kind {
            TokenKind::Key => {
                let mark = self.skip()?.end;
                if matches!(
                    self.peek_kind()?,
                    TokenKind::Key | TokenKind::Value | TokenKind::BlockEnd
                ) {
                    self.state = State::BlockMappingValue;
                    Ok(empty_scalar(mark))
                } else {
                    self.states.push(State::BlockMappingValue);
                    self.parse_node(true, true)
                }
            }
            // An empty key: `: value`.
            TokenKind::Value => {
                let m = tok.start;
                self.state = State::BlockMappingValue;
                Ok(empty_scalar(m))
            }
            TokenKind::BlockEnd => {
                let t = self.skip()?;
                self.state = self.pop_state();
                Ok((Event::MappingEnd, Span::new(t.start, t.end)))
            }
            _ => Err(err(tok.start, "expected a mapping key here")),
        }
    }

    fn parse_block_mapping_value(&mut self) -> Res<Ev<'a>> {
        let tok = self.peek()?;
        if tok.kind == TokenKind::Value {
            let mark = self.skip()?.end;
            if matches!(
                self.peek_kind()?,
                TokenKind::Key | TokenKind::Value | TokenKind::BlockEnd
            ) {
                self.state = State::BlockMappingKey;
                Ok(empty_scalar(mark))
            } else {
                self.states.push(State::BlockMappingKey);
                self.parse_node(true, true)
            }
        } else {
            let m = tok.start;
            self.state = State::BlockMappingKey;
            Ok(empty_scalar(m))
        }
    }

    // ------------------------------------------------------------------
    // Flow collections
    // ------------------------------------------------------------------

    fn parse_flow_sequence_entry(&mut self, first: bool) -> Res<Ev<'a>> {
        if first {
            self.skip()?; // FLOW-SEQUENCE-START
        }
        if *self.peek_kind()? != TokenKind::FlowSequenceEnd {
            if !first {
                let tok = self.peek()?;
                if tok.kind == TokenKind::FlowEntry {
                    self.skip()?;
                } else {
                    return Err(err(tok.start, "expected `,` or `]` in flow sequence"));
                }
            }
            let tok = self.peek()?;
            let (s, e) = (tok.start, tok.end);
            match tok.kind {
                TokenKind::Key => {
                    self.skip()?;
                    self.state = State::FlowSequenceEntryMappingKey;
                    return Ok((
                        Event::MappingStart {
                            anchor: None,
                            tag: None,
                            flow: true,
                        },
                        Span::new(s, e),
                    ));
                }
                // A single pair with an empty key: `[: v]`.
                TokenKind::Value => {
                    self.state = State::FlowSequenceEntryMappingKey;
                    return Ok((
                        Event::MappingStart {
                            anchor: None,
                            tag: None,
                            flow: true,
                        },
                        Span::new(s, s),
                    ));
                }
                TokenKind::FlowSequenceEnd => {}
                _ => {
                    self.states.push(State::FlowSequenceEntry);
                    return self.parse_node(false, false);
                }
            }
        }
        let t = self.skip()?; // FLOW-SEQUENCE-END
        self.state = self.pop_state();
        Ok((Event::SequenceEnd, Span::new(t.start, t.end)))
    }

    fn parse_flow_sequence_entry_mapping_key(&mut self) -> Res<Ev<'a>> {
        let tok = self.peek()?;
        if matches!(
            tok.kind,
            TokenKind::Value | TokenKind::FlowEntry | TokenKind::FlowSequenceEnd
        ) {
            let m = tok.start;
            self.state = State::FlowSequenceEntryMappingValue;
            Ok(empty_scalar(m))
        } else {
            self.states.push(State::FlowSequenceEntryMappingValue);
            self.parse_node(false, false)
        }
    }

    fn parse_flow_sequence_entry_mapping_value(&mut self) -> Res<Ev<'a>> {
        let tok = self.peek()?;
        if tok.kind == TokenKind::Value {
            self.skip()?;
            if !matches!(
                self.peek_kind()?,
                TokenKind::FlowEntry | TokenKind::FlowSequenceEnd
            ) {
                self.states.push(State::FlowSequenceEntryMappingEnd);
                return self.parse_node(false, false);
            }
        }
        let m = self.peek()?.start;
        self.state = State::FlowSequenceEntryMappingEnd;
        Ok(empty_scalar(m))
    }

    fn parse_flow_mapping_key(&mut self, first: bool) -> Res<Ev<'a>> {
        if first {
            self.skip()?; // FLOW-MAPPING-START
        }
        if *self.peek_kind()? != TokenKind::FlowMappingEnd {
            if !first {
                let tok = self.peek()?;
                if tok.kind == TokenKind::FlowEntry {
                    self.skip()?;
                } else {
                    return Err(err(tok.start, "expected `,` or `}` in flow mapping"));
                }
            }
            let tok = self.peek()?;
            match tok.kind {
                TokenKind::Key => {
                    self.skip()?;
                    let tok = self.peek()?;
                    if matches!(
                        tok.kind,
                        TokenKind::Value | TokenKind::FlowEntry | TokenKind::FlowMappingEnd
                    ) {
                        let m = tok.start;
                        self.state = State::FlowMappingValue;
                        return Ok(empty_scalar(m));
                    }
                    self.states.push(State::FlowMappingValue);
                    return self.parse_node(false, false);
                }
                // An empty key: `{: v}`.
                TokenKind::Value => {
                    let m = tok.start;
                    self.state = State::FlowMappingValue;
                    return Ok(empty_scalar(m));
                }
                TokenKind::FlowMappingEnd => {}
                _ => {
                    self.states.push(State::FlowMappingEmptyValue);
                    return self.parse_node(false, false);
                }
            }
        }
        let t = self.skip()?; // FLOW-MAPPING-END
        self.state = self.pop_state();
        Ok((Event::MappingEnd, Span::new(t.start, t.end)))
    }

    fn parse_flow_mapping_value(&mut self, empty: bool) -> Res<Ev<'a>> {
        let tok = self.peek()?;
        let m = tok.start;
        if empty {
            self.state = State::FlowMappingKey;
            return Ok(empty_scalar(m));
        }
        if tok.kind == TokenKind::Value {
            self.skip()?;
            if !matches!(
                self.peek_kind()?,
                TokenKind::FlowEntry | TokenKind::FlowMappingEnd
            ) {
                self.states.push(State::FlowMappingKey);
                return self.parse_node(false, false);
            }
        }
        let m2 = self.peek()?.start;
        self.state = State::FlowMappingKey;
        Ok(empty_scalar(m2))
    }
}

/// Pull iterator over the events of a YAML stream.
pub struct Events<'a> {
    parser: Parser<'a>,
}

impl<'a> Events<'a> {
    pub fn new(src: &'a str) -> Self {
        Events {
            parser: Parser::new(src),
        }
    }

    /// Comments scanned so far; complete once the iterator is exhausted.
    pub fn comments(&self) -> &[Comment<'a>] {
        self.parser.comments()
    }

    /// Scan up to the next token (so comments before it are available).
    pub(crate) fn scan_ahead(&mut self) {
        self.parser.scan_ahead();
    }
}

impl<'a> Iterator for Events<'a> {
    type Item = Result<(Event<'a>, Span), Error>;

    fn next(&mut self) -> Option<Self::Item> {
        self.parser.next_event().transpose()
    }
}
