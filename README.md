# forgo-rust

A monorepo of Rust projects.

## Projects Overview

| Project         | Type       | Description                                                                      | Location               |
| --------------- | ---------- | -------------------------------------------------------------------------------- | ---------------------- |
| `forgo_lib_cli` | Library    | A library to help build common features into your CLI (e.g. - device code flow). | `crates/forgo_lib_cli` |
| `forgo`         | Executable | A CLI tool for doing awesome things. (binary: `forgo`)                           | `crates/forgo`         |

## Repository Structure

```bash
forgo-rust/
├── .gitignore              # Git ignore file
├── Cargo.lock
├── Cargo.toml              # Workspace manifest
├── LICENSE
├── Makefile.toml
├── README.md               # Repo documentation
├── rust-toolchain.toml     # Toolchain config
├── .github/
│   ├── actions/
│   │   ├── setup-rust/
│   │   │   ├── action.yml  # Composite GitHub action for common Rust setup
│   └── workflows/
│       ├── ci.yaml         # Workflow to run on Pull Requests and mergin to `main` branch
│       └── release.yaml    # Workflow to trigger manually when publishing a new tag/release.
├── crates/
│   ├── forgo/              # Crate for binary
│   │   ├── Cargo.toml
│   │   └── src/
│   └── forgo_lib_cli/      # Crate for library
│       ├── Cargo.toml
│       └── src/


```

## Prerequisites

- [rustup](https://rustup.rs/):
- [cargo-make](https://github.com/sagiegurari/cargo-make)
- [cargo-watch](https://github.com/watchexec/cargo-watch)
- [cargo-audit](https://github.com/RustSec/rustsec/tree/main/cargo-audit)
- [sem-tool](https://github.com/canardleteer/sem-tool)
- [toml-cli](https://github.com/gnprice/toml-cli)

## Building the Workspace

From the root of the repository, run:

```bash
# build all projects in workspace
cargo build --all

# syntax and dependency checks
cargo check --all

# enforce consistent code style
cargo fmt --all

# check formatting w/out modifying files
cargo fmt -- --check

# catch common mistakes and enforce best practices
cargo clippy --all -- -D warnings

# unit and integration tests
cargo test --all

# generate and view documentation for crates
cargo doc --open

# check for vulnerabilities in dependencies
cargo audit

# run built binary
./target/debug/forgo

# clean artifacts
cargo clean
```
