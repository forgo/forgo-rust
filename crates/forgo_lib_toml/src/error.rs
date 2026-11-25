//! [`Error`]: why text isn't valid TOML, and where.

use std::fmt;

/// Why text isn't valid TOML: what is wrong ([`Error::kind`],
/// [`Error::message`]) and where ([`Error::line`], [`Error::column`],
/// [`Error::offset`]).
///
/// ```
/// use forgo_lib_toml::{Document, ErrorKind};
///
/// let err = Document::parse("a = 1\na = 2\n").unwrap_err();
/// assert_eq!(err.kind(), ErrorKind::Duplicate);
/// assert_eq!((err.line(), err.column()), (2, 1));
/// assert_eq!(err.to_string(), "the key `a` is already defined at line 2, column 1");
/// ```
#[derive(Clone, PartialEq, Eq)]
pub struct Error(Box<Inner>);

// Boxed, so results stay small on the parser's hot paths.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Inner {
    kind: ErrorKind,
    message: String,
    offset: usize,
    line: usize,
    column: usize,
}

/// The kind of problem; see [`Error`].
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ErrorKind {
    /// Unexpected text or end of input.
    Syntax,
    /// A string with a bad escape, a control character, or no end.
    String,
    /// A malformed number, or one out of range.
    Number,
    /// A malformed or impossible date or time.
    Datetime,
    /// A key defined twice.
    Duplicate,
    /// A table defined twice, or extended where TOML doesn't allow it.
    Table,
    /// Arrays and inline tables nested deeper than the limit.
    Depth,
    /// A document of 4 GiB or more (positions are 32-bit).
    TooLarge,
    /// Input that isn't UTF-8.
    Encoding,
}

impl Error {
    /// An error at byte `offset` of `src`.
    pub(crate) fn at(
        src: &str,
        offset: usize,
        kind: ErrorKind,
        message: impl Into<String>,
    ) -> Error {
        let offset = offset.min(src.len());
        let before = &src.as_bytes()[..offset];
        let line = 1 + before.iter().filter(|&&b| b == b'\n').count();
        let line_start = before
            .iter()
            .rposition(|&b| b == b'\n')
            .map_or(0, |i| i + 1);
        let column = 1 + src[line_start..offset].chars().count();
        Error(Box::new(Inner {
            kind,
            message: message.into(),
            offset,
            line,
            column,
        }))
    }

    /// The kind of problem.
    pub fn kind(&self) -> ErrorKind {
        self.0.kind
    }

    /// What is wrong, without the position.
    pub fn message(&self) -> &str {
        &self.0.message
    }

    /// The byte offset where the problem was found.
    pub fn offset(&self) -> usize {
        self.0.offset
    }

    /// The line (from 1).
    pub fn line(&self) -> usize {
        self.0.line
    }

    /// The column (from 1, in characters).
    pub fn column(&self) -> usize {
        self.0.column
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} at line {}, column {}",
            self.0.message, self.0.line, self.0.column
        )
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Error")
            .field("kind", &self.0.kind)
            .field("message", &self.0.message)
            .field("offset", &self.0.offset)
            .field("line", &self.0.line)
            .field("column", &self.0.column)
            .finish()
    }
}

impl std::error::Error for Error {}
