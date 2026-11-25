//! What the report compares: adapters for each crate, the SemVer BNF as an
//! oracle, and deterministic generators of inputs.

use std::fmt::Debug;

// ---------------------------------------------------------------------------
// The SemVer 2.0.0 BNF (the same transcription as forgo_lib_semver's
// tests/spec.rs)
// ---------------------------------------------------------------------------

pub mod bnf {
    pub fn valid(s: &str) -> bool {
        let b = s.as_bytes();
        let Some(mut i) = core(b) else { return false };
        if b.get(i) == Some(&b'-') {
            match dotted(b, i + 1, pre_id) {
                Some(j) => i = j,
                None => return false,
            }
        }
        if b.get(i) == Some(&b'+') {
            match dotted(b, i + 1, build_id) {
                Some(j) => i = j,
                None => return false,
            }
        }
        i == b.len()
    }

    fn core(b: &[u8]) -> Option<usize> {
        let i = num(b, 0)?;
        let i = dot(b, i)?;
        let i = num(b, i)?;
        let i = dot(b, i)?;
        num(b, i)
    }

    fn dot(b: &[u8], i: usize) -> Option<usize> {
        (b.get(i) == Some(&b'.')).then_some(i + 1)
    }

    fn num(b: &[u8], i: usize) -> Option<usize> {
        match b.get(i) {
            Some(b'0') => Some(i + 1),
            Some(b'1'..=b'9') => {
                let mut j = i + 1;
                while b.get(j).is_some_and(u8::is_ascii_digit) {
                    j += 1;
                }
                Some(j)
            }
            _ => None,
        }
    }

    fn dotted(b: &[u8], i: usize, id: fn(&[u8], usize) -> Option<usize>) -> Option<usize> {
        let mut i = id(b, i)?;
        while b.get(i) == Some(&b'.') {
            i = id(b, i + 1)?;
        }
        Some(i)
    }

    fn chars(b: &[u8], i: usize) -> Option<usize> {
        let mut j = i;
        while b
            .get(j)
            .is_some_and(|c| c.is_ascii_alphanumeric() || *c == b'-')
        {
            j += 1;
        }
        (j > i).then_some(j)
    }

    fn pre_id(b: &[u8], i: usize) -> Option<usize> {
        let end = chars(b, i)?;
        let id = &b[i..end];
        let numeric = id.iter().all(u8::is_ascii_digit);
        (!numeric || id == b"0" || id[0] != b'0').then_some(end)
    }

    fn build_id(b: &[u8], i: usize) -> Option<usize> {
        chars(b, i)
    }
}

// ---------------------------------------------------------------------------
// Deterministic inputs
// ---------------------------------------------------------------------------

/// xorshift64*: the same inputs on every run.
pub struct Rng(pub u64);

impl Rng {
    pub fn next_u64(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }

    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len())]
    }

    pub fn chance(&mut self, percent: u64) -> bool {
        self.next_u64() % 100 < percent
    }
}

/// A version as published: mostly `M.m.p`, some pre-releases and builds.
pub fn typical_version(r: &mut Rng) -> String {
    let mut s = format!("{}.{}.{}", r.below(20), r.below(40), r.below(60));
    if r.chance(20) {
        s.push('-');
        s.push_str(r.pick(&["alpha", "beta.2", "rc.1", "0", "dev.20261124", "alpha.1.x"]));
    }
    if r.chance(5) {
        s.push('+');
        s.push_str(r.pick(&["build.5", "sha.5114f85", "001"]));
    }
    s
}

fn number(r: &mut Rng) -> String {
    match r.below(12) {
        0 => "0".into(),
        1 => u64::MAX.to_string(),
        2 => {
            let mut s = String::from("1");
            for _ in 0..(19 + r.below(10)) {
                s.push(char::from(b'0' + r.below(10) as u8));
            }
            s
        }
        _ => r.below(100).to_string(),
    }
}

fn identifier(r: &mut Rng) -> String {
    const CHARS: &[u8] = b"0123456789abzAZ-";
    if r.chance(30) {
        return number(r);
    }
    let len = 1 + r.below(5);
    (0..len).map(|_| char::from(*r.pick(CHARS))).collect()
}

/// A version from the grammar (numbers of any size).
pub fn grammar_version(r: &mut Rng) -> String {
    let mut s = format!("{}.{}.{}", number(r), number(r), number(r));
    if r.chance(50) {
        let n = 1 + r.below(3);
        let ids: Vec<String> = (0..n).map(|_| identifier(r)).collect();
        s.push('-');
        s.push_str(&ids.join("."));
    }
    if r.chance(30) {
        let n = 1 + r.below(3);
        let ids: Vec<String> = (0..n).map(|_| identifier(r)).collect();
        s.push('+');
        s.push_str(&ids.join("."));
    }
    s
}

/// One or two characters changed.
pub fn mutate(r: &mut Rng, s: &str) -> String {
    const POOL: &[char] = &[
        '.', '+', '-', '0', '9', 'a', '_', ' ', 'v', '=', 'é', '①', '\0',
    ];
    let mut chars: Vec<char> = s.chars().collect();
    for _ in 0..1 + r.below(2) {
        let i = r.below(chars.len() + 1);
        match r.below(3) {
            0 if i < chars.len() => {
                chars.remove(i);
            }
            1 if i < chars.len() => chars[i] = *r.pick(POOL),
            _ => chars.insert(i, *r.pick(POOL)),
        }
    }
    chars.into_iter().collect()
}

/// A requirement in Cargo's syntax.
pub fn cargo_req(r: &mut Rng) -> String {
    let comparator = |r: &mut Rng| {
        let op = *r.pick(&["", "=", ">", ">=", "<", "<=", "~", "^"]);
        let (a, b, c) = (r.below(4), r.below(4), r.below(4));
        let space = if r.chance(20) { " " } else { "" };
        let v = match r.below(8) {
            0 => format!("{a}"),
            1 => format!("{a}.{b}"),
            2 => format!("{a}.{b}.*"),
            3 => format!("{a}.*"),
            4 => format!("{a}.{b}.{c}-{}", r.pick(&["alpha", "rc.1", "0"])),
            _ => format!("{a}.{b}.{c}"),
        };
        format!("{op}{space}{v}")
    };
    match r.below(10) {
        0 => "*".into(),
        1 | 2 => format!("{}, {}", comparator(r), comparator(r)),
        3 => format!("{}, {}, {}", comparator(r), comparator(r), comparator(r)),
        _ => comparator(r),
    }
}

/// A range in npm's syntax, including its odder forms.
pub fn npm_range(r: &mut Rng) -> String {
    let version = |r: &mut Rng| {
        let (a, b, c) = (r.below(4), r.below(4), r.below(4));
        let prefix = *r.pick(&["", "", "", "", "v", "=", "=v"]);
        let body = match r.below(9) {
            0 => format!("{a}"),
            1 => format!("{a}.{b}"),
            2 => format!("{a}.x"),
            3 => format!("{a}.{b}.*"),
            4 => format!("{a}.{b}.{c}-{}", r.pick(&["alpha", "rc.1", "0", "beta.2"])),
            5 => "*".into(),
            _ => format!("{a}.{b}.{c}"),
        };
        format!("{prefix}{body}")
    };
    let comparator = |r: &mut Rng| {
        let op = *r.pick(&["", "", "=", ">", ">=", "<", "<=", "~", "~>", "^"]);
        let space = if r.chance(15) { " " } else { "" };
        format!("{op}{space}{}", version(r))
    };
    let set = |r: &mut Rng| match r.below(8) {
        0 => format!("{} - {}", version(r), version(r)),
        1 | 2 => format!("{} {}", comparator(r), comparator(r)),
        _ => comparator(r),
    };
    let mut s = set(r);
    while r.chance(25) {
        s.push_str(r.pick(&[" || ", "||", " ||"]));
        s.push_str(&set(r));
    }
    if r.chance(5) {
        s = format!("  {s} ");
    }
    s
}

/// A version to match ranges against: small numbers, some pre-releases.
pub fn probe_version(r: &mut Rng) -> String {
    let mut s = format!("{}.{}.{}", r.below(5), r.below(5), r.below(5));
    if r.chance(30) {
        s.push('-');
        s.push_str(r.pick(&["0", "alpha", "alpha.1", "beta.2", "rc.1"]));
    }
    s
}

// ---------------------------------------------------------------------------
// Adapters
// ---------------------------------------------------------------------------

/// A crate that parses versions.
pub struct Parser {
    pub name: &'static str,
    pub version: &'static str,
    /// Whether it accepts the text as a version.
    pub accepts: fn(&str) -> bool,
    /// A note on what it parses (shown in the report).
    pub note: &'static str,
}

fn ok<T, E: Debug>(r: Result<T, E>) -> bool {
    r.is_ok()
}

pub const PARSERS: &[Parser] = &[
    Parser {
        name: "forgo_lib_semver",
        version: "0.0.0",
        accepts: |s| ok(forgo_lib_semver::Version::parse(s)),
        note: "SemVer 2.0.0",
    },
    Parser {
        name: "semver",
        version: "1.0.28",
        accepts: |s| ok(semver::Version::parse(s)),
        note: "SemVer 2.0.0, `u64` numbers",
    },
    Parser {
        name: "semver-parser",
        version: "0.10.3",
        accepts: |s| ok(semver_parser::version::parse(s)),
        note: "SemVer 2.0.0",
    },
    Parser {
        name: "js-semver",
        version: "1.0.2",
        accepts: |s| ok(js_semver::Version::parse(s)),
        note: "npm's grammar",
    },
    Parser {
        name: "node-semver",
        version: "2.2.0",
        accepts: |s| ok(node_semver::Version::parse(s)),
        note: "npm's grammar",
    },
    Parser {
        name: "nodejs-semver",
        version: "7.0.0",
        accepts: |s| ok(nodejs_semver::Version::parse(s)),
        note: "npm's grammar",
    },
    Parser {
        name: "semver_rs",
        version: "0.2.0",
        accepts: |s| ok(semver_rs::Version::new(s).parse()),
        note: "npm's grammar",
    },
    Parser {
        name: "deno_semver",
        version: "0.10.1",
        accepts: |s| ok(deno_semver::Version::parse_standard(s)),
        note: "npm's grammar (loose)",
    },
    Parser {
        name: "versions",
        version: "8.0.1",
        accepts: |s| versions::SemVer::new(s).is_some(),
        note: "SemVer, and other schemes",
    },
    Parser {
        name: "lenient_semver",
        version: "0.4.2",
        accepts: |s| ok(lenient_semver::parse(s)),
        note: "lenient by design",
    },
];

/// Parse a range and test versions: `None` if the range doesn't parse;
/// else whether each version (by text) satisfies it (`None`: the version
/// doesn't parse).
pub type Eval = fn(&str, &[String]) -> Option<Vec<Option<bool>>>;

/// A crate with npm ranges.
pub struct NpmLib {
    pub name: &'static str,
    pub eval: Eval,
}

pub const NPM_LIBS: &[NpmLib] = &[
    NpmLib {
        name: "forgo_lib_semver",
        eval: |r, vs| {
            let range = forgo_lib_semver::Req::npm(r).ok()?;
            Some(
                vs.iter()
                    .map(|v| {
                        forgo_lib_semver::npm::parse(v, Default::default())
                            .map(|v| range.matches(&v))
                    })
                    .collect(),
            )
        },
    },
    NpmLib {
        name: "js-semver",
        eval: |r, vs| {
            let range = js_semver::Range::parse(r).ok()?;
            Some(
                vs.iter()
                    .map(|v| {
                        js_semver::Version::parse(v)
                            .ok()
                            .map(|v| range.satisfies(&v))
                    })
                    .collect(),
            )
        },
    },
    NpmLib {
        name: "node-semver",
        eval: |r, vs| {
            let range = node_semver::Range::parse(r).ok()?;
            Some(
                vs.iter()
                    .map(|v| {
                        node_semver::Version::parse(v)
                            .ok()
                            .map(|v| range.satisfies(&v))
                    })
                    .collect(),
            )
        },
    },
    NpmLib {
        name: "nodejs-semver",
        eval: |r, vs| {
            let range = nodejs_semver::Range::parse(r).ok()?;
            Some(
                vs.iter()
                    .map(|v| {
                        nodejs_semver::Version::parse(v)
                            .ok()
                            .map(|v| range.satisfies(&v))
                    })
                    .collect(),
            )
        },
    },
    NpmLib {
        name: "semver_rs",
        eval: |r, vs| {
            let range = semver_rs::Range::new(r).parse().ok()?;
            Some(
                vs.iter()
                    .map(|v| {
                        semver_rs::Version::new(v)
                            .parse()
                            .ok()
                            .map(|v| range.test(&v))
                    })
                    .collect(),
            )
        },
    },
    NpmLib {
        name: "deno_semver",
        eval: |r, vs| {
            let range = deno_semver::VersionReq::parse_from_npm(r).ok()?;
            Some(
                vs.iter()
                    .map(|v| {
                        deno_semver::Version::parse_from_npm(v)
                            .ok()
                            .map(|v| range.matches(&v))
                    })
                    .collect(),
            )
        },
    },
];
