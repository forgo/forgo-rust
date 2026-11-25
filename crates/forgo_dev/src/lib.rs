//! `cargo x`: repo-native developer tools for the forgo-rust workspace.
//!
//! `cargo x new` creates a crate under `crates/` and lists it everywhere
//! the workspace keeps a list of crates; `cargo x rm` undoes that. See
//! [`HELP`] for the commands and `README.md` for the conventions.
//!
//! Every command checks everything first and computes every file change
//! before writing any of them (see [`fsx::Plan`]). The lists in the GitHub
//! workflows are edited with [`forgo_lib_yaml`], keeping the rest of each
//! file byte for byte; when that isn't possible the command fails instead
//! of rewriting the file.
//!
//! The binary is a thin wrapper around [`run`], which takes the workspace
//! root as a parameter, so everything here can be tested against a
//! scratch workspace.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod cli;
pub mod commands;
pub mod error;
pub mod fsx;
pub mod metadata;
pub mod names;
pub mod semver;
pub mod templates;
pub mod workflows;
pub mod workspace;

pub use error::{Error, Result};

use std::path::Path;

/// The help text.
pub const HELP: &str = "\
cargo x: developer tools for the forgo-rust workspace

Usage:
  cargo x new <lib|bin|wasm> <name> [--public]
  cargo x rm [<lib|bin|wasm>] <name> [--yes]
  cargo x semver <command> ...
  cargo x help

new   Create crates/<name> (Cargo.toml, README.md and a source file) and add
      it to the workspace members and the CI matrices; with --public, also
      to the release workflow's choices (and set `publish = true`).
      Names: lib  forgo_lib_<name>
             wasm forgo_lib_<name>_wasm
             bin  any other name (not forgo_lib_*, not *_wasm)
rm    Remove a crate: the reverse of `new`. Without --yes (-y), only shows
      what would change. Refuses forgo_dev, crates that aren't workspace
      members, crates other crates depend on, and crates with uncommitted
      changes.
semver Versions and requirements: validate, explain, compare, sort,
      bump, matches, max, min (`cargo x semver help`).

Crate names use a-z, 0-9 and _, and start with a letter.";

/// Run the `cargo x` command line `args` (without the program name) on the
/// workspace at `root`. Returns the report to print.
pub fn run(args: &[String], root: &Path) -> Result<String> {
    match cli::parse_args(args)? {
        cli::Command::Help => Ok(format!("{HELP}\n")),
        cli::Command::New { kind, name, public } => commands::new_crate(root, kind, &name, public),
        cli::Command::Rm { kind, name, yes } => commands::remove_crate(root, kind, &name, yes),
        cli::Command::Semver(args) => semver::run(&args),
    }
}
