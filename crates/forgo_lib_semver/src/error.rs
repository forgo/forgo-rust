//! [`Error`]: why text isn't a valid version or requirement.

use alloc::boxed::Box;
use core::fmt;

/// Why text isn't a valid version, pre-release, build metadata or
/// requirement: what is wrong ([`Error::kind`]) and where
/// ([`Error::pos`], a byte offset into the input).
///
/// ```
/// use forgo_lib_semver::{ErrorKind, Part, Version};
///
/// let err = Version::parse("1.02.3").unwrap_err();
/// assert_eq!(err.kind(), ErrorKind::LeadingZero(Part::Minor));
/// assert_eq!(err.pos(), 2);
/// assert_eq!(err.to_string(), "leading zero in the minor version at byte 2");
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error {
    kind: ErrorKind,
    pos: usize,
    detail: Option<Box<str>>,
}

/// What is wrong; see [`Error`].
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ErrorKind {
    /// The input is empty.
    Empty,
    /// The input ends where more was expected.
    UnexpectedEnd(Part),
    /// A character that can't appear in this part.
    UnexpectedChar(Part, char),
    /// A character that can't follow this (complete) part.
    UnexpectedCharAfter(Part, char),
    /// A number with a leading zero, such as `01` (allowed only in build
    /// metadata).
    LeadingZero(Part),
    /// An empty identifier: `1.0.0-`, `1.0.0-a..b`, `1.0.0+`.
    EmptyIdentifier(Part),
    /// Cargo: a number after a wildcard, as in `1.*.3`.
    UnexpectedAfterWildcard,
    /// Cargo: a lone wildcard (`*`, `x`) combined with other comparators.
    WildcardNotTheOnlyComparator(char),
    /// Cargo: two comparators without a comma between them.
    ExpectedComma(char),
    /// npm: text that isn't a valid comparator, range or identifier
    /// (the detail, shown in the message, says which).
    InvalidNpm,
    /// An argument that doesn't apply, such as bumping the pre-release of
    /// a version that has none (the detail says which).
    InvalidArgument,
}

/// The part of a version or requirement where an [`Error`] is.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Part {
    /// The major version number.
    Major,
    /// The minor version number.
    Minor,
    /// The patch version number.
    Patch,
    /// The pre-release identifiers.
    Pre,
    /// The build metadata identifiers.
    Build,
}

impl Error {
    pub(crate) fn new(kind: ErrorKind, pos: usize) -> Error {
        Error {
            kind,
            pos,
            detail: None,
        }
    }

    pub(crate) fn with_detail(kind: ErrorKind, pos: usize, detail: &str) -> Error {
        Error {
            kind,
            pos,
            detail: Some(detail.into()),
        }
    }

    /// What is wrong.
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }

    /// The byte offset in the input where the problem was found.
    pub fn pos(&self) -> usize {
        self.pos
    }
}

impl fmt::Display for Part {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Part::Major => "the major version",
            Part::Minor => "the minor version",
            Part::Patch => "the patch version",
            Part::Pre => "the pre-release",
            Part::Build => "the build metadata",
        })
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            ErrorKind::Empty => f.write_str("empty input")?,
            ErrorKind::UnexpectedEnd(part) => write!(f, "missing {part}")?,
            ErrorKind::UnexpectedChar(part, c) => {
                write!(f, "unexpected character {c:?} in {part}")?
            }
            ErrorKind::UnexpectedCharAfter(part, c) => {
                write!(f, "unexpected character {c:?} after {part}")?;
            }
            ErrorKind::LeadingZero(Part::Pre) => {
                f.write_str("leading zero in a numeric pre-release identifier")?;
            }
            ErrorKind::LeadingZero(part) => write!(f, "leading zero in {part}")?,
            ErrorKind::EmptyIdentifier(part) => write!(f, "empty identifier in {part}")?,
            ErrorKind::UnexpectedAfterWildcard => {
                f.write_str("a number can't follow a wildcard (`1.*.3`)")?;
            }
            ErrorKind::WildcardNotTheOnlyComparator(c) => {
                write!(f, "the wildcard {c:?} must be the only comparator")?;
            }
            ErrorKind::ExpectedComma(c) => {
                write!(f, "expected a comma between comparators, found {c:?}")?;
            }
            ErrorKind::InvalidNpm | ErrorKind::InvalidArgument => {
                f.write_str(self.detail.as_deref().unwrap_or("invalid input"))?;
                return Ok(());
            }
        }
        if let Some(d) = &self.detail {
            write!(f, " ({d})")?;
        }
        write!(f, " at byte {}", self.pos)
    }
}

impl core::error::Error for Error {}
