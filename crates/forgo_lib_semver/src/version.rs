//! [`Version`]: a SemVer 2.0.0 version.

use crate::Release;
use crate::error::{Error, ErrorKind, Part};
use crate::ident::{self, BuildMetadata, Prerelease};
use crate::num::Num;
use core::cmp::Ordering;
use core::fmt;
use core::str::FromStr;

/// A version: `MAJOR.MINOR.PATCH`, an optional `-pre-release` and an
/// optional `+build` ([SemVer 2.0.0](https://semver.org/spec/v2.0.0.html)).
///
/// [`Version::parse`] accepts exactly the grammar of the specification:
/// no `v` prefix, no surrounding whitespace, no leading zeros in numbers
/// or numeric pre-release identifiers, and numbers of any size.
/// [`Version::parse_lenient`] accepts the forms npm does.
///
/// Two orders apply:
///
/// - **Precedence** (SemVer §11, [`Version::cmp_precedence`]) ignores
///   build metadata: `1.0.0+a` and `1.0.0+b` have the same precedence.
/// - **`Ord`** is precedence, then build metadata to break ties
///   ([`BuildMetadata`]), so it agrees with `==` (which compares
///   everything) and sorting is deterministic.
///
/// ```
/// use forgo_lib_semver::Version;
///
/// let v = Version::parse("1.0.0-rc.1+build.5")?;
/// assert_eq!(v.major, 1);
/// assert_eq!(v.pre.as_str(), "rc.1");
/// assert_eq!(v.build.as_str(), "build.5");
/// assert!(v < Version::parse("1.0.0")?);
/// assert!(Version::parse("v1.0.0").is_err());
/// assert!(Version::parse("18446744073709551616.0.0").is_ok());
/// # Ok::<(), forgo_lib_semver::Error>(())
/// ```
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Version {
    /// The major version: incompatible changes.
    pub major: Num,
    /// The minor version: compatible additions.
    pub minor: Num,
    /// The patch version: compatible fixes.
    pub patch: Num,
    /// The pre-release (empty for a release).
    pub pre: Prerelease,
    /// The build metadata (empty when there is none).
    pub build: BuildMetadata,
}

impl Version {
    /// `major.minor.patch`, without pre-release or build metadata.
    pub const fn new(major: u64, minor: u64, patch: u64) -> Version {
        Version {
            major: Num::new(major),
            minor: Num::new(minor),
            patch: Num::new(patch),
            pre: Prerelease::EMPTY,
            build: BuildMetadata::EMPTY,
        }
    }

    /// Parse a version, exactly as SemVer 2.0.0 defines it.
    ///
    /// ```
    /// use forgo_lib_semver::Version;
    ///
    /// assert!(Version::parse("1.2.3-alpha.01").is_err()); // leading zero
    /// assert!(Version::parse("1.2.3+build.01").is_ok()); // fine in build metadata
    /// assert!(Version::parse("1.2.3--").is_ok()); // `-` is an identifier
    /// ```
    pub fn parse(text: &str) -> Result<Version, Error> {
        parse_fast(text).map_err(|(kind, pos)| Error::new(kind, pos))
    }

    /// Parse a version the way npm does in loose mode: surrounding
    /// whitespace, `v` and `=` prefixes, leading zeros and a pre-release
    /// without its `-` are accepted (`" v01.2.3beta "` is `1.2.3-beta`).
    ///
    /// ```
    /// use forgo_lib_semver::Version;
    ///
    /// assert_eq!(Version::parse_lenient("v1.2.3")?.to_string(), "1.2.3");
    /// assert_eq!(Version::parse_lenient(" =1.02.3rc1 ")?.to_string(), "1.2.3-rc1");
    /// assert!(Version::parse_lenient("1.2").is_err());
    /// # Ok::<(), forgo_lib_semver::Error>(())
    /// ```
    pub fn parse_lenient(text: &str) -> Result<Version, Error> {
        crate::npm::parse(text, crate::npm::NpmOptions::LOOSE).ok_or_else(|| {
            Error::with_detail(
                ErrorKind::InvalidNpm,
                0,
                "not a version, even loosely (it needs major.minor.patch)",
            )
        })
    }

    /// Find a version in free text, as npm's `coerce` does: the first run
    /// of up to three dot-separated numbers, with missing parts zero.
    ///
    /// ```
    /// use forgo_lib_semver::Version;
    ///
    /// let c = |s| Version::coerce(s).map(|v| v.to_string());
    /// assert_eq!(c("v2").as_deref(), Some("2.0.0"));
    /// assert_eq!(c("rust 1.88 stable").as_deref(), Some("1.88.0"));
    /// assert_eq!(c("1.2.3.4").as_deref(), Some("1.2.3"));
    /// assert_eq!(c("no digits"), None);
    /// ```
    pub fn coerce(text: &str) -> Option<Version> {
        crate::npm::coerce(text, crate::npm::CoerceOptions::default())
    }

    /// The next version for a kind of release, following npm's `inc`
    /// (SemVer §7 and §8: lower numbers reset to zero). Build metadata is
    /// dropped.
    ///
    /// A pre-release moves to its own release first: `major` of
    /// `2.0.0-rc.1` is `2.0.0`, `patch` of `1.2.4-rc.1` is `1.2.4`.
    ///
    /// ```
    /// use forgo_lib_semver::{Bump, Version};
    ///
    /// let v = Version::parse("1.2.3")?;
    /// assert_eq!(v.bump(Bump::Minor)?.to_string(), "1.3.0");
    /// assert_eq!(v.bump(Bump::Premajor(Some("rc")))?.to_string(), "2.0.0-rc.0");
    /// let rc = Version::parse("2.0.0-rc.0")?;
    /// assert_eq!(rc.bump(Bump::Prerelease(Some("rc")))?.to_string(), "2.0.0-rc.1");
    /// assert_eq!(rc.bump(Bump::Release)?.to_string(), "2.0.0");
    /// assert!(v.bump(Bump::Release).is_err()); // not a pre-release
    /// # Ok::<(), forgo_lib_semver::Error>(())
    /// ```
    pub fn bump(&self, bump: Bump<'_>) -> Result<Version, Error> {
        let (release, identifier) = match bump {
            Bump::Major => ("major", None),
            Bump::Minor => ("minor", None),
            Bump::Patch => ("patch", None),
            Bump::Premajor(id) => ("premajor", id),
            Bump::Preminor(id) => ("preminor", id),
            Bump::Prepatch(id) => ("prepatch", id),
            Bump::Prerelease(id) => ("prerelease", id),
            Bump::Release => ("release", None),
        };
        let invalid = |why: &str| Error::with_detail(ErrorKind::InvalidArgument, 0, why);
        if let Some(id) = identifier
            && (id.is_empty() || Prerelease::new(id).is_err())
        {
            return Err(invalid("the pre-release identifier isn't valid"));
        }
        if bump == Bump::Release && self.pre.is_empty() {
            return Err(invalid("the version isn't a pre-release"));
        }
        crate::npm::inc_version(self, release, false, identifier, crate::npm::IdBase::Zero)
            .ok()
            .flatten()
            .ok_or_else(|| invalid("this bump doesn't apply to the version"))
    }

    /// The kind of release between two versions, as npm's `diff` reports
    /// it (`None` if they have the same precedence).
    ///
    /// ```
    /// use forgo_lib_semver::{Release, Version};
    ///
    /// let d = |a: &str, b: &str| Version::parse(a).unwrap().diff(&Version::parse(b).unwrap());
    /// assert_eq!(d("1.2.3", "1.3.0"), Some(Release::Minor));
    /// assert_eq!(d("1.2.3", "2.0.0-rc.1"), Some(Release::Premajor));
    /// assert_eq!(d("1.0.0-rc.1", "1.0.0"), Some(Release::Major));
    /// assert_eq!(d("1.2.3+a", "1.2.3+b"), None);
    /// ```
    pub fn diff(&self, other: &Version) -> Option<Release> {
        crate::npm::diff_versions(self, other).map(Release::of_diff)
    }

    /// Whether this is a pre-release (has pre-release identifiers).
    #[inline]
    pub fn is_prerelease(&self) -> bool {
        !self.pre.is_empty()
    }

    /// Compare by precedence (SemVer §11): major, minor and patch
    /// numerically, then pre-release; build metadata is ignored.
    ///
    /// ```
    /// use forgo_lib_semver::Version;
    /// use std::cmp::Ordering;
    ///
    /// let a = Version::parse("1.0.0+a")?;
    /// let b = Version::parse("1.0.0+b")?;
    /// assert_eq!(a.cmp_precedence(&b), Ordering::Equal);
    /// assert_ne!(a, b);
    /// # Ok::<(), forgo_lib_semver::Error>(())
    /// ```
    #[inline]
    pub fn cmp_precedence(&self, other: &Version) -> Ordering {
        self.major
            .cmp(&other.major)
            .then_with(|| self.minor.cmp(&other.minor))
            .then_with(|| self.patch.cmp(&other.patch))
            .then_with(|| self.pre.cmp(&other.pre))
    }

    /// Whether the two versions have the same precedence (they differ at
    /// most in build metadata).
    #[inline]
    pub fn precedence_eq(&self, other: &Version) -> bool {
        self.major == other.major
            && self.minor == other.minor
            && self.patch == other.patch
            && self.pre == other.pre
    }

    /// Whether `major.minor.patch` are equal.
    #[inline]
    pub(crate) fn same_core(&self, other: &Version) -> bool {
        self.major == other.major && self.minor == other.minor && self.patch == other.patch
    }

    /// The version without pre-release and build metadata.
    pub fn core(&self) -> Version {
        Version {
            major: self.major.clone(),
            minor: self.minor.clone(),
            patch: self.patch.clone(),
            pre: Prerelease::EMPTY,
            build: BuildMetadata::EMPTY,
        }
    }

    /// The version without build metadata.
    pub fn without_build(&self) -> Version {
        Version {
            build: BuildMetadata::EMPTY,
            ..self.clone()
        }
    }
}

/// What [`Version::bump`] increments; the `Option` is the pre-release
/// identifier to use (`rc` gives `-rc.0`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Bump<'a> {
    /// The next major version (or this version's own, for a pre-release
    /// of `X.0.0`).
    Major,
    /// The next minor version (or this version's own, for a pre-release
    /// of `X.Y.0`).
    Minor,
    /// The next patch version (or this version's own, for a pre-release).
    Patch,
    /// A pre-release of the next major version.
    Premajor(Option<&'a str>),
    /// A pre-release of the next minor version.
    Preminor(Option<&'a str>),
    /// A pre-release of the next patch version.
    Prepatch(Option<&'a str>),
    /// The next pre-release: the last number of the pre-release goes up
    /// (`rc.1` to `rc.2`), or a pre-release of the next patch version
    /// for a release.
    Prerelease(Option<&'a str>),
    /// The release of a pre-release (`2.0.0-rc.1` to `2.0.0`).
    Release,
}

/// A parse failure while scanning: cheap to pass around (no heap, no drop
/// glue); turned into an [`Error`] once, at the API boundary.
pub(crate) type Fail = (ErrorKind, usize);

/// `major.minor.patch` when each is at most 19 digits (so it fits a
/// `u64`) without a leading zero: the numbers and where the core ends.
/// `None` for anything else, which [`number`] then reports precisely.
#[inline]
fn core_fast(b: &[u8]) -> Option<([u64; 3], usize)> {
    let mut nums = [0u64; 3];
    let mut i = 0;
    for (k, slot) in nums.iter_mut().enumerate() {
        let start = i;
        let mut n = 0u64;
        while let Some(&c) = b.get(i) {
            if !c.is_ascii_digit() || i - start == 19 {
                break;
            }
            n = n * 10 + u64::from(c - b'0');
            i += 1;
        }
        let len = i - start;
        if len == 0 || (len > 1 && b[start] == b'0') || b.get(i).is_some_and(u8::is_ascii_digit) {
            return None;
        }
        *slot = n;
        if k < 2 {
            if b.get(i) != Some(&b'.') {
                return None;
            }
            i += 1;
        }
    }
    Some((nums, i))
}

/// [`Version::parse`], failing with a [`Fail`].
#[inline]
fn parse_fast(text: &str) -> Result<Version, Fail> {
    let b = text.as_bytes();
    if b.is_empty() {
        return Err((ErrorKind::Empty, 0));
    }
    let (mut v, mut i) = match core_fast(b) {
        Some(([major, minor, patch], i)) => (Version::new(major, minor, patch), i),
        None => {
            let mut i = 0;
            let major = number(text, &mut i, Part::Major)?;
            dot(text, &mut i, Part::Major)?;
            let minor = number(text, &mut i, Part::Minor)?;
            dot(text, &mut i, Part::Minor)?;
            let patch = number(text, &mut i, Part::Patch)?;
            let v = Version {
                major,
                minor,
                patch,
                pre: Prerelease::EMPTY,
                build: BuildMetadata::EMPTY,
            };
            (v, i)
        }
    };
    if i == b.len() {
        return Ok(v);
    }
    let mut last = Part::Patch;
    if b[i] == b'-' {
        let end = ident::scan(text, i + 1, Part::Pre).map_err(|e| (e.kind(), e.pos()))?;
        v.pre = Prerelease::from_valid(&text[i + 1..end]);
        i = end;
        last = Part::Pre;
    }
    if b.get(i) == Some(&b'+') {
        let end = ident::scan(text, i + 1, Part::Build).map_err(|e| (e.kind(), e.pos()))?;
        v.build = BuildMetadata::from_valid(&text[i + 1..end]);
        i = end;
        last = Part::Build;
    }
    match text[i..].chars().next() {
        None => Ok(v),
        Some(c) => Err((
            match last {
                // Inside identifiers: a character that can't be in one.
                Part::Pre | Part::Build => ErrorKind::UnexpectedChar(last, c),
                _ => ErrorKind::UnexpectedCharAfter(last, c),
            },
            i,
        )),
    }
}

/// A number without leading zeros, of any size.
#[inline]
pub(crate) fn number(text: &str, i: &mut usize, part: Part) -> Result<Num, Fail> {
    let b = text.as_bytes();
    let start = *i;
    let mut n = 0u64;
    let mut overflow = false;
    let mut j = start;
    while let Some(&c) = b.get(j) {
        if !c.is_ascii_digit() {
            break;
        }
        if !overflow {
            match n
                .checked_mul(10)
                .and_then(|n| n.checked_add(u64::from(c - b'0')))
            {
                Some(m) => n = m,
                None => overflow = true,
            }
        }
        j += 1;
    }
    if j == start {
        return Err(match text[start..].chars().next() {
            Some(c) => (ErrorKind::UnexpectedChar(part, c), start),
            None => (ErrorKind::UnexpectedEnd(part), start),
        });
    }
    if b[start] == b'0' && j - start > 1 {
        return Err((ErrorKind::LeadingZero(part), start));
    }
    *i = j;
    Ok(if overflow {
        Num::from_canonical_digits(&text[start..j])
    } else {
        Num::new(n)
    })
}

/// The `.` after `part`.
#[inline]
fn dot(text: &str, i: &mut usize, part: Part) -> Result<(), Fail> {
    match text.as_bytes().get(*i) {
        Some(b'.') => {
            *i += 1;
            Ok(())
        }
        None => {
            let next = if part == Part::Major {
                Part::Minor
            } else {
                Part::Patch
            };
            Err((ErrorKind::UnexpectedEnd(next), *i))
        }
        Some(_) => {
            let c = text[*i..].chars().next().unwrap_or('\0');
            Err((ErrorKind::UnexpectedCharAfter(part, c), *i))
        }
    }
}

impl FromStr for Version {
    type Err = Error;

    fn from_str(text: &str) -> Result<Version, Error> {
        Version::parse(text)
    }
}

impl Ord for Version {
    #[inline]
    fn cmp(&self, other: &Version) -> Ordering {
        self.cmp_precedence(other)
            .then_with(|| self.build.cmp(&other.build))
    }
}

impl PartialOrd for Version {
    #[inline]
    fn partial_cmp(&self, other: &Version) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Version {
    fn write_to(&self, out: &mut impl fmt::Write) -> fmt::Result {
        write!(out, "{}.{}.{}", self.major, self.minor, self.patch)?;
        if !self.pre.is_empty() {
            write!(out, "-{}", self.pre.as_str())?;
        }
        if !self.build.is_empty() {
            write!(out, "+{}", self.build.as_str())?;
        }
        Ok(())
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.width().is_some() || f.precision().is_some() {
            let mut s = alloc::string::String::new();
            self.write_to(&mut s)?;
            return f.pad(&s);
        }
        self.write_to(f)
    }
}

impl fmt::Debug for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Version({self})")
    }
}
