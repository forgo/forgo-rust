//! The set of versions a requirement matches, and exact set operations.
//!
//! A requirement (in either dialect) is a union of [`Piece`]s. A piece
//! matches:
//!
//! - releases in its release interval, and
//! - pre-releases in its pre-release interval whose `major.minor.patch`
//!   it admits: those its comparators name with a pre-release, or all
//!   (npm's `includePrerelease`).
//!
//! Every endpoint of every interval cuts the version line into finitely
//! many cells: the endpoints, and the open gaps between neighbours. Within
//! a cell every interval either contains all of it or none of it, so what
//! a requirement matches there depends only on the kind of version:
//! a release, a pre-release of a `major.minor.patch` some comparator
//! names, or a pre-release of any other. Checking each kind that occurs in
//! each cell decides intersection, inclusion and emptiness exactly, and
//! gives the lowest matching version.

use crate::ident::{BuildMetadata, Prerelease};
use crate::version::Version;
use alloc::vec::Vec;
use core::cmp::Ordering;

/// One end of an interval. Versions in bounds never have build metadata.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Bound {
    Unbounded,
    Incl(Version),
    Excl(Version),
}

/// An interval of versions (by precedence).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Interval {
    lo: Bound,
    hi: Bound,
    empty: bool,
}

impl Interval {
    pub(crate) const ALL: Interval = Interval {
        lo: Bound::Unbounded,
        hi: Bound::Unbounded,
        empty: false,
    };
    pub(crate) const EMPTY: Interval = Interval {
        lo: Bound::Unbounded,
        hi: Bound::Unbounded,
        empty: true,
    };

    pub(crate) fn new(lo: Bound, hi: Bound) -> Interval {
        Interval {
            lo,
            hi,
            empty: false,
        }
    }

    pub(crate) fn intersect(&self, other: &Interval) -> Interval {
        if self.empty || other.empty {
            return Interval::EMPTY;
        }
        let lo = match (&self.lo, &other.lo) {
            (Bound::Unbounded, b) | (b, Bound::Unbounded) => b.clone(),
            (a, b) => {
                let (x, y) = (bound_version(a), bound_version(b));
                match x.cmp_precedence(y) {
                    Ordering::Greater => a.clone(),
                    Ordering::Less => b.clone(),
                    Ordering::Equal if matches!(a, Bound::Excl(_)) => a.clone(),
                    Ordering::Equal => b.clone(),
                }
            }
        };
        let hi = match (&self.hi, &other.hi) {
            (Bound::Unbounded, b) | (b, Bound::Unbounded) => b.clone(),
            (a, b) => {
                let (x, y) = (bound_version(a), bound_version(b));
                match x.cmp_precedence(y) {
                    Ordering::Less => a.clone(),
                    Ordering::Greater => b.clone(),
                    Ordering::Equal if matches!(a, Bound::Excl(_)) => a.clone(),
                    Ordering::Equal => b.clone(),
                }
            }
        };
        Interval {
            lo,
            hi,
            empty: false,
        }
    }

    pub(crate) fn contains(&self, v: &Version) -> bool {
        if self.empty {
            return false;
        }
        let above = match &self.lo {
            Bound::Unbounded => true,
            Bound::Incl(l) => v.cmp_precedence(l) != Ordering::Less,
            Bound::Excl(l) => v.cmp_precedence(l) == Ordering::Greater,
        };
        let below = match &self.hi {
            Bound::Unbounded => true,
            Bound::Incl(h) => v.cmp_precedence(h) != Ordering::Greater,
            Bound::Excl(h) => v.cmp_precedence(h) == Ordering::Less,
        };
        above && below
    }

    /// Whether every version of `cell` is in the interval.
    fn covers(&self, cell: &Cell<'_>) -> bool {
        if self.empty {
            return false;
        }
        match cell {
            Cell::Point(e) => self.contains(e),
            Cell::Open(a, b) => {
                let lo = match &self.lo {
                    Bound::Unbounded => true,
                    Bound::Incl(l) | Bound::Excl(l) => {
                        a.is_some_and(|a| l.cmp_precedence(a) != Ordering::Greater)
                    }
                };
                let hi = match &self.hi {
                    Bound::Unbounded => true,
                    Bound::Incl(h) | Bound::Excl(h) => {
                        b.is_some_and(|b| h.cmp_precedence(b) != Ordering::Less)
                    }
                };
                lo && hi
            }
        }
    }

    fn endpoints<'a>(&'a self, out: &mut Vec<&'a Version>) {
        if self.empty {
            return;
        }
        for b in [&self.lo, &self.hi] {
            if let Bound::Incl(v) | Bound::Excl(v) = b {
                out.push(v);
            }
        }
    }
}

fn bound_version(b: &Bound) -> &Version {
    match b {
        Bound::Incl(v) | Bound::Excl(v) => v,
        Bound::Unbounded => unreachable!("bounded"),
    }
}

/// Part of a requirement's set: see the module documentation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Piece {
    rel: Interval,
    pre: Interval,
    /// The `major.minor.patch` whose pre-releases are admitted, or `None`
    /// for all.
    tuples: Option<Vec<Version>>,
}

impl Piece {
    pub(crate) fn new(rel: Interval, pre: Interval, tuples: Option<Vec<Version>>) -> Piece {
        Piece { rel, pre, tuples }
    }

    fn admits(&self, core: &Version) -> bool {
        match &self.tuples {
            None => true,
            Some(ts) => ts.iter().any(|t| t == core),
        }
    }

    fn matches(&self, kind: &Kind<'_>, cell: &Cell<'_>) -> bool {
        match kind {
            Kind::Release => self.rel.covers(cell),
            Kind::Pre(t) => self.admits(t) && self.pre.covers(cell),
            Kind::PreOther => self.tuples.is_none() && self.pre.covers(cell),
        }
    }
}

/// A cell: an endpoint, or the versions strictly between two neighbouring
/// endpoints (`None`: unbounded).
enum Cell<'a> {
    Point(&'a Version),
    Open(Option<&'a Version>, Option<&'a Version>),
}

/// A kind of version within a cell.
enum Kind<'a> {
    Release,
    /// Pre-releases of this `major.minor.patch` (one a comparator names).
    Pre(&'a Version),
    /// Pre-releases of a `major.minor.patch` no comparator names.
    PreOther,
}

fn with_pre(core: &Version, pre: &str) -> Version {
    Version {
        pre: Prerelease::from_valid(pre),
        build: BuildMetadata::EMPTY,
        ..core.clone()
    }
}

/// The lowest version with this core (`core-0`).
fn lowest(core: &Version) -> Version {
    with_pre(core, "0")
}

/// The next pre-release after `v` (a pre-release): `v.0`.
fn pre_succ(v: &Version) -> Version {
    let mut s = alloc::string::String::from(v.pre.as_str());
    s.push_str(".0");
    with_pre(&v.core(), &s)
}

fn next_patch(v: &Version) -> Version {
    Version {
        patch: v.patch.succ(),
        ..v.core()
    }
}

fn lt(a: &Version, b: Option<&Version>) -> bool {
    b.is_none_or(|b| a.cmp_precedence(b) == Ordering::Less)
}

/// The kinds of version in `cell`, each with the lowest such version.
fn kinds<'a>(cell: &Cell<'a>, tuples: &'a [Version]) -> Vec<(Kind<'a>, Version)> {
    let mut out = Vec::new();
    match cell {
        Cell::Point(e) => {
            let kind = if e.pre.is_empty() {
                Kind::Release
            } else {
                let core = e.core();
                match tuples.iter().find(|t| **t == core) {
                    Some(t) => Kind::Pre(t),
                    None => Kind::PreOther,
                }
            };
            out.push((kind, (*e).clone()));
        }
        Cell::Open(a, b) => {
            let (a, b) = (*a, *b);
            // The first release above a.
            let release = match a {
                None => Version::new(0, 0, 0),
                Some(a) if a.pre.is_empty() => next_patch(a),
                Some(a) => a.core(),
            };
            if lt(&release, b) {
                out.push((Kind::Release, release));
            }
            // The first pre-release of each named core above a.
            for t in tuples {
                let first = match a {
                    None => Some(lowest(t)),
                    Some(a) => match a.core().cmp_precedence(t) {
                        Ordering::Less => Some(lowest(t)),
                        Ordering::Equal if !a.pre.is_empty() => Some(pre_succ(a)),
                        _ => None,
                    },
                };
                if let Some(first) = first
                    && lt(&first, b)
                {
                    out.push((Kind::Pre(t), first));
                }
            }
            // The first pre-release above a of a core no comparator names.
            let (mut core, mut first) = match a {
                None => (Version::new(0, 0, 0), lowest(&Version::new(0, 0, 0))),
                Some(a) if a.pre.is_empty() => {
                    let c = next_patch(a);
                    let l = lowest(&c);
                    (c, l)
                }
                Some(a) => (a.core(), pre_succ(a)),
            };
            for _ in 0..=tuples.len() {
                if !lt(&first, b) {
                    break;
                }
                if !tuples.contains(&core) {
                    out.push((Kind::PreOther, first));
                    break;
                }
                core = next_patch(&core);
                first = lowest(&core);
            }
        }
    }
    out
}

/// Visit every (cell, kind) of the version line cut at the endpoints of
/// `reqs`, in increasing order of the kind's lowest version within each
/// cell. Stops when `f` returns `true`.
fn visit(reqs: &[&[Piece]], mut f: impl FnMut(&Cell<'_>, &Kind<'_>, &Version) -> bool) -> bool {
    let mut points: Vec<&Version> = Vec::new();
    let mut tuples: Vec<Version> = Vec::new();
    for pieces in reqs {
        for p in *pieces {
            p.rel.endpoints(&mut points);
            p.pre.endpoints(&mut points);
            if let Some(ts) = &p.tuples {
                tuples.extend(ts.iter().cloned());
            }
        }
    }
    points.sort_by(|a, b| a.cmp_precedence(b));
    points.dedup_by(|a, b| a.precedence_eq(b));
    tuples.sort_by(|a, b| a.cmp_precedence(b));
    tuples.dedup();

    let mut cells = Vec::with_capacity(2 * points.len() + 1);
    let mut prev = None;
    for &p in &points {
        cells.push(Cell::Open(prev, Some(p)));
        cells.push(Cell::Point(p));
        prev = Some(p);
    }
    cells.push(Cell::Open(prev, None));

    for cell in &cells {
        let mut ks = kinds(cell, &tuples);
        ks.sort_by(|x, y| x.1.cmp_precedence(&y.1));
        for (kind, low) in &ks {
            if f(cell, kind, low) {
                return true;
            }
        }
    }
    false
}

fn matches(pieces: &[Piece], cell: &Cell<'_>, kind: &Kind<'_>) -> bool {
    pieces.iter().any(|p| p.matches(kind, cell))
}

/// The lowest version where `test(in_a, in_b)` holds, `in_a` and `in_b`
/// saying whether `a` and `b` match it.
fn first(a: &[Piece], b: &[Piece], test: impl Fn(bool, bool) -> bool) -> Option<Version> {
    let mut found = None;
    visit(&[a, b], |cell, kind, low| {
        if test(matches(a, cell, kind), matches(b, cell, kind)) {
            found = Some(low.clone());
            true
        } else {
            false
        }
    });
    found
}

/// The lowest version in both.
pub(crate) fn min_intersection(a: &[Piece], b: &[Piece]) -> Option<Version> {
    first(a, b, |x, y| x && y)
}

/// The lowest version in `a` but not `b`.
pub(crate) fn min_difference(a: &[Piece], b: &[Piece]) -> Option<Version> {
    first(a, b, |x, y| x && !y)
}

/// The lowest version in the set.
pub(crate) fn min(a: &[Piece]) -> Option<Version> {
    first(a, &[], |x, _| x)
}
