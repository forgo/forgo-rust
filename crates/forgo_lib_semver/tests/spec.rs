//! Semantic Versioning 2.0.0: parsing and precedence.
//!
//! There is no official test suite. These checks are, in order:
//!
//! 1. The valid and invalid versions that semver.org's FAQ links to
//!    (<https://regex101.com/r/Ly7O1x/3>), and the specification's own
//!    examples, with §11's precedence chain. (semver.org's text and
//!    examples: CC BY 3.0.)
//! 2. An independent oracle: the specification's BNF, transcribed rule by
//!    rule as a recursive validator.
//! 3. Generated inputs: random versions from the grammar (numbers of any
//!    size) and mutations of them, each parsed and compared with the
//!    oracle; valid ones must print back exactly as written.
//! 4. Precedence against a reference written from §11, and the laws
//!    tying `==`, `Ord` and `Hash` together.

use forgo_lib_semver::{BuildMetadata, ErrorKind, Num, Part, Prerelease, Version};
use std::cmp::Ordering;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

// ---------------------------------------------------------------------------
// 1. Published vectors
// ---------------------------------------------------------------------------

/// semver.org's FAQ list of valid versions.
const VALID: &[&str] = &[
    "0.0.4",
    "1.2.3",
    "10.20.30",
    "1.1.2-prerelease+meta",
    "1.1.2+meta",
    "1.1.2+meta-valid",
    "1.0.0-alpha",
    "1.0.0-beta",
    "1.0.0-alpha.beta",
    "1.0.0-alpha.beta.1",
    "1.0.0-alpha.1",
    "1.0.0-alpha0.valid",
    "1.0.0-alpha.0valid",
    "1.0.0-alpha-a.b-c-somethinglong+build.1-aef.1-its-okay",
    "1.0.0-rc.1+build.1",
    "2.0.0-rc.1+build.123",
    "1.2.3-beta",
    "10.2.3-DEV-SNAPSHOT",
    "1.2.3-SNAPSHOT-123",
    "1.0.0",
    "2.0.0",
    "1.1.7",
    "2.0.0+build.1848",
    "2.0.1-alpha.1227",
    "1.0.0-alpha+beta",
    "1.2.3----RC-SNAPSHOT.12.9.1--.12+788",
    "1.2.3----R-S.12.9.1--.12+meta",
    "1.2.3----RC-SNAPSHOT.12.9.1--.12",
    "1.0.0+0.build.1-rc.10000aaa-kk-0.1",
    "99999999999999999999999.999999999999999999.99999999999999999",
    "1.0.0-0A.is.legal",
];

/// semver.org's FAQ list of invalid versions.
const INVALID: &[&str] = &[
    "1",
    "1.2",
    "1.2.3-0123",
    "1.2.3-0123.0123",
    "1.1.2+.123",
    "+invalid",
    "-invalid",
    "-invalid+invalid",
    "-invalid.01",
    "alpha",
    "alpha.beta",
    "alpha.beta.1",
    "alpha.1",
    "alpha+beta",
    "alpha_beta",
    "alpha.",
    "alpha..",
    "beta",
    "1.0.0-alpha_beta",
    "-alpha.",
    "1.0.0-alpha..",
    "1.0.0-alpha..1",
    "1.0.0-alpha...1",
    "1.0.0-alpha....1",
    "1.0.0-alpha.....1",
    "1.0.0-alpha......1",
    "1.0.0-alpha.......1",
    "01.1.1",
    "1.01.1",
    "1.1.01",
    "1.2",
    "1.2.3.DEV",
    "1.2-SNAPSHOT",
    "1.2.31.2.3----RC-SNAPSHOT.12.09.1--..12+788",
    "1.2-RC-SNAPSHOT",
    "-1.0.3-gamma+b7718",
    "+justmeta",
    "9.8.7+meta+meta",
    "9.8.7-whatever+meta+meta",
    "99999999999999999999999.999999999999999999.99999999999999999----RC-SNAPSHOT.12.09.1--------------------------------..12",
];

/// Edge cases of the grammar (and of semver/semver#833): `-` is a
/// non-digit, so `--` is an identifier; only ASCII counts.
const EDGE_VALID: &[&str] = &[
    "1.2.3--",
    "1.2.3---",
    "1.2.3-0-",
    "1.2.3--0",
    "1.2.3-0a",
    "1.2.3-00a",
    "1.2.3-a-.b-",
    "1.2.3+-",
    "1.2.3+0",
    "1.2.3+00",
    "1.2.3+01.002.0003",
    "0.0.0",
    "0.0.0-0",
    "18446744073709551615.18446744073709551615.18446744073709551615",
    "18446744073709551616.0.0",
];
const EDGE_INVALID: &[&str] = &[
    "",
    " 1.2.3",
    "1.2.3 ",
    "v1.2.3",
    "=1.2.3",
    "1.2.3-",
    "1.2.3+",
    "1.2.3-+",
    "1.2.3-.",
    "1.2.3+a+b",
    "1.2.3-00",
    "1.2.3-01",
    "1.2.3-a.01",
    "\u{2460}.2.3",
    "\u{663}.2.3",
    "1.2.3-\u{e9}",
    "1.2.3+\u{e9}",
    "1.2.3-a_b",
    "1.2.3\n",
    "1..3",
    "1.2.3.4",
    "-1.2.3",
    "1.2.-3",
];

#[test]
fn published_valid_versions_parse_and_print_back() {
    for &s in VALID.iter().chain(EDGE_VALID) {
        let v = Version::parse(s).unwrap_or_else(|e| panic!("{s:?}: {e}"));
        assert_eq!(v.to_string(), s);
        assert!(bnf::valid(s), "oracle rejects {s:?}");
    }
}

#[test]
fn published_invalid_versions_are_rejected() {
    for &s in INVALID.iter().chain(EDGE_INVALID) {
        assert!(Version::parse(s).is_err(), "{s:?} parsed");
        assert!(!bnf::valid(s), "oracle accepts {s:?}");
    }
}

#[test]
fn specification_examples() {
    // §2: X.Y.Z; §9: pre-releases; §10: build metadata.
    for s in [
        "1.9.0",
        "1.10.0",
        "1.11.0",
        "1.0.0-alpha",
        "1.0.0-alpha.1",
        "1.0.0-0.3.7",
        "1.0.0-x.7.z.92",
        "1.0.0-x-y-z.--",
        "1.0.0-alpha+001",
        "1.0.0+20130313144700",
        "1.0.0-beta+exp.sha.5114f85",
        "1.0.0+21AF26D3----117B344092BD",
    ] {
        assert!(Version::parse(s).is_ok(), "{s}");
    }
    // §11.2: major, minor and patch numerically.
    chain(&["1.0.0", "2.0.0", "2.1.0", "2.1.1"]);
    // §11.3: a pre-release before its release.
    chain(&["1.0.0-alpha", "1.0.0"]);
    // §11.4: identifier by identifier.
    chain(&[
        "1.0.0-alpha",
        "1.0.0-alpha.1",
        "1.0.0-alpha.beta",
        "1.0.0-beta",
        "1.0.0-beta.2",
        "1.0.0-beta.11",
        "1.0.0-rc.1",
        "1.0.0",
    ]);
    // §10: build metadata doesn't change precedence.
    let (a, b) = (v("1.0.0+a"), v("1.0.0+b"));
    assert_eq!(a.cmp_precedence(&b), Ordering::Equal);
    assert!(a.precedence_eq(&b));
    assert_ne!(a, b);
}

fn v(s: &str) -> Version {
    Version::parse(s).unwrap()
}

#[track_caller]
fn chain(versions: &[&str]) {
    for pair in versions.windows(2) {
        let (a, b) = (v(pair[0]), v(pair[1]));
        assert_eq!(a.cmp_precedence(&b), Ordering::Less, "{a} < {b}");
        assert_eq!(b.cmp_precedence(&a), Ordering::Greater, "{b} > {a}");
        assert!(a < b);
    }
}

#[test]
fn errors_say_what_and_where() {
    let cases: &[(&str, ErrorKind, usize)] = &[
        ("", ErrorKind::Empty, 0),
        ("v1.2.3", ErrorKind::UnexpectedChar(Part::Major, 'v'), 0),
        ("1", ErrorKind::UnexpectedEnd(Part::Minor), 1),
        ("1.2", ErrorKind::UnexpectedEnd(Part::Patch), 3),
        ("01.2.3", ErrorKind::LeadingZero(Part::Major), 0),
        ("1.02.3", ErrorKind::LeadingZero(Part::Minor), 2),
        ("1.2.3-01", ErrorKind::LeadingZero(Part::Pre), 6),
        ("1.2.3-a..b", ErrorKind::EmptyIdentifier(Part::Pre), 8),
        ("1.2.3+", ErrorKind::EmptyIdentifier(Part::Build), 6),
        (
            "1.2.3.4",
            ErrorKind::UnexpectedCharAfter(Part::Patch, '.'),
            5,
        ),
        ("1.2.3-a_b", ErrorKind::UnexpectedChar(Part::Pre, '_'), 7),
        ("1.2.3+a+b", ErrorKind::UnexpectedChar(Part::Build, '+'), 7),
    ];
    for (s, kind, pos) in cases {
        let e = Version::parse(s).unwrap_err();
        assert_eq!((e.kind(), e.pos()), (*kind, *pos), "{s:?}: {e}");
    }
    assert_eq!(
        Version::parse("1.2.3-01").unwrap_err().to_string(),
        "leading zero in a numeric pre-release identifier at byte 6"
    );
}

// ---------------------------------------------------------------------------
// 2. The BNF, rule by rule
// ---------------------------------------------------------------------------

/// The grammar from semver.org, as a recursive validator. Each function is
/// one rule; `None` means the rule doesn't match at that position.
mod bnf {
    // <valid semver> ::= <version core>
    //                  | <version core> "-" <pre-release>
    //                  | <version core> "+" <build>
    //                  | <version core> "-" <pre-release> "+" <build>
    pub fn valid(s: &str) -> bool {
        let b = s.as_bytes();
        let Some(i) = version_core(b, 0) else {
            return false;
        };
        let i = match b.get(i) {
            Some(b'-') => match pre_release(b, i + 1) {
                Some(j) => j,
                None => return false,
            },
            _ => i,
        };
        let i = match b.get(i) {
            Some(b'+') => match build(b, i + 1) {
                Some(j) => j,
                None => return false,
            },
            _ => i,
        };
        i == b.len()
    }

    // <version core> ::= <major> "." <minor> "." <patch>
    // <major>, <minor>, <patch> ::= <numeric identifier>
    fn version_core(b: &[u8], i: usize) -> Option<usize> {
        let i = numeric_identifier(b, i)?;
        let i = dot(b, i)?;
        let i = numeric_identifier(b, i)?;
        let i = dot(b, i)?;
        numeric_identifier(b, i)
    }

    fn dot(b: &[u8], i: usize) -> Option<usize> {
        (b.get(i) == Some(&b'.')).then_some(i + 1)
    }

    // <pre-release> ::= <dot-separated pre-release identifiers>
    fn pre_release(b: &[u8], i: usize) -> Option<usize> {
        dot_separated(b, i, pre_release_identifier)
    }

    // <build> ::= <dot-separated build identifiers>
    fn build(b: &[u8], i: usize) -> Option<usize> {
        dot_separated(b, i, build_identifier)
    }

    // <dot-separated …> ::= <… identifier> | <… identifier> "." <dot-separated …>
    fn dot_separated(
        b: &[u8],
        i: usize,
        ident: fn(&[u8], usize) -> Option<usize>,
    ) -> Option<usize> {
        let mut i = ident(b, i)?;
        while b.get(i) == Some(&b'.') {
            i = ident(b, i + 1)?;
        }
        Some(i)
    }

    // <pre-release identifier> ::= <alphanumeric identifier> | <numeric identifier>
    // The identifier runs to the next character that isn't an identifier
    // character; it must then be one or the other.
    fn pre_release_identifier(b: &[u8], i: usize) -> Option<usize> {
        let end = identifier_characters(b, i)?;
        let id = &b[i..end];
        if alphanumeric_identifier(id) || numeric_identifier(id, 0) == Some(id.len()) {
            Some(end)
        } else {
            None
        }
    }

    // <build identifier> ::= <alphanumeric identifier> | <digits>
    fn build_identifier(b: &[u8], i: usize) -> Option<usize> {
        let end = identifier_characters(b, i)?;
        let id = &b[i..end];
        (alphanumeric_identifier(id) || id.iter().all(|c| digit(*c))).then_some(end)
    }

    // <alphanumeric identifier> ::= <non-digit>
    //   | <non-digit> <identifier characters>
    //   | <identifier characters> <non-digit>
    //   | <identifier characters> <non-digit> <identifier characters>
    // That is: identifier characters with at least one non-digit.
    fn alphanumeric_identifier(id: &[u8]) -> bool {
        !id.is_empty()
            && id.iter().all(|c| identifier_character(*c))
            && id.iter().any(|c| non_digit(*c))
    }

    // <numeric identifier> ::= "0" | <positive digit> | <positive digit> <digits>
    fn numeric_identifier(b: &[u8], i: usize) -> Option<usize> {
        match b.get(i) {
            Some(b'0') => Some(i + 1),
            Some(c) if positive_digit(*c) => {
                let mut j = i + 1;
                while b.get(j).is_some_and(|c| digit(*c)) {
                    j += 1;
                }
                Some(j)
            }
            _ => None,
        }
    }

    // <identifier characters> ::= <identifier character> | <identifier character> <identifier characters>
    fn identifier_characters(b: &[u8], i: usize) -> Option<usize> {
        let mut j = i;
        while b.get(j).is_some_and(|c| identifier_character(*c)) {
            j += 1;
        }
        (j > i).then_some(j)
    }

    // <identifier character> ::= <digit> | <non-digit>
    fn identifier_character(c: u8) -> bool {
        digit(c) || non_digit(c)
    }

    // <non-digit> ::= <letter> | "-"
    fn non_digit(c: u8) -> bool {
        letter(c) || c == b'-'
    }

    // <digit> ::= "0" | <positive digit>
    fn digit(c: u8) -> bool {
        c == b'0' || positive_digit(c)
    }

    // <positive digit> ::= "1" | … | "9"
    fn positive_digit(c: u8) -> bool {
        (b'1'..=b'9').contains(&c)
    }

    // <letter> ::= "A" | … | "Z" | "a" | … | "z"
    fn letter(c: u8) -> bool {
        c.is_ascii_alphabetic()
    }
}

// ---------------------------------------------------------------------------
// 3. Generated inputs
// ---------------------------------------------------------------------------

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
}

fn gen_number(r: &mut Rng) -> String {
    match r.below(10) {
        0 => "0".into(),
        1 => {
            // Beyond u64.
            let mut s = String::from("1");
            for _ in 0..(19 + r.below(30)) {
                s.push(char::from(b'0' + r.below(10) as u8));
            }
            s
        }
        2 => u64::MAX.to_string(),
        _ => (r.below(100)).to_string(),
    }
}

fn gen_identifier(r: &mut Rng, numeric_ok: bool) -> String {
    const CHARS: &[u8] = b"0123456789abcxyzABCXYZ-";
    if numeric_ok && r.below(3) == 0 {
        return gen_number(r);
    }
    let len = 1 + r.below(6);
    let mut s: String = (0..len)
        .map(|_| char::from(CHARS[r.below(CHARS.len())]))
        .collect();
    if s.bytes().all(|c| c.is_ascii_digit()) && s.len() > 1 && s.starts_with('0') {
        s.push('a'); // keep it valid: alphanumeric
    }
    s
}

fn gen_identifiers(r: &mut Rng, numeric_ok: bool) -> String {
    let n = 1 + r.below(4);
    (0..n)
        .map(|_| gen_identifier(r, numeric_ok))
        .collect::<Vec<_>>()
        .join(".")
}

fn gen_version(r: &mut Rng) -> String {
    let mut s = format!("{}.{}.{}", gen_number(r), gen_number(r), gen_number(r));
    if r.below(2) == 0 {
        s.push('-');
        s.push_str(&gen_identifiers(r, true));
    }
    if r.below(3) == 0 {
        s.push('+');
        s.push_str(&gen_identifiers(r, true));
    }
    s
}

fn mutate(r: &mut Rng, s: &str) -> String {
    const POOL: &[char] = &[
        '.', '+', '-', '0', '1', '9', 'a', 'Z', '_', ' ', 'v', '\u{e9}', '\u{2460}', '\u{663}',
        '\0',
    ];
    let mut chars: Vec<char> = s.chars().collect();
    for _ in 0..1 + r.below(2) {
        let i = r.below(chars.len() + 1);
        match r.below(3) {
            0 if i < chars.len() => {
                chars.remove(i);
            }
            1 if i < chars.len() => chars[i] = POOL[r.below(POOL.len())],
            _ => chars.insert(i, POOL[r.below(POOL.len())]),
        }
    }
    chars.into_iter().collect()
}

#[test]
fn generated_versions_agree_with_the_oracle() {
    let mut r = Rng(0x5e3f_0001);
    let (mut valid, mut invalid) = (0, 0);
    for _ in 0..40_000 {
        let original = gen_version(&mut r);
        for s in [original.clone(), mutate(&mut r, &original)] {
            let ok = bnf::valid(&s);
            match Version::parse(&s) {
                Ok(v) => {
                    assert!(ok, "parsed {s:?}, which the oracle rejects");
                    assert_eq!(v.to_string(), s, "doesn't print back");
                    valid += 1;
                }
                Err(e) => {
                    assert!(!ok, "rejected {s:?} ({e}), which the oracle accepts");
                    assert!(e.pos() <= s.len());
                    invalid += 1;
                }
            }
        }
    }
    println!("{valid} valid and {invalid} invalid generated versions agree with the BNF");
    assert!(valid > 40_000 && invalid > 15_000);
}

// ---------------------------------------------------------------------------
// 4. Precedence and the laws of ==, Ord and Hash
// ---------------------------------------------------------------------------

/// Precedence written from §11, on the strings.
fn reference_precedence(a: &str, b: &str) -> Ordering {
    fn split(s: &str) -> (Vec<&str>, Option<&str>) {
        let s = s.split('+').next().unwrap();
        let (core, pre) = match s.split_once('-') {
            Some((c, p)) => (c, Some(p)),
            None => (s, None),
        };
        (core.split('.').collect(), pre)
    }
    fn num(a: &str, b: &str) -> Ordering {
        a.len().cmp(&b.len()).then(a.cmp(b))
    }
    let ((ca, pa), (cb, pb)) = (split(a), split(b));
    for (x, y) in ca.iter().zip(&cb) {
        match num(x, y) {
            Ordering::Equal => {}
            o => return o,
        }
    }
    match (pa, pb) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Greater,
        (Some(_), None) => Ordering::Less,
        (Some(pa), Some(pb)) => {
            let (ia, ib): (Vec<&str>, Vec<&str>) =
                (pa.split('.').collect(), pb.split('.').collect());
            for (x, y) in ia.iter().zip(&ib) {
                let (nx, ny) = (
                    x.bytes().all(|c| c.is_ascii_digit()),
                    y.bytes().all(|c| c.is_ascii_digit()),
                );
                let o = match (nx, ny) {
                    (true, true) => num(x, y),
                    (true, false) => Ordering::Less,
                    (false, true) => Ordering::Greater,
                    (false, false) => x.cmp(y),
                };
                if o != Ordering::Equal {
                    return o;
                }
            }
            ia.len().cmp(&ib.len())
        }
    }
}

fn hash_of(v: &Version) -> u64 {
    let mut h = DefaultHasher::new();
    v.hash(&mut h);
    h.finish()
}

#[test]
fn precedence_matches_the_reference_and_the_laws_hold() {
    let mut r = Rng(0x5e3f_0002);
    // A pool with deliberate near-duplicates.
    let mut pool: Vec<String> = (0..400).map(|_| gen_version(&mut r)).collect();
    for i in 0..100 {
        let base = pool[i].split('+').next().unwrap().to_string();
        pool.push(format!("{base}+x"));
        pool.push(base);
    }
    let versions: Vec<Version> = pool.iter().map(|s| v(s)).collect();
    for (sa, a) in pool.iter().zip(&versions) {
        for (sb, b) in pool.iter().zip(&versions) {
            assert_eq!(
                a.cmp_precedence(b),
                reference_precedence(sa, sb),
                "{sa} vs {sb}"
            );
            assert_eq!(a.precedence_eq(b), a.cmp_precedence(b) == Ordering::Equal);
            // Ord refines precedence and agrees with ==.
            if a.cmp_precedence(b) != Ordering::Equal {
                assert_eq!(a.cmp(b), a.cmp_precedence(b));
            }
            assert_eq!(a == b, a.cmp(b) == Ordering::Equal, "{sa} vs {sb}");
            assert_eq!(a.cmp(b), b.cmp(a).reverse());
            if a == b {
                assert_eq!(hash_of(a), hash_of(b));
            }
        }
    }
}

#[test]
fn build_metadata_breaks_ties_deterministically() {
    let mut vs: Vec<Version> = [
        "1.0.0+01",
        "1.0.0",
        "1.0.0+1",
        "1.0.0+b",
        "1.0.0+0",
        "1.0.0+1.a",
    ]
    .iter()
    .map(|s| v(s))
    .collect();
    vs.sort();
    let sorted: Vec<String> = vs.iter().map(Version::to_string).collect();
    // Identifier by identifier: `1.a` starts with `1`, which is before `01`.
    assert_eq!(
        sorted,
        [
            "1.0.0",
            "1.0.0+0",
            "1.0.0+1",
            "1.0.0+1.a",
            "1.0.0+01",
            "1.0.0+b"
        ]
    );
}

#[test]
fn numbers_of_any_size() {
    let max = Num::new(u64::MAX);
    let next = max.succ();
    assert_eq!(next.to_string(), "18446744073709551616");
    assert!(next > max && next > u64::MAX);
    assert_eq!(next.succ().to_string(), "18446744073709551617");
    let big: Num = "99999999999999999999".parse().unwrap();
    assert_eq!(big.succ().to_string(), "100000000000000000000");
    assert!(big < big.succ());
    assert_eq!("0".parse::<Num>().unwrap(), 0);
    assert!("01".parse::<Num>().is_err());
    assert_eq!(format!("{:>6}", Num::new(42)), "    42");
    assert_eq!(format!("{:>22}", next), "  18446744073709551616");
    let a = v("18446744073709551616.0.0");
    let b = v("18446744073709551615.99.99");
    assert!(a > b);
}

#[test]
fn identifiers() {
    let p: Prerelease = "alpha.1.x-y".parse().unwrap();
    let ids: Vec<(bool, &str)> = p
        .identifiers()
        .map(|i| (i.is_numeric(), i.as_str()))
        .collect();
    assert_eq!(ids, [(false, "alpha"), (true, "1"), (false, "x-y")]);
    assert!(Prerelease::new("01").is_err());
    assert!(BuildMetadata::new("01").is_ok());
    assert_eq!(Prerelease::new("").unwrap(), Prerelease::EMPTY);
    assert!(Prerelease::EMPTY > p);
    assert!(BuildMetadata::EMPTY < BuildMetadata::new("0").unwrap());
}

#[test]
fn display_honours_width() {
    assert_eq!(format!("{:<9}|", v("1.2.3")), "1.2.3    |");
    assert_eq!(format!("{:>12}", v("1.2.3-rc.1")), "  1.2.3-rc.1");
}
