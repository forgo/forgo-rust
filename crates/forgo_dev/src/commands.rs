//! `cargo x new` and `cargo x rm`.
//!
//! Both work the same way: check everything, compute every file change
//! (failing before anything is written if a file can't be edited exactly),
//! apply the changes together ([`Plan::apply`]), then bring `Cargo.lock`
//! up to date with `cargo update --workspace --offline`.

use crate::cli::Kind;
use crate::error::{Result, failed};
use crate::fsx::{self, Plan};
use crate::metadata::{self, Package};
use crate::names::{validate_name, validate_new};
use crate::templates;
use crate::workflows::{self, CI_YAML, Change, RELEASE_YAML};
use crate::workspace::{self, CARGO_TOML};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// `cargo x new <kind> <name> [--public]`: create `crates/<name>` and list
/// it in the workspace members, the CI matrices and, when `public`, the
/// release workflow's choices. Returns the report to print.
pub fn new_crate(root: &Path, kind: Kind, name: &str, public: bool) -> Result<String> {
    validate_new(kind, name)?;
    let rel = format!("crates/{name}");
    let dir = root.join("crates").join(name);
    if fs::symlink_metadata(&dir).is_ok() {
        return failed(format!("{rel} already exists"));
    }

    let mut plan = Plan::new(root);
    let mut notes = Vec::new();
    let manifest = root.join(CARGO_TOML);
    let text = fsx::read(&manifest)?;
    match workspace::add_member(&text, &rel)? {
        Some(new) => plan.edit(manifest, new),
        None => notes.push(format!("{CARGO_TOML} already lists {rel}")),
    }
    edit_workflow(root, CI_YAML, &mut plan, &mut notes, |t| {
        workflows::edit_ci_matrix(t, Change::Add(name))
    })?;
    if public {
        edit_workflow(root, RELEASE_YAML, &mut plan, &mut notes, |t| {
            workflows::edit_release_options(t, Change::Add(name), true)
        })?;
    }
    plan.create(
        dir.join("Cargo.toml"),
        templates::manifest(kind, name, public),
    );
    plan.create(dir.join("README.md"), templates::readme(kind, name));
    let (src, code) = templates::source(kind);
    plan.create(dir.join(src), code.to_string());

    let changes = plan.describe();
    plan.apply()?;
    let visibility = if public { "public" } else { "private" };
    let mut report = format!(
        "Created {rel} ({}, {visibility}):\n{changes}",
        kind.as_str()
    );
    report.push_str(&update_lockfile(root));
    push_notes(&mut report, &notes);
    Ok(report)
}

/// `cargo x rm [<kind>] <name> [--yes]`: the reverse of [`new_crate`].
///
/// Refuses `forgo_dev`, a crate that isn't a workspace member at
/// `crates/<name>`, a crate another crate depends on, and a crate with
/// uncommitted changes. Without `yes`, reports what it would do and
/// changes nothing.
pub fn remove_crate(root: &Path, kind: Option<Kind>, name: &str, yes: bool) -> Result<String> {
    match kind {
        Some(kind) => validate_new(kind, name)?,
        None => validate_name(name)?,
    }
    if name == "forgo_dev" {
        return failed("forgo_dev is the tool that runs `cargo x`; it can't remove itself");
    }
    let rel = format!("crates/{name}");
    let dir = root.join("crates").join(name);
    let manifest = root.join(CARGO_TOML);
    let text = fsx::read(&manifest)?;
    if !workspace::members(&text)?.contains(&rel) {
        return failed(format!(
            "{rel} isn't a member of the workspace ({CARGO_TOML})"
        ));
    }
    if !dir.join("Cargo.toml").is_file() {
        return failed(format!("{rel}/Cargo.toml doesn't exist"));
    }
    let dependents = dependents(root, &dir, name)?;
    if !dependents.is_empty() {
        return failed(format!(
            "{} depend{} on {name}; remove {} first",
            dependents.join(", "),
            if dependents.len() == 1 { "s" } else { "" },
            if dependents.len() == 1 {
                "that dependency"
            } else {
                "those dependencies"
            },
        ));
    }
    let mut notes = Vec::new();
    check_committed(root, name, &rel, &mut notes)?;

    let mut plan = Plan::new(root);
    let new = workspace::remove_member(&text, &rel)?;
    plan.edit(manifest, new.expect("the member is listed"));
    let ci = edit_workflow(root, CI_YAML, &mut plan, &mut notes, |t| {
        workflows::edit_ci_matrix(t, Change::Remove(name))
    })?;
    let release = edit_workflow(root, RELEASE_YAML, &mut plan, &mut notes, |t| {
        workflows::edit_release_options(t, Change::Remove(name), false)
    })?;
    plan.remove_dir(dir);
    let leftovers = leftover_mentions(root, name, &[(CI_YAML, ci), (RELEASE_YAML, release)]);

    let changes = plan.describe();
    if !yes {
        let mut report = format!("Would remove {rel}:\n{changes}  update Cargo.lock\n");
        push_mentions(
            &mut report,
            "Would still be mentioned (line numbers after the change):",
            &leftovers,
        );
        push_notes(&mut report, &notes);
        report.push_str(&format!(
            "Nothing was changed. To remove it, run: cargo x rm {name} --yes\n"
        ));
        return Ok(report);
    }
    plan.apply()?;
    let mut report = format!("Removed {rel}:\n{changes}");
    report.push_str(&update_lockfile(root));
    push_mentions(
        &mut report,
        "Still mentioned (check these by hand):",
        &leftovers,
    );
    push_notes(&mut report, &notes);
    Ok(report)
}

/// Read a workflow, edit it, and add the change to the plan. Returns the
/// workflow's text after the edit, or `None` (with a note) if the file
/// doesn't exist.
fn edit_workflow(
    root: &Path,
    file: &str,
    plan: &mut Plan,
    notes: &mut Vec<String>,
    edit: impl FnOnce(&str) -> Result<Option<String>>,
) -> Result<Option<String>> {
    let path = root.join(file);
    if !path.exists() {
        notes.push(format!("{file} doesn't exist; skipped it"));
        return Ok(None);
    }
    let text = fsx::read(&path)?;
    Ok(Some(match edit(&text)? {
        Some(new) => {
            plan.edit(path, new.clone());
            new
        }
        None => text,
    }))
}

/// The crates that depend on the crate in `dir`: workspace members (from
/// `cargo metadata`), and crates under `crates/` outside the workspace
/// (such as benchmark crates) whose manifest has a path dependency on it.
fn dependents(root: &Path, dir: &Path, name: &str) -> Result<Vec<String>> {
    let packages = metadata::workspace_packages(root)?;
    let target = canonical(dir);
    let is_target = |p: &Package| p.manifest.parent().map(canonical).as_ref() == Some(&target);
    if !packages.iter().any(is_target) {
        return failed(format!(
            "`cargo metadata` doesn't list a package at crates/{name}"
        ));
    }
    let mut out: Vec<String> = packages
        .iter()
        .filter(|p| !is_target(p))
        .filter(|p| {
            p.deps
                .iter()
                .any(|d| d.path.as_deref().map(canonical).as_ref() == Some(&target))
        })
        .map(|p| p.name.clone())
        .collect();

    let members: Vec<PathBuf> = packages
        .iter()
        .filter_map(|p| p.manifest.parent().map(canonical))
        .collect();
    if let Ok(entries) = fs::read_dir(root.join("crates")) {
        let mut others: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
        others.sort();
        for other in others {
            let manifest = other.join("Cargo.toml");
            if !manifest.is_file() || members.contains(&canonical(&other)) {
                continue;
            }
            if let Ok(text) = fs::read_to_string(&manifest)
                && has_path_dependency(&text, name)
            {
                let dir_name = other.file_name().unwrap_or_default().to_string_lossy();
                out.push(format!("crates/{dir_name} (outside the workspace)"));
            }
        }
    }
    Ok(out)
}

/// Whether a manifest has a line like `x = { path = "../<name>" }`.
fn has_path_dependency(manifest: &str, name: &str) -> bool {
    let targets = [
        format!("\"../{name}\""),
        format!("\"../{name}/\""),
        format!("'../{name}'"),
        format!("'../{name}/'"),
    ];
    manifest.lines().any(|line| {
        let code = line.split('#').next().unwrap_or("");
        code.contains("path") && targets.iter().any(|t| code.contains(t.as_str()))
    })
}

fn canonical(p: &Path) -> PathBuf {
    fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf())
}

/// Refuse to delete uncommitted work in the crate `name` at `rel`.
/// Untracked files exactly as `cargo x new` wrote them don't count, so a
/// crate can be removed right after it was created. Outside a git work
/// tree there is nothing to check (a note says so).
fn check_committed(root: &Path, name: &str, rel: &str, notes: &mut Vec<String>) -> Result<()> {
    let git = |args: &[&str]| Command::new("git").arg("-C").arg(root).args(args).output();
    let top = match git(&["rev-parse", "--show-toplevel"]) {
        Ok(out) if out.status.success() => {
            PathBuf::from(String::from_utf8_lossy(&out.stdout).trim_end_matches(['\n', '\r']))
        }
        _ => {
            notes.push("not in a git work tree; skipped the check for uncommitted changes".into());
            return Ok(());
        }
    };
    let out = git(&[
        "status",
        "--porcelain",
        "-z",
        "--untracked-files=all",
        "--",
        rel,
    ])
    .ok()
    .filter(|o| o.status.success());
    let Some(out) = out else {
        return failed(format!(
            "couldn't check {rel} for uncommitted changes (`git status` failed)"
        ));
    };
    let crate_dir = canonical(&root.join(rel));
    let mut dirty = Vec::new();
    // `-z` entries: `XY path`, NUL-terminated; renames and copies are
    // followed by the original path.
    let mut fields = out.stdout.split(|b| *b == 0).filter(|f| !f.is_empty());
    while let Some(field) = fields.next() {
        let field = String::from_utf8_lossy(field);
        let (status, path) = field.split_at(field.len().min(3));
        if status.starts_with(['R', 'C']) {
            fields.next();
        }
        let generated = status == "?? "
            && canonical(&top.join(path))
                .strip_prefix(&crate_dir)
                .is_ok_and(|inner| is_generated(&crate_dir.join(inner), inner, name));
        if !generated {
            dirty.push(format!("  {} {path}", status.trim_end()));
        }
    }
    if !dirty.is_empty() {
        return failed(format!(
            "{rel} has uncommitted changes; commit or stash them first:\n{}",
            dirty.join("\n")
        ));
    }
    Ok(())
}

/// Whether the file at `path` (`inner` within the crate) is exactly what
/// `cargo x new` writes there for some kind of crate.
fn is_generated(path: &Path, inner: &Path, name: &str) -> bool {
    let Ok(text) = fs::read_to_string(path) else {
        return false;
    };
    let inner = inner.to_string_lossy().replace('\\', "/");
    Kind::ALL.iter().any(|&kind| match inner.as_str() {
        "Cargo.toml" => [false, true]
            .iter()
            .any(|&public| text == templates::manifest(kind, name, public)),
        "README.md" => text == templates::readme(kind, name),
        src => templates::source(kind) == (src, text.as_str()),
    })
}

/// Lines in `.github` YAML files that still mention `name` after the
/// change. `edited` gives the new text of files the change edits.
fn leftover_mentions(root: &Path, name: &str, edited: &[(&str, Option<String>)]) -> Vec<String> {
    let mut files = Vec::new();
    yaml_files(&root.join(".github"), &mut files);
    files.sort();
    let mut out = Vec::new();
    for path in files {
        let rel = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        let text = match edited.iter().find(|(f, _)| *f == rel) {
            Some((_, Some(text))) => text.clone(),
            _ => match fs::read_to_string(&path) {
                Ok(text) => text,
                Err(_) => continue,
            },
        };
        for (line, content) in workflows::mentions(&text, name) {
            out.push(format!("{rel}:{line}: {content}"));
        }
    }
    out
}

fn yaml_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let path = e.path();
        match e.file_type() {
            Ok(t) if t.is_dir() => yaml_files(&path, out),
            Ok(t) if t.is_file() && path.extension().is_some_and(|x| x == "yml" || x == "yaml") => {
                out.push(path);
            }
            _ => {}
        }
    }
}

/// Run `cargo update --workspace --offline` so `Cargo.lock` lists exactly
/// the workspace's crates. A failure is reported, not fatal: the next
/// cargo build updates the lockfile anyway.
fn update_lockfile(root: &Path) -> String {
    let lock = root.join("Cargo.lock");
    let before = fs::read(&lock).ok();
    let out = Command::new(metadata::cargo())
        .args(["update", "--workspace", "--offline", "--quiet"])
        .arg("--manifest-path")
        .arg(root.join(CARGO_TOML))
        .current_dir(root)
        .output();
    match out {
        Ok(out) if out.status.success() => {
            if fs::read(&lock).ok() != before {
                "  update Cargo.lock\n".into()
            } else {
                String::new()
            }
        }
        Ok(out) => format!(
            "warning: Cargo.lock wasn't updated (`cargo update --workspace --offline`: {}); \
             run `cargo update --workspace`\n",
            String::from_utf8_lossy(&out.stderr).trim()
        ),
        Err(e) => format!(
            "warning: Cargo.lock wasn't updated (couldn't run cargo: {e}); \
             run `cargo update --workspace`\n"
        ),
    }
}

fn push_mentions(report: &mut String, heading: &str, mentions: &[String]) {
    if mentions.is_empty() {
        return;
    }
    report.push_str(heading);
    report.push('\n');
    for m in mentions {
        report.push_str(&format!("  {m}\n"));
    }
}

fn push_notes(report: &mut String, notes: &[String]) {
    for n in notes {
        report.push_str(&format!("note: {n}\n"));
    }
}
