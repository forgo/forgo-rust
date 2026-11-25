//! A fast path for ordinary npm ranges.
//!
//! Most ranges are simple: `^1.2.3`, `~1.2`, `>=1.2.3 <2`, `1.x || 2.x`,
//! `1.2.3 - 2.3.4`. For those, the result of node-semver's regular
//! expression pipeline is computed directly, with the same formulas (its
//! `replaceCaret`, `replaceTilde`, `replaceXRange`, `replaceStars`,
//! `replaceGTE0` and `hyphenReplace`). Anything else returns `None` and
//! goes through the full pipeline (`Range::parse`): loose mode, `v` or `=`
//! before a version, build metadata, spaces after an operator, tabs and
//! other whitespace, leading zeros, numbers over 19 digits, empty
//! alternatives, wildcards out of order.
//!
//! The tests below check that both paths give the same range on random
//! inputs, and the full pipeline is checked against node-semver itself.

use super::{Comparator, Op, Options, Range, normalize_sets};
use crate::ident::{BuildMetadata, Prerelease};
use crate::num::Num;
use crate::version::Version;
use alloc::vec::Vec;

/// A version that may leave out parts: `None` for a missing part or
/// `x`, `X`, `*`.
struct Partial<'a> {
    parts: [Option<Num>; 3],
    pre: Option<&'a str>,
    /// The whole text (for `>=${from}` in hyphen ranges).
    text: &'a str,
}

/// The bytes the fast path takes; anything else goes to the full
/// pipeline.
const FAST: [bool; 256] = {
    let mut t = [false; 256];
    let mut c = 0;
    while c < 256 {
        let b = c as u8;
        t[c] = b.is_ascii_alphanumeric()
            || matches!(
                b,
                b'*' | b'^' | b'~' | b'<' | b'>' | b'=' | b'|' | b'.' | b'-' | b' '
            );
        c += 1;
    }
    t
};

/// One part: `0`, a number without a leading zero (up to 19 digits), or a
/// wildcard (`Ok(None)`). `Err` for anything else.
#[inline]
fn part(s: &str) -> Result<Option<Num>, ()> {
    let b = s.as_bytes();
    match b {
        [b'x' | b'X' | b'*'] => Ok(None),
        [b'0'] => Ok(Some(Num::ZERO)),
        [b'1'..=b'9', ..] if b.len() <= 19 => {
            let mut n = 0u64;
            for &c in b {
                if !c.is_ascii_digit() {
                    return Err(());
                }
                n = n * 10 + u64::from(c - b'0');
            }
            Ok(Some(Num::new(n)))
        }
        _ => Err(()),
    }
}

fn partial(text: &str) -> Option<Partial<'_>> {
    let (core, pre) = match text.split_once('-') {
        Some((c, p)) => (c, Some(p)),
        None => (text, None),
    };
    let mut pieces = core.split('.');
    let mut parts = [None, None, None];
    let mut count = 0;
    for (k, s) in pieces.by_ref().take(3).enumerate() {
        parts[k] = part(s).ok()?;
        count = k + 1;
    }
    if pieces.next().is_some() {
        return None;
    }
    if let Some(p) = pre {
        // A pre-release only after three numbers, each short enough for
        // node-semver's patterns.
        if count < 3
            || parts.iter().any(Option::is_none)
            || p.split('.').any(|id| id.len() > 64)
            || Prerelease::new(p).is_err()
            || p.is_empty()
        {
            return None;
        }
    }
    // Wildcards out of order (`1.x.3`, `x.1`) are node-semver errors: let
    // the full pipeline report them.
    let x = |k: usize| k >= count || parts[k].is_none();
    if (x(0) && !x(1)) || (x(1) && count >= 3 && !x(2)) {
        return None;
    }
    // Missing trailing parts are wildcards.
    let _ = count;
    Some(Partial { parts, pre, text })
}

fn version(major: &Num, minor: &Num, patch: &Num, pre: &str) -> Version {
    Version {
        major: major.clone(),
        minor: minor.clone(),
        patch: patch.clone(),
        pre: Prerelease::from_valid(pre),
        build: BuildMetadata::EMPTY,
    }
}

fn cmp(op: Op, v: Version) -> Comparator {
    Comparator {
        op,
        version: Some(v),
    }
}

const ANY: Comparator = Comparator {
    op: Op::Eq,
    version: None,
};

fn null_set() -> Comparator {
    let z = Num::ZERO;
    cmp(Op::Lt, version(&z, &z, &z, "0"))
}

/// One token (operator and version) expanded into comparators, as
/// `parseComparator` does.
fn expand(token: &str, inc_pr: bool, out: &mut Vec<Comparator>) -> Option<()> {
    let z = if inc_pr { "0" } else { "" };
    let zero = Num::ZERO;
    let (op, rest) = if let Some(r) = token.strip_prefix('^') {
        ("^", r)
    } else if let Some(r) = token.strip_prefix("~>") {
        ("~", r)
    } else if let Some(r) = token.strip_prefix('~') {
        ("~", r)
    } else if let Some(r) = token.strip_prefix(">=") {
        (">=", r)
    } else if let Some(r) = token.strip_prefix("<=") {
        ("<=", r)
    } else if let Some(r) = token.strip_prefix('>') {
        (">", r)
    } else if let Some(r) = token.strip_prefix('<') {
        ("<", r)
    } else if let Some(r) = token.strip_prefix('=') {
        ("=", r)
    } else {
        ("", token)
    };
    let p = partial(rest)?;
    let [maj, min, pat] = &p.parts;
    match op {
        "^" => {
            let Some(m) = maj else {
                out.push(ANY);
                return Some(());
            };
            let Some(n) = min else {
                out.push(cmp(Op::Ge, version(m, &zero, &zero, z)));
                out.push(cmp(Op::Lt, version(&m.succ(), &zero, &zero, "0")));
                return Some(());
            };
            let Some(q) = pat else {
                out.push(cmp(Op::Ge, version(m, n, &zero, z)));
                out.push(if m.is_zero() {
                    cmp(Op::Lt, version(m, &n.succ(), &zero, "0"))
                } else {
                    cmp(Op::Lt, version(&m.succ(), &zero, &zero, "0"))
                });
                return Some(());
            };
            out.push(cmp(Op::Ge, version(m, n, q, p.pre.unwrap_or(""))));
            out.push(if m.is_zero() {
                if n.is_zero() {
                    cmp(Op::Lt, version(m, n, &q.succ(), "0"))
                } else {
                    cmp(Op::Lt, version(m, &n.succ(), &zero, "0"))
                }
            } else {
                cmp(Op::Lt, version(&m.succ(), &zero, &zero, "0"))
            });
        }
        "~" => {
            let Some(m) = maj else {
                out.push(ANY);
                return Some(());
            };
            let Some(n) = min else {
                out.push(cmp(Op::Ge, version(m, &zero, &zero, z)));
                out.push(cmp(Op::Lt, version(&m.succ(), &zero, &zero, "0")));
                return Some(());
            };
            let Some(q) = pat else {
                out.push(cmp(Op::Ge, version(m, n, &zero, z)));
                out.push(cmp(Op::Lt, version(m, &n.succ(), &zero, "0")));
                return Some(());
            };
            out.push(cmp(Op::Ge, version(m, n, q, p.pre.unwrap_or(""))));
            out.push(cmp(Op::Lt, version(m, &n.succ(), &zero, "0")));
        }
        _ => xrange(op, &p, inc_pr, out),
    }
    Some(())
}

/// `replaceXRange` (then `replaceStars`) for one token.
fn xrange(gtlt: &str, p: &Partial<'_>, inc_pr: bool, out: &mut Vec<Comparator>) {
    let zero = Num::ZERO;
    let [maj, min, pat] = &p.parts;
    let xm_major = maj.is_none();
    let xm = xm_major || min.is_none();
    let xp = xm || pat.is_none();
    let mut gtlt = if gtlt == "=" && xp { "" } else { gtlt };
    let pr = if inc_pr { "0" } else { "" };
    if xm_major {
        // `*` is any version (`replaceStars` removes it); `>*` and `<*`
        // match nothing.
        out.push(if gtlt == ">" || gtlt == "<" {
            null_set()
        } else {
            ANY
        });
        return;
    }
    let m = maj.as_ref().expect("not x");
    if !gtlt.is_empty() && xp {
        let mut major = m.clone();
        let mut minor = if xm {
            zero.clone()
        } else {
            min.clone().expect("not x")
        };
        let patch = zero.clone();
        let mut pre = pr;
        if gtlt == ">" {
            gtlt = ">=";
            if xm {
                major = m.succ();
                minor = zero.clone();
            } else {
                minor = minor.succ();
            }
        } else if gtlt == "<=" {
            gtlt = "<";
            if xm {
                major = m.succ();
            } else {
                minor = minor.succ();
            }
        }
        if gtlt == "<" {
            pre = "0";
        }
        let op = match gtlt {
            ">=" => Op::Ge,
            _ => Op::Lt,
        };
        out.push(cmp(op, version(&major, &minor, &patch, pre)));
    } else if xm {
        out.push(cmp(Op::Ge, version(m, &zero, &zero, pr)));
        out.push(cmp(Op::Lt, version(&m.succ(), &zero, &zero, "0")));
    } else if xp {
        let n = min.as_ref().expect("not x");
        out.push(cmp(Op::Ge, version(m, n, &zero, pr)));
        out.push(cmp(Op::Lt, version(m, &n.succ(), &zero, "0")));
    } else {
        let (n, q) = (min.as_ref().expect("full"), pat.as_ref().expect("full"));
        let op = match gtlt {
            ">" => Op::Gt,
            ">=" => Op::Ge,
            "<" => Op::Lt,
            "<=" => Op::Le,
            _ => Op::Eq,
        };
        out.push(cmp(op, version(m, n, q, p.pre.unwrap_or(""))));
    }
}

/// `hyphenReplace` for `from - to`.
fn hyphen(from: &Partial<'_>, to: &Partial<'_>, inc_pr: bool, out: &mut Vec<Comparator>) {
    let zero = Num::ZERO;
    let z = if inc_pr { "0" } else { "" };
    let [fm, fn_, fp] = &from.parts;
    match (fm, fn_, fp) {
        (None, _, _) => {}
        (Some(m), None, _) => out.push(cmp(Op::Ge, version(m, &zero, &zero, z))),
        (Some(m), Some(n), None) => out.push(cmp(Op::Ge, version(m, n, &zero, z))),
        (Some(m), Some(n), Some(q)) => {
            let pre = from.pre.unwrap_or(z);
            let _ = from.text;
            out.push(cmp(Op::Ge, version(m, n, q, pre)));
        }
    }
    let [tm, tn, tp] = &to.parts;
    match (tm, tn, tp) {
        (None, _, _) => {}
        (Some(m), None, _) => out.push(cmp(Op::Lt, version(&m.succ(), &zero, &zero, "0"))),
        (Some(m), Some(n), None) => out.push(cmp(Op::Lt, version(m, &n.succ(), &zero, "0"))),
        (Some(m), Some(n), Some(q)) => match to.pre {
            Some(pr) => out.push(cmp(Op::Le, version(m, n, q, pr))),
            None if inc_pr => out.push(cmp(Op::Lt, version(m, n, &q.succ(), "0"))),
            None => out.push(cmp(Op::Le, version(m, n, q, ""))),
        },
    }
    if out.is_empty() {
        out.push(ANY);
    }
}

/// One `||` alternative (`parseRange`), or `None` to use the full
/// pipeline.
fn set(text: &str, inc_pr: bool) -> Option<Vec<Comparator>> {
    let mut tokens = text.split(' ').filter(|t| !t.is_empty());
    let first = tokens.next()?;
    let mut comps = Vec::with_capacity(2);
    // `from - to`: exactly three tokens, the middle one `-`.
    let second = tokens.next();
    if second == Some("-") {
        let to = tokens.next()?;
        if tokens.next().is_some() {
            return None;
        }
        hyphen(&partial(first)?, &partial(to)?, inc_pr, &mut comps);
    } else {
        for t in [Some(first), second].into_iter().flatten().chain(tokens) {
            if t == "-" {
                return None;
            }
            expand(t, inc_pr, &mut comps)?;
        }
    }
    // replaceGTE0: `>=0.0.0` (`>=0.0.0-0` with includePrerelease) is any.
    let gte0 = |c: &Comparator| {
        c.op == Op::Ge
            && c.version.as_ref().is_some_and(|v| {
                v.major.is_zero()
                    && v.minor.is_zero()
                    && v.patch.is_zero()
                    && v.pre.bytes() == if inc_pr { &b"0"[..] } else { &b""[..] }
            })
    };
    // The Map by value: first position, duplicates dropped.
    let mut n = 0;
    for i in 0..comps.len() {
        if gte0(&comps[i]) {
            comps[i] = ANY;
        }
        if comps[i].is_null_set() {
            let c = comps.swap_remove(i);
            comps.clear();
            comps.push(c);
            return Some(comps);
        }
        if !comps[..n].contains(&comps[i]) {
            comps.swap(n, i);
            n += 1;
        }
    }
    comps.truncate(n);
    if comps.len() > 1 {
        comps.retain(|c| !c.is_any());
    }
    Some(comps)
}

/// The fast path for `new Range(text, options)`.
pub(crate) fn parse(text: &str, options: Options) -> Option<Range> {
    if options.loose || !text.bytes().all(|b| FAST[usize::from(b)]) {
        return None;
    }
    let mut sets = Vec::with_capacity(1);
    for part in text.split("||") {
        let part = part.trim_matches(' ');
        if part.is_empty() {
            return None;
        }
        sets.push(set(part, options.include_prerelease)?);
    }
    Some(Range {
        sets: normalize_sets(sets),
        options,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::format;
    use alloc::string::{String, ToString};

    struct Rng(u64);

    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 >> 12;
            self.0 ^= self.0 << 25;
            self.0 ^= self.0 >> 27;
            self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
        }
        fn below(&mut self, n: usize) -> usize {
            (self.next() % n as u64) as usize
        }
        fn pick<'a>(&mut self, xs: &[&'a str]) -> &'a str {
            xs[self.below(xs.len())]
        }
    }

    fn token(r: &mut Rng) -> String {
        let op = r.pick(&["", "", "=", ">", ">=", "<", "<=", "~", "~>", "^"]);
        // Mostly what the fast path takes; now and then what it leaves to
        // the full pipeline.
        let n = |r: &mut Rng| {
            if r.below(40) == 0 {
                r.pick(&["01", "99999999999999999999"]).to_string()
            } else {
                r.pick(&["0", "1", "2", "10", "x", "X", "*"]).to_string()
            }
        };
        let v = match r.below(24) {
            0..=4 => n(r),
            5..=9 => format!("{}.{}", n(r), n(r)),
            10..=17 => format!("{}.{}.{}", n(r), n(r), n(r)),
            18..=21 => format!(
                "{}.{}.{}-{}",
                n(r),
                n(r),
                n(r),
                r.pick(&["alpha", "0", "rc.1", "a.b", "x-y.2", "01"])
            ),
            22 => format!("{}.{}.{}+build", n(r), n(r), n(r)),
            _ => format!("v{}.{}", n(r), n(r)),
        };
        format!("{op}{v}")
    }

    fn range(r: &mut Rng) -> String {
        let mut sets = Vec::new();
        for _ in 0..1 + r.below(3) {
            let set = match r.below(12) {
                0 | 1 => format!(
                    "{} - {}",
                    token(r).trim_start_matches(['>', '<', '=', '~', '^']),
                    token(r).trim_start_matches(['>', '<', '=', '~', '^'])
                ),
                2 => String::new(),
                _ => (0..1 + r.below(3))
                    .map(|_| token(r))
                    .collect::<Vec<_>>()
                    .join(r.pick(&[" ", "  "])),
            };
            sets.push(set);
        }
        sets.join(r.pick(&[" || ", "||", "  ||  "]))
    }

    #[test]
    fn the_fast_path_agrees_with_the_full_pipeline() {
        let mut r = Rng(0x0fa5_7001);
        let mut fast = 0;
        for _ in 0..200_000 {
            let text = range(&mut r);
            for options in [
                Options::default(),
                Options {
                    loose: false,
                    include_prerelease: true,
                },
            ] {
                let Some(quick) = parse(&text, options) else {
                    continue;
                };
                fast += 1;
                match Range::parse_full(&text, options) {
                    Ok(full) => assert_eq!(quick, full, "{text:?} {options:?}"),
                    Err(e) => {
                        panic!("{text:?} {options:?}: fast path parsed, full pipeline: {e:?}")
                    }
                }
            }
        }
        assert!(fast > 40_000, "the fast path took only {fast} ranges");
    }

    #[test]
    fn ordinary_ranges_take_the_fast_path() {
        for text in [
            "^1.2.3",
            "~1.2.3",
            "~1.2",
            ">=1.2.3 <2.0.0",
            "1.x",
            "1.2.x",
            "*",
            "",
            "1.2.3",
            "=1.2.3",
            "1.2.3 - 2.3.4",
            "^1.0.0 || ^2.0.0",
            ">=0.10.0",
            "~0.4",
            "^0.3.1-alpha.2",
            "<=2.0.0",
            ">1",
            "^0.0.3",
            "1 || 2 || 3",
            "~>1.2",
        ] {
            if text.is_empty() {
                continue; // `""` is any version, left to the full pipeline
            }
            for include_prerelease in [false, true] {
                let options = Options {
                    loose: false,
                    include_prerelease,
                };
                let quick = parse(text, options)
                    .unwrap_or_else(|| panic!("{text:?} took the full pipeline"));
                assert_eq!(Ok(quick), Range::parse_full(text, options), "{text:?}");
            }
        }
    }
}
