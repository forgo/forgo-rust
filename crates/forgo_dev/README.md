# forgo_dev (`cargo x`)

The workspace's own developer tool, run as `cargo x` (an alias in
`.cargo/config.toml`). It adds and removes crates, sets their versions,
reads and edits TOML files, and checks the workspace for CI. It keeps
every list of crates in the repository in step:

- `[workspace].members` in the root `Cargo.toml`;
- the `project` matrix of each job in `.github/workflows/ci.yaml`;
- the release choices in `.github/workflows/release.yaml` (crates made
  with `--public`);
- `Cargo.lock`.

forgo_dev depends only on forgo libraries. Manifests and `Cargo.lock` are
read and edited with [forgo_lib_toml](../forgo_lib_toml), the workflows
with [forgo_lib_yaml](../forgo_lib_yaml), and versions are checked with
[forgo_lib_semver](../forgo_lib_semver). Each edit keeps everything else
in the file byte for byte.

## Commands

```bash
cargo x new <lib|bin|wasm> <name> [--public]
cargo x rm [<lib|bin|wasm>] <name> [--yes]
cargo x version <get|set|bump> <crate> ...
cargo x toml <get|set> <file> <key> ...
cargo x semver <validate|explain|compare|sort|bump|matches|max|min> ...
cargo x ci matrix
cargo x deps check
cargo x help
```

### `cargo x new`

```console
$ cargo x new lib forgo_lib_widgets --public
Created crates/forgo_lib_widgets (lib, public):
  update Cargo.toml
  update .github/workflows/ci.yaml
  update .github/workflows/release.yaml
  create crates/forgo_lib_widgets/Cargo.toml
  create crates/forgo_lib_widgets/README.md
  create crates/forgo_lib_widgets/src/lib.rs
  update Cargo.lock
```

- Writes `Cargo.toml`, `README.md` and `src/lib.rs` (or `src/main.rs`)
  under `crates/<name>`, with edition 2024 and `publish = false`.
- Adds `crates/<name>` to the workspace members, kept sorted and in the
  array's own style (one per line, or on one line), with each member's
  comments.
- Adds the name to every CI matrix list, kept sorted, in the list's own
  style (`[a, b]` or `- a` lines).
- With `--public`, sets `publish = true` and adds the crate to the release
  workflow's choices.
- `wasm` crates also get `crate-type = ["cdylib", "rlib"]` and an
  optional `wasm-bindgen` dependency behind a default `wasm` feature.

### `cargo x rm`

```console
$ cargo x rm forgo_lib_widgets
Would remove crates/forgo_lib_widgets:
  update Cargo.toml
  update .github/workflows/ci.yaml
  update .github/workflows/release.yaml
  delete crates/forgo_lib_widgets/ (3 files)
  update Cargo.lock
Nothing was changed. To remove it, run: cargo x rm forgo_lib_widgets --yes
```

Without `--yes` (`-y`), `rm` only reports what it would do. It refuses:

- `forgo_dev` itself;
- anything that isn't a workspace member at `crates/<name>`;
- a crate other crates depend on: workspace members (from
  `cargo metadata`), and crates under `crates/` outside the workspace,
  such as benchmark crates, with a path dependency on it (in any of their
  dependency tables);
- a crate with uncommitted changes (`git status`). Files exactly as
  `cargo x new` wrote them don't count, so a crate can be removed right
  after it was created.

When the crate was the release workflow's `default`, `default` moves to
the first remaining choice. Lines in `.github` that still mention the
crate (a `matrix.project == '<name>'` condition, a path) are listed
afterwards, to check by hand.

### `cargo x version`

A crate's version, checked as Semantic Versioning 2.0.0:

```console
$ cargo x version get forgo_lib_toml
0.0.0
$ cargo x version set forgo_lib_toml 0.1.0
forgo_lib_toml: 0.0.0 → 0.1.0
  update crates/forgo_lib_toml/Cargo.toml
  update Cargo.lock
$ cargo x version bump forgo_lib_toml prerelease rc
forgo_lib_toml: 0.1.0 → 0.1.1-rc.0
  update crates/forgo_lib_toml/Cargo.toml
  update Cargo.lock
$ cargo x version set forgo_lib_toml v1.0.0
error: `v1.0.0` isn't a version: unexpected character 'v' in the major version at byte 0
```

- `set` and `bump` keep the workspace consistent. Every path dependency
  on the crate whose version requirement the new version wouldn't meet
  gets the new version as its requirement: in members, in crates outside
  the workspace, and in `[workspace.dependencies]`. Requirements it still
  meets are left alone. The crate's entry in `Cargo.lock` changes with
  it, and `cargo metadata --locked` checks the result.
- A version inherited from `[workspace.package]`
  (`version.workspace = true`) is set there, for every crate that
  inherits it.
- `bump` takes `major`, `minor`, `patch`, `premajor`, `preminor`,
  `prepatch`, `prerelease` or `release`. The pre-release ones take an
  identifier (`rc`, `beta`).
- `--dry-run` shows the changes and makes none.

Releases use it: a version changes by pull request, and the release
workflow only checks that the crate's version is the one being released
(see [Releasing](#releasing)).

### `cargo x toml`

Any value in any TOML file, by key:

```console
$ cargo x toml get Cargo.toml workspace.members[0]
crates/forgo
$ cargo x toml get crates/forgo_dev/Cargo.toml dependencies.forgo_lib_toml
{ path = "../forgo_lib_toml" }
$ cargo x toml set crates/forgo_dev/Cargo.toml package.edition '"2024"'
$ cargo x toml set --string crates/forgo_dev/Cargo.toml package.license MIT
```

- Keys are TOML keys, dotted for nested tables (`"quoted"` when needed),
  with `[N]` for an array item: `package[3].version` in a `Cargo.lock`.
- `get` prints a string as its text and anything else as TOML. It exits
  with code 1 when the key isn't set.
- `set` takes a TOML value (`'"text"'`, `42`, `true`, `[1, 2]`), or a
  string with `--string`. It makes missing tables, keeps the comments
  and layout of the rest of the file, and writes it atomically.

### `cargo x ci` and `cargo x deps`

- `cargo x ci matrix` prints the crates that declare a minimum Rust
  version, as a GitHub Actions matrix. The CI's `msrv` job builds each
  one with its own version:
  `{"include":[{"crate":"forgo_lib_toml","msrv":"1.88"}, …]}`.
- `cargo x deps check` makes sure every third-party crate in `Cargo.lock`
  (one with a `source`) is listed in `.github/allowed-dependencies.txt`.
  It exits with code 1 if not.

### `cargo x semver`

Versions and requirements, with [forgo_lib_semver](../forgo_lib_semver):
exact SemVer 2.0.0, numbers of any size, requirements in Cargo's syntax or
npm's (`--npm`).

```console
$ cargo x semver validate 1.2.3 v1.2.3
1.2.3: valid
v1.2.3: invalid: unexpected character 'v' in the major version at byte 0 (--lenient accepts a `v` or `=` prefix)
$ cargo x semver bump premajor 1.2.3 rc
2.0.0-rc.0
$ cargo x semver max '^1.2' 1.2.0 1.9.3 2.0.0
1.9.3
$ cargo x semver explain --npm '^1.2.3 || 2.x'
requirement  ^1.2.3 || 2.x (npm)
canonical    >=1.2.3 <2.0.0-0||>=2.0.0 <3.0.0-0
lowest       1.2.3
$ printf '1.0.0+b\n1.0.0+a\n0.9.0\n' | cargo x semver sort
0.9.0
1.0.0+a
1.0.0+b  (same precedence as the line above)
```

| Command | Answers |
|---|---|
| `validate <version>...` | is each a valid version? |
| `explain <version or requirement>` | its parts, or its canonical form and lowest version |
| `compare <a> <b>` | `<`, `=` or `>` by precedence (noting differing build metadata) |
| `sort [<version>...]` | sorted, ties of precedence marked (standard input if none) |
| `bump <release> <version> [<id>]` | the next version (`major` … `prerelease`, `release`) |
| `matches <requirement> <version>...` | does each meet it? |
| `max`/`min <requirement> [<version>...]` | the highest or lowest that meets it |

`--json` prints JSON; `--lenient` reads versions as npm does loosely.
Exit codes: 0 yes, 1 no (invalid, doesn't match, none found), 2 usage
mistake, so scripts can branch on them.

## Names

| Kind | Rule | Example |
|---|---|---|
| `lib` | starts with `forgo_lib_`, doesn't end with `_wasm` | `forgo_lib_storage` |
| `wasm` | starts with `forgo_lib_`, ends with `_wasm` | `forgo_lib_storage_wasm` |
| `bin` | neither of those (by convention `forgo_<name>`) | `forgo_inspect` |

Every name is `[a-z][a-z0-9_]*`, at most 64 characters, and not a name
Windows reserves (`con`, `nul`, `com1`, …). A name is never a path, so
nothing outside `crates/` can be created or deleted.

## How changes are made

1. Check everything: the name, the workspace, dependents, git status.
2. Compute every file change in memory. If a file can't be edited exactly,
   stop with an error before anything is written. That includes a list
   with an item that isn't a plain string (an alias, a number), a matrix
   computed with `${{ … }}`, or a layout forgo_lib_yaml can't keep (bare
   CR line breaks).
3. Write each file next to its target, then rename it into place, keeping
   its permissions. A crate directory is deleted last.
4. Run `cargo update --workspace --offline`, so `Cargo.lock` lists
   exactly the workspace's crates (a failure here is a warning: the next
   build updates it anyway).

`cargo x new` followed by `cargo x rm --yes` leaves every file as it
was, `Cargo.lock` included (`tests/commands.rs` checks this).

## Library

The binary is a thin wrapper around `forgo_dev::run(args, root)`. It
returns the report, or an `Error`:

| Error | Meaning | Exit code |
|---|---|---:|
| `Usage` | a wrong command line | 2 |
| `Unmet` | a check didn't hold (a key isn't set, a dependency isn't allowed); the report is the output | 1 |
| `Failed` | anything else | 1 |

| Module | Does |
|---|---|
| `workspace` | `add_member`, `remove_member`, `members` (pure functions from text to text); reading manifests and their dependency tables |
| `workflows` | `edit_ci_matrix`, `edit_release_options`, `mentions` (text to text) |
| `version` | `cargo x version` |
| `tomlcmd` | `cargo x toml` |
| `ci`, `deps` | `cargo x ci matrix`, `cargo x deps check` |
| `semver` | `cargo x semver` |
| `templates` | the files `new` writes |
| `fsx` | `Plan`: file changes applied together |
| `metadata` | workspace packages from `cargo metadata` |

## Tests

```bash
cargo test -p forgo_dev
```

| Target | Covers |
|---|---|
| `workflows` | exact output of the list edits: styles, comments, quoting, CRLF, no final newline, `default`, errors |
| `workspace` | exact output of the members edits; `default-members`, other tables, CRLF, errors |
| `cli` | command lines and names |
| `semver` | every `cargo x semver` command, text and JSON, exit codes |
| `commands` | `new` and `rm` on scratch workspaces: byte-exact round trips, dry runs, every refusal, uncommitted changes |
| `manifests` | `version` (inherited versions, requirements, Cargo.lock checked by `cargo metadata --locked`, every bump), `toml`, `ci matrix`, `deps check` on scratch workspaces |
| `repo_workflows` | this repository's own workflows: crates can be added to and removed from their lists, byte for byte |

CI runs them on Linux and Windows.

## Adding a crate by hand (rare)

Prefer `cargo x new`. Otherwise:

1. Create `crates/<name>` with a `Cargo.toml` like the ones below.
2. Add `"crates/<name>"` to `members` in the root `Cargo.toml`.
3. Add the name to each `project` list in `.github/workflows/ci.yaml`,
   and for a public crate to the `options` in `release.yaml`.
4. Run `cargo update --workspace`.

**Library**

```toml
[package]
name = "forgo_lib_<name>"
version = "0.0.0"
edition = "2024"
publish = false
license = "MIT"
description = "<one-line purpose>"
repository = "https://github.com/forgo/forgo-rust"
readme = "README.md"

[dependencies]
```

**Binary**: the same, plus

```toml
[[bin]]
name = "forgo_<name>"
path = "src/main.rs"
```

**WebAssembly library**: the library manifest, plus

```toml
[lib]
crate-type = ["cdylib", "rlib"]

[features]
default = ["wasm"]
wasm = ["dep:wasm-bindgen"]

[dependencies]
wasm-bindgen = { version = "0.2", optional = true }
```

## Dependencies

The workspace has no third-party dependencies. CI checks `Cargo.lock`
against `.github/allowed-dependencies.txt` with `cargo x deps check`. The
list holds only wasm-bindgen and the crates it pulls in, so adding
anything else is a deliberate edit to that file. Benchmark crates
(`crates/*_bench`) have their own workspace and lockfile. The daily
security audit (`.github/workflows/audit.yaml`) checks every lockfile,
theirs included, and opens an issue for each new advisory.

## Checks

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -p <crate> -- -D warnings
cargo test -p <crate>
```

or per project, as CI runs them: `cargo make fmt-check-project <crate>`,
`clippy-check-project`, `doc-check-project`, `test-project`.

## PR checklist

- [ ] Created with `cargo x new` (or the manual steps above)
- [ ] `publish = false` unless intentionally public
- [ ] README says what the crate is for
- [ ] No new third-party dependencies (or `.github/allowed-dependencies.txt` updated on purpose)
- [ ] `fmt`, `clippy -D warnings`, docs and tests pass

## Releasing

`.github/workflows/release.yaml` publishes a crate whose version is
already merged:

1. Open a pull request with `cargo x version set <crate> <version>` (or
   `bump`), and merge it.
2. Run the Release workflow with the crate and that version. It's a dry
   run by default: it checks that the version is valid and is the
   crate's, then tests and packages the crate from the locked
   dependencies.
3. Run it again with `dry_run` off to publish to crates.io and create a
   GitHub release tagged `<crate>@<version>` at that commit.

The workflow's inputs reach the shell only as environment variables, and
the crates.io token is set only on the publish step.
