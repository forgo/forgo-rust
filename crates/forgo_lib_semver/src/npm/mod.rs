//! node-semver's API: npm's version and range syntax, exactly.
//!
//! These functions take and return what node-semver's do: strings in,
//! `None` where it returns `null`, an error where it throws. They give
//! the same results: `tests/npm.rs` replays about 448,000 results
//! recorded from node-semver 7.8.6 running its own test suite. Use them
//! when you need npm's behaviour, quirks included; [`Req::npm`] and
//! [`Version`] are the idiomatic API over the same code.
//!
//! The one deliberate difference: numbers have no size limit and versions
//! no length limit (node-semver rejects numbers above 2^53 − 1 and
//! versions over 256 characters).
//!
//! [`intersects`] and [`subset`] compute what node-semver computes, which
//! is not always the exact answer; [`Req::intersects`] and
//! [`Req::is_subset`] are exact.
//!
//! ```
//! use forgo_lib_semver::npm::{self, NpmOptions};
//!
//! let o = NpmOptions::default();
//! assert_eq!(npm::valid_range("~1.2", o).as_deref(), Some(">=1.2.0 <1.3.0-0"));
//! assert_eq!(npm::max_satisfying(&["1.2.0", "1.2.9", "1.3.0"], "~1.2", o), Some("1.2.9"));
//! assert_eq!(npm::clean(" =v1.2.3 ", o).as_deref(), Some("1.2.3"));
//! ```
//!
//! [`Req::npm`]: crate::Req::npm
//! [`Req::intersects`]: crate::Req::intersects
//! [`Req::is_subset`]: crate::Req::is_subset

// How: each function follows the node-semver code it is named after, step
// by step, with node-semver's regular expressions (`patterns`, generated
// from its source) run by an engine with JavaScript's semantics (`regex`).

pub(crate) mod patterns;
pub(crate) mod regex;
mod trim;

use crate::ident::{BuildMetadata, Prerelease, is_numeric};
use crate::num::Num;
use crate::version::Version;
use alloc::borrow::ToOwned;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;
use core::cmp::Ordering;
use patterns as p;
use regex::{Captures, TooComplex, is_js_space, replace};

/// node-semver's options.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub(crate) struct Options {
    pub(crate) loose: bool,
    pub(crate) include_prerelease: bool,
}

/// A comparator's operator (`''` and `=` are both [`Op::Eq`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Op {
    Eq,
    Gt,
    Ge,
    Lt,
    Le,
}

impl Op {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Op::Eq => "",
            Op::Gt => ">",
            Op::Ge => ">=",
            Op::Lt => "<",
            Op::Le => "<=",
        }
    }
}

/// A comparator: an operator and a version, or any version (`''`).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Comparator {
    pub(crate) op: Op,
    /// `None`: matches any version.
    pub(crate) version: Option<Version>,
}

/// A range: sets of comparators, joined by `||`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Range {
    pub(crate) sets: Vec<Vec<Comparator>>,
    pub(crate) options: Options,
}

/// Why npm text was rejected.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Invalid {
    /// `Invalid comparator: …`
    Comparator(String),
    /// `Invalid SemVer Range: …`
    Range(String),
    /// A pattern took too many steps.
    TooComplex,
}

impl From<TooComplex> for Invalid {
    fn from(_: TooComplex) -> Invalid {
        Invalid::TooComplex
    }
}

// ---------------------------------------------------------------------------
// JavaScript string operations
// ---------------------------------------------------------------------------

/// `s.trim()`.
pub(crate) fn js_trim(s: &str) -> &str {
    s.trim_matches(is_js_space)
}

/// `s.replace(/\s+/g, ' ')`.
fn collapse_spaces(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_space = false;
    for c in s.chars() {
        if is_js_space(c) {
            if !in_space {
                out.push(' ');
                in_space = true;
            }
        } else {
            out.push(c);
            in_space = false;
        }
    }
    out
}

/// `s.split(/\s+/)`.
fn split_spaces(s: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut in_space = false;
    for (i, c) in s.char_indices() {
        if is_js_space(c) {
            if !in_space {
                out.push(&s[start..i]);
                in_space = true;
            }
        } else if in_space {
            in_space = false;
            start = i;
        }
    }
    out.push(if in_space { "" } else { &s[start..] });
    out
}

/// `isX(id)`: missing, empty, `x`, `X` or `*`.
fn is_x(id: Option<&str>) -> bool {
    matches!(id, None | Some("" | "x" | "X" | "*"))
}

/// `+n + 1` for a captured number (leading zeros allowed).
fn plus_one(n: Option<&str>) -> String {
    Num::from_digits_lenient(n.unwrap_or("0"))
        .succ()
        .to_string()
}

fn cap<'t>(c: &Captures, text: &'t str, i: usize) -> Option<&'t str> {
    c.get(text, i)
}

// ---------------------------------------------------------------------------
// Versions
// ---------------------------------------------------------------------------

/// `new SemVer(text, { loose })`, without the length and size limits.
pub(crate) fn parse_version(text: &str, loose: bool) -> Result<Option<Version>, TooComplex> {
    let t = js_trim(text);
    let re = if loose { &p::LOOSE } else { &p::FULL };
    let Some(c) = re.find(t)? else {
        return Ok(None);
    };
    let num = |i| Num::from_digits_lenient(cap(&c, t, i).unwrap_or("0"));
    let pre = match cap(&c, t, 4) {
        None | Some("") => Prerelease::EMPTY,
        Some(pr) => {
            // Numeric identifiers become numbers (losing leading zeros).
            let ids: Vec<String> = pr
                .split('.')
                .map(|id| {
                    if is_numeric(id) {
                        Num::from_digits_lenient(id).to_string()
                    } else {
                        id.to_owned()
                    }
                })
                .collect();
            Prerelease::from_valid(&ids.join("."))
        }
    };
    let build = BuildMetadata::from_valid(cap(&c, t, 5).unwrap_or(""));
    Ok(Some(Version {
        major: num(1),
        minor: num(2),
        patch: num(3),
        pre,
        build,
    }))
}

/// `SemVer.version`: the version without build metadata.
fn version_str(v: &Version) -> String {
    let mut s = format!("{}.{}.{}", v.major, v.minor, v.patch);
    if !v.pre.is_empty() {
        s.push('-');
        s.push_str(v.pre.as_str());
    }
    s
}

// ---------------------------------------------------------------------------
// Comparators
// ---------------------------------------------------------------------------

impl Comparator {
    /// `new Comparator(text, { loose })`.
    pub(crate) fn parse(text: &str, loose: bool) -> Result<Option<Comparator>, TooComplex> {
        let comp = split_spaces(js_trim(text)).join(" ");
        let re = if loose {
            &p::COMPARATOR_LOOSE
        } else {
            &p::COMPARATOR
        };
        let Some(m) = re.find(&comp)? else {
            return Ok(None);
        };
        let op = match cap(&m, &comp, 1).unwrap_or("") {
            ">" => Op::Gt,
            ">=" => Op::Ge,
            "<" => Op::Lt,
            "<=" => Op::Le,
            _ => Op::Eq,
        };
        match cap(&m, &comp, 2) {
            None | Some("") => Ok(Some(Comparator { op, version: None })),
            Some(v) => Ok(parse_version(v, loose)?.map(|version| Comparator {
                op,
                version: Some(version),
            })),
        }
    }

    /// `comparator.value`: `''` for any version, else the operator and
    /// the version.
    pub(crate) fn value(&self) -> String {
        match &self.version {
            None => String::new(),
            Some(v) => format!("{}{}", self.op.as_str(), version_str(v)),
        }
    }

    /// `comparator.test(version)`.
    pub(crate) fn test(&self, v: &Version) -> bool {
        let Some(c) = &self.version else {
            return true;
        };
        let o = v.cmp_precedence(c);
        match self.op {
            Op::Eq => o == Ordering::Equal,
            Op::Gt => o == Ordering::Greater,
            Op::Ge => o != Ordering::Less,
            Op::Lt => o == Ordering::Less,
            Op::Le => o != Ordering::Greater,
        }
    }

    fn is_null_set(&self) -> bool {
        self.op == Op::Lt
            && self.version.as_ref().is_some_and(|v| {
                v.major.is_zero() && v.minor.is_zero() && v.patch.is_zero() && v.pre.as_str() == "0"
            })
    }

    fn is_any(&self) -> bool {
        self.version.is_none()
    }
}

// ---------------------------------------------------------------------------
// Ranges
// ---------------------------------------------------------------------------

impl Range {
    /// `new Range(text, options)`.
    pub(crate) fn parse(text: &str, options: Options) -> Result<Range, Invalid> {
        Range::parse_keyed(text, options).map(|(r, _)| r)
    }

    /// [`Range::parse`], with each set's key in node-semver's parse cache
    /// (`None` for a null set, which isn't cached): sets with the same key
    /// are the same object in node-semver.
    fn parse_keyed(text: &str, options: Options) -> Result<(Range, Vec<Option<String>>), Invalid> {
        let raw = collapse_spaces(js_trim(text));
        let mut sets = Vec::new();
        for part in raw.split("||") {
            let (set, key) = parse_set(js_trim(part), options)?;
            if !set.is_empty() {
                sets.push((set, key));
            }
        }
        if sets.is_empty() {
            return Err(Invalid::Range(raw));
        }
        if sets.len() > 1 {
            let first = sets[0].clone();
            sets.retain(|s| !s.0[0].is_null_set());
            if sets.is_empty() {
                sets.push(first);
            } else if sets.len() > 1
                && let Some(any) = sets.iter().find(|s| s.0.len() == 1 && s.0[0].is_any())
            {
                sets = vec![any.clone()];
            }
        }
        let (sets, keys) = sets.into_iter().unzip();
        Ok((Range { sets, options }, keys))
    }

    /// `range.test(version)`.
    pub(crate) fn test(&self, v: &Version) -> bool {
        self.sets.iter().any(|set| test_set(set, v, self.options))
    }

    /// `range.range`: comparators joined by spaces, sets by `||`.
    pub(crate) fn range_string(&self) -> String {
        let mut out = String::new();
        for (i, set) in self.sets.iter().enumerate() {
            if i > 0 {
                out.push_str("||");
            }
            for (k, c) in set.iter().enumerate() {
                if k > 0 {
                    out.push(' ');
                }
                out.push_str(&c.value());
            }
        }
        out
    }
}

impl Range {
    /// The versions the range matches, for set operations.
    pub(crate) fn pieces(&self) -> Vec<crate::set::Piece> {
        use crate::set::{Bound, Interval, Piece};
        self.sets
            .iter()
            .map(|set| {
                let mut iv = Interval::ALL;
                let mut tuples = Vec::new();
                for c in set {
                    let Some(v) = &c.version else { continue };
                    let v = v.without_build();
                    if !v.pre.is_empty() {
                        tuples.push(v.core());
                    }
                    let i = match c.op {
                        Op::Eq => Interval::new(Bound::Incl(v.clone()), Bound::Incl(v)),
                        Op::Gt => Interval::new(Bound::Excl(v), Bound::Unbounded),
                        Op::Ge => Interval::new(Bound::Incl(v), Bound::Unbounded),
                        Op::Lt => Interval::new(Bound::Unbounded, Bound::Excl(v)),
                        Op::Le => Interval::new(Bound::Unbounded, Bound::Incl(v)),
                    };
                    iv = iv.intersect(&i);
                }
                let admit = (!self.options.include_prerelease).then_some(tuples);
                Piece::new(iv.clone(), iv, admit)
            })
            .collect()
    }
}

/// `testSet`: every comparator matches, and a pre-release version is
/// allowed only by a comparator with a pre-release on the same
/// `major.minor.patch` (unless `includePrerelease`).
pub(crate) fn test_set(set: &[Comparator], v: &Version, options: Options) -> bool {
    if !set.iter().all(|c| c.test(v)) {
        return false;
    }
    if v.pre.is_empty() || options.include_prerelease {
        return true;
    }
    set.iter().any(|c| {
        c.version
            .as_ref()
            .is_some_and(|a| !a.pre.is_empty() && a.same_core(v))
    })
}

/// `Range.parseRange`: one `||` alternative, and its cache key.
fn parse_set(range: &str, options: Options) -> Result<(Vec<Comparator>, Option<String>), Invalid> {
    let loose = options.loose;
    let range = replace(&p::BUILD_STRIP, range, true, |_| String::new())?;
    let key = range.clone();
    let hyphen = if loose {
        &p::HYPHEN_RANGE_LOOSE
    } else {
        &p::HYPHEN_RANGE
    };
    let range = replace(hyphen, &range, false, |c| {
        hyphen_replace(c, &range, options.include_prerelease)
    })?;
    let range = trim::trim_comparator_whitespace(&range);
    let range = replace(&p::TILDE_TRIM, &range, true, |c| {
        format!("{}~", cap(c, &range, 1).unwrap_or(""))
    })?;
    let range = replace(&p::CARET_TRIM, &range, true, |c| {
        format!("{}^", cap(c, &range, 1).unwrap_or(""))
    })?;

    let mut parts = Vec::new();
    for comp in range.split(' ') {
        parts.push(parse_comparator(comp, options)?);
    }
    let joined = parts.join(" ");
    let mut list = Vec::new();
    for comp in split_spaces(&joined) {
        list.push(replace_gte0(comp, options)?);
    }
    if loose {
        let mut kept = Vec::new();
        for comp in list {
            if p::COMPARATOR_LOOSE.find(&comp)?.is_some() {
                kept.push(comp);
            }
        }
        list = kept;
    }

    // Every comparator is parsed before any is used, as in node-semver.
    let mut comparators = Vec::with_capacity(list.len());
    for comp in &list {
        match Comparator::parse(comp, loose)? {
            Some(c) => comparators.push(c),
            None => return Err(Invalid::Comparator(split_spaces(js_trim(comp)).join(" "))),
        }
    }
    // A `Map` keyed by value: the first position, the last comparator.
    let mut map: Vec<(String, Comparator)> = Vec::new();
    for c in comparators {
        if c.is_null_set() {
            return Ok((vec![c], None));
        }
        let value = c.value();
        match map.iter_mut().find(|(k, _)| *k == value) {
            Some(entry) => entry.1 = c,
            None => map.push((value, c)),
        }
    }
    if map.len() > 1
        && let Some(i) = map.iter().position(|(k, _)| k.is_empty())
    {
        map.remove(i);
    }
    Ok((map.into_iter().map(|(_, c)| c).collect(), Some(key)))
}

/// `parseComparator`: carets, tildes, x-ranges and stars to comparators.
fn parse_comparator(comp: &str, options: Options) -> Result<String, TooComplex> {
    let comp = replace(&p::BUILD, comp, false, |_| String::new())?;
    let comp = replace_each(js_trim(&comp), options, replace_caret)?;
    let comp = replace_each(js_trim(&comp), options, replace_tilde)?;
    let comp = replace_each(&comp, options, replace_xrange)?;
    replace_star(&comp)
}

/// `comp.split(/\s+/).map(f).join(' ')`.
fn replace_each(
    comp: &str,
    options: Options,
    f: fn(&str, Options) -> Result<String, TooComplex>,
) -> Result<String, TooComplex> {
    let mut out = Vec::new();
    for c in split_spaces(comp) {
        out.push(f(c, options)?);
    }
    Ok(out.join(" "))
}

/// `replaceTilde`: `~1.2.3` to `>=1.2.3 <1.3.0-0`.
fn replace_tilde(comp: &str, options: Options) -> Result<String, TooComplex> {
    let re = if options.loose {
        &p::TILDE_LOOSE
    } else {
        &p::TILDE
    };
    let z = if options.include_prerelease { "-0" } else { "" };
    replace(re, comp, false, |c| {
        let (maj, min, pat, pr) = (
            cap(c, comp, 1),
            cap(c, comp, 2),
            cap(c, comp, 3),
            cap(c, comp, 4),
        );
        let (mj, mn, pt) = (maj.unwrap_or(""), min.unwrap_or(""), pat.unwrap_or(""));
        if is_x(maj) {
            String::new()
        } else if is_x(min) {
            format!(">={mj}.0.0{z} <{}.0.0-0", plus_one(maj))
        } else if is_x(pat) {
            format!(">={mj}.{mn}.0{z} <{mj}.{}.0-0", plus_one(min))
        } else if let Some(pr) = pr.filter(|s| !s.is_empty()) {
            format!(">={mj}.{mn}.{pt}-{pr} <{mj}.{}.0-0", plus_one(min))
        } else {
            format!(">={mj}.{mn}.{pt} <{mj}.{}.0-0", plus_one(min))
        }
    })
}

/// `replaceCaret`: `^1.2.3` to `>=1.2.3 <2.0.0-0`.
fn replace_caret(comp: &str, options: Options) -> Result<String, TooComplex> {
    let re = if options.loose {
        &p::CARET_LOOSE
    } else {
        &p::CARET
    };
    let z = if options.include_prerelease { "-0" } else { "" };
    replace(re, comp, false, |c| {
        let (maj, min, pat, pr) = (
            cap(c, comp, 1),
            cap(c, comp, 2),
            cap(c, comp, 3),
            cap(c, comp, 4),
        );
        let (mj, mn, pt) = (maj.unwrap_or(""), min.unwrap_or(""), pat.unwrap_or(""));
        if is_x(maj) {
            String::new()
        } else if is_x(min) {
            format!(">={mj}.0.0{z} <{}.0.0-0", plus_one(maj))
        } else if is_x(pat) {
            if mj == "0" {
                format!(">={mj}.{mn}.0{z} <{mj}.{}.0-0", plus_one(min))
            } else {
                format!(">={mj}.{mn}.0{z} <{}.0.0-0", plus_one(maj))
            }
        } else if let Some(pr) = pr.filter(|s| !s.is_empty()) {
            if mj == "0" {
                if mn == "0" {
                    format!(">={mj}.{mn}.{pt}-{pr} <{mj}.{mn}.{}-0", plus_one(pat))
                } else {
                    format!(">={mj}.{mn}.{pt}-{pr} <{mj}.{}.0-0", plus_one(min))
                }
            } else {
                format!(">={mj}.{mn}.{pt}-{pr} <{}.0.0-0", plus_one(maj))
            }
        } else if mj == "0" {
            if mn == "0" {
                format!(">={mj}.{mn}.{pt} <{mj}.{mn}.{}-0", plus_one(pat))
            } else {
                format!(">={mj}.{mn}.{pt} <{mj}.{}.0-0", plus_one(min))
            }
        } else {
            format!(">={mj}.{mn}.{pt} <{}.0.0-0", plus_one(maj))
        }
    })
}

/// `replaceXRange`: `1.x`, `>=1.2`, `<=1` and the like to comparators.
fn replace_xrange(comp: &str, options: Options) -> Result<String, TooComplex> {
    let comp = js_trim(comp);
    let re = if options.loose {
        &p::XRANGE_LOOSE
    } else {
        &p::XRANGE
    };
    replace(re, comp, false, |c| {
        let ret = &comp[c.start()..c.end()];
        let (maj, min, pat) = (cap(c, comp, 2), cap(c, comp, 3), cap(c, comp, 4));
        // invalidXRangeOrder
        if (is_x(maj) && !is_x(min))
            || (is_x(min) && pat.is_some_and(|p| !p.is_empty()) && !is_x(pat))
        {
            return comp.to_owned();
        }
        let mut gtlt = cap(c, comp, 1).unwrap_or("");
        let xm_major = is_x(maj);
        let xm = xm_major || is_x(min);
        let xp = xm || is_x(pat);
        let any_x = xp;
        if gtlt == "=" && any_x {
            gtlt = "";
        }
        let mut pr = if options.include_prerelease { "-0" } else { "" };
        if xm_major {
            if gtlt == ">" || gtlt == "<" {
                "<0.0.0-0".into()
            } else {
                "*".into()
            }
        } else if !gtlt.is_empty() && any_x {
            let mut major = maj.unwrap_or("").to_owned();
            let mut minor = if xm {
                "0".to_owned()
            } else {
                min.unwrap_or("").to_owned()
            };
            let mut patch = "0".to_owned();
            if gtlt == ">" {
                gtlt = ">=";
                if xm {
                    major = plus_one(maj);
                    minor = "0".into();
                    patch = "0".into();
                } else {
                    minor = plus_one(min);
                    patch = "0".into();
                }
            } else if gtlt == "<=" {
                gtlt = "<";
                if xm {
                    major = plus_one(maj);
                } else {
                    minor = plus_one(min);
                }
            }
            if gtlt == "<" {
                pr = "-0";
            }
            format!("{gtlt}{major}.{minor}.{patch}{pr}")
        } else if xm {
            let mj = maj.unwrap_or("");
            format!(">={mj}.0.0{pr} <{}.0.0-0", plus_one(maj))
        } else if xp {
            let (mj, mn) = (maj.unwrap_or(""), min.unwrap_or(""));
            format!(">={mj}.{mn}.0{pr} <{mj}.{}.0-0", plus_one(min))
        } else {
            ret.to_owned()
        }
    })
}

/// `replaceStars`: drop the first `*` (with its operator).
fn replace_star(comp: &str) -> Result<String, TooComplex> {
    replace(&p::STAR, js_trim(comp), false, |_| String::new())
}

/// `replaceGTE0`: `>=0.0.0` (or `>=0.0.0-0` with `includePrerelease`) is
/// any version.
fn replace_gte0(comp: &str, options: Options) -> Result<String, TooComplex> {
    let re = if options.include_prerelease {
        &p::GTE0_PRE
    } else {
        &p::GTE0
    };
    replace(re, js_trim(comp), false, |_| String::new())
}

/// `hyphenReplace`: `1.2 - 3.4` to `>=1.2.0 <3.5.0-0`.
fn hyphen_replace(c: &Captures, text: &str, inc_pr: bool) -> String {
    let g = |i| cap(c, text, i);
    let z = if inc_pr { "-0" } else { "" };
    let (from, f_maj, f_min, f_pat, f_pr) = (g(1).unwrap_or(""), g(2), g(3), g(4), g(5));
    let (to, t_maj, t_min, t_pat, t_pr) = (g(7).unwrap_or(""), g(8), g(9), g(10), g(11));
    let s = |x: Option<&str>| x.unwrap_or("").to_owned();
    let from = if is_x(f_maj) {
        String::new()
    } else if is_x(f_min) {
        format!(">={}.0.0{z}", s(f_maj))
    } else if is_x(f_pat) {
        format!(">={}.{}.0{z}", s(f_maj), s(f_min))
    } else if f_pr.is_some_and(|p| !p.is_empty()) {
        format!(">={from}")
    } else {
        format!(">={from}{z}")
    };
    let to = if is_x(t_maj) {
        String::new()
    } else if is_x(t_min) {
        format!("<{}.0.0-0", plus_one(t_maj))
    } else if is_x(t_pat) {
        format!("<{}.{}.0-0", s(t_maj), plus_one(t_min))
    } else if let Some(pr) = t_pr.filter(|p| !p.is_empty()) {
        format!("<={}.{}.{}-{pr}", s(t_maj), s(t_min), s(t_pat))
    } else if inc_pr {
        format!("<{}.{}.{}-0", s(t_maj), s(t_min), plus_one(t_pat))
    } else {
        format!("<={to}")
    };
    js_trim(&format!("{from} {to}")).to_owned()
}

// ---------------------------------------------------------------------------
// Functions
// ---------------------------------------------------------------------------

/// A pre-release identifier while incrementing: a number or a string.
#[derive(Clone, Debug, PartialEq, Eq)]
enum PreId {
    Num(Num),
    Str(String),
}

impl PreId {
    fn text(&self) -> String {
        match self {
            PreId::Num(n) => n.to_string(),
            PreId::Str(s) => s.clone(),
        }
    }
}

fn pre_ids(pre: &Prerelease) -> Vec<PreId> {
    pre.identifiers()
        .map(|id| {
            if id.is_numeric() {
                PreId::Num(Num::from_digits_lenient(id.as_str()))
            } else {
                PreId::Str(id.as_str().to_owned())
            }
        })
        .collect()
}

fn join(ids: &[PreId]) -> String {
    ids.iter().map(PreId::text).collect::<Vec<_>>().join(".")
}

/// `compareIdentifiers` for one identifier (numbers before strings).
fn cmp_ident(a: &PreId, b: &PreId) -> Ordering {
    match (a, b) {
        (PreId::Num(x), PreId::Num(y)) => x.cmp(y),
        (PreId::Num(_), PreId::Str(_)) => Ordering::Less,
        (PreId::Str(_), PreId::Num(_)) => Ordering::Greater,
        (PreId::Str(x), PreId::Str(y)) => match (is_numeric(x), is_numeric(y)) {
            (true, true) => Num::from_digits_lenient(x).cmp(&Num::from_digits_lenient(y)),
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
            (false, false) => x.cmp(y),
        },
    }
}

/// `identifierBase`: absent, `'0'`, `'1'`, or `false` (no number).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum IdBase {
    Zero,
    One,
    None,
}

/// `inc(version, release, { loose }, identifier, identifierBase)`.
///
/// Returns `None` where node-semver throws (and so returns `null`).
pub(crate) fn inc_version(
    v: &Version,
    release: &str,
    loose: bool,
    identifier: Option<&str>,
    base: IdBase,
) -> Result<Option<Version>, TooComplex> {
    let identifier = identifier.filter(|s| !s.is_empty());
    if release.starts_with("pre") {
        if identifier.is_none() && base == IdBase::None {
            return Ok(None);
        }
        if let Some(id) = identifier {
            let re = if loose {
                &p::PRERELEASE_LOOSE
            } else {
                &p::PRERELEASE
            };
            let text = format!("-{id}");
            match re.find(&text)? {
                Some(c) if cap(&c, &text, 1) == Some(id) => {}
                _ => return Ok(None),
            }
        }
    }
    let mut s = State {
        major: v.major.clone(),
        minor: v.minor.clone(),
        patch: v.patch.clone(),
        pre: pre_ids(&v.pre),
    };
    if !s.inc(release, identifier, base) {
        return Ok(None);
    }
    let pre = join(&s.pre);
    // The result must be a valid version (node-semver re-parses it).
    let text = if pre.is_empty() {
        format!("{}.{}.{}", s.major, s.minor, s.patch)
    } else {
        format!("{}.{}.{}-{pre}", s.major, s.minor, s.patch)
    };
    Ok(Version::parse(&text).ok().or(parse_version(&text, true)?))
}

struct State {
    major: Num,
    minor: Num,
    patch: Num,
    pre: Vec<PreId>,
}

impl State {
    fn inc(&mut self, release: &str, identifier: Option<&str>, base: IdBase) -> bool {
        match release {
            "premajor" => {
                self.pre.clear();
                self.patch = Num::ZERO;
                self.minor = Num::ZERO;
                self.major = self.major.succ();
                self.inc("pre", identifier, base)
            }
            "preminor" => {
                self.pre.clear();
                self.patch = Num::ZERO;
                self.minor = self.minor.succ();
                self.inc("pre", identifier, base)
            }
            "prepatch" => {
                self.pre.clear();
                self.inc("patch", identifier, base) && self.inc("pre", identifier, base)
            }
            "prerelease" => {
                if self.pre.is_empty() && !self.inc("patch", identifier, base) {
                    return false;
                }
                self.inc("pre", identifier, base)
            }
            "release" => {
                if self.pre.is_empty() {
                    return false;
                }
                self.pre.clear();
                true
            }
            "major" => {
                if !self.minor.is_zero() || !self.patch.is_zero() || self.pre.is_empty() {
                    self.major = self.major.succ();
                }
                self.minor = Num::ZERO;
                self.patch = Num::ZERO;
                self.pre.clear();
                true
            }
            "minor" => {
                if !self.patch.is_zero() || self.pre.is_empty() {
                    self.minor = self.minor.succ();
                }
                self.patch = Num::ZERO;
                self.pre.clear();
                true
            }
            "patch" => {
                if self.pre.is_empty() {
                    self.patch = self.patch.succ();
                }
                self.pre.clear();
                true
            }
            "pre" => self.inc_pre(identifier, base),
            _ => false,
        }
    }

    fn inc_pre(&mut self, identifier: Option<&str>, base: IdBase) -> bool {
        let n = Num::new(u64::from(base == IdBase::One));
        if self.pre.is_empty() {
            self.pre = vec![PreId::Num(n.clone())];
        } else {
            let mut bumped = false;
            for id in self.pre.iter_mut().rev() {
                if let PreId::Num(x) = id {
                    *x = x.succ();
                    bumped = true;
                    break;
                }
            }
            if !bumped {
                if identifier == Some(join(&self.pre).as_str()) && base == IdBase::None {
                    return false;
                }
                self.pre.push(PreId::Num(n.clone()));
            }
        }
        if let Some(id) = identifier {
            let replacement = if base == IdBase::None {
                vec![PreId::Str(id.to_owned())]
            } else {
                vec![PreId::Str(id.to_owned()), PreId::Num(n)]
            };
            // isPrereleaseIdentifier(this.prerelease, identifier)
            let parts: Vec<&str> = id.split('.').collect();
            let starts_with = parts.len() <= self.pre.len()
                && parts.iter().zip(&self.pre).all(|(p, q)| {
                    let p = if is_numeric(p) {
                        PreId::Num(Num::from_digits_lenient(p))
                    } else {
                        PreId::Str((*p).to_owned())
                    };
                    cmp_ident(q, &p) == Ordering::Equal
                });
            if starts_with {
                // isNaN(this.prerelease[identifier.split('.').length])
                let is_nan = match self.pre.get(parts.len()) {
                    None => true,
                    Some(PreId::Num(_)) => false,
                    Some(PreId::Str(s)) => !is_numeric(s),
                };
                if is_nan {
                    self.pre = replacement;
                }
            } else {
                self.pre = replacement;
            }
        }
        true
    }
}

/// The kind of change between two versions (`diff`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DiffKind {
    Major,
    Premajor,
    Minor,
    Preminor,
    Patch,
    Prepatch,
    Prerelease,
}

/// `diff(a, b)`: `None` if equal (by precedence).
pub(crate) fn diff_versions(a: &Version, b: &Version) -> Option<DiffKind> {
    let comparison = a.cmp_precedence(b);
    if comparison == Ordering::Equal {
        return None;
    }
    let (high, low) = if comparison == Ordering::Greater {
        (a, b)
    } else {
        (b, a)
    };
    let high_pre = !high.pre.is_empty();
    let low_pre = !low.pre.is_empty();
    if low_pre && !high_pre {
        if low.patch.is_zero() && low.minor.is_zero() {
            return Some(DiffKind::Major);
        }
        if low.same_core(high) {
            if !low.minor.is_zero() && low.patch.is_zero() {
                return Some(DiffKind::Minor);
            }
            return Some(DiffKind::Patch);
        }
    }
    let pre = high_pre;
    if a.major != b.major {
        return Some(if pre {
            DiffKind::Premajor
        } else {
            DiffKind::Major
        });
    }
    if a.minor != b.minor {
        return Some(if pre {
            DiffKind::Preminor
        } else {
            DiffKind::Minor
        });
    }
    if a.patch != b.patch {
        return Some(if pre {
            DiffKind::Prepatch
        } else {
            DiffKind::Patch
        });
    }
    Some(DiffKind::Prerelease)
}

/// `coerce(text, { includePrerelease, rtl })`.
pub(crate) fn coerce_text(
    text: &str,
    include_prerelease: bool,
    rtl: bool,
) -> Result<Option<Version>, TooComplex> {
    let re = if include_prerelease {
        &p::COERCE_FULL
    } else {
        &p::COERCE
    };
    let m = if !rtl {
        re.find(text)?
    } else {
        // The right-most match that doesn't end where a more left-ward
        // one ends ('1.2.3.4' coerces to '2.3.4').
        let mut found: Option<Captures> = None;
        let mut from = 0;
        while from <= text.len() {
            let Some(next) = re.find_at(text, from)? else {
                break;
            };
            if found.is_some_and(|m| m.end() == text.len()) {
                break;
            }
            if found.is_none_or(|m| next.end() != m.end()) {
                found = Some(next);
            }
            let g1 = cap(&next, text, 1).map_or(0, str::len);
            let g2 = cap(&next, text, 2).map_or(0, str::len);
            from = next.start() + g1 + g2;
        }
        found
    };
    let Some(m) = m else {
        return Ok(None);
    };
    let major = cap(&m, text, 2).unwrap_or("0");
    let minor = cap(&m, text, 3).unwrap_or("0");
    let patch = cap(&m, text, 4).unwrap_or("0");
    let mut s = format!("{major}.{minor}.{patch}");
    if include_prerelease {
        if let Some(pr) = cap(&m, text, 5).filter(|s| !s.is_empty()) {
            s.push('-');
            s.push_str(pr);
        }
        if let Some(b) = cap(&m, text, 6).filter(|s| !s.is_empty()) {
            s.push('+');
            s.push_str(b);
        }
    }
    parse_version(&s, false)
}

/// `clean(text, { loose })`.
pub(crate) fn clean_text(text: &str, loose: bool) -> Result<Option<Version>, TooComplex> {
    let t = js_trim(text).trim_start_matches(['=', 'v']);
    parse_version(t, loose)
}

/// `minVersion(range)`.
pub(crate) fn min_version_of(range: &Range) -> Option<Version> {
    let zero = Version::new(0, 0, 0);
    if range.test(&zero) {
        return Some(zero);
    }
    let zero_pre = Version {
        pre: Prerelease::from_valid("0"),
        ..Version::new(0, 0, 0)
    };
    if range.test(&zero_pre) {
        return Some(zero_pre);
    }
    let mut min: Option<Version> = None;
    for set in &range.sets {
        let mut set_min: Option<Version> = None;
        for c in set {
            let Some(cv) = &c.version else { continue };
            let mut compver = cv.without_build();
            match c.op {
                Op::Gt | Op::Ge | Op::Eq => {
                    if c.op == Op::Gt {
                        if compver.pre.is_empty() {
                            compver.patch = compver.patch.succ();
                        } else {
                            let pre = format!("{}.0", compver.pre.as_str());
                            compver.pre = Prerelease::from_valid(&pre);
                        }
                    }
                    if set_min
                        .as_ref()
                        .is_none_or(|m| compver.cmp_precedence(m) == Ordering::Greater)
                    {
                        set_min = Some(compver);
                    }
                }
                Op::Lt | Op::Le => {}
            }
        }
        if let Some(s) = set_min
            && min
                .as_ref()
                .is_none_or(|m| m.cmp_precedence(&s) == Ordering::Greater)
        {
            min = Some(s);
        }
    }
    min.filter(|m| range.test(m))
}

/// `outside(version, range, hilo)`: whether `v` is above (`greater`) or
/// below every version the range matches.
pub(crate) fn outside(v: &Version, range: &Range, greater: bool) -> bool {
    if range.test(v) {
        return false;
    }
    let (comp, ecomp) = if greater {
        (Op::Gt, Op::Ge)
    } else {
        (Op::Lt, Op::Le)
    };
    // gtfn: is a further from the range's side than b?
    let gtfn = |a: &Version, b: &Version| {
        let o = a.cmp_precedence(b);
        if greater {
            o == Ordering::Greater
        } else {
            o == Ordering::Less
        }
    };
    let ltfn = |a: &Version, b: &Version| {
        let o = a.cmp_precedence(b);
        if greater {
            o == Ordering::Less
        } else {
            o == Ordering::Greater
        }
    };
    let ltefn = |a: &Version, b: &Version| !gtfn(a, b);
    let zero = Comparator {
        op: Op::Ge,
        version: Some(Version::new(0, 0, 0)),
    };
    for set in &range.sets {
        let mut high: Option<&Comparator> = None;
        let mut low: Option<&Comparator> = None;
        for c in set {
            let c = if c.version.is_none() { &zero } else { c };
            let h = *high.get_or_insert(c);
            let l = *low.get_or_insert(c);
            let cv = c.version.as_ref().expect("set above");
            if gtfn(cv, h.version.as_ref().expect("set")) {
                high = Some(c);
            } else if ltfn(cv, l.version.as_ref().expect("set")) {
                low = Some(c);
            }
        }
        let (high, low) = (high.expect("non-empty set"), low.expect("non-empty set"));
        if high.op == comp || high.op == ecomp {
            return false;
        }
        let lv = low.version.as_ref().expect("set");
        let no_op = low.op == Op::Eq;
        if ((no_op || low.op == comp) && ltefn(v, lv)) || (low.op == ecomp && ltfn(v, lv)) {
            return false;
        }
    }
    true
}

/// `comparator.intersects(other, options)`.
fn comparator_intersects(
    a: &Comparator,
    b: &Comparator,
    options: Options,
) -> Result<bool, Invalid> {
    if a.op == Op::Eq {
        let Some(av) = &a.version else {
            return Ok(true);
        };
        // new Range(b.value, options).test(a.value)
        let r = Range::parse(&b.value(), options)?;
        return Ok(match parse_version(&version_str(av), options.loose)? {
            Some(v) => r.test(&v),
            None => false,
        });
    }
    if b.op == Op::Eq {
        let Some(bv) = &b.version else {
            return Ok(true);
        };
        return Ok(Range::parse(&a.value(), options)?.test(bv));
    }
    let (av, bv) = (
        a.version.as_ref().expect("op"),
        b.version.as_ref().expect("op"),
    );
    let (a_value, b_value) = (a.value(), b.value());
    if options.include_prerelease && (a_value == "<0.0.0-0" || b_value == "<0.0.0-0") {
        return Ok(false);
    }
    if !options.include_prerelease
        && (a_value.starts_with("<0.0.0") || b_value.starts_with("<0.0.0"))
    {
        return Ok(false);
    }
    let gt = |o: Op| matches!(o, Op::Gt | Op::Ge);
    let lt = |o: Op| matches!(o, Op::Lt | Op::Le);
    let inclusive = |o: Op| matches!(o, Op::Ge | Op::Le);
    if (gt(a.op) && gt(b.op)) || (lt(a.op) && lt(b.op)) {
        return Ok(true);
    }
    if version_str(av) == version_str(bv) && inclusive(a.op) && inclusive(b.op) {
        return Ok(true);
    }
    let o = av.cmp_precedence(bv);
    if o == Ordering::Less && gt(a.op) && lt(b.op) {
        return Ok(true);
    }
    if o == Ordering::Greater && lt(a.op) && gt(b.op) {
        return Ok(true);
    }
    Ok(false)
}

/// `isSatisfiable(comparators, options)`.
fn is_satisfiable(set: &[Comparator], options: Options) -> Result<bool, Invalid> {
    let mut remaining: Vec<&Comparator> = set.iter().collect();
    let mut test = remaining.pop();
    let mut result = true;
    while result && !remaining.is_empty() {
        let t = test.expect("popped while non-empty");
        result = true;
        for other in &remaining {
            if !comparator_intersects(t, other, options)? {
                result = false;
                break;
            }
        }
        test = remaining.pop();
    }
    Ok(result)
}

/// `range.intersects(other, options)`.
pub(crate) fn range_intersects(a: &Range, b: &Range, options: Options) -> Result<bool, Invalid> {
    for sa in &a.sets {
        if !is_satisfiable(sa, options)? {
            continue;
        }
        for sb in &b.sets {
            if !is_satisfiable(sb, options)? {
                continue;
            }
            let mut all = true;
            'pairs: for ca in sa {
                for cb in sb {
                    if !comparator_intersects(ca, cb, options)? {
                        all = false;
                        break 'pairs;
                    }
                }
            }
            if all {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

/// `subset(sub, dom, options)`: node-semver's answer (each set of `sub`
/// must fit in one set of `dom`).
pub(crate) fn subset_of(sub: &str, dom: &str, options: Options) -> Result<bool, Invalid> {
    if sub == dom {
        return Ok(true);
    }
    let (sub, sub_keys) = Range::parse_keyed(sub, options)?;
    let (dom, dom_keys) = Range::parse_keyed(dom, options)?;
    let mut saw_non_null = false;
    'outer: for (s, sk) in sub.sets.iter().zip(&sub_keys) {
        for (d, dk) in dom.sets.iter().zip(&dom_keys) {
            let same = sk.is_some() && sk == dk;
            let is_sub = if same {
                Some(true)
            } else {
                simple_subset(s, d, options)?
            };
            saw_non_null = saw_non_null || is_sub.is_some();
            if is_sub == Some(true) {
                continue 'outer;
            }
        }
        if saw_non_null {
            return Ok(false);
        }
    }
    Ok(true)
}

fn cmp_op(op: Op, v: Version) -> Comparator {
    Comparator {
        op,
        version: Some(v),
    }
}

/// `higherGT(a, b)`.
fn higher_gt<'c>(a: Option<&'c Comparator>, b: &'c Comparator) -> &'c Comparator {
    let Some(a) = a else { return b };
    let o = a
        .version
        .as_ref()
        .expect("gt")
        .cmp_precedence(b.version.as_ref().expect("gt"));
    match o {
        Ordering::Greater => a,
        Ordering::Less => b,
        Ordering::Equal if b.op == Op::Gt && a.op == Op::Ge => b,
        Ordering::Equal => a,
    }
}

/// `lowerLT(a, b)`.
fn lower_lt<'c>(a: Option<&'c Comparator>, b: &'c Comparator) -> &'c Comparator {
    let Some(a) = a else { return b };
    let o = a
        .version
        .as_ref()
        .expect("lt")
        .cmp_precedence(b.version.as_ref().expect("lt"));
    match o {
        Ordering::Less => a,
        Ordering::Greater => b,
        Ordering::Equal if b.op == Op::Lt && a.op == Op::Le => b,
        Ordering::Equal => a,
    }
}

/// `satisfies(version, comparator.value, options)` for a parsed version.
fn satisfies_comparator(v: &Version, c: &Comparator, options: Options) -> Result<bool, Invalid> {
    Ok(Range::parse(&c.value(), options)?.test(v))
}

/// `simpleSubset(sub, dom, options)`: `None` for a null set.
fn simple_subset(
    sub: &[Comparator],
    dom: &[Comparator],
    options: Options,
) -> Result<Option<bool>, Invalid> {
    let min_pre = [cmp_op(
        Op::Ge,
        Version {
            pre: Prerelease::from_valid("0"),
            ..Version::new(0, 0, 0)
        },
    )];
    let min = [cmp_op(Op::Ge, Version::new(0, 0, 0))];
    let mut sub = sub;
    let mut dom = dom;
    if sub.len() == 1 && sub[0].is_any() {
        if dom.len() == 1 && dom[0].is_any() {
            return Ok(Some(true));
        }
        sub = if options.include_prerelease {
            &min_pre
        } else {
            &min
        };
    }
    if dom.len() == 1 && dom[0].is_any() {
        if options.include_prerelease {
            return Ok(Some(true));
        }
        dom = &min;
    }

    let mut eq_set: Vec<&Version> = Vec::new();
    let mut gt: Option<&Comparator> = None;
    let mut lt: Option<&Comparator> = None;
    for c in sub {
        match c.op {
            Op::Gt | Op::Ge => gt = Some(higher_gt(gt, c)),
            Op::Lt | Op::Le => lt = Some(lower_lt(lt, c)),
            Op::Eq => {
                // A `Set` of SemVer objects: equal versions are still
                // different objects, so every one counts.
                if let Some(v) = &c.version {
                    eq_set.push(v);
                }
            }
        }
    }
    if eq_set.len() > 1 {
        return Ok(None);
    }
    let mut gtlt_comp = None;
    if let (Some(g), Some(l)) = (gt, lt) {
        let o = g
            .version
            .as_ref()
            .expect("gt")
            .cmp_precedence(l.version.as_ref().expect("lt"));
        gtlt_comp = Some(o);
        if o == Ordering::Greater || (o == Ordering::Equal && (g.op != Op::Ge || l.op != Op::Le)) {
            return Ok(None);
        }
    }
    if let Some(eq) = eq_set.first() {
        if let Some(g) = gt
            && !satisfies_comparator(eq, g, options)?
        {
            return Ok(None);
        }
        if let Some(l) = lt
            && !satisfies_comparator(eq, l, options)?
        {
            return Ok(None);
        }
        for c in dom {
            if !satisfies_comparator(eq, c, options)? {
                return Ok(Some(false));
            }
        }
        return Ok(Some(true));
    }

    let mut has_dom_lt = false;
    let mut has_dom_gt = false;
    let pre_of = |c: Option<&Comparator>| {
        c.and_then(|c| c.version.clone())
            .filter(|v| !options.include_prerelease && !v.pre.is_empty())
    };
    let mut need_dom_lt_pre = pre_of(lt);
    let mut need_dom_gt_pre = pre_of(gt);
    if let (Some(n), Some(l)) = (&need_dom_lt_pre, lt)
        && l.op == Op::Lt
        && n.pre.as_str() == "0"
    {
        need_dom_lt_pre = None;
    }
    let ne = |o: Option<Ordering>| o != Some(Ordering::Equal);
    for c in dom {
        let c_gt = matches!(c.op, Op::Gt | Op::Ge);
        let c_lt = matches!(c.op, Op::Lt | Op::Le);
        has_dom_gt = has_dom_gt || c_gt;
        has_dom_lt = has_dom_lt || c_lt;
        let c_pre_core = |n: &Version| {
            c.version
                .as_ref()
                .is_some_and(|v| !v.pre.is_empty() && v.same_core(n))
        };
        if let Some(g) = gt {
            if need_dom_gt_pre.as_ref().is_some_and(c_pre_core) {
                need_dom_gt_pre = None;
            }
            if c_gt {
                let higher = higher_gt(Some(g), c);
                if core::ptr::eq(higher, c) && !core::ptr::eq(higher, g) {
                    return Ok(Some(false));
                }
            } else if g.op == Op::Ge && !c.test(g.version.as_ref().expect("gt")) {
                return Ok(Some(false));
            }
        }
        if let Some(l) = lt {
            if need_dom_lt_pre.as_ref().is_some_and(c_pre_core) {
                need_dom_lt_pre = None;
            }
            if c_lt {
                let lower = lower_lt(Some(l), c);
                if core::ptr::eq(lower, c) && !core::ptr::eq(lower, l) {
                    return Ok(Some(false));
                }
            } else if l.op == Op::Le && !c.test(l.version.as_ref().expect("lt")) {
                return Ok(Some(false));
            }
        }
        if c.op == Op::Eq && (lt.is_some() || gt.is_some()) && ne(gtlt_comp) {
            return Ok(Some(false));
        }
    }
    if gt.is_some() && has_dom_lt && lt.is_none() && ne(gtlt_comp) {
        return Ok(Some(false));
    }
    if lt.is_some() && has_dom_gt && gt.is_none() && ne(gtlt_comp) {
        return Ok(Some(false));
    }
    if need_dom_gt_pre.is_some() || need_dom_lt_pre.is_some() {
        return Ok(Some(false));
    }
    Ok(Some(true))
}

/// `truncate(version, type)`.
pub(crate) fn truncate_version(v: &Version, kind: &str) -> Option<Version> {
    if !matches!(
        kind,
        "major" | "premajor" | "minor" | "preminor" | "patch" | "prepatch" | "prerelease"
    ) {
        return None;
    }
    if kind.starts_with("pre") {
        return Some(v.without_build());
    }
    let mut t = v.core();
    match kind {
        "major" => {
            t.minor = Num::ZERO;
            t.patch = Num::ZERO;
        }
        "minor" => t.patch = Num::ZERO,
        _ => {}
    }
    Some(t)
}

/// `simplifyRange(versions, range)`: a shorter range matching the same
/// versions of `versions`, or the range's own text when that is not
/// shorter. `versions` pairs each version as given with its parse.
pub(crate) fn simplify(versions: &[(&str, Version)], range: &Range, raw: &str) -> String {
    let mut v: Vec<&(&str, Version)> = versions.iter().collect();
    v.sort_by(|a, b| a.1.cmp_precedence(&b.1));
    let mut set: Vec<(&str, Option<&str>)> = Vec::new();
    let mut first: Option<&str> = None;
    let mut prev: Option<&str> = None;
    for &(text, version) in &v {
        if range.test(version) {
            prev = Some(text);
            if first.is_none() {
                first = Some(text);
            }
        } else {
            if let (Some(f), Some(p)) = (first, prev) {
                set.push((f, Some(p)));
            }
            prev = None;
            first = None;
        }
    }
    if let Some(f) = first {
        set.push((f, None));
    }
    let lowest = v.first().map(|e| e.0);
    let mut ranges = Vec::new();
    for (min, max) in set {
        match max {
            Some(max) if min == max => ranges.push(min.to_owned()),
            None if Some(min) == lowest => ranges.push("*".into()),
            None => ranges.push(format!(">={min}")),
            Some(max) if Some(min) == lowest => ranges.push(format!("<={max}")),
            Some(max) => ranges.push(format!("{min} - {max}")),
        }
    }
    let simplified = ranges.join(" || ");
    if simplified.len() < raw.len() {
        simplified
    } else {
        raw.to_owned()
    }
}

// ===========================================================================
// Public API: node-semver's functions
// ===========================================================================

/// node-semver's options (`{ loose, includePrerelease }`).
///
/// - `loose` accepts forms like `v1.2.3`, `=1.2.3`, `1.2.3beta` and
///   leading zeros, in versions and in ranges.
/// - `include_prerelease` lets ranges match pre-release versions of any
///   `major.minor.patch`, not just those a comparator names.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct NpmOptions {
    /// node-semver's `loose`.
    pub loose: bool,
    /// node-semver's `includePrerelease`.
    pub include_prerelease: bool,
}

impl NpmOptions {
    /// `{ loose: true }`.
    pub const LOOSE: NpmOptions = NpmOptions {
        loose: true,
        include_prerelease: false,
    };
    /// `{ includePrerelease: true }`.
    pub const INCLUDE_PRERELEASE: NpmOptions = NpmOptions {
        loose: false,
        include_prerelease: true,
    };

    pub(crate) fn internal(self) -> Options {
        Options {
            loose: self.loose,
            include_prerelease: self.include_prerelease,
        }
    }
}

/// Options for [`coerce`] (`{ loose, includePrerelease, rtl }`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct CoerceOptions {
    /// Parse the coerced version loosely.
    pub loose: bool,
    /// Keep the pre-release and build metadata that follow the numbers.
    pub include_prerelease: bool,
    /// Take the right-most version-like text (`1.2.3.4` gives `2.3.4`).
    pub rtl: bool,
}

pub use crate::Release;

impl Release {
    pub(crate) fn of_diff(d: DiffKind) -> Release {
        match d {
            DiffKind::Major => Release::Major,
            DiffKind::Premajor => Release::Premajor,
            DiffKind::Minor => Release::Minor,
            DiffKind::Preminor => Release::Preminor,
            DiffKind::Patch => Release::Patch,
            DiffKind::Prepatch => Release::Prepatch,
            DiffKind::Prerelease => Release::Prerelease,
        }
    }
}

/// The `identifierBase` argument of [`inc`]: what number starts a new
/// pre-release.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum IdentifierBase {
    /// `0` (the default; also `'0'`).
    #[default]
    Zero,
    /// `1` (`'1'`).
    One,
    /// No number (`false`): `-beta` rather than `-beta.0`.
    None,
}

impl IdentifierBase {
    fn internal(self) -> IdBase {
        match self {
            IdentifierBase::Zero => IdBase::Zero,
            IdentifierBase::One => IdBase::One,
            IdentifierBase::None => IdBase::None,
        }
    }
}

fn npm_error(message: String) -> crate::Error {
    crate::Error::with_detail(crate::ErrorKind::InvalidNpm, 0, &message)
}

impl From<Invalid> for crate::Error {
    fn from(i: Invalid) -> crate::Error {
        npm_error(match i {
            Invalid::Comparator(c) => format!("Invalid comparator: {c}"),
            Invalid::Range(r) => format!("Invalid SemVer Range: {r}"),
            Invalid::TooComplex => "range too complex to parse".into(),
        })
    }
}

impl From<TooComplex> for crate::Error {
    fn from(_: TooComplex) -> crate::Error {
        npm_error("too complex to parse".into())
    }
}

/// `new SemVer(version, options)`, as an error when node-semver throws.
fn version_or_error(version: &str, options: NpmOptions) -> Result<Version, crate::Error> {
    parse_version(version, options.loose)?
        .ok_or_else(|| npm_error(format!("Invalid Version: {}", js_trim(version))))
}

/// `semver.parse(version, options)`: the version, or `None` if it isn't
/// one.
///
/// Unlike node-semver, numbers have no size limit and versions no length
/// limit.
///
/// ```
/// use forgo_lib_semver::npm::{self, NpmOptions};
///
/// assert_eq!(npm::parse("v1.2.3", NpmOptions::default()).unwrap().to_string(), "1.2.3");
/// assert!(npm::parse("1.2.3beta", NpmOptions::default()).is_none());
/// assert_eq!(npm::parse("1.2.3beta", NpmOptions::LOOSE).unwrap().to_string(), "1.2.3-beta");
/// ```
pub fn parse(version: &str, options: NpmOptions) -> Option<Version> {
    parse_version(version, options.loose).ok().flatten()
}

/// `semver.valid(version, options)`: the normalized version (without
/// build metadata), or `None`.
pub fn valid(version: &str, options: NpmOptions) -> Option<String> {
    parse(version, options).map(|v| version_str(&v))
}

/// `semver.clean(version, options)`: [`valid`] after trimming and removing
/// leading `=` and `v`.
pub fn clean(version: &str, options: NpmOptions) -> Option<String> {
    clean_inner(version, options.loose)
        .ok()
        .flatten()
        .map(|v| version_str(&v))
}

fn clean_inner(version: &str, loose: bool) -> Result<Option<Version>, TooComplex> {
    clean_text(version, loose)
}

/// `semver.coerce(text, options)`: the first version-like run of numbers
/// in `text` (`v3`, `version 1.2`, `1.2.3.4`), completed with zeros.
///
/// ```
/// use forgo_lib_semver::npm::{self, CoerceOptions};
///
/// let c = |s| npm::coerce(s, CoerceOptions::default()).map(|v| v.to_string());
/// assert_eq!(c("v2").as_deref(), Some("2.0.0"));
/// assert_eq!(c("release-1.4-final").as_deref(), Some("1.4.0"));
/// assert_eq!(c("none"), None);
/// ```
pub fn coerce(text: &str, options: CoerceOptions) -> Option<Version> {
    coerce_inner(text, options).ok().flatten()
}

fn coerce_inner(text: &str, o: CoerceOptions) -> Result<Option<Version>, TooComplex> {
    let Some(v) = coerce_text(text, o.include_prerelease, o.rtl)? else {
        return Ok(None);
    };
    if o.loose {
        return parse_version(&v.to_string(), true);
    }
    Ok(Some(v))
}

/// `semver.inc(version, release, options, identifier, identifierBase)`:
/// the next version, or `None` where node-semver returns `null`.
///
/// `release` is one of `major`, `minor`, `patch`, `premajor`, `preminor`,
/// `prepatch`, `prerelease` or `release`.
///
/// ```
/// use forgo_lib_semver::npm::{self, IdentifierBase, NpmOptions};
///
/// let inc = |v, r, id| npm::inc(v, r, NpmOptions::default(), id, IdentifierBase::Zero);
/// assert_eq!(inc("1.2.3", "minor", None).as_deref(), Some("1.3.0"));
/// assert_eq!(inc("1.2.3", "prerelease", Some("rc")).as_deref(), Some("1.2.4-rc.0"));
/// assert_eq!(inc("1.2.4-rc.0", "prerelease", Some("rc")).as_deref(), Some("1.2.4-rc.1"));
/// assert_eq!(inc("1.0.0-rc.1", "major", None).as_deref(), Some("1.0.0"));
/// ```
pub fn inc(
    version: &str,
    release: &str,
    options: NpmOptions,
    identifier: Option<&str>,
    base: IdentifierBase,
) -> Option<String> {
    let v = parse(version, options)?;
    inc_inner(&v, release, options.loose, identifier, base.internal())
        .ok()
        .flatten()
        .map(|v| version_str(&v))
}

fn inc_inner(
    v: &Version,
    release: &str,
    loose: bool,
    identifier: Option<&str>,
    base: IdBase,
) -> Result<Option<Version>, TooComplex> {
    inc_version(v, release, loose, identifier, base)
}

/// `semver.diff(a, b)`: the kind of release between two versions, or
/// `None` if they have the same precedence.
pub fn diff(a: &str, b: &str) -> Result<Option<Release>, crate::Error> {
    let a = version_or_error(a, NpmOptions::default())?;
    let b = version_or_error(b, NpmOptions::default())?;
    Ok(diff_inner(&a, &b).map(Release::of_diff))
}

fn diff_inner(a: &Version, b: &Version) -> Option<DiffKind> {
    diff_versions(a, b)
}

/// `semver.compare(a, b, options)`: by precedence.
pub fn compare(a: &str, b: &str, options: NpmOptions) -> Result<Ordering, crate::Error> {
    let a = version_or_error(a, options)?;
    let b = version_or_error(b, options)?;
    Ok(a.cmp_precedence(&b))
}

/// `semver.compareBuild(a, b, options)`: by precedence, then build
/// metadata as node-semver compares it (numbers by value: `+1` equals
/// `+01`).
pub fn compare_build(a: &str, b: &str, options: NpmOptions) -> Result<Ordering, crate::Error> {
    let a = version_or_error(a, options)?;
    let b = version_or_error(b, options)?;
    Ok(a.cmp_precedence(&b).then_with(|| {
        let (x, y): (Vec<&str>, Vec<&str>) = (
            a.build.identifiers().map(|i| i.as_str()).collect(),
            b.build.identifiers().map(|i| i.as_str()).collect(),
        );
        for i in 0.. {
            match (x.get(i), y.get(i)) {
                (None, None) => return Ordering::Equal,
                (Some(_), None) => return Ordering::Greater,
                (None, Some(_)) => return Ordering::Less,
                (Some(p), Some(q)) => {
                    let o = cmp_ident(&PreId::Str((*p).to_owned()), &PreId::Str((*q).to_owned()));
                    if o != Ordering::Equal {
                        return o;
                    }
                }
            }
        }
        Ordering::Equal
    }))
}

/// `semver.satisfies(version, range, options)`: `false` for an invalid
/// version or range.
///
/// ```
/// use forgo_lib_semver::npm::{self, NpmOptions};
///
/// let o = NpmOptions::default();
/// assert!(npm::satisfies("1.4.0", "^1.2.3", o));
/// assert!(!npm::satisfies("1.4.0-beta", "^1.2.3", o));
/// assert!(npm::satisfies("1.4.0-beta", "^1.2.3", NpmOptions::INCLUDE_PRERELEASE));
/// ```
pub fn satisfies(version: &str, range: &str, options: NpmOptions) -> bool {
    let Ok(r) = Range::parse(range, options.internal()) else {
        return false;
    };
    match parse(version, options) {
        Some(v) => r.test(&v),
        None => false,
    }
}

/// `semver.validRange(range, options)`: the range as comparators (`*` for
/// any version), or `None` if it isn't valid.
///
/// ```
/// use forgo_lib_semver::npm::{self, NpmOptions};
///
/// let o = NpmOptions::default();
/// assert_eq!(npm::valid_range("^1.2.3", o).as_deref(), Some(">=1.2.3 <2.0.0-0"));
/// assert_eq!(npm::valid_range("1.x || >=2.5.0", o).as_deref(), Some(">=1.0.0 <2.0.0-0||>=2.5.0"));
/// assert_eq!(npm::valid_range("", o).as_deref(), Some("*"));
/// assert_eq!(npm::valid_range("> invalid", o), None);
/// ```
pub fn valid_range(range: &str, options: NpmOptions) -> Option<String> {
    let r = Range::parse(range, options.internal()).ok()?;
    let s = r.range_string();
    Some(if s.is_empty() { "*".into() } else { s })
}

/// `semver.toComparators(range, options)`: each set's comparators.
pub fn to_comparators(range: &str, options: NpmOptions) -> Result<Vec<Vec<String>>, crate::Error> {
    let r = Range::parse(range, options.internal())?;
    Ok(r.sets
        .iter()
        .map(|set| {
            let joined = set
                .iter()
                .map(Comparator::value)
                .collect::<Vec<_>>()
                .join(" ");
            js_trim(&joined).split(' ').map(String::from).collect()
        })
        .collect())
}

/// `semver.minVersion(range, options)`: the lowest version the range
/// matches, or `None` if it matches none.
pub fn min_version(range: &str, options: NpmOptions) -> Result<Option<Version>, crate::Error> {
    let r = Range::parse(range, options.internal())?;
    Ok(min_version_inner(&r))
}

fn min_version_inner(r: &Range) -> Option<Version> {
    min_version_of(r)
}

/// `semver.maxSatisfying(versions, range, options)`: the highest of
/// `versions` the range matches (the first of equal ones).
pub fn max_satisfying<'v, S: AsRef<str>>(
    versions: &'v [S],
    range: &str,
    options: NpmOptions,
) -> Option<&'v str> {
    extreme(versions, range, options, Ordering::Less)
}

/// `semver.minSatisfying(versions, range, options)`: the lowest of
/// `versions` the range matches (the first of equal ones).
pub fn min_satisfying<'v, S: AsRef<str>>(
    versions: &'v [S],
    range: &str,
    options: NpmOptions,
) -> Option<&'v str> {
    extreme(versions, range, options, Ordering::Greater)
}

fn extreme<'v, S: AsRef<str>>(
    versions: &'v [S],
    range: &str,
    options: NpmOptions,
    replace_when: Ordering,
) -> Option<&'v str> {
    let r = Range::parse(range, options.internal()).ok()?;
    let mut best: Option<(&str, Version)> = None;
    for v in versions {
        let text = v.as_ref();
        let Some(parsed) = parse(text, options) else {
            continue;
        };
        if !r.test(&parsed) {
            continue;
        }
        if best
            .as_ref()
            .is_none_or(|(_, b)| b.cmp_precedence(&parsed) == replace_when)
        {
            best = Some((text, parsed));
        }
    }
    best.map(|(t, _)| t)
}

/// `semver.gtr(version, range, options)`: whether `version` is above every
/// version the range could match.
pub fn gtr(version: &str, range: &str, options: NpmOptions) -> Result<bool, crate::Error> {
    outside_api(version, range, true, options)
}

/// `semver.ltr(version, range, options)`: whether `version` is below every
/// version the range could match.
pub fn ltr(version: &str, range: &str, options: NpmOptions) -> Result<bool, crate::Error> {
    outside_api(version, range, false, options)
}

fn outside_api(
    version: &str,
    range: &str,
    greater: bool,
    options: NpmOptions,
) -> Result<bool, crate::Error> {
    let v = version_or_error(version, options)?;
    let r = Range::parse(range, options.internal())?;
    Ok(outside(&v, &r, greater))
}

/// `semver.intersects(a, b, options)`, computed as node-semver does.
/// [`crate::Req::intersects`] gives the exact answer.
pub fn intersects(a: &str, b: &str, options: NpmOptions) -> Result<bool, crate::Error> {
    let o = options.internal();
    let ra = Range::parse(a, o)?;
    let rb = Range::parse(b, o)?;
    Ok(range_intersects(&ra, &rb, o)?)
}

/// `semver.subset(sub, dom, options)`, computed as node-semver does: each
/// `||` alternative of `sub` must fit in a single alternative of `dom`.
/// [`crate::Req::is_subset`] gives the exact answer.
pub fn subset(sub: &str, dom: &str, options: NpmOptions) -> Result<bool, crate::Error> {
    Ok(subset_inner(sub, dom, options.internal())?)
}

fn subset_inner(sub: &str, dom: &str, o: Options) -> Result<bool, Invalid> {
    subset_of(sub, dom, o)
}

/// `semver.simplifyRange(versions, range, options)`: a range matching the
/// same of `versions` as `range`, if that is shorter; else `range`.
pub fn simplify_range<S: AsRef<str>>(
    versions: &[S],
    range: &str,
    options: NpmOptions,
) -> Result<String, crate::Error> {
    let mut parsed = Vec::with_capacity(versions.len());
    for v in versions {
        parsed.push((v.as_ref(), version_or_error(v.as_ref(), options)?));
    }
    let r = match Range::parse(range, options.internal()) {
        Ok(r) => r,
        Err(_) => {
            // satisfies() is false for every version: nothing is included.
            return Ok(range.to_owned());
        }
    };
    Ok(simplify(&parsed, &r, range))
}

/// `semver.truncate(version, release, options)`: the version cut down to
/// `release` (`major`: `1.0.0`; pre-release kinds keep the pre-release).
pub fn truncate(version: &str, release: &str, options: NpmOptions) -> Option<String> {
    let v = parse(version, options)?;
    truncate_version(&v, release).map(|t| version_str(&t))
}
