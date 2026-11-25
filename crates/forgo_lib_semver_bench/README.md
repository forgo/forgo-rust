# forgo_lib_semver_bench

Compares forgo_lib_semver with the other Rust semver crates, and, for npm's
range syntax, with node-semver itself. Not part of the workspace: it has its
own `Cargo.lock`, so the crates it compares never enter the main lockfile.

```bash
crates/forgo_lib_semver/scripts/fetch-node-semver.sh   # once; needs Node.js
cargo run --release --manifest-path crates/forgo_lib_semver_bench/Cargo.toml --bin report > crates/forgo_lib_semver_bench/RESULTS.md
```

`--quick` runs fewer timing rounds. The latest report is
[RESULTS.md](RESULTS.md).

## What it measures

| Section | Inputs | Measures |
|---|---|---|
| Parsing versions: compliance | semver.org's 31 valid and 40 invalid versions; 40,000 versions generated from the SemVer BNF and mutated | agreement with the specification (the BNF as oracle), numbers above `u64` |
| npm ranges: compliance | 4,000 random ranges (odd spacing, prefixes, x-ranges, hyphen ranges, `\|\|`) × 24 versions | same verdict and same `satisfies` results as node-semver 7.8.6, run with Node.js |
| Cargo requirements: compliance | 20,000 random requirements × 24 versions | same verdict, canonical text and `matches` as the `semver` crate |
| Performance | versions and requirements as published (`Cargo.toml`, `package.json`) | ns per parse and per match, ms per sort of 100,000 versions, allocations, `size_of` |

Timings are medians of 21 rounds, each about 20 ms; allocations are
counted with a counting global allocator, separately from timing.

## Crates

| Crate | Version | Syntax |
|---|---|---|
| forgo_lib_semver | (this repo) | SemVer 2.0.0; Cargo and npm requirements |
| [semver](https://crates.io/crates/semver) | 1.0.28 | SemVer 2.0.0; Cargo requirements (Cargo's own) |
| [js-semver](https://crates.io/crates/js-semver) | 1.0.2 | npm |
| [node-semver](https://crates.io/crates/node-semver) | 2.2.0 | npm |
| [nodejs-semver](https://crates.io/crates/nodejs-semver) | 7.0.0 | npm |
| [semver_rs](https://crates.io/crates/semver_rs) | 0.2.0 | npm |
| [deno_semver](https://crates.io/crates/deno_semver) | 0.10.1 | npm (Deno's) |
| [semver-parser](https://crates.io/crates/semver-parser) | 0.10.3 | SemVer 2.0.0 |
| [versions](https://crates.io/crates/versions) | 8.0.1 | SemVer and other schemes |
| [lenient_semver](https://crates.io/crates/lenient_semver) | 0.4.2 | lenient |

`src/lib.rs` has the adapters (one function per crate and task) and the
input generators; `src/bin/report.rs` runs everything;
`other/node_eval.js` asks node-semver for its answers.
