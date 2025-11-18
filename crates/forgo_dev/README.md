# Adding a New Crate to `forgo-rust`

This repo is a Cargo **workspace**. New projects live under `crates/` and plug into shared tooling (fmt, clippy, CI, release).

## TL;DR (Conventions)

| Topic              | Rule                                                               |
| ------------------ | ------------------------------------------------------------------ |
| Naming (libs)      | Must start with `forgo_lib_` and must NOT end with `_wasm`         |
| Naming (libs/wasm) | Must start with `forgo_lib_` and must end with `_wasm`             |
| Naming (bins)      | Must start with `forgo_` but not follow library naming conventions |
| Location           | `crates/<name>`                                                    |
| Edition            | `2024`                                                             |
| Visibility         | **Private by default** (`publish = false`), opt-in with `--public` |
| Docs               | Each crate includes a `README.md`                                  |
| Deps               | Prefer std; feature-gate optional deps                             |
| CI                 | Matrix auto-updated by `cargo x`                                   |
| Release            | Only publish crates you intentionally list in the release workflow |

---

## Quick commands: scaffold with `cargo x`

We ship a tiny repo-native tool (`forgo_dev`) aliased as `cargo x`.

**Alias** (already in repo): `.cargo/config.toml`

```toml
[alias]
x = "run -p forgo_dev --"
```

**Create a library** (must start with `forgo_lib_`):

```bash
cargo x new lib forgo_lib_<name> [--public]
```

**Create a binary** (must **not** start with `forgo_lib_`):

```bash
cargo x new bin forgo_<name> [--public]
```

**Create a WASM library** (must start with `forgo_lib_`):

```bash
cargo x new wasm forgo_lib_<name> [--public]
```

**What it does**

- Creates `crates/<name>` via `cargo new --edition 2024`
- Sets visibility (default **private**; `--public` flips to `publish = true`)
- Adds minimal `Cargo.toml` metadata (license, description, repository, readme)
- Scaffolds a tiny `README.md`
- Adds the crate to the **workspace members** (root `Cargo.toml`)
- Appends the crate to **CI matrices** in `.github/workflows/ci.yaml` (lint + test)
- With `--public`, appends crate to options in `.github/workflows/release.yaml`(crates.io publish)
- For `new wasm`, also writes WASM boilerplate in `Cargo.toml`:
  - `[lib] crate-type = ["cdylib","rlib"]`
  - `[features] default = ["wasm"]`
  - and optional `wasm-bindgen` dependency

**Examples**

```bash
cargo x new lib forgo_lib_data_indexes
cargo x new bin forgo_data_inspect
cargo x new lib forgo_lib_widgets --public
cargo x new wasm forgo_lib_data_wasm
```

**Change visibility later**

```toml
# crates/<name>/Cargo.toml
[package]
publish = false  # private (default)
# publish = true # public (opt-in)
```

---

## If you must do it manually (rare)

> Prefer `cargo x`. Only do this if you intentionally want to avoid the automation.

1. **Create**

```bash
cargo new --lib crates/forgo_lib_<name> --edition 2024
# or
cargo new --bin  crates/forgo_<name>      --edition 2024
```

2. **Set metadata / visibility** in `crates/<name>/Cargo.toml`

```toml
[package]
publish = false   # default to private
license = "MIT"
description = "<one-line purpose>"
repository = "https://github.com/forgo/forgo-rust"
readme = "README.md"
```

3. **Workspace members** (root `Cargo.toml`)

```toml
[workspace]
members = [
  # ...
  "crates/forgo_lib_<name>",  # or: "crates/forgo_<name>"
]
```

4. **CI matrix** (`.github/workflows/ci.yaml`)
   Add your crate name to each `project: [ ... ]` list in both `lint` and `test` jobs.

---

## Minimal `Cargo.toml` patterns

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

[features]
default = []
# serde = ["dep:serde"]

[dependencies]
# serde = { version = "1", features = ["derive"], optional = true }
```

**Binary**

```toml
[package]
name = "forgo_<name>"
version = "0.0.0"
edition = "2024"
publish = false
license = "MIT"
description = "<one-line purpose>"
repository = "https://github.com/forgo/forgo-rust"
readme = "README.md"

[dependencies]
# forgo_lib_something = { path = "../forgo_lib_something" }

[[bin]]
name = "forgo_<name>"
path = "src/main.rs"
```

**WASM-exposed lib (special case)**

```toml
[lib]
crate-type = ["cdylib", "rlib"]

[features]
default = ["wasm"]
wasm = ["dep:wasm-bindgen"]

[dependencies]
wasm-bindgen = { version = "0.2", optional = true }
```

---

## Style & hygiene (essentials)

- Keep modules shallow and obvious.
- Re-export the few public types from `lib.rs` (`pub use ...`).
- Use crate-local `Error` + `Result<T, Error>`; avoid leaking foreign errors.
- Write short `///` docs with one minimal example.
- Prefer colocated unit tests; add golden tests for formats/serializers.
- Feature-gate optional dependencies (e.g., `serde`) to keep core lean.

---

## Quick checks

From repo root:

```bash
cargo fmt --all
cargo clippy --all -- -D warnings
cargo test --all
```

Per project:

```bash
cargo make fmt-check-project <crate>
cargo make clippy-check-project <crate>
cargo make test-project <crate>
```

---

## PR checklist

- [ ] Name follows convention (`forgo_lib_*` for libs, `forgo_*` for bins)
- [ ] Lives under `crates/<name>` and uses edition `2024`
- [ ] `publish = false` unless intentionally public
- [ ] Added to workspace members (or created via `cargo x`)
- [ ] README present (purpose + quick start)
- [ ] Optional deps are feature-gated
- [ ] CI matrix includes the crate (or created via `cargo x`)
- [ ] `fmt`, `clippy -D warnings`, and `test` pass
