# forgo-rust

Rust libraries for the formats a Rust project runs on (YAML, TOML and
semantic versions), and the workspace's own developer tool, built on
them. No third-party dependencies, no `unsafe`, and each library held to
its specification's official test suite.

## The libraries

| Crate | What it does | Checked against | Speed |
|---|---|---|---|
| [forgo_lib_yaml](crates/forgo_lib_yaml) | YAML 1.2.2: parse, emit, and edit keeping a file's layout | all 402 cases of the official yaml-test-suite, plus the rules it skips | the fastest Rust YAML loader measured, with the fewest allocations |
| [forgo_lib_toml](crates/forgo_lib_toml) | TOML 1.1 and 1.0: parse, encode, and edit keeping a file's layout | every case of toml-test for both versions, also through toml-test's own runner | reads zed's Cargo.lock at about 700 MB/s: nearly 4× `toml`, 8× `toml_edit` |
| [forgo_lib_semver](crates/forgo_lib_semver) | Semantic Versioning 2.0.0: versions of any size, bumping, Cargo and npm requirements, exact set operations | the BNF on 80,000 inputs; node-semver's own test suite, with identical results; Cargo's requirement tests | the fastest version parsing measured |

Each library has:

- a README with examples that run as tests;
- a COMPLIANCE.md: what is checked, against what, and the results;
- a benchmark crate (`crates/<library>_bench`) whose RESULTS.md compares
  it with the other Rust crates for the format, including where they
  beat it;
- every public item documented, and a minimum supported Rust version
  (1.88) that CI checks.

## The developer tool: `cargo x`

[forgo_dev](crates/forgo_dev) runs as `cargo x` and uses those libraries:

```bash
cargo x new lib forgo_lib_widgets       # a crate, listed everywhere crates are listed
cargo x rm forgo_lib_widgets --yes      # and gone again, byte for byte
cargo x version bump forgo_lib_toml minor   # also updates requirements and Cargo.lock
cargo x toml get Cargo.toml workspace.members
cargo x semver max '^1.2' 1.2.0 1.9.3 2.0.0
```

It keeps every list of crates in step: the workspace members, the CI
matrices, the release choices and `Cargo.lock`. Each edit changes only
what it must; comments, layout and line endings elsewhere stay as they
were. See its [README](crates/forgo_dev/README.md).

## Other crates

| Crate | Type | Description |
|---|---|---|
| [forgo](crates/forgo) | binary | A CLI tool (binary `forgo`) |
| [forgo_lib_cli](crates/forgo_lib_cli) | library | Common CLI features, such as the OAuth device code flow |

## No third-party dependencies

`Cargo.lock` holds only this workspace's crates. CI fails on any other
crate that isn't listed in `.github/allowed-dependencies.txt` (`cargo x
deps check`). The list has only wasm-bindgen, for `cargo x new wasm`
crates, so adding anything else is a deliberate edit to it.

The benchmark crates are outside the workspace, with their own
lockfiles, so the crates they compare against never enter it. A daily
security audit checks every lockfile, theirs included.

## Layout

```text
forgo-rust/
├── Cargo.toml                  # the workspace
├── Makefile.toml               # cargo-make tasks CI runs per crate
├── .cargo/config.toml          # the `cargo x` alias
├── .github/
│   ├── actions/setup-rust/     # toolchain, caches, cargo-make, cargo-audit
│   ├── allowed-dependencies.txt
│   └── workflows/
│       ├── ci.yaml             # lint, MSRV, tests (Linux and Windows), dependencies
│       ├── release.yaml        # check and publish a merged version (manual)
│       └── audit.yaml          # daily security audit of every lockfile
└── crates/
    ├── forgo_lib_yaml/         # and forgo_lib_yaml_bench/
    ├── forgo_lib_toml/         # and forgo_lib_toml_bench/
    ├── forgo_lib_semver/       # and forgo_lib_semver_bench/
    ├── forgo_dev/              # cargo x
    ├── forgo_lib_cli/
    └── forgo/
```

## Building and testing

Prerequisites: [rustup](https://rustup.rs/), and for the tasks CI runs,
[cargo-make](https://github.com/sagiegurari/cargo-make) and
[cargo-audit](https://github.com/RustSec/rustsec/tree/main/cargo-audit).
Versions, TOML edits and release checks need nothing else: `cargo x` does
them.

```bash
cargo build --workspace
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace -- -D warnings
cargo doc --workspace --no-deps
```

The compliance tests use the official suites, fetched once (they aren't
committed). Without them those tests are skipped:

```bash
crates/forgo_lib_yaml/scripts/fetch-yaml-test-suite.sh
crates/forgo_lib_toml/scripts/fetch-toml-test.sh
crates/forgo_lib_semver/scripts/fetch-node-semver.sh   # needs Node.js
```

CI runs each crate's checks with cargo-make: `cargo make test-project
<crate>`, `fmt-check-project`, `clippy-check-project`,
`doc-check-project`.

## Releasing

A crate's version changes by pull request (`cargo x version set`), and
the Release workflow checks and publishes a merged version; see
[forgo_dev's README](crates/forgo_dev/README.md#releasing). Before a
library's first release, go through [PUBLISHING.md](PUBLISHING.md).

## License

[MIT](LICENSE).
