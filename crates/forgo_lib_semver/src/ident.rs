//! Pre-release and build metadata identifiers.

use crate::error::{Error, ErrorKind, Part};
use alloc::boxed::Box;
use core::cmp::Ordering;
use core::fmt;
use core::str::FromStr;

/// The pre-release of a version: `alpha.1` in `1.0.0-alpha.1`. Empty for
/// a release.
///
/// Identifiers are compared as SemVer §11 says: numbers numerically,
/// others in ASCII order, numbers before others, and a shorter list before
/// a longer one it starts. An empty pre-release (a release) compares
/// greater than any other.
///
/// ```
/// use forgo_lib_semver::Prerelease;
///
/// let a: Prerelease = "alpha.2".parse()?;
/// let b: Prerelease = "alpha.10".parse()?;
/// assert!(a < b);
/// assert!(b < Prerelease::EMPTY);
/// # Ok::<(), forgo_lib_semver::Error>(())
/// ```
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Prerelease(Text);

/// The build metadata of a version: `exp.sha.5114f85` in
/// `1.0.0+exp.sha.5114f85`. Empty when there is none.
///
/// Build metadata doesn't affect precedence (SemVer §10). It still has an
/// order, used only to break ties so that sorting is deterministic:
/// empty first, then identifier by identifier, numbers before others,
/// numbers by value and then by length (`1 < 01`), others in ASCII order.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct BuildMetadata(Text);

/// Identifiers as text: up to 8 bytes inline (padded with NUL, which
/// identifiers never contain), longer ones on the heap. Text of 8 bytes or
/// less is always inline, so the derived `==` and `Hash` compare the text.
#[derive(Clone, PartialEq, Eq, Hash)]
enum Text {
    Inline([u8; 8]),
    Heap(Box<str>),
}

impl Text {
    const EMPTY: Text = Text::Inline([0; 8]);

    fn new(s: &str) -> Text {
        if s.len() <= 8 {
            let mut b = [0u8; 8];
            b[..s.len()].copy_from_slice(s.as_bytes());
            Text::Inline(b)
        } else {
            Text::Heap(s.into())
        }
    }

    #[inline]
    fn as_bytes(&self) -> &[u8] {
        match self {
            Text::Inline(b) => {
                let n = b.iter().position(|&c| c == 0).unwrap_or(8);
                &b[..n]
            }
            Text::Heap(s) => s.as_bytes(),
        }
    }

    fn as_str(&self) -> &str {
        core::str::from_utf8(self.as_bytes()).expect("identifiers are ASCII")
    }

    #[inline]
    fn is_empty(&self) -> bool {
        matches!(self, Text::Inline(b) if b[0] == 0)
    }
}

/// One identifier of a [`Prerelease`] or [`BuildMetadata`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Identifier<'a> {
    /// Digits only. Pre-releases compare these numerically.
    Numeric(&'a str),
    /// Contains a letter or `-`. Compared in ASCII order.
    Alphanumeric(&'a str),
}

impl<'a> Identifier<'a> {
    fn of(s: &'a str) -> Identifier<'a> {
        if is_numeric(s) {
            Identifier::Numeric(s)
        } else {
            Identifier::Alphanumeric(s)
        }
    }

    /// The identifier's text.
    pub fn as_str(&self) -> &'a str {
        match self {
            Identifier::Numeric(s) | Identifier::Alphanumeric(s) => s,
        }
    }

    /// Whether it is digits only.
    pub fn is_numeric(&self) -> bool {
        matches!(self, Identifier::Numeric(_))
    }
}

/// The identifiers of a [`Prerelease`] or [`BuildMetadata`], in order.
#[derive(Clone, Debug)]
pub struct Identifiers<'a>(Option<core::str::Split<'a, char>>);

impl<'a> Iterator for Identifiers<'a> {
    type Item = Identifier<'a>;

    fn next(&mut self) -> Option<Identifier<'a>> {
        self.0.as_mut()?.next().map(Identifier::of)
    }
}

pub(crate) fn is_numeric(s: &str) -> bool {
    s.bytes().all(|b| b.is_ascii_digit())
}

fn is_ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'-'
}

/// Scan dot-separated identifiers from `start`: the end of the last one.
/// Stops at the first byte that can't continue them; the caller checks
/// what that byte is.
pub(crate) fn scan(text: &str, start: usize, part: Part) -> Result<usize, Error> {
    let b = text.as_bytes();
    let mut i = start;
    loop {
        let id = i;
        let mut digits = true;
        while let Some(&c) = b.get(i) {
            if !is_ident_byte(c) {
                break;
            }
            digits &= c.is_ascii_digit();
            i += 1;
        }
        if i == id {
            return Err(Error::new(ErrorKind::EmptyIdentifier(part), i));
        }
        if part == Part::Pre && digits && i - id > 1 && b[id] == b'0' {
            return Err(Error::new(ErrorKind::LeadingZero(part), id));
        }
        if b.get(i) != Some(&b'.') {
            return Ok(i);
        }
        i += 1;
    }
}

/// Parse a whole identifier list (`text` is all identifiers).
fn parse_all(text: &str, part: Part) -> Result<Text, Error> {
    if text.is_empty() {
        return Ok(Text::EMPTY);
    }
    let end = scan(text, 0, part)?;
    match text[end..].chars().next() {
        None => Ok(Text::new(text)),
        Some(c) => Err(Error::new(ErrorKind::UnexpectedChar(part, c), end)),
    }
}

fn numeric(s: &[u8]) -> bool {
    s.iter().all(u8::is_ascii_digit)
}

fn cmp_pre_ident(a: &[u8], b: &[u8]) -> Ordering {
    match (numeric(a), numeric(b)) {
        // No leading zeros: a longer number is larger.
        (true, true) => a.len().cmp(&b.len()).then_with(|| a.cmp(b)),
        (true, false) => Ordering::Less,
        (false, true) => Ordering::Greater,
        (false, false) => a.cmp(b),
    }
}

fn cmp_build_ident(a: &[u8], b: &[u8]) -> Ordering {
    match (numeric(a), numeric(b)) {
        (true, true) => {
            let (x, y) = (trim_zeros(a), trim_zeros(b));
            x.len()
                .cmp(&y.len())
                .then_with(|| x.cmp(y))
                .then_with(|| a.len().cmp(&b.len()))
        }
        (true, false) => Ordering::Less,
        (false, true) => Ordering::Greater,
        (false, false) => a.cmp(b),
    }
}

fn trim_zeros(s: &[u8]) -> &[u8] {
    let z = s.iter().take_while(|&&c| c == b'0').count();
    &s[z..]
}

/// Compare identifier lists one identifier at a time; a list that is a
/// prefix of the other is less.
fn cmp_lists(a: &[u8], b: &[u8], ident: fn(&[u8], &[u8]) -> Ordering) -> Ordering {
    if a == b {
        return Ordering::Equal;
    }
    let (mut x, mut y) = (a.split(|&c| c == b'.'), b.split(|&c| c == b'.'));
    loop {
        match (x.next(), y.next()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(p), Some(q)) => match ident(p, q) {
                Ordering::Equal => {}
                o => return o,
            },
        }
    }
}

macro_rules! identifier_list {
    ($t:ident, $part:expr) => {
        impl $t {
            /// None: the empty list.
            pub const EMPTY: $t = $t(Text::EMPTY);

            /// Parse dot-separated identifiers (without the leading `-` or
            /// `+`). The empty string gives [`Self::EMPTY`].
            pub fn new(text: &str) -> Result<$t, Error> {
                parse_all(text, $part).map($t)
            }

            /// Identifiers already checked by the caller.
            pub(crate) fn from_valid(text: &str) -> $t {
                $t(Text::new(text))
            }

            /// Whether there are no identifiers.
            #[inline]
            pub fn is_empty(&self) -> bool {
                self.0.is_empty()
            }

            /// The identifiers as written, joined by dots (`""` when empty).
            pub fn as_str(&self) -> &str {
                self.0.as_str()
            }

            /// [`Self::as_str`] as bytes, without checking UTF-8.
            #[inline]
            #[allow(dead_code)] // generated for both lists; used for one
            pub(crate) fn bytes(&self) -> &[u8] {
                self.0.as_bytes()
            }

            /// The identifiers, in order.
            pub fn identifiers(&self) -> Identifiers<'_> {
                Identifiers((!self.is_empty()).then(|| self.as_str().split('.')))
            }
        }

        impl Default for $t {
            fn default() -> $t {
                $t::EMPTY
            }
        }

        impl FromStr for $t {
            type Err = Error;

            fn from_str(text: &str) -> Result<$t, Error> {
                $t::new(text)
            }
        }

        impl fmt::Display for $t {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.pad(self.as_str())
            }
        }

        impl fmt::Debug for $t {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}({:?})", stringify!($t), self.as_str())
            }
        }

        impl PartialOrd for $t {
            fn partial_cmp(&self, other: &$t) -> Option<Ordering> {
                Some(self.cmp(other))
            }
        }
    };
}

identifier_list!(Prerelease, Part::Pre);
identifier_list!(BuildMetadata, Part::Build);

impl Ord for Prerelease {
    #[inline]
    fn cmp(&self, other: &Prerelease) -> Ordering {
        match (self.is_empty(), other.is_empty()) {
            (true, true) => Ordering::Equal,
            // A release is greater than any pre-release of it.
            (true, false) => Ordering::Greater,
            (false, true) => Ordering::Less,
            (false, false) => cmp_lists(self.0.as_bytes(), other.0.as_bytes(), cmp_pre_ident),
        }
    }
}

impl Ord for BuildMetadata {
    #[inline]
    fn cmp(&self, other: &BuildMetadata) -> Ordering {
        match (self.is_empty(), other.is_empty()) {
            (true, true) => Ordering::Equal,
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
            (false, false) => cmp_lists(self.0.as_bytes(), other.0.as_bytes(), cmp_build_ident),
        }
    }
}
