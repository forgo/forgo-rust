//! Cargo's requirement syntax: the rules of the `semver` crate's
//! `VersionReq` (1.0.28), which Cargo uses, without its limits (numbers
//! of any size, any number of comparators).

use crate::error::{Error, ErrorKind, Part};
use crate::ident::{self, Prerelease};
use crate::num::Num;
use crate::set::{Bound, Interval, Piece};
use crate::version::{Version, number};
use alloc::vec::Vec;
use core::fmt;

/// A comparator's operator.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Op {
    /// `=1.2.3`
    Exact,
    /// `>1.2.3`
    Greater,
    /// `>=1.2.3`
    GreaterEq,
    /// `<1.2.3`
    Less,
    /// `<=1.2.3`
    LessEq,
    /// `~1.2.3`
    Tilde,
    /// `^1.2.3`, and a bare `1.2.3`
    Caret,
    /// `1.*`, `1.2.*` (a bare version with a wildcard)
    Wildcard,
}

/// A comparator: an operator and a version that may leave out the minor
/// and patch numbers.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Comparator {
    pub(crate) op: Op,
    pub(crate) major: Num,
    pub(crate) minor: Option<Num>,
    pub(crate) patch: Option<Num>,
    pub(crate) pre: Prerelease,
}

fn skip_spaces(text: &str, mut i: usize) -> usize {
    while text.as_bytes().get(i) == Some(&b' ') {
        i += 1;
    }
    i
}

fn wildcard_at(text: &str, i: usize) -> Option<char> {
    match text.as_bytes().get(i) {
        Some(b'*') => Some('*'),
        Some(b'x') => Some('x'),
        Some(b'X') => Some('X'),
        _ => None,
    }
}

fn op_at(text: &str, i: usize) -> (Op, usize) {
    let b = &text.as_bytes()[i..];
    match b {
        [b'=', ..] => (Op::Exact, 1),
        [b'>', b'=', ..] => (Op::GreaterEq, 2),
        [b'>', ..] => (Op::Greater, 1),
        [b'<', b'=', ..] => (Op::LessEq, 2),
        [b'<', ..] => (Op::Less, 1),
        [b'~', ..] => (Op::Tilde, 1),
        [b'^', ..] => (Op::Caret, 1),
        _ => (Op::Caret, 0),
    }
}

fn char_at(text: &str, i: usize) -> char {
    text[i..].chars().next().unwrap_or('\0')
}

/// Parse a requirement: comparators separated by commas. A lone wildcard
/// (`*`, `x`, `X`) gives no comparators (any version that isn't a
/// pre-release).
pub(crate) fn parse(text: &str) -> Result<Vec<Comparator>, Error> {
    let start = skip_spaces(text, 0);
    if let Some(ch) = wildcard_at(text, start) {
        let rest = skip_spaces(text, start + 1);
        return match text.as_bytes().get(rest) {
            None => Ok(Vec::new()),
            Some(b',') => Err(Error::new(
                ErrorKind::WildcardNotTheOnlyComparator(ch),
                start,
            )),
            Some(_) => Err(Error::new(ErrorKind::UnexpectedAfterWildcard, rest)),
        };
    }
    let mut out = Vec::new();
    let mut i = start;
    loop {
        let (c, part, next) = match comparator(text, i) {
            Ok(r) => r,
            Err(e) => {
                if let Some(ch) = wildcard_at(text, i) {
                    let rest = skip_spaces(text, i + 1);
                    if matches!(text.as_bytes().get(rest), None | Some(b',')) {
                        return Err(Error::new(ErrorKind::WildcardNotTheOnlyComparator(ch), i));
                    }
                }
                return Err(e);
            }
        };
        out.push(c);
        match text.as_bytes().get(next) {
            None => return Ok(out),
            Some(b',') => i = skip_spaces(text, next + 1),
            Some(_) => {
                let _ = part;
                return Err(Error::new(
                    ErrorKind::ExpectedComma(char_at(text, next)),
                    next,
                ));
            }
        }
    }
}

/// One comparator from `start`: it, the last part parsed, and where the
/// text after it (and its trailing spaces) starts.
fn comparator(text: &str, start: usize) -> Result<(Comparator, Part, usize), Error> {
    let (mut op, len) = op_at(text, start);
    let default_op = len == 0;
    let mut i = skip_spaces(text, start + len);
    let mut part = Part::Major;
    let major = number(text, &mut i, Part::Major)?;
    let mut has_wildcard = false;
    let mut minor = None;
    if text.as_bytes().get(i) == Some(&b'.') {
        i += 1;
        part = Part::Minor;
        if wildcard_at(text, i).is_some() {
            has_wildcard = true;
            if default_op {
                op = Op::Wildcard;
            }
            i += 1;
        } else {
            minor = Some(number(text, &mut i, Part::Minor)?);
        }
    }
    let mut patch = None;
    if text.as_bytes().get(i) == Some(&b'.') {
        i += 1;
        part = Part::Patch;
        if wildcard_at(text, i).is_some() {
            if default_op {
                op = Op::Wildcard;
            }
            i += 1;
        } else if has_wildcard {
            return Err(Error::new(ErrorKind::UnexpectedAfterWildcard, i));
        } else {
            patch = Some(number(text, &mut i, Part::Patch)?);
        }
    }
    let mut pre = Prerelease::EMPTY;
    if patch.is_some() && text.as_bytes().get(i) == Some(&b'-') {
        part = Part::Pre;
        let end = ident::scan(text, i + 1, Part::Pre)?;
        pre = Prerelease::from_valid(&text[i + 1..end]);
        i = end;
    }
    if patch.is_some() && text.as_bytes().get(i) == Some(&b'+') {
        // Build metadata is allowed, and ignored.
        part = Part::Build;
        i = ident::scan(text, i + 1, Part::Build)?;
    }
    let i = skip_spaces(text, i);
    Ok((
        Comparator {
            op,
            major,
            minor,
            patch,
            pre,
        },
        part,
        i,
    ))
}

// ---------------------------------------------------------------------------
// Matching (the `semver` crate's eval.rs)
// ---------------------------------------------------------------------------

/// Whether `v` matches every comparator, and is either a release or a
/// pre-release of a `major.minor.patch` that a comparator with a
/// pre-release names.
pub(crate) fn matches(comparators: &[Comparator], v: &Version) -> bool {
    if !comparators.iter().all(|c| matches_op(c, v)) {
        return false;
    }
    v.pre.is_empty() || comparators.iter().any(|c| pre_is_compatible(c, v))
}

fn pre_is_compatible(c: &Comparator, v: &Version) -> bool {
    c.major == v.major
        && c.minor.as_ref() == Some(&v.minor)
        && c.patch.as_ref() == Some(&v.patch)
        && !c.pre.is_empty()
}

fn matches_op(c: &Comparator, v: &Version) -> bool {
    match c.op {
        Op::Exact | Op::Wildcard => exact(c, v),
        Op::Greater => greater(c, v),
        Op::GreaterEq => exact(c, v) || greater(c, v),
        Op::Less => less(c, v),
        Op::LessEq => exact(c, v) || less(c, v),
        Op::Tilde => tilde(c, v),
        Op::Caret => caret(c, v),
    }
}

fn exact(c: &Comparator, v: &Version) -> bool {
    v.major == c.major
        && c.minor.as_ref().is_none_or(|m| v.minor == *m)
        && c.patch.as_ref().is_none_or(|p| v.patch == *p)
        && v.pre == c.pre
}

fn greater(c: &Comparator, v: &Version) -> bool {
    if v.major != c.major {
        return v.major > c.major;
    }
    let Some(minor) = &c.minor else { return false };
    if v.minor != *minor {
        return v.minor > *minor;
    }
    let Some(patch) = &c.patch else { return false };
    if v.patch != *patch {
        return v.patch > *patch;
    }
    v.pre > c.pre
}

fn less(c: &Comparator, v: &Version) -> bool {
    if v.major != c.major {
        return v.major < c.major;
    }
    let Some(minor) = &c.minor else { return false };
    if v.minor != *minor {
        return v.minor < *minor;
    }
    let Some(patch) = &c.patch else { return false };
    if v.patch != *patch {
        return v.patch < *patch;
    }
    v.pre < c.pre
}

fn tilde(c: &Comparator, v: &Version) -> bool {
    if v.major != c.major {
        return false;
    }
    if let Some(minor) = &c.minor
        && v.minor != *minor
    {
        return false;
    }
    if let Some(patch) = &c.patch
        && v.patch != *patch
    {
        return v.patch > *patch;
    }
    v.pre >= c.pre
}

fn caret(c: &Comparator, v: &Version) -> bool {
    if v.major != c.major {
        return false;
    }
    let Some(minor) = &c.minor else { return true };
    let Some(patch) = &c.patch else {
        return if !c.major.is_zero() {
            v.minor >= *minor
        } else {
            v.minor == *minor
        };
    };
    if !c.major.is_zero() {
        if v.minor != *minor {
            return v.minor > *minor;
        } else if v.patch != *patch {
            return v.patch > *patch;
        }
    } else if !minor.is_zero() {
        if v.minor != *minor {
            return false;
        } else if v.patch != *patch {
            return v.patch > *patch;
        }
    } else if v.minor != *minor || v.patch != *patch {
        return false;
    }
    v.pre >= c.pre
}

// ---------------------------------------------------------------------------
// The set of versions a requirement matches
// ---------------------------------------------------------------------------

/// `major.minor.patch-0`: the lowest version with that core.
fn lowest(major: &Num, minor: &Num, patch: &Num) -> Version {
    Version {
        major: major.clone(),
        minor: minor.clone(),
        patch: patch.clone(),
        pre: Prerelease::from_valid("0"),
        build: crate::ident::BuildMetadata::EMPTY,
    }
}

fn release(major: &Num, minor: &Num, patch: &Num) -> Version {
    Version {
        major: major.clone(),
        minor: minor.clone(),
        patch: patch.clone(),
        pre: Prerelease::EMPTY,
        build: crate::ident::BuildMetadata::EMPTY,
    }
}

impl Comparator {
    /// The full version, if minor and patch are given.
    fn full(&self) -> Option<Version> {
        let (minor, patch) = (self.minor.as_ref()?, self.patch.as_ref()?);
        Some(Version {
            pre: self.pre.clone(),
            ..release(&self.major, minor, patch)
        })
    }

    /// The versions this comparator matches, as an interval for releases
    /// and one for pre-releases (`matches_op`, worked out per operator).
    pub(crate) fn intervals(&self) -> (Interval, Interval) {
        let z = Num::ZERO;
        let (major, minor) = (&self.major, self.minor.as_ref());
        // The lowest version whose (major, minor) or major is above the
        // given part: `M.(m+1).0-0` or `(M+1).0.0-0`.
        let next_lowest = || match minor {
            Some(m) => lowest(major, &m.succ(), &z),
            None => lowest(&major.succ(), &z, &z),
        };
        // The lowest version starting the given part: `M.m.0-0` or `M.0.0-0`.
        let this_lowest = || lowest(major, minor.unwrap_or(&z), &z);
        // The first release of the given part: `M.m.0` or `M.0.0`.
        let this_release = || release(major, minor.unwrap_or(&z), &z);
        let both = |i: Interval| (i.clone(), i);
        let none = Interval::EMPTY;
        if let Some(v) = self.full() {
            return match self.op {
                Op::Exact | Op::Wildcard => {
                    let point = Interval::new(Bound::Incl(v.clone()), Bound::Incl(v.clone()));
                    if v.pre.is_empty() {
                        (point, none)
                    } else {
                        (none, point)
                    }
                }
                Op::Greater => both(Interval::new(Bound::Excl(v), Bound::Unbounded)),
                Op::GreaterEq => both(Interval::new(Bound::Incl(v), Bound::Unbounded)),
                Op::Less => both(Interval::new(Bound::Unbounded, Bound::Excl(v))),
                Op::LessEq => both(Interval::new(Bound::Unbounded, Bound::Incl(v))),
                Op::Tilde => {
                    let m = self.minor.as_ref().expect("full");
                    both(Interval::new(
                        Bound::Incl(v),
                        Bound::Excl(lowest(major, &m.succ(), &z)),
                    ))
                }
                Op::Caret => {
                    let (m, p) = (
                        self.minor.as_ref().expect("full"),
                        self.patch.as_ref().expect("full"),
                    );
                    let hi = if !major.is_zero() {
                        lowest(&major.succ(), &z, &z)
                    } else if !m.is_zero() {
                        lowest(major, &m.succ(), &z)
                    } else {
                        lowest(major, m, &p.succ())
                    };
                    both(Interval::new(Bound::Incl(v), Bound::Excl(hi)))
                }
            };
        }
        // A partial version: only `M` or `M.m`.
        let block = Interval::new(Bound::Incl(this_release()), Bound::Excl(next_lowest()));
        match self.op {
            // Releases of the block only (`ver.pre == EMPTY`).
            Op::Exact | Op::Wildcard | Op::Tilde => (block, none),
            Op::Greater => both(Interval::new(Bound::Incl(next_lowest()), Bound::Unbounded)),
            Op::GreaterEq => (
                Interval::new(Bound::Incl(this_release()), Bound::Unbounded),
                Interval::new(Bound::Incl(next_lowest()), Bound::Unbounded),
            ),
            Op::Less => both(Interval::new(Bound::Unbounded, Bound::Excl(this_lowest()))),
            Op::LessEq => (
                Interval::new(Bound::Unbounded, Bound::Excl(next_lowest())),
                Interval::new(Bound::Unbounded, Bound::Excl(this_lowest())),
            ),
            Op::Caret => {
                let hi = match minor {
                    None => lowest(&major.succ(), &z, &z),
                    Some(m) if major.is_zero() => lowest(major, &m.succ(), &z),
                    Some(_) => lowest(&major.succ(), &z, &z),
                };
                both(Interval::new(Bound::Incl(this_lowest()), Bound::Excl(hi)))
            }
        }
    }
}

/// The versions a requirement matches, as one piece of the set model.
pub(crate) fn piece(comparators: &[Comparator]) -> Piece {
    let mut rel = Interval::ALL;
    let mut pre = Interval::ALL;
    let mut tuples = Vec::new();
    for c in comparators {
        let (r, p) = c.intervals();
        rel = rel.intersect(&r);
        pre = pre.intersect(&p);
        if !c.pre.is_empty()
            && let Some(v) = c.full()
        {
            tuples.push(v.core());
        }
    }
    Piece::new(rel, pre, Some(tuples))
}

// ---------------------------------------------------------------------------
// Display (the `semver` crate's format)
// ---------------------------------------------------------------------------

impl fmt::Display for Comparator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self.op {
            Op::Exact => "=",
            Op::Greater => ">",
            Op::GreaterEq => ">=",
            Op::Less => "<",
            Op::LessEq => "<=",
            Op::Tilde => "~",
            Op::Caret => "^",
            Op::Wildcard => "",
        })?;
        write!(f, "{}", self.major)?;
        match &self.minor {
            Some(minor) => {
                write!(f, ".{minor}")?;
                match &self.patch {
                    Some(patch) => {
                        write!(f, ".{patch}")?;
                        if !self.pre.is_empty() {
                            write!(f, "-{}", self.pre.as_str())?;
                        }
                    }
                    None if self.op == Op::Wildcard => f.write_str(".*")?,
                    None => {}
                }
            }
            None if self.op == Op::Wildcard => f.write_str(".*")?,
            None => {}
        }
        Ok(())
    }
}

/// The `semver` crate's text for a requirement: comparators joined by
/// `, `, or `*` when there are none.
pub(crate) fn display(comparators: &[Comparator], f: &mut fmt::Formatter<'_>) -> fmt::Result {
    if comparators.is_empty() {
        return f.write_str("*");
    }
    for (i, c) in comparators.iter().enumerate() {
        if i > 0 {
            f.write_str(", ")?;
        }
        write!(f, "{c}")?;
    }
    Ok(())
}
