//! YAML 1.2.2 syntax: input decoding, scanning and event parsing.
//!
//! [`events`] turns a `&str` into a stream of [`Event`]s with source spans.
//! Comments are collected on the side ([`Events::comments`]) so the
//! composer can attach them to nodes.

mod chars;
mod event;
pub mod input;
mod parser;
mod scanner;
pub mod test_event;

pub use event::{Event, Mark, ScalarStyle, Span, Tag};
pub use parser::Events;
pub use scanner::Comment;

use crate::ast::Error;

/// Parse a YAML stream into events.
pub fn events(input: &str) -> Events<'_> {
    Events::new(input)
}

/// Decode a byte stream (UTF-8/16/32, detected per spec §5.2) and parse it
/// into owned events.
pub fn events_from_bytes(bytes: &[u8]) -> Result<Vec<(Event<'static>, Span)>, Error> {
    let text = input::decode(bytes)?;
    events(&text)
        .map(|r| r.map(|(e, s)| (e.into_owned(), s)))
        .collect()
}

/// Byte ranges of the input that no token or comment covers and that hold
/// something other than white space, line breaks or byte order marks.
/// Empty for any input the scanner accepts; used to prove that the scanner
/// never silently drops content.
#[doc(hidden)]
pub fn uncovered_content(input: &str) -> Result<Vec<std::ops::Range<usize>>, Error> {
    let mut sc = scanner::Scanner::new(input);
    let mut covered: Vec<(usize, usize)> = Vec::new();
    loop {
        let t = sc.next_token()?;
        if t.end.index > t.start.index {
            covered.push((t.start.index, t.end.index));
        }
        if t.kind == scanner::TokenKind::StreamEnd {
            break;
        }
    }
    for c in &sc.comments {
        covered.push((c.start.index, c.end.index));
    }
    covered.sort_unstable();
    let mut gaps = Vec::new();
    let mut pos = 0usize;
    let bytes = input.as_bytes();
    let check = |from: usize, to: usize, gaps: &mut Vec<std::ops::Range<usize>>| {
        let gap = &input[from..to];
        let rest = gap.replace('\u{FEFF}', "");
        if rest
            .bytes()
            .any(|b| !matches!(b, b' ' | b'\t' | b'\n' | b'\r'))
        {
            gaps.push(from..to);
        }
    };
    for (s, e) in covered {
        if s > pos {
            check(pos, s, &mut gaps);
        }
        pos = pos.max(e);
    }
    if pos < bytes.len() {
        check(pos, bytes.len(), &mut gaps);
    }
    Ok(gaps)
}
