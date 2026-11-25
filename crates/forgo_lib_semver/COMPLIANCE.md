# Compliance

There is no official test suite for Semantic Versioning (semver/semver
issues #718 and #833 ask for one). So each part of this crate is checked
against its definition's own source, and, where possible, against an
independent implementation run directly:

| Part | Checked against | Result |
|---|---|---|
| Versions (SemVer 2.0.0) | semver.org's lists (31 valid, 40 invalid); the specification's examples and §11 chain; the BNF as an independent validator on 80,000 generated inputs | all agree |
| npm syntax | node-semver 7.8.6, run on its own test suite: 5,723 recorded calls and 442,040 cross-product results | all identical |
| Cargo syntax | the `semver` crate's own requirement tests (1.0.28) | all pass |
| Set operations | direct matching: witnesses for every "yes", about 1,100 versions for every "no", on 12,000 random pairs | all agree |

## Versions

`Version::parse` accepts exactly the grammar of
[SemVer 2.0.0](https://semver.org/spec/v2.0.0.html):

- three numbers without leading zeros, of any size;
- optional pre-release identifiers: numbers without leading zeros, or
  identifiers with a letter or `-` (so `--` and `0a` are identifiers);
- optional build metadata: any identifiers, leading zeros allowed;
- ASCII only, nothing around it.

**Checks** (`tests/spec.rs`):

1. semver.org's FAQ links regex101 lists of 31 valid and 40 invalid
   versions; all are classified correctly, and the valid ones print back
   exactly. Edge cases such as `1.2.3--` (valid) and `①.2.3` (invalid)
   are added (semver/semver#833).
2. The specification's examples from §2, §9, §10 and §11 parse, and §11's
   precedence chain (`1.0.0-alpha < 1.0.0-alpha.1 < … < 1.0.0`) holds.
3. The BNF, transcribed rule by rule as a recursive validator, is the
   oracle. Random versions from the grammar (numbers up to 50 digits) and
   one- and two-character mutations of them must be accepted exactly when
   the oracle accepts them: 54,000 valid and 26,000 invalid inputs.
4. Precedence is compared with a reference implementation of §11 written
   on the strings, for every pair of 600 versions. `==`, `Ord` and `Hash`
   agree: `a == b` exactly when `a.cmp(&b)` is `Equal`, and equal
   versions hash equally.

**Precedence and order.** `cmp_precedence` is §11 exactly; build metadata
is ignored. `Ord` is precedence, then build metadata to break ties, so
that `Ord` agrees with `==` (which compares everything, as the derived
`Hash` does). The tie-break order is the `semver` crate's: identifier by
identifier, numbers first and by value, then by length (`1` before `01`).

## npm syntax

npm's syntax is defined by node-semver's code: regular expressions applied
in a fixed order, with their quirks. The `npm` module follows that code
step by step, and runs node-semver's own regular expressions (generated
from its source by `scripts/gen-npm-patterns.js`) on an engine with
JavaScript's semantics.

**Checks** (`tests/npm.rs`): `scripts/fetch-node-semver.sh` downloads
node-semver 7.8.6 at its commit and runs its test suite (test/functions
and test/ranges) under `scripts/capture-node-semver.js`. That script
records every call the tests make and node-semver's result. It then adds a
cross product: every range those calls used (228), under all four option
combinations, with `validRange`, `minVersion`, and `satisfies` for each of
213 versions.

- 5,723 calls to 36 of its functions (`satisfies`, `validRange`, `inc`,
  `coerce`, `intersects`, `subset`, `minVersion`, `outside`, …):
  identical results.
- 442,040 cross-product results: identical.
- 216 calls and rows use numbers above 2^53 − 1 or versions over 256
  characters, which node-semver rejects. They are counted, not compared
  (see below).
- 14 calls pass arguments Rust can't express (a regular expression, an
  object with `toString`) and are skipped.

**Deliberate differences.** Numbers have no size limit, and versions no
length limit. node-semver rejects numbers above 2^53 − 1
(`Number.MAX_SAFE_INTEGER`) and versions over 256 characters; these are
limits of JavaScript's numbers, not of npm's syntax. Within node-semver's
regular expressions, identifiers stay capped at about 250 characters as
node-semver caps them.

## Cargo syntax

Cargo parses requirements with the `semver` crate (`VersionReq`), so its
rules are the definition. `tests/cargo.rs` ports that crate's own
requirement tests (1.0.28): matching, canonical text and errors, all
passing. Errors are checked by kind and position, since the messages
differ.

**Deliberate differences.** No limit on the number of comparators (the
`semver` crate stops at 32), and numbers of any size (it stops at
`u64::MAX`).

## Set operations

`Req::intersects`, `is_subset`, `is_equivalent`, `min_version`,
`min_intersection`, `min_difference` and `simplify` are exact, over the
versions each requirement matches.

**How.** A requirement is a union of pieces. Each piece has an interval of
releases, an interval of pre-releases, and the `major.minor.patch` whose
pre-releases it admits (all, with npm's `include_prerelease`). The
interval endpoints cut the version line into finitely many cells. Inside
a cell, a piece matches either every release or none, and either every
pre-release of a given `major.minor.patch` or none. So checking each kind
of version in each cell is exact.

**Checks** (`tests/sets.rs`): 12,000 random pairs of requirements, in both
syntaxes and with npm options:

- **Every "yes" proves itself.** `min_intersection` and `min_difference`
  return a version, which must really match (or not) when tested
  directly.
- **Every "no" is checked against a universe** of 1,088 versions: all
  `M.m.p` from 0 to 3, each with 16 pre-releases. The pre-releases include
  the ones bounds step to, such as `alpha.0`.
- **Every witness is the lowest** qualifying version of that universe.
- **The model agrees with direct matching** for 150 requirements on every
  version of the universe.
- **`simplify` keeps the set** for 1,500 requirements.

node-semver's own `intersects` and `subset` use shortcuts that differ from
the exact answer. For example, `subset(">=1 <3", ">=1 <2 || >=2 <3")` is
false because neither alternative covers the range alone. The `npm`
module reproduces node-semver's answers; `Req` gives the exact ones.

## Reproducing

```bash
crates/forgo_lib_semver/scripts/fetch-node-semver.sh   # needs Node.js
SEMVER_NODE_REQUIRED=1 cargo test -p forgo_lib_semver -- --nocapture
```
