# forgo_lib_semver

[Semantic Versioning 2.0.0](https://semver.org) for Rust: versions, and
version requirements in Cargo's syntax and npm's. No dependencies, no
`unsafe`, `no_std` (with `alloc`).

- **Exact parsing.** [`Version::parse`] accepts exactly the specification's
  grammar, with numbers of any size (no `u64` limit). It agrees with the
  BNF on 80,000 generated inputs and with semver.org's published lists of
  valid and invalid versions ([COMPLIANCE.md](COMPLIANCE.md)).
- **Both requirement syntaxes.** Cargo's (the `semver` crate's rules,
  which Cargo uses) and npm's. The npm syntax matches node-semver 7.8.6 on
  all of about 448,000 results recorded from node-semver's own test suite.
- **Exact set operations.** Intersection, inclusion, equivalence, lowest
  version and simplification, across both syntaxes. Each answer comes with
  the version that proves it.

## Versions

```rust
use forgo_lib_semver::{Bump, Release, Version};

let v = Version::parse("1.0.0-rc.1+build.5")?;
assert_eq!((v.major.to_u64(), v.pre.as_str(), v.build.as_str()), (Some(1), "rc.1", "build.5"));

// Precedence (SemVer §11) ignores build metadata; `Ord` breaks the tie
// with it, so sorting is deterministic and agrees with `==`.
let (a, b) = (Version::parse("1.0.0+a")?, Version::parse("1.0.0+b")?);
assert!(a.precedence_eq(&b) && a != b && a < b);

// Bumping follows SemVer §7–8 and npm's `inc`.
assert_eq!(v.bump(Bump::Release)?.to_string(), "1.0.0");
assert_eq!(Version::parse("1.2.3")?.bump(Bump::Preminor(Some("beta")))?.to_string(), "1.3.0-beta.0");
assert_eq!(v.diff(&Version::parse("1.0.0")?), Some(Release::Major));

// No size limit.
let big = Version::parse("18446744073709551616.0.0")?;
assert!(big > Version::new(u64::MAX, 0, 0));
# Ok::<(), forgo_lib_semver::Error>(())
```

`Version::parse` is strict: `v1.2.3`, `1.2` and `01.2.3` are errors, with
the reason and the byte where it went wrong. [`Version::parse_lenient`]
accepts what npm's loose mode does (`v1.2.3`, `=1.2.3`, `1.2.3beta`), and
[`Version::coerce`] finds a version in free text (`"rust 1.88"` gives
`1.88.0`).

## Requirements

```rust
use forgo_lib_semver::{Req, Version};

let v = |s| Version::parse(s).unwrap();

// Cargo: a bare version means `^`, comparators are separated by commas.
let cargo = Req::cargo("1.4, <1.8")?;
assert!(cargo.matches(&v("1.7.2")) && !cargo.matches(&v("1.8.0")));
assert_eq!(cargo.to_string(), "^1.4, <1.8");

// npm: a bare version means `=`, plus `||`, hyphen ranges and x-ranges.
let npm = Req::npm("1.4 - 1.7 || ^2.1.0")?;
assert!(npm.matches(&v("2.9.0")));
assert_eq!(npm.to_string(), ">=1.4.0 <1.8.0-0||>=2.1.0 <3.0.0-0");

// In both, a pre-release matches only when a comparator names its
// major.minor.patch with a pre-release of its own.
let rc = Req::npm(">=2.0.0-rc.1")?;
assert!(rc.matches(&v("2.0.0-rc.2")) && !rc.matches(&v("2.1.0-rc.1")));

// The best of a list.
let published = [v("1.4.0"), v("1.7.3"), v("1.8.0")];
assert_eq!(cargo.max_satisfying(&published), Some(&published[1]));
# Ok::<(), forgo_lib_semver::Error>(())
```

| | Cargo ([`Req::cargo`]) | npm ([`Req::npm`]) |
|---|---|---|
| `1.2.3` | `^1.2.3` | `=1.2.3` |
| all must match | `>=1.2, <1.5` | `>=1.2 <1.5` |
| alternatives | (none) | `1.x \|\| >=2.5` |
| ranges | (none) | `1.2.3 - 2.3.4` |
| wildcards | `*`, `1.*`, `1.2.x` | `*`, `1.x`, `1.2.*`, `""` |
| `v` prefix, spaces after operators | no | yes |
| limits | none (the `semver` crate: `u64`, 32 comparators) | none (node-semver: 2^53 − 1, 256 characters) |

## Set operations

Exact, over the versions each requirement matches, and across syntaxes:

```rust
use forgo_lib_semver::Req;

let cargo = Req::cargo("^1.2")?;
let npm = Req::npm(">=1.2.0 <2.0.0-0")?;
assert!(cargo.is_equivalent(&npm));

// Each answer comes with the version that proves it.
let a = Req::npm("^1.2.3-rc.1")?;
let b = Req::npm(">=1.2.3")?;
assert!(!a.is_subset(&b));
assert_eq!(a.min_difference(&b).unwrap().to_string(), "1.2.3-rc.1");
assert_eq!(a.min_intersection(&b).unwrap().to_string(), "1.2.3");

// Unsatisfiable requirements, and the lowest version that matches.
assert!(Req::npm(">2 <1")?.is_unsatisfiable());
assert_eq!(Req::cargo(">1.2.3")?.min_version().unwrap().to_string(), "1.2.4");

// Drop what doesn't change the set.
assert_eq!(Req::npm("1.2.x || 1.2.3 || ^1.0.0")?.simplify().to_string(), ">=1.0.0 <2.0.0-0");
# Ok::<(), forgo_lib_semver::Error>(())
```

How: the endpoints of every comparator cut the version line into
finitely many pieces. In each piece, a requirement matches all releases or
none, and all pre-releases of a given `major.minor.patch` or none. So
comparing the two requirements piece by piece decides each question.
Random requirements are checked against direct matching (`tests/sets.rs`).

## npm's own API

[`npm`] mirrors node-semver's functions, with the same results:
`satisfies`, `valid_range`, `inc`, `coerce`, `min_version`,
`max_satisfying`, `gtr`, `subset`, `simplify_range` and the rest, with the
`loose` and `include_prerelease` options.

```rust
use forgo_lib_semver::npm::{self, IdentifierBase, NpmOptions};

let o = NpmOptions::default();
assert!(npm::satisfies("1.2.3-beta.4", ">=1.2.3-beta.1 <1.3", o));
assert_eq!(npm::inc("1.2.3", "prerelease", o, Some("beta"), IdentifierBase::One).as_deref(), Some("1.2.4-beta.1"));
assert_eq!(npm::valid_range(">=1.2 <2 || 3.x", o).as_deref(), Some(">=1.2.0 <2.0.0-0||>=3.0.0 <4.0.0-0"));
assert_eq!(npm::valid("v1.2.3", o).as_deref(), Some("1.2.3"));
```

node-semver computes `intersects` and `subset` with shortcuts; [`npm`]
gives the same answers, and [`Req`] the exact ones. For example,
`>=1 <3` is a subset of `>=1 <2 || >=2 <3`, but node-semver says it
isn't, because neither alternative covers it alone.

## Errors

```rust
use forgo_lib_semver::{ErrorKind, Part, Version};

let err = Version::parse("1.2.3-01").unwrap_err();
assert_eq!(err.kind(), ErrorKind::LeadingZero(Part::Pre));
assert_eq!(err.to_string(), "leading zero in a numeric pre-release identifier at byte 6");
```

[`Version::parse`]: https://docs.rs/forgo_lib_semver/latest/forgo_lib_semver/struct.Version.html#method.parse
[`Version::parse_lenient`]: https://docs.rs/forgo_lib_semver/latest/forgo_lib_semver/struct.Version.html#method.parse_lenient
[`Version::coerce`]: https://docs.rs/forgo_lib_semver/latest/forgo_lib_semver/struct.Version.html#method.coerce
[`Req::cargo`]: https://docs.rs/forgo_lib_semver/latest/forgo_lib_semver/struct.Req.html#method.cargo
[`Req::npm`]: https://docs.rs/forgo_lib_semver/latest/forgo_lib_semver/struct.Req.html#method.npm
[`Req`]: https://docs.rs/forgo_lib_semver/latest/forgo_lib_semver/struct.Req.html
[`npm`]: https://docs.rs/forgo_lib_semver/latest/forgo_lib_semver/npm/index.html
