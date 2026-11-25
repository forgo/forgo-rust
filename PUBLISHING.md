# Publishing a library

None of the libraries has been published yet. This is what to go through
before the first release of forgo_lib_yaml, forgo_lib_toml and
forgo_lib_semver, and before each release after that.

## State today

| | forgo_lib_yaml | forgo_lib_toml | forgo_lib_semver |
|---|---|---|---|
| Version | 0.0.0 | 0.0.0 | 0.0.0 |
| `publish` | `true` | `false` | `false` |
| In the Release workflow's choices | yes | no | no |
| README, COMPLIANCE.md, tests/README.md | yes | yes | yes |
| RESULTS.md (bench crate) | yes | yes | yes |
| CHANGELOG.md | "Unreleased" | "Unreleased" | "Unreleased" |
| Minimum Rust version (checked in CI) | 1.88 | 1.88 | 1.88 |
| Package size (`cargo package --list`, uncompressed) | about 1 MB | about 0.3 MB | about 1 MB |

All three already have a description, `license = "MIT"`, the repository,
the README, keywords and categories in their manifests. Every public item
is documented, and `cargo doc` passes with warnings denied, as docs.rs
will build it.

## Before the first release

1. **The name.** Check that the crate's name is still free on crates.io
   (`cargo search <name>`).
2. **The license text.** The root `LICENSE` isn't inside any crate's
   directory, so it isn't in the package. Copy it into the crate, or set
   `license-file`.
3. **Attributions.** Each package ships its tests. Credit what they
   quote:
   - forgo_lib_semver: the specification's examples and semver.org's
     lists of valid and invalid versions (CC BY 3.0). The credit is in
     `tests/spec.rs`; add it to COMPLIANCE.md too.
   - forgo_lib_yaml: `tests/spec/` quotes the YAML 1.2.2 specification's
     examples. The files link to the specification but don't credit it:
     confirm the specification's terms, and add the credit.
   - The official suites (yaml-test-suite and toml-test, both MIT) and
     node-semver's tests (ISC) are fetched by scripts and git-ignored, so
     they aren't in the packages. `cargo package --list` confirms it.
4. **README links.** Links from a README to files outside its crate work
   on GitHub but not on crates.io. Make them absolute
   (`https://github.com/forgo/forgo-rust/blob/main/crates/...`):
   - forgo_lib_yaml: `../forgo_lib_yaml_bench` and
     `../forgo_lib_yaml_bench/RESULTS.md`;
   - forgo_lib_toml: `../forgo_lib_toml_bench/RESULTS.md`;
   - forgo_lib_semver: `../forgo_lib_semver_bench/RESULTS.md`.
5. **Dependencies between the libraries.** forgo_lib_toml's tests read
   JSON with forgo_lib_yaml (a path dev-dependency). Cargo drops path
   dev-dependencies from a published manifest, so nothing else is
   needed, but publish forgo_lib_yaml first if that ever becomes a normal
   dependency.
6. **Version and changelog.** By pull request:
   - `cargo x version set <crate> 0.1.0`;
   - CHANGELOG.md's "Unreleased" becomes "0.1.0" with the date;
   - `publish = true`, and the crate added to the `project` options in
     `.github/workflows/release.yaml`.
7. **A dry run locally.** `cargo publish -p <crate> --dry-run` builds the
   packaged crate on its own, from the files that would be uploaded.
8. **Release.** After the pull request is merged, run the Release
   workflow with the crate and its version. It's a dry run by default:
   it checks the version is the crate's, then tests and packages the
   crate. Then run it again with `dry_run` off. It publishes to crates.io
   (needs the `CARGO_REGISTRY_TOKEN` secret) and creates the GitHub
   release `<crate>@<version>`.
9. **After.** Check the crate's page on crates.io (the README renders,
   the links work) and its docs.rs build.

## Every release after

1. A pull request with `cargo x version bump <crate> <release>` (or
   `set`), and the CHANGELOG.md entry. `cargo x version` also updates
   the requirements on the crate and Cargo.lock.
2. CI passes, including the compliance suites, which CI requires.
3. Refresh the comparison when the release changes performance: in the
   bench crate, `cargo run --release --bin report > RESULTS.md`, and
   update the README's numbers from it.
4. Release from the merged commit (dry run, then publish).

Version numbers follow Semantic Versioning: before 1.0.0, a change that
breaks the API bumps the minor version.
