# forgo_dev (`cargo x`)

The workspace's own developer tool, run as `cargo x` (an alias in
`.cargo/config.toml`). It adds and removes crates, and keeps every list
of crates in the repository in step:

- `[workspace].members` in the root `Cargo.toml`;
- the `project` matrix of each job in `.github/workflows/ci.yaml`;
- the release choices in `.github/workflows/release.yaml` (crates made
  with `--public`);
- `Cargo.lock`.

forgo_dev depends only on forgo libraries: the workflows are edited with
[forgo_lib_yaml](../forgo_lib_yaml), which keeps everything around the
edited list byte for byte.

## Commands

```bash
cargo x new <lib|bin|wasm> <name> [--public]
cargo x rm [<lib|bin|wasm>] <name> [--yes]
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
- Adds `crates/<name>` to the workspace members, kept sorted, one per
  line.
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
  such as benchmark crates, with a `path = "../<name>"` dependency;
- a crate with uncommitted changes (`git status`). Files exactly as
  `cargo x new` wrote them don't count, so a crate can be removed right
  after it was created.

When the crate was the release workflow's `default`, `default` moves to
the first remaining choice. Lines in `.github` that still mention the
crate (a `matrix.project == '<name>'` condition, a path) are listed
afterwards, to check by hand.

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

The binary is a thin wrapper around `forgo_dev::run(args, root)`, which
returns the report or an `Error` (`Usage` for a wrong command line, exit
code 2; `Failed` otherwise, exit code 1). Each edit is also a pure
function from text to text:

| Module | Edits |
|---|---|
| `workspace` | `add_member`, `remove_member`, `members` |
| `workflows` | `edit_ci_matrix`, `edit_release_options`, `mentions` |
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
| `commands` | `new` and `rm` on scratch workspaces: byte-exact round trips, dry runs, every refusal, uncommitted changes |

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
against `.github/allowed-dependencies.txt`, which lists only wasm-bindgen
and the crates it pulls in; adding anything else is a deliberate edit to
that file. Benchmark crates (`crates/*_bench`) have their own workspace
and lockfile.

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
