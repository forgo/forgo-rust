//! `cargo x version`: a workspace crate's version, read and set in its
//! manifest, with the requirements on it and `Cargo.lock` kept in step.
//!
//! Versions are checked as Semantic Versioning 2.0.0 (forgo_lib_semver):
//! `v1.2.3` and `01.2.3` are refused. Manifests and the lockfile are edited
//! with forgo_lib_toml, so everything else in them stays as it was.

use crate::error::{Error, Result, failed};
use crate::fsx::{self, Plan};
use crate::names::validate_name;
use crate::workspace::{self, CARGO_TOML};
use forgo_lib_semver::{Bump, Req, Version};
use forgo_lib_toml::{Item, Seg, Value};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Help for `cargo x version`.
pub const HELP: &str = "\
cargo x version: a workspace crate's version

  get <crate>                       Print the version of crates/<crate>
  set <crate> <version>             Set it (Semantic Versioning 2.0.0)
  bump <crate> <release> [<id>]     Set the next version: major, minor,
                                    patch, premajor, preminor, prepatch,
                                    prerelease or release; <id> names the
                                    pre-release (rc, beta, ...)

Options (set, bump):
  --dry-run   show what would change, and change nothing

Setting a version also updates every path dependency on the crate whose
version requirement the new version wouldn't meet, and the crate's entry in
Cargo.lock. A version inherited from [workspace.package]
(`version.workspace = true`) is set there, for every crate that inherits it.";

fn usage<T>(msg: impl Into<String>) -> Result<T> {
    Err(Error::Usage(format!("version: {}", msg.into())))
}

/// Run `cargo x version <args>` on the workspace at `root`.
pub fn run(root: &Path, args: &[String]) -> Result<String> {
    let dry = args.iter().any(|a| a == "--dry-run");
    let args: Vec<&str> = args
        .iter()
        .map(String::as_str)
        .filter(|a| *a != "--dry-run")
        .collect();
    match args.as_slice() {
        [] | ["help"] => Ok(format!("{HELP}\n")),
        ["get", name] if !dry => Ok(format!("{}\n", locate(root, name)?.version)),
        ["set", name, version] => {
            let version = Version::parse(version)
                .map_err(|e| Error::Failed(format!("`{version}` isn't a version: {e}")))?;
            set(root, name, &version, dry)
        }
        ["bump", name, release, rest @ ..] if rest.len() <= 1 => {
            let id = rest.first().copied();
            let bump = match (*release, id) {
                ("major", None) => Bump::Major,
                ("minor", None) => Bump::Minor,
                ("patch", None) => Bump::Patch,
                ("release", None) => Bump::Release,
                ("premajor", id) => Bump::Premajor(id),
                ("preminor", id) => Bump::Preminor(id),
                ("prepatch", id) => Bump::Prepatch(id),
                ("prerelease", id) => Bump::Prerelease(id),
                (r, Some(_)) if ["major", "minor", "patch", "release"].contains(&r) => {
                    return usage(format!("bump: `{r}` takes no pre-release identifier"));
                }
                (r, _) => return usage(format!("bump: unknown release `{r}`")),
            };
            let current = locate(root, name)?;
            let now = Version::parse(&current.version).map_err(|e| {
                Error::Failed(format!(
                    "crates/{name}: `{}` isn't a version: {e}",
                    current.version
                ))
            })?;
            let next = now
                .bump(bump)
                .map_err(|e| Error::Failed(format!("bump: {e}")))?;
            set(root, name, &next, dry)
        }
        ["get", ..] => usage("get: expected <crate>"),
        ["set", ..] => usage("set: expected <crate> <version>"),
        ["bump", ..] => usage("bump: expected <crate> <release> [<id>]"),
        [other, ..] => usage(format!(
            "unknown command `{other}` (expected get, set or bump)"
        )),
    }
}

/// Where a crate's version is.
struct Located {
    /// The crate's directory.
    dir: PathBuf,
    /// Its version, as written (`0.0.0` when the manifest has none).
    version: String,
    /// Inherited from `[workspace.package]`.
    inherited: bool,
}

/// The workspace member `crates/<name>` and its version.
fn locate(root: &Path, name: &str) -> Result<Located> {
    validate_name(name)?;
    let rel = format!("crates/{name}");
    if !workspace::members(&fsx::read(&root.join(CARGO_TOML))?)?.contains(&rel) {
        return failed(format!("`{rel}` isn't a workspace member"));
    }
    let dir = root.join(&rel);
    let manifest = format!("{rel}/{CARGO_TOML}");
    let doc = workspace::parse(&fsx::read(&dir.join(CARGO_TOML))?, &manifest)?;
    match doc.get(["package", "version"]) {
        // Cargo's default when the manifest doesn't say.
        None => Ok(Located {
            dir,
            version: "0.0.0".into(),
            inherited: false,
        }),
        Some(Value::String(v)) => Ok(Located {
            dir,
            version: v.to_string(),
            inherited: false,
        }),
        Some(Value::Table(t)) if t.get("workspace").and_then(|w| w.as_bool()) == Some(true) => {
            let root_doc = workspace::parse(&fsx::read(&root.join(CARGO_TOML))?, CARGO_TOML)?;
            match root_doc
                .get(["workspace", "package", "version"])
                .and_then(|v| v.as_str())
            {
                Some(v) => Ok(Located {
                    dir,
                    version: v.to_string(),
                    inherited: true,
                }),
                None => failed(format!(
                    "{manifest} inherits its version, but {CARGO_TOML} has no `[workspace.package]` version"
                )),
            }
        }
        Some(_) => failed(format!("{manifest}: `package.version` isn't a string")),
    }
}

/// The names of the members whose version is inherited from
/// `[workspace.package]`.
fn inheriting(root: &Path, members: &[String]) -> Result<Vec<String>> {
    let mut out = Vec::new();
    for rel in members {
        let path = root.join(rel).join(CARGO_TOML);
        let doc = workspace::parse(&fsx::read(&path)?, &format!("{rel}/{CARGO_TOML}"))?;
        let inherits = doc
            .get(["package", "version", "workspace"])
            .and_then(|w| w.as_bool())
            == Some(true);
        if inherits && let Some(name) = doc.get(["package", "name"]).and_then(|n| n.as_str()) {
            out.push(name.to_string());
        }
    }
    Ok(out)
}

/// Set the version of `crates/<name>` (or of the workspace, if it inherits
/// it) to `new`.
fn set(root: &Path, name: &str, new: &Version, dry: bool) -> Result<String> {
    let located = locate(root, name)?;
    let new_text = new.to_string();
    let mut plan = Plan::new(root);
    let mut report = String::new();
    let root_manifest = root.join(CARGO_TOML);
    let members = workspace::members(&fsx::read(&root_manifest)?)?;

    // The version itself, and which crates have it.
    let affected = if located.inherited {
        let mut doc = workspace::parse(&fsx::read(&root_manifest)?, CARGO_TOML)?;
        doc.set(["workspace", "package", "version"], new_text.as_str())
            .map_err(|e| Error::Failed(format!("{CARGO_TOML}: {e}")))?;
        plan.edit(root_manifest.clone(), doc.to_string());
        let affected = inheriting(root, &members)?;
        report.push_str(&format!(
            "{} → {new_text} in [workspace.package], for {}\n",
            located.version,
            affected.join(", ")
        ));
        affected
    } else {
        let path = located.dir.join(CARGO_TOML);
        let mut doc = workspace::parse(&fsx::read(&path)?, &format!("crates/{name}/{CARGO_TOML}"))?;
        doc.set(["package", "version"], new_text.as_str())
            .map_err(|e| Error::Failed(format!("crates/{name}/{CARGO_TOML}: {e}")))?;
        plan.edit(path, doc.to_string());
        report.push_str(&format!("{name}: {} → {new_text}\n", located.version));
        vec![name.to_string()]
    };

    // Requirements on the affected crates that the new version wouldn't
    // meet: every manifest under crates/ (members and others), and the
    // root's [workspace.dependencies].
    let mut manifests = vec![root_manifest.clone()];
    if let Ok(entries) = std::fs::read_dir(root.join("crates")) {
        let mut dirs: Vec<PathBuf> = entries
            .flatten()
            .map(|e| e.path().join(CARGO_TOML))
            .filter(|p| p.is_file())
            .collect();
        dirs.sort();
        manifests.extend(dirs);
    }
    for path in manifests {
        let rel = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        // The root manifest may already be edited above.
        let text = match plan_text(&plan, &path) {
            Some(t) => t,
            None => fsx::read(&path)?,
        };
        let mut doc = workspace::parse(&text, &rel)?;
        let mut changed = Vec::new();
        for table in workspace::dependency_tables(&doc) {
            let Some(t) = doc.get(table.as_slice()).and_then(|t| t.as_table()) else {
                continue;
            };
            let mut hits = Vec::new();
            for (key, dep) in t.iter() {
                let Some(dep) = dep.as_table() else { continue };
                let package = dep.get("package").and_then(|p| p.as_str()).unwrap_or(key);
                if !affected.iter().any(|a| a == package) || dep.get("path").is_none() {
                    continue;
                }
                let Some(req) = dep.get("version").and_then(|v| v.as_str()) else {
                    continue;
                };
                let meets = Req::cargo(req).map(|r| r.matches(new)).unwrap_or(false);
                if !meets {
                    hits.push((key.to_string(), req.to_string()));
                }
            }
            for (key, old) in hits {
                let mut path_keys: Vec<&str> = table.iter().map(String::as_str).collect();
                path_keys.push(&key);
                path_keys.push("version");
                doc.set(path_keys.as_slice(), new_text.as_str())
                    .map_err(|e| Error::Failed(format!("{rel}: {e}")))?;
                changed.push(format!(
                    "{}.{key}.version: \"{old}\" → \"{new_text}\"",
                    table.join(".")
                ));
            }
        }
        if !changed.is_empty() {
            report.push_str(&format!("  {rel}: {}\n", changed.join(", ")));
            plan.edit(path, doc.to_string());
        }
    }

    // Cargo.lock: the affected crates' own entries (path crates have no
    // `source`), and references that name their old version.
    let lock_path = root.join("Cargo.lock");
    if lock_path.is_file() {
        let mut lock = workspace::parse(&fsx::read(&lock_path)?, "Cargo.lock")?;
        let mut updates: Vec<(usize, String)> = Vec::new();
        let mut renames: Vec<(String, String)> = Vec::new();
        if let Some(packages) = lock.get(["package"]).and_then(|p| p.as_array()) {
            for (i, p) in packages.iter().enumerate() {
                let Some(p) = p.as_table() else { continue };
                let (Some(n), Some(v)) = (
                    p.get("name").and_then(|n| n.as_str()),
                    p.get("version").and_then(|v| v.as_str()),
                ) else {
                    continue;
                };
                if affected.iter().any(|a| a == n) && p.get("source").is_none() && v != new_text {
                    updates.push((i, n.to_string()));
                    renames.push((format!("{n} {v}"), format!("{n} {new_text}")));
                }
            }
        }
        for (i, _) in &updates {
            lock.set(
                [Seg::Key("package"), Seg::Index(*i), Seg::Key("version")],
                new_text.as_str(),
            )
            .map_err(|e| Error::Failed(format!("Cargo.lock: {e}")))?;
        }
        // `dependencies = ["name version"]` where a name is ambiguous.
        let count = lock
            .get(["package"])
            .and_then(|p| p.as_array())
            .map_or(0, |p| p.len());
        for i in 0..count {
            let path = [Seg::Key("package"), Seg::Index(i), Seg::Key("dependencies")];
            if lock.get(path).is_none() {
                continue;
            }
            lock.edit_array(path, |deps| {
                for d in deps.iter_mut() {
                    if let Item::String(s) = d {
                        for (old, new) in &renames {
                            if s == old || s.starts_with(&format!("{old} ")) {
                                *s = s.replacen(old.as_str(), new, 1);
                            }
                        }
                    }
                }
            })
            .map_err(|e| Error::Failed(format!("Cargo.lock: {e}")))?;
        }
        if !updates.is_empty() {
            plan.edit(lock_path, lock.to_string());
        }
    }

    report.push_str(&plan.describe());
    if dry {
        report.push_str("(dry run: nothing changed)\n");
        return Ok(report);
    }
    plan.apply()?;
    report.push_str(&check_lock(root));
    Ok(report)
}

/// The new contents the plan gives `path`, if it edits it.
fn plan_text(plan: &Plan, path: &Path) -> Option<String> {
    plan.edited(path).map(str::to_string)
}

/// Whether Cargo agrees the lockfile is up to date (`cargo metadata
/// --locked`): a warning if not.
fn check_lock(root: &Path) -> String {
    let out = Command::new(crate::metadata::cargo())
        .args([
            "metadata",
            "--locked",
            "--offline",
            "--format-version",
            "1",
            "--quiet",
        ])
        .arg("--manifest-path")
        .arg(root.join(CARGO_TOML))
        .current_dir(root)
        .output();
    match out {
        Ok(out) if out.status.success() => String::new(),
        Ok(out) => format!(
            "warning: `cargo metadata --locked` reports Cargo.lock out of date: {}\n",
            String::from_utf8_lossy(&out.stderr).trim()
        ),
        Err(e) => format!("warning: couldn't run cargo to check Cargo.lock: {e}\n"),
    }
}
