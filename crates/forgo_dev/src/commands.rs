//! High-level subcommands wired to the smaller modules.

use std::fs;

use crate::{
    ci::{update_ci_matrix_add, update_ci_matrix_remove},
    release::{update_release_options_add, update_release_options_remove},
    templates::{scaffold_readme, write_manifest_from_template},
    util::{CI_YAML, EDITION, RELEASE_YAML, WORKSPACE_CARGO_TOML, die, repo_root, run_in},
    workspace::{add_workspace_member_sorted, remove_workspace_member},
};

/// `cargo x new <lib|bin|wasm> <name> [--public]`
pub fn new(kind: &str, name: &str, make_public: bool) {
    validate_name(kind, name);

    // 1) Create the crate under <repo_root>/crates/<name>
    let rel_path = format!("crates/{name}");
    let repo = repo_root();
    let abs_path = repo.join(&rel_path);

    if abs_path.exists() {
        die(&format!("Path already exists: {}", abs_path.display()));
    }

    // Ensure <repo_root>/crates exists
    if let Some(parent) = abs_path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)
                .unwrap_or_else(|e| die(&format!("mkdir -p {}: {e}", parent.display())));
        }
    }

    // Always run `cargo new` from the REPO ROOT so the relative path is correct.
    match kind {
        "lib" | "wasm" => run_in(
            &repo,
            "cargo",
            &["new", "--lib", &rel_path, "--edition", EDITION],
        ),
        "bin" => run_in(
            &repo,
            "cargo",
            &["new", "--bin", &rel_path, "--edition", EDITION],
        ),
        _ => die("new: kind must be 'lib' or 'bin' or 'wasm'"),
    }

    // 2) Overwrite Cargo.toml with our template so field order is predictable.
    write_manifest_from_template(&abs_path, kind, name, make_public);

    // 3) Scaffold a README from template
    scaffold_readme(&abs_path, kind, name);

    // 4) Add to workspace members in root Cargo.toml (alphabetically, one-per-line)
    let workspace_toml = repo.join(WORKSPACE_CARGO_TOML);
    add_workspace_member_sorted(&workspace_toml, &rel_path);

    // 5) Append to CI matrices (both lint and test jobs)
    let ci_path = repo.join(CI_YAML);
    if ci_path.exists() {
        update_ci_matrix_add(&ci_path, name);
    } else {
        eprintln!("WARN: {} not found; skipped CI matrix update.", CI_YAML);
    }

    // 6) If public, append to Release workflow project options (alphabetical)
    if make_public {
        let release_path = repo.join(RELEASE_YAML);
        if release_path.exists() {
            update_release_options_add(&release_path, name);
        } else {
            eprintln!(
                "WARN: {} not found; skipped Release options update.",
                RELEASE_YAML
            );
        }
    }

    println!(
        "✓ Created {}\n   - normalized workspace members (alphabetical)\n   - updated CI matrices\n   - scaffolded README\n   - {}{}\n",
        rel_path,
        if make_public {
            "added to Release options\n   - "
        } else {
            ""
        },
        if make_public {
            "visibility: PUBLIC (publish = true)"
        } else {
            "visibility: PRIVATE (publish = false)"
        }
    );
}

/// `cargo x rm [lib|bin|wasm] <name>`
pub fn rm(name: &str) {
    // 1) Remove from workspace members
    let repo = repo_root();
    let workspace_toml = repo.join(WORKSPACE_CARGO_TOML);
    let rel_path = format!("crates/{name}");
    if workspace_toml.exists() {
        remove_workspace_member(&workspace_toml, &rel_path);
    }

    // 2) Remove from CI matrices
    let ci_path = repo.join(CI_YAML);
    if ci_path.exists() {
        update_ci_matrix_remove(&ci_path, name);
    } else {
        eprintln!("WARN: {} not found; skipped CI matrix cleanup.", CI_YAML);
    }

    // 3) Remove from Release options (if present)
    let release_path = repo.join(RELEASE_YAML);
    if release_path.exists() {
        update_release_options_remove(&release_path, name);
    } else {
        eprintln!(
            "WARN: {} not found; skipped Release options cleanup.",
            RELEASE_YAML
        );
    }

    // 4) Delete the crate directory
    let abs_path = repo.join(&rel_path);
    if abs_path.exists() {
        if let Err(e) = fs::remove_dir_all(&abs_path) {
            die(&format!(
                "failed to remove crate directory '{}': {e}",
                abs_path.display()
            ));
        } else {
            println!("• removed crate directory: {}", abs_path.display());
        }
    } else {
        println!(
            "• crate directory not found (already removed?): {}",
            abs_path.display()
        );
    }

    println!("✓ Removed {}", name);
}

/// Enforce naming policy for lib/bin/wasm.
fn validate_name(kind: &str, name: &str) {
    match kind {
        "lib" => {
            if name.ends_with("_wasm") {
                die(
                    "library crates (non-WASM) must NOT end with `_wasm`. Use `cargo x new wasm <forgo_lib_name_wasm>` for WASM.",
                );
            }
            if !name.starts_with("forgo_lib_") {
                die("library crates must start with `forgo_lib_` (e.g., forgo_lib_storage)");
            }
        }
        "wasm" => {
            if !name.starts_with("forgo_lib_") || !name.ends_with("_wasm") {
                die(
                    "WASM crates must start with `forgo_lib_` AND end with `_wasm` (e.g., forgo_lib_storage_wasm).",
                );
            }
        }
        "bin" => {
            if name.starts_with("forgo_lib_") || name.ends_with("_wasm") {
                die(
                    "binary crates must NOT start with `forgo_lib_` and must NOT end with `_wasm` (use e.g., forgo_tool).",
                );
            }
        }
        _ => {}
    }
}
