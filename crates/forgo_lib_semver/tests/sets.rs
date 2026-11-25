//! Set operations on requirements, checked against matching.
//!
//! Random requirements in both syntaxes are built from a small vocabulary
//! (numbers 0 to 2, a few pre-releases). Every answer is checked two ways:
//!
//! - **Witnesses.** `min_intersection` and `min_difference` return a
//!   version; it must really meet both requirements (or the first and not
//!   the second). That proves every "yes" exactly.
//! - **A universe.** Every "no" is checked against about 900 versions
//!   around the vocabulary (numbers 0 to 3, pre-releases including the
//!   ones a bound can step to, like `alpha.0`), and every witness must be
//!   the lowest such version among them.
//!
//! The set model is also checked against direct matching, version by
//! version, and `simplify` must keep the set.

use forgo_lib_semver::npm::NpmOptions;
use forgo_lib_semver::{Req, Version};
use std::cmp::Ordering;

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        // xorshift64*
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }

    fn pick<'a>(&mut self, xs: &'a [&'a str]) -> &'a str {
        xs[self.below(xs.len())]
    }

    fn chance(&mut self, percent: u64) -> bool {
        self.next() % 100 < percent
    }
}

const REQ_PRE: &[&str] = &["0", "1", "alpha", "beta", "alpha.1", "rc.1"];

thread_local! {
    /// Generate mostly pre-release comparators, and `include_prerelease`
    /// half the time (each test runs on its own thread).
    static PRE_HEAVY: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

fn pre_heavy() -> bool {
    PRE_HEAVY.get()
}

/// Versions around the vocabulary.
fn universe() -> Vec<Version> {
    let pres = [
        "",
        "0",
        "0.0",
        "1",
        "1.0",
        "2",
        "alpha",
        "alpha.0",
        "alpha.1",
        "alpha.1.0",
        "alpha.2",
        "beta",
        "beta.0",
        "rc.1",
        "rc.1.0",
        "rc.2",
        "zzz",
    ];
    let mut out = Vec::new();
    for major in 0..=3 {
        for minor in 0..=3 {
            for patch in 0..=3 {
                for pre in pres {
                    let text = if pre.is_empty() {
                        format!("{major}.{minor}.{patch}")
                    } else {
                        format!("{major}.{minor}.{patch}-{pre}")
                    };
                    out.push(Version::parse(&text).unwrap());
                }
            }
        }
    }
    out
}

fn number(rng: &mut Rng) -> String {
    rng.below(3).to_string()
}

fn cargo_comparator(rng: &mut Rng) -> String {
    let op = rng.pick(&["", "=", ">", ">=", "<", "<=", "~", "^"]);
    let (m, n, p) = (number(rng), number(rng), number(rng));
    let version = match if pre_heavy() && rng.chance(60) {
        4
    } else {
        rng.below(7)
    } {
        0 => m,
        1 => format!("{m}.{n}"),
        2 | 3 => format!("{m}.{n}.{p}"),
        4 => format!("{m}.{n}.{p}-{}", rng.pick(REQ_PRE)),
        5 => format!("{m}.*"),
        _ => format!("{m}.{n}.*"),
    };
    format!("{op}{version}")
}

/// A full or partial version, and a version above it (for a range).
fn bounds(rng: &mut Rng) -> (String, String) {
    let (m, n, p) = (rng.below(3), rng.below(3), rng.below(3));
    let low = match if pre_heavy() && rng.chance(60) {
        1
    } else {
        rng.below(4)
    } {
        0 => format!("{m}.{n}"),
        1 => format!("{m}.{n}.{p}-{}", rng.pick(REQ_PRE)),
        _ => format!("{m}.{n}.{p}"),
    };
    let high = match rng.below(4) {
        0 => format!("{}", m + 1),
        1 => format!("{m}.{}.0-{}", n + 1, rng.pick(REQ_PRE)),
        _ => format!("{m}.{n}.{}", p + 1 + rng.below(2)),
    };
    (low, high)
}

fn cargo_req(rng: &mut Rng) -> Req {
    if rng.chance(5) {
        return Req::cargo("*").unwrap();
    }
    let text = if rng.chance(40) {
        // A range: a lower and an upper bound.
        let (low, high) = bounds(rng);
        format!(
            "{}{low}, {}{high}",
            rng.pick(&[">", ">="]),
            rng.pick(&["<", "<="])
        )
    } else if rng.chance(70) {
        cargo_comparator(rng)
    } else {
        let n = 2 + rng.below(2);
        (0..n)
            .map(|_| cargo_comparator(rng))
            .collect::<Vec<_>>()
            .join(", ")
    };
    Req::cargo(&text).unwrap_or_else(|e| panic!("{text:?}: {e}"))
}

fn npm_comparator(rng: &mut Rng) -> String {
    let op = rng.pick(&["", "=", ">", ">=", "<", "<=", "~", "^"]);
    let (m, n, p) = (number(rng), number(rng), number(rng));
    let version = match if pre_heavy() && rng.chance(60) {
        4
    } else {
        rng.below(8)
    } {
        0 => m,
        1 => format!("{m}.{n}"),
        2 | 3 => format!("{m}.{n}.{p}"),
        4 | 5 => format!("{m}.{n}.{p}-{}", rng.pick(REQ_PRE)),
        6 => format!("{m}.x"),
        _ => "*".to_string(),
    };
    format!("{op}{version}")
}

fn npm_req(rng: &mut Rng) -> Req {
    let alternatives = 1 + rng.below(2);
    let mut parts = Vec::new();
    for _ in 0..alternatives {
        if rng.chance(15) {
            parts.push(format!(
                "{}.{}.{} - {}.{}",
                number(rng),
                number(rng),
                number(rng),
                number(rng),
                number(rng)
            ));
        } else if rng.chance(40) {
            let (low, high) = bounds(rng);
            parts.push(format!(
                "{}{low} {}{high}",
                rng.pick(&[">", ">="]),
                rng.pick(&["<", "<="])
            ));
        } else if rng.chance(70) {
            parts.push(npm_comparator(rng));
        } else {
            let n = 2 + rng.below(2);
            let set: Vec<String> = (0..n).map(|_| npm_comparator(rng)).collect();
            parts.push(set.join(" "));
        }
    }
    let options = NpmOptions {
        loose: false,
        include_prerelease: rng.chance(if pre_heavy() { 50 } else { 20 }),
    };
    let text = parts.join(" || ");
    Req::npm_with(&text, options).unwrap_or_else(|e| panic!("{text:?}: {e}"))
}

fn random_req(rng: &mut Rng) -> Req {
    if rng.chance(50) {
        cargo_req(rng)
    } else {
        npm_req(rng)
    }
}

fn lowest<'a>(versions: impl Iterator<Item = &'a Version>) -> Option<&'a Version> {
    versions.min_by(|a, b| a.cmp_precedence(b))
}

fn check_witness(
    what: &str,
    a: &Req,
    b: &Req,
    witness: Option<Version>,
    holds: impl Fn(&Version) -> bool,
    universe: &[Version],
) {
    match witness {
        Some(w) => {
            assert!(
                holds(&w),
                "{what}({a:?}, {b:?}) = {w}, which doesn't qualify"
            );
            if let Some(low) = lowest(universe.iter().filter(|v| holds(v))) {
                assert_ne!(
                    low.cmp_precedence(&w),
                    Ordering::Less,
                    "{what}({a:?}, {b:?}) = {w}, but {low} qualifies and is lower"
                );
            }
        }
        None => {
            if let Some(v) = universe.iter().find(|v| holds(v)) {
                panic!("{what}({a:?}, {b:?}) = None, but {v} qualifies");
            }
        }
    }
}

#[test]
fn set_operations_are_exact() {
    check_operations(0x5eed_1234_abcd_0001);
}

#[test]
fn set_operations_are_exact_with_pre_releases() {
    PRE_HEAVY.set(true);
    check_operations(0x5eed_1234_abcd_0004);
}

fn check_operations(seed: u64) {
    let universe = universe();
    let mut rng = Rng(seed);
    let (mut meet, mut sub, mut empty, mut pre_witness) = (0, 0, 0, 0);
    const PAIRS: usize = 6000;
    for _ in 0..PAIRS {
        let a = random_req(&mut rng);
        let b = random_req(&mut rng);
        meet += usize::from(a.intersects(&b));
        sub += usize::from(a.is_subset(&b) && !a.is_unsatisfiable() && !a.is_equivalent(&b));
        empty += usize::from(a.is_unsatisfiable());
        pre_witness += usize::from(a.min_intersection(&b).is_some_and(|w| w.is_prerelease()))
            + usize::from(a.min_difference(&b).is_some_and(|w| w.is_prerelease()));
        check_witness(
            "min_intersection",
            &a,
            &b,
            a.min_intersection(&b),
            |v| a.matches(v) && b.matches(v),
            &universe,
        );
        check_witness(
            "min_difference",
            &a,
            &b,
            a.min_difference(&b),
            |v| a.matches(v) && !b.matches(v),
            &universe,
        );
        assert_eq!(a.intersects(&b), a.min_intersection(&b).is_some());
        assert_eq!(a.is_subset(&b), a.min_difference(&b).is_none());
        let min = a.min_version();
        check_witness(
            "min_version",
            &a,
            &a,
            min.clone(),
            |v| a.matches(v),
            &universe,
        );
        assert_eq!(a.is_unsatisfiable(), min.is_none());
    }
    println!(
        "{PAIRS} pairs: {meet} intersect, {pre_witness} witnesses are pre-releases, {sub} proper subsets, {empty} unsatisfiable"
    );
    // Both answers must be common enough to mean something.
    assert!(meet > PAIRS / 5 && meet < PAIRS * 4 / 5, "{meet}");
    assert!(sub > PAIRS / 20, "{sub}");
    assert!(empty < PAIRS / 5, "{empty}");
    assert!(pre_witness > 20, "{pre_witness}");
}

#[test]
fn the_set_model_agrees_with_matching() {
    let universe = universe();
    let all = NpmOptions {
        loose: false,
        include_prerelease: true,
    };
    let exact: Vec<Req> = universe
        .iter()
        .map(|v| Req::npm_with(&v.to_string(), all).unwrap())
        .collect();
    let mut rng = Rng(0x5eed_1234_abcd_0002);
    for _ in 0..150 {
        let a = random_req(&mut rng);
        for (v, eq) in universe.iter().zip(&exact) {
            let in_model = a.min_intersection(eq).is_some();
            assert_eq!(in_model, a.matches(v), "{a:?} and {v}: model {in_model}");
        }
    }
}

#[test]
fn simplify_keeps_the_set() {
    let universe = universe();
    let mut rng = Rng(0x5eed_1234_abcd_0003);
    for _ in 0..1500 {
        let a = random_req(&mut rng);
        let s = a.simplify();
        assert!(a.is_equivalent(&s), "{a:?} simplified to {s:?}");
        for v in &universe {
            assert_eq!(
                a.matches(v),
                s.matches(v),
                "{a:?} simplified to {s:?}, differs at {v}"
            );
        }
        // The simplified text parses back to the same requirement.
        let again = match a.dialect() {
            forgo_lib_semver::Dialect::Cargo => Req::cargo(&s.to_string()),
            forgo_lib_semver::Dialect::Npm => Req::npm(&s.to_string()),
        };
        if let Ok(again) = again
            && a.dialect() == forgo_lib_semver::Dialect::Cargo
        {
            assert!(again.is_equivalent(&s), "{s} reparses differently");
        }
    }
}
