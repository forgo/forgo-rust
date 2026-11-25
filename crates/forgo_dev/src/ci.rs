//! `cargo x ci`: facts about the workspace for the CI workflows.

use crate::error::{Error, Result};
use crate::fsx;
use crate::workspace::{self, CARGO_TOML};
use forgo_lib_toml::Value;
use std::path::Path;

/// Help for `cargo x ci`.
pub const HELP: &str = "\
cargo x ci: facts about the workspace for CI

  matrix    The workspace crates that declare a minimum Rust version
            (`rust-version`, or inherited from [workspace.package]), as a
            GitHub Actions matrix: {\"include\":[{\"crate\":...,\"msrv\":...}]}";

/// Run `cargo x ci <args>` on the workspace at `root`.
pub fn run(root: &Path, args: &[String]) -> Result<String> {
    match args
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        [] | ["help"] => Ok(format!("{HELP}\n")),
        ["matrix"] => matrix(root),
        _ => Err(Error::Usage("ci: expected `matrix`".into())),
    }
}

fn json_string(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn matrix(root: &Path) -> Result<String> {
    let root_text = fsx::read(&root.join(CARGO_TOML))?;
    let root_doc = workspace::parse(&root_text, CARGO_TOML)?;
    let inherited = root_doc
        .get(["workspace", "package", "rust-version"])
        .and_then(|v| v.as_str())
        .map(str::to_string);
    let mut entries = Vec::new();
    for rel in workspace::members(&root_text)? {
        let manifest = format!("{rel}/{CARGO_TOML}");
        let doc = workspace::parse(&fsx::read(&root.join(&manifest))?, &manifest)?;
        let msrv = match doc.get(["package", "rust-version"]) {
            Some(Value::String(v)) => Some(v.to_string()),
            Some(Value::Table(t)) if t.get("workspace").and_then(|w| w.as_bool()) == Some(true) => {
                inherited.clone()
            }
            _ => None,
        };
        let name = doc.get(["package", "name"]).and_then(|n| n.as_str());
        if let (Some(name), Some(msrv)) = (name, msrv) {
            entries.push(format!(
                "{{\"crate\":{},\"msrv\":{}}}",
                json_string(name),
                json_string(&msrv)
            ));
        }
    }
    Ok(format!("{{\"include\":[{}]}}\n", entries.join(",")))
}
