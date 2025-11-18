//! Small shared utilities and constants.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

pub const EDITION: &str = "2024";
pub const WORKSPACE_CARGO_TOML: &str = "Cargo.toml";
pub const CI_YAML: &str = ".github/workflows/ci.yaml";
pub const RELEASE_YAML: &str = ".github/workflows/release.yaml";

/// Top-level help text.
pub fn help() {
    eprintln!(
        "forgo_dev — repo-native dev utilities

USAGE:
  cargo x new lib  <name> [--public]   # create library crate, add to workspace, update CI, README; if --public, add to Release options
  cargo x new bin  <name> [--public]   # create binary crate,  add to workspace, update CI, README; if --public, add to Release options
  cargo x new wasm <name> [--public]   # create WASM lib,      add to workspace, update CI, README; if --public, add to Release options
  cargo x rm [lib|bin|wasm] <name>     # remove crate and scrub workspace members, CI matrices, and Release options

RULES:
  • Library crates MUST start with `forgo_lib_` and MUST NOT end with `_wasm`
  • WASM crates  MUST start with `forgo_lib_` and MUST end with `_wasm`
  • Binary crates MUST NOT start with `forgo_lib_` and MUST NOT end with `_wasm`
  • Default visibility is PRIVATE (publish = false). Use --public to opt in.
"
    );
}

pub fn help_and_exit() -> ! {
    help();
    std::process::exit(2)
}

/// Find the index of the matching ']' for a '[' at `open_idx`.
pub fn find_matching_bracket(s: &str, open_idx: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (i, ch) in s[open_idx..].char_indices() {
        match ch {
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    return Some(open_idx + i);
                }
            }
            _ => {}
        }
    }
    None
}

/// Run a subprocess (cargo, etc.) in a specific working directory, and fail with a nice message if it fails.
pub fn run_in(cwd: &Path, bin: &str, args: &[&str]) {
    let status = Command::new(bin)
        .args(args)
        .current_dir(cwd)
        .status()
        .expect("spawn");
    if !status.success() {
        die(&format!(
            "command failed (cwd={}): {} {}",
            cwd.display(),
            bin,
            args.join(" ")
        ));
    }
}

/// Exit the binary after emitting an error.
pub fn die(msg: &str) -> ! {
    eprintln!("{msg}");
    std::process::exit(1)
}

/// Repo root based on CARGO_MANIFEST_DIR of crates/forgo_dev.
pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent() // -> crates/
        .and_then(|p| p.parent()) // -> repo root
        .unwrap()
        .to_path_buf()
}

/// Read a UTF-8 file into a String, or die.
pub fn read_to_string(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|e| die(&format!("read {}: {e}", path.display())))
}

/// Write a String into a file, or die.
pub fn write_string(path: &Path, s: String) {
    fs::write(path, s).unwrap_or_else(|e| die(&format!("write {}: {e}", path.display())))
}
