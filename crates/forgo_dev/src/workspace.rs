//! The workspace's manifests: `[workspace].members` in the root
//! `Cargo.toml`, and reading any manifest, with forgo_lib_toml.
//!
//! Edits keep the rest of each file byte for byte. The members array stays
//! sorted and without duplicates, in its own style (one entry per line, or
//! on one line), with each entry's comments.

use crate::error::{Error, Result, failed};
use forgo_lib_toml::{Document, ParseOptions};

/// The workspace manifest, relative to the workspace root.
pub const CARGO_TOML: &str = "Cargo.toml";

/// Parse a manifest (or any TOML file) for reading and editing; `what`
/// names it in errors.
pub fn parse(text: &str, what: &str) -> Result<Document> {
    Document::parse_with(text, &ParseOptions::default().for_editing())
        .map_err(|e| Error::Failed(format!("{what}: {e}")))
}

/// The manifest with `member` added to `[workspace].members`, or `Ok(None)`
/// if it's already there.
pub fn add_member(text: &str, member: &str) -> Result<Option<String>> {
    edit(text, |items| {
        if items.iter().any(|i| i == member) {
            false
        } else {
            items.push(member.to_string());
            true
        }
    })
}

/// The manifest with `member` removed from `[workspace].members`, or
/// `Ok(None)` if it isn't there.
pub fn remove_member(text: &str, member: &str) -> Result<Option<String>> {
    edit(text, |items| {
        let before = items.len();
        items.retain(|i| i != member);
        items.len() != before
    })
}

/// The members currently listed.
pub fn members(text: &str) -> Result<Vec<String>> {
    members_of(&parse(text, CARGO_TOML)?)
}

fn members_of(doc: &Document) -> Result<Vec<String>> {
    if doc.get(["workspace"]).and_then(|w| w.as_table()).is_none() {
        return failed(format!("{CARGO_TOML}: no `[workspace]` table"));
    }
    let Some(list) = doc.get(["workspace", "members"]) else {
        return failed(format!("{CARGO_TOML}: no `members` array in `[workspace]`"));
    };
    let Some(list) = list.as_array() else {
        return failed(format!("{CARGO_TOML}: `members` isn't an array"));
    };
    list.iter()
        .map(|v| match v.as_str() {
            Some(s) => Ok(s.to_string()),
            None => failed(format!("{CARGO_TOML}: a `members` entry isn't a string")),
        })
        .collect()
}

/// Change the members with `change` (which says whether it changed
/// anything); the new manifest, or `Ok(None)` if nothing changed.
fn edit(text: &str, change: impl FnOnce(&mut Vec<String>) -> bool) -> Result<Option<String>> {
    let mut doc = parse(text, CARGO_TOML)?;
    let mut items = members_of(&doc)?;
    if !change(&mut items) {
        return Ok(None);
    }
    doc.edit_string_set(["workspace", "members"], |m| *m = items)
        .map_err(|e| Error::Failed(format!("{CARGO_TOML}: {e}")))?;
    Ok(Some(doc.to_string()))
}

/// The dependency tables of a manifest, as paths of keys: `dependencies`,
/// `dev-dependencies`, `build-dependencies`, the same under each
/// `target.<cfg>`, and `workspace.dependencies`.
pub fn dependency_tables(doc: &Document) -> Vec<Vec<String>> {
    const KINDS: [&str; 3] = ["dependencies", "dev-dependencies", "build-dependencies"];
    let mut out: Vec<Vec<String>> = KINDS.iter().map(|k| vec![k.to_string()]).collect();
    if let Some(targets) = doc.get(["target"]).and_then(|t| t.as_table()) {
        for (cfg, _) in targets.iter() {
            out.extend(
                KINDS
                    .iter()
                    .map(|k| vec!["target".to_string(), cfg.to_string(), k.to_string()]),
            );
        }
    }
    out.push(vec!["workspace".into(), "dependencies".into()]);
    out.retain(|p| doc.get(p.as_slice()).and_then(|t| t.as_table()).is_some());
    out
}

/// The `path`s of a manifest's path dependencies, as written.
pub fn path_dependencies(doc: &Document) -> Vec<String> {
    let mut out = Vec::new();
    for table in dependency_tables(doc) {
        if let Some(t) = doc.get(table.as_slice()).and_then(|t| t.as_table()) {
            for (_, dep) in t.iter() {
                if let Some(path) = dep
                    .as_table()
                    .and_then(|d| d.get("path"))
                    .and_then(|p| p.as_str())
                {
                    out.push(path.to_string());
                }
            }
        }
    }
    out
}
