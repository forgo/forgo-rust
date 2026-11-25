//! `cargo x deps`: the workspace's third-party dependencies, checked
//! against the list of those allowed.

use crate::error::{Error, Result};
use crate::fsx;
use crate::workspace;
use std::path::Path;

/// The list of third-party crates allowed in `Cargo.lock`.
pub const ALLOWED: &str = ".github/allowed-dependencies.txt";

/// Help for `cargo x deps`.
pub const HELP: &str = "\
cargo x deps: third-party dependencies

  check     Every crate in Cargo.lock that comes from a registry or git
            (it has a `source`) must be listed in
            .github/allowed-dependencies.txt (one name per line, # for
            comments). Exit code 1 if one isn't.";

/// Run `cargo x deps <args>` on the workspace at `root`.
pub fn run(root: &Path, args: &[String]) -> Result<String> {
    match args
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        [] | ["help"] => Ok(format!("{HELP}\n")),
        ["check"] => check(root),
        _ => Err(Error::Usage("deps: expected `check`".into())),
    }
}

fn check(root: &Path) -> Result<String> {
    let allowed: Vec<String> = fsx::read(&root.join(ALLOWED))?
        .lines()
        .map(|l| l.split('#').next().unwrap_or("").trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();
    let lock = workspace::parse(&fsx::read(&root.join("Cargo.lock"))?, "Cargo.lock")?;
    let mut third_party = Vec::new();
    let mut not_allowed = Vec::new();
    if let Some(packages) = lock.get(["package"]).and_then(|p| p.as_array()) {
        for p in packages.iter().filter_map(|p| p.as_table()) {
            let Some(name) = p.get("name").and_then(|n| n.as_str()) else {
                continue;
            };
            if p.get("source").is_none() {
                continue;
            }
            let version = p.get("version").and_then(|v| v.as_str()).unwrap_or("?");
            third_party.push(format!("{name} {version}"));
            if !allowed.iter().any(|a| a == name) {
                not_allowed.push(format!("{name} {version}"));
            }
        }
    }
    if not_allowed.is_empty() {
        return Ok(format!(
            "Cargo.lock: {} third-party crates, all in {ALLOWED}\n",
            third_party.len()
        ));
    }
    let mut report = format!("Cargo.lock: not in {ALLOWED}:\n");
    for n in &not_allowed {
        report.push_str(&format!("  {n}\n"));
    }
    report.push_str("Add a crate to the list only on purpose: the workspace is meant to have no third-party dependencies.\n");
    Err(Error::Unmet(report))
}
