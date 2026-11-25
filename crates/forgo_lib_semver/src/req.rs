//! [`Req`]: a version requirement, in Cargo's syntax or npm's.

use crate::cargo;
use crate::error::Error;
use crate::npm::{self, NpmOptions};
use crate::set::{self, Piece};
use crate::version::Version;
use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use core::cmp::Ordering;
use core::fmt;

/// A version requirement: which versions a dependency may use.
///
/// Two syntaxes are supported, each with its own matching rules:
///
/// | | Cargo ([`Req::cargo`]) | npm ([`Req::npm`]) |
/// |---|---|---|
/// | a bare `1.2.3` | `^1.2.3` | `=1.2.3` |
/// | several comparators | `>=1.2, <1.5` (comma: all must match) | `>=1.2 <1.5` (space: all must match) |
/// | alternatives | — | `1.x \|\| >=2.5` |
/// | ranges | — | `1.2.3 - 2.3.4` |
/// | wildcards | `*`, `1.*`, `1.2.x` | `*`, `1.x`, `1.2.*`, `""` |
/// | `v` prefix, spaces after operators | no | yes |
///
/// In both, a pre-release version matches only if a comparator with a
/// pre-release names its `major.minor.patch` (`>=1.2.3-rc.1` lets in
/// `1.2.3-rc.2`, not `1.2.4-rc.1`), unless npm's `include_prerelease`
/// option is set.
///
/// Set operations are exact, over the versions each requirement matches,
/// including across syntaxes: [`intersects`](Req::intersects),
/// [`is_subset`](Req::is_subset), [`is_equivalent`](Req::is_equivalent),
/// [`min_version`](Req::min_version).
///
/// ```
/// use forgo_lib_semver::{Req, Version};
///
/// let cargo = Req::cargo("1.2")?; // ^1.2: >=1.2.0, <2.0.0
/// let npm = Req::npm(">=1.2.0 <2.0.0-0")?;
/// assert!(cargo.matches(&Version::parse("1.9.3")?));
/// assert!(!cargo.matches(&Version::parse("1.9.3-rc.1")?));
/// assert!(cargo.is_equivalent(&npm));
/// assert_eq!(npm.to_string(), ">=1.2.0 <2.0.0-0");
/// # Ok::<(), forgo_lib_semver::Error>(())
/// ```
#[derive(Clone)]
pub struct Req {
    kind: Kind,
    text: Box<str>,
}

/// Equal when parsed the same (in the same syntax): `>=1.0` equals
/// `>= 1.0`, and `*` equals `x`. For the same versions, see
/// [`Req::is_equivalent`].
impl PartialEq for Req {
    fn eq(&self, other: &Req) -> bool {
        self.kind == other.kind
    }
}

impl Eq for Req {}

impl core::hash::Hash for Req {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.kind.hash(state);
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
enum Kind {
    Cargo(Vec<cargo::Comparator>),
    Npm(npm::Range),
}

/// The syntax a [`Req`] was written in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Dialect {
    /// Cargo's (the `semver` crate's `VersionReq`).
    Cargo,
    /// npm's (node-semver's ranges).
    Npm,
}

impl Req {
    /// Parse a requirement in Cargo's syntax: comparators (`=`, `>`,
    /// `>=`, `<`, `<=`, `~`, `^`, or none for `^`) separated by commas, or
    /// a lone wildcard.
    ///
    /// Unlike the `semver` crate, numbers may have any size and there may
    /// be any number of comparators.
    ///
    /// ```
    /// use forgo_lib_semver::Req;
    ///
    /// let r = Req::cargo(">= 1.2.0, < 1.8")?;
    /// assert_eq!(r.to_string(), ">=1.2.0, <1.8");
    /// assert!(Req::cargo("1.2.3 || 2").is_err()); // `||` is npm's
    /// # Ok::<(), forgo_lib_semver::Error>(())
    /// ```
    pub fn cargo(text: &str) -> Result<Req, Error> {
        Ok(Req {
            kind: Kind::Cargo(cargo::parse(text)?),
            text: text.into(),
        })
    }

    /// Parse a requirement in npm's syntax, as node-semver does without
    /// options.
    ///
    /// ```
    /// use forgo_lib_semver::Req;
    ///
    /// let r = Req::npm("^1.2.3 || 2.x")?;
    /// assert_eq!(r.to_string(), ">=1.2.3 <2.0.0-0||>=2.0.0 <3.0.0-0");
    /// # Ok::<(), forgo_lib_semver::Error>(())
    /// ```
    pub fn npm(text: &str) -> Result<Req, Error> {
        Req::npm_with(text, NpmOptions::default())
    }

    /// Parse a requirement in npm's syntax with node-semver's options.
    pub fn npm_with(text: &str, options: NpmOptions) -> Result<Req, Error> {
        let range = npm::Range::parse(text, options.internal())?;
        Ok(Req {
            kind: Kind::Npm(range),
            text: text.into(),
        })
    }

    /// The syntax this requirement was parsed with.
    pub fn dialect(&self) -> Dialect {
        match self.kind {
            Kind::Cargo(_) => Dialect::Cargo,
            Kind::Npm(_) => Dialect::Npm,
        }
    }

    /// The requirement as it was written.
    pub fn as_str(&self) -> &str {
        &self.text
    }

    /// Whether `version` meets the requirement (build metadata doesn't
    /// matter).
    pub fn matches(&self, version: &Version) -> bool {
        match &self.kind {
            Kind::Cargo(c) => cargo::matches(c, version),
            Kind::Npm(r) => r.test(version),
        }
    }

    /// The highest version (by precedence) that meets the requirement;
    /// the first of equal ones.
    ///
    /// ```
    /// use forgo_lib_semver::{Req, Version};
    ///
    /// let published: Vec<Version> = ["1.2.0", "1.4.1", "2.0.0"]
    ///     .iter()
    ///     .map(|v| v.parse().unwrap())
    ///     .collect();
    /// let best = Req::cargo("^1.2")?.max_satisfying(&published);
    /// assert_eq!(best.map(|v| v.to_string()).as_deref(), Some("1.4.1"));
    /// # Ok::<(), forgo_lib_semver::Error>(())
    /// ```
    pub fn max_satisfying<'v, I>(&self, versions: I) -> Option<&'v Version>
    where
        I: IntoIterator<Item = &'v Version>,
    {
        self.extreme(versions, Ordering::Greater)
    }

    /// The lowest version (by precedence) that meets the requirement; the
    /// first of equal ones.
    pub fn min_satisfying<'v, I>(&self, versions: I) -> Option<&'v Version>
    where
        I: IntoIterator<Item = &'v Version>,
    {
        self.extreme(versions, Ordering::Less)
    }

    fn extreme<'v, I>(&self, versions: I, better: Ordering) -> Option<&'v Version>
    where
        I: IntoIterator<Item = &'v Version>,
    {
        let mut best: Option<&'v Version> = None;
        for v in versions {
            if self.matches(v) && best.is_none_or(|b| v.cmp_precedence(b) == better) {
                best = Some(v);
            }
        }
        best
    }

    fn pieces(&self) -> Vec<Piece> {
        match &self.kind {
            Kind::Cargo(c) => vec![cargo::piece(c)],
            Kind::Npm(r) => r.pieces(),
        }
    }

    /// The lowest version that meets the requirement, if any.
    ///
    /// ```
    /// use forgo_lib_semver::Req;
    ///
    /// let min = |r: &Req| r.min_version().map(|v| v.to_string());
    /// assert_eq!(min(&Req::cargo(">1.2.3")?).as_deref(), Some("1.2.4"));
    /// assert_eq!(min(&Req::npm(">1.2.3-rc.1")?).as_deref(), Some("1.2.3-rc.1.0"));
    /// assert_eq!(min(&Req::npm(">2 <1")?), None);
    /// # Ok::<(), forgo_lib_semver::Error>(())
    /// ```
    pub fn min_version(&self) -> Option<Version> {
        set::min(&self.pieces())
    }

    /// Whether no version meets the requirement (`>2 <1`).
    pub fn is_unsatisfiable(&self) -> bool {
        self.min_version().is_none()
    }

    /// The lowest version that meets both requirements, if any.
    ///
    /// ```
    /// use forgo_lib_semver::Req;
    ///
    /// let v = Req::cargo("^1.2")?.min_intersection(&Req::npm(">=1.4.0-rc.1 || 2")?);
    /// assert_eq!(v.map(|v| v.to_string()).as_deref(), Some("1.4.0"));
    /// # Ok::<(), forgo_lib_semver::Error>(())
    /// ```
    pub fn min_intersection(&self, other: &Req) -> Option<Version> {
        set::min_intersection(&self.pieces(), &other.pieces())
    }

    /// The lowest version that meets this requirement but not `other`: a
    /// counterexample to [`is_subset`](Req::is_subset).
    ///
    /// ```
    /// use forgo_lib_semver::Req;
    ///
    /// let v = Req::npm("^1.2.3-rc.1")?.min_difference(&Req::npm(">=1.2.3")?);
    /// assert_eq!(v.map(|v| v.to_string()).as_deref(), Some("1.2.3-rc.1"));
    /// # Ok::<(), forgo_lib_semver::Error>(())
    /// ```
    pub fn min_difference(&self, other: &Req) -> Option<Version> {
        set::min_difference(&self.pieces(), &other.pieces())
    }

    /// Whether some version meets both requirements.
    ///
    /// ```
    /// use forgo_lib_semver::Req;
    ///
    /// assert!(Req::cargo("^1.2")?.intersects(&Req::npm("1.5.x || >=3")?));
    /// assert!(!Req::cargo("^1.2")?.intersects(&Req::cargo(">=2")?));
    /// # Ok::<(), forgo_lib_semver::Error>(())
    /// ```
    pub fn intersects(&self, other: &Req) -> bool {
        self.min_intersection(other).is_some()
    }

    /// Whether every version that meets this requirement meets `other`.
    ///
    /// Exact, unlike node-semver's `subset`, which requires each `||`
    /// alternative to fit in a single alternative of `other`:
    ///
    /// ```
    /// use forgo_lib_semver::{Req, npm::{self, NpmOptions}};
    ///
    /// let (sub, dom) = (">=1.0.0 <3.0.0", ">=1.0.0 <2.0.0 || >=2.0.0 <3.0.0");
    /// assert!(Req::npm(sub)?.is_subset(&Req::npm(dom)?));
    /// assert!(!npm::subset(sub, dom, NpmOptions::default())?);
    /// # Ok::<(), forgo_lib_semver::Error>(())
    /// ```
    pub fn is_subset(&self, other: &Req) -> bool {
        self.min_difference(other).is_none()
    }

    /// Whether the two requirements are met by exactly the same versions.
    pub fn is_equivalent(&self, other: &Req) -> bool {
        let (a, b) = (self.pieces(), other.pieces());
        set::min_difference(&a, &b).is_none() && set::min_difference(&b, &a).is_none()
    }

    /// An equivalent requirement without the comparators and `||`
    /// alternatives that don't change which versions it matches.
    ///
    /// ```
    /// use forgo_lib_semver::Req;
    ///
    /// assert_eq!(Req::cargo(">=1.0, >=1.2.0, <2")?.simplify().to_string(), ">=1.2.0, <2");
    /// assert_eq!(Req::npm("1.2.x || 1.2.3 || ^1.0.0")?.simplify().to_string(), ">=1.0.0 <2.0.0-0");
    /// # Ok::<(), forgo_lib_semver::Error>(())
    /// ```
    pub fn simplify(&self) -> Req {
        let target = self.pieces();
        let equivalent = |pieces: &[Piece]| {
            set::min_difference(pieces, &target).is_none()
                && set::min_difference(&target, pieces).is_none()
        };
        match &self.kind {
            Kind::Cargo(comparators) => {
                let mut keep = comparators.clone();
                let mut i = 0;
                while i < keep.len() {
                    let mut without = keep.clone();
                    without.remove(i);
                    if equivalent(&[cargo::piece(&without)]) {
                        keep = without;
                    } else {
                        i += 1;
                    }
                }
                let text = alloc::format!("{}", CargoDisplay(&keep));
                Req {
                    kind: Kind::Cargo(keep),
                    text: text.into(),
                }
            }
            Kind::Npm(range) => {
                let mut r = range.clone();
                // Alternatives covered by the others.
                let mut i = 0;
                while r.sets.len() > 1 && i < r.sets.len() {
                    let mut without = r.clone();
                    without.sets.remove(i);
                    if equivalent(&without.pieces()) {
                        r = without;
                    } else {
                        i += 1;
                    }
                }
                // Comparators that don't narrow their alternative.
                for s in 0..r.sets.len() {
                    let mut i = 0;
                    while r.sets[s].len() > 1 && i < r.sets[s].len() {
                        let mut without = r.clone();
                        without.sets[s].remove(i);
                        if equivalent(&without.pieces()) {
                            r = without;
                        } else {
                            i += 1;
                        }
                    }
                }
                let text = r.range_string();
                Req {
                    kind: Kind::Npm(r),
                    text: text.into(),
                }
            }
        }
    }
}

struct CargoDisplay<'a>(&'a [cargo::Comparator]);

impl fmt::Display for CargoDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        cargo::display(self.0, f)
    }
}

impl fmt::Display for Req {
    /// The requirement in its syntax's canonical form: Cargo's as the
    /// `semver` crate prints it (`>=1.2.0, <1.8`), npm's as node-semver
    /// does (`>=1.2.3 <2.0.0-0`, `*` for any version).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match &self.kind {
            Kind::Cargo(c) => alloc::format!("{}", CargoDisplay(c)),
            Kind::Npm(r) => {
                let s = r.range_string();
                if s.is_empty() { String::from("*") } else { s }
            }
        };
        f.pad(&text)
    }
}

impl fmt::Debug for Req {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Req::{}({:?})",
            match self.dialect() {
                Dialect::Cargo => "cargo",
                Dialect::Npm => "npm",
            },
            self.text
        )
    }
}
