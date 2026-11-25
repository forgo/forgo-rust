//! Workspace packages, from `cargo metadata` (its JSON is valid YAML 1.2,
//! so forgo_lib_yaml reads it).

use crate::error::{Error, Result, failed};
use forgo_lib_yaml::{Doc, Elem, Node};
use std::path::{Path, PathBuf};
use std::process::Command;

/// A workspace member.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Package {
    /// The package name.
    pub name: String,
    /// The absolute path of its `Cargo.toml`.
    pub manifest: PathBuf,
    /// Its dependencies, of every kind (normal, dev and build).
    pub deps: Vec<Dependency>,
}

/// A dependency of a [`Package`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dependency {
    /// The name of the package depended on.
    pub name: String,
    /// The directory of a path dependency.
    pub path: Option<PathBuf>,
}

/// The cargo binary: the one running us when there is one.
pub(crate) fn cargo() -> std::ffi::OsString {
    std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into())
}

/// The members of the workspace at `root`.
pub fn workspace_packages(root: &Path) -> Result<Vec<Package>> {
    let out = Command::new(cargo())
        .args([
            "metadata",
            "--no-deps",
            "--format-version",
            "1",
            "--offline",
        ])
        .arg("--manifest-path")
        .arg(root.join("Cargo.toml"))
        .current_dir(root)
        .output()
        .map_err(|e| Error::Failed(format!("run cargo metadata: {e}")))?;
    if !out.status.success() {
        return failed(format!(
            "cargo metadata failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    let json = String::from_utf8(out.stdout)
        .map_err(|_| Error::Failed("cargo metadata printed invalid UTF-8".into()))?;
    parse_packages(&json)
}

/// The packages listed in `cargo metadata` JSON.
pub fn parse_packages(json: &str) -> Result<Vec<Package>> {
    let bad = || Error::Failed("unexpected `cargo metadata` output".into());
    let doc = Doc::from_str(json).map_err(|e| Error::Failed(format!("cargo metadata: {e}")))?;
    let packages = field(doc.root(), "packages")
        .and_then(|p| p.node().as_seq())
        .ok_or_else(bad)?;
    packages
        .iter()
        .map(|p| {
            let name = field(p, "name")
                .and_then(|n| n.node().as_str())
                .ok_or_else(bad)?;
            let manifest = field(p, "manifest_path")
                .and_then(|m| m.node().as_str())
                .ok_or_else(bad)?;
            let deps = field(p, "dependencies")
                .and_then(|d| d.node().as_seq())
                .map(|deps| {
                    deps.iter()
                        .filter_map(|d| {
                            let name = field(d, "name")?.node().as_str()?;
                            let path = field(d, "path").and_then(|p| p.node().as_str());
                            Some(Dependency {
                                name: name.to_string(),
                                path: path.map(PathBuf::from),
                            })
                        })
                        .collect()
                })
                .unwrap_or_default();
            Ok(Package {
                name: name.to_string(),
                manifest: PathBuf::from(manifest),
                deps,
            })
        })
        .collect()
}

fn field<'e>(e: &'e Elem, key: &str) -> Option<&'e Elem> {
    match e.node() {
        Node::Map(entries) => entries.iter().rfind(|(k, _)| k == key).map(|(_, v)| v),
        _ => None,
    }
}
