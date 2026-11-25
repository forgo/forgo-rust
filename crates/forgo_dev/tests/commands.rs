//! `cargo x new` and `cargo x rm` on scratch workspaces, end to end (with
//! the real `cargo metadata`, `cargo update` and `git`).

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const ROOT_MANIFEST: &str = "\
[workspace]
resolver = \"3\"
members = [
    \"crates/forgo_dev\",
    \"crates/forgo_lib_a\",
    \"crates/forgo_lib_b\",
    \"crates/forgo_lib_c\",
    \"crates/forgo_tool\",
]
# A benchmark crate with its own workspace.
exclude = [\"crates/forgo_lib_c_bench\"]
";

const CI: &str = "\
name: CI

on:
  pull_request:
    branches: [main]

jobs:
  lint:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        # Every crate in the workspace.
        project: [forgo_dev, forgo_lib_a, forgo_lib_b, forgo_lib_c, forgo_tool]
    steps:
      - run: cargo clippy -p ${{ matrix.project }}

  test:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        project:
          - forgo_dev
          - forgo_lib_a
          - forgo_lib_b   # the one with fixtures
          - forgo_lib_c
          - forgo_tool
    steps:
      - name: Fetch fixtures
        if: matrix.project == 'forgo_lib_b'
        run: ./fetch.sh
      - run: cargo test -p ${{ matrix.project }}
";

const RELEASE: &str = "\
name: Release

on:
  workflow_dispatch:
    inputs:
      project:
        type: choice
        required: true
        default: forgo_lib_b
        options:
          - forgo_lib_a
          - forgo_lib_b
";

/// A scratch workspace:
/// - `forgo_dev`, `forgo_lib_a`, `forgo_lib_b`, `forgo_lib_c`, `forgo_tool`;
/// - `forgo_tool` depends on `forgo_lib_a`;
/// - `crates/forgo_lib_c_bench` (outside the workspace) depends on
///   `forgo_lib_c`;
/// - CI and release workflows, and a `Cargo.lock`.
struct Scratch {
    root: PathBuf,
}

impl Scratch {
    fn new(test: &str) -> Scratch {
        let base = Path::new(env!("CARGO_TARGET_TMPDIR")).join("forgo_dev");
        let mut root = base.join(test);
        if root.exists() && fs::remove_dir_all(&root).is_err() {
            root = base.join(format!("{test}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&root);
        }
        let s = Scratch { root };
        s.write("Cargo.toml", ROOT_MANIFEST);
        s.write(".github/workflows/ci.yaml", CI);
        s.write(".github/workflows/release.yaml", RELEASE);
        s.package("forgo_dev", "src/main.rs", "fn main() {}\n", "");
        s.package("forgo_lib_a", "src/lib.rs", "", "");
        s.package("forgo_lib_b", "src/lib.rs", "", "");
        s.package("forgo_lib_c", "src/lib.rs", "", "");
        s.package(
            "forgo_tool",
            "src/main.rs",
            "fn main() {}\n",
            "forgo_lib_a = { path = \"../forgo_lib_a\" }\n",
        );
        s.package(
            "forgo_lib_c_bench",
            "src/lib.rs",
            "",
            "forgo_lib_c = { path = \"../forgo_lib_c\" }\n\n[workspace]\n",
        );
        s.cargo(&["generate-lockfile", "--offline"]);
        s
    }

    fn path(&self, rel: &str) -> PathBuf {
        self.root.join(rel)
    }

    fn write(&self, rel: &str, text: &str) {
        let p = self.path(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, text).unwrap();
    }

    fn read(&self, rel: &str) -> String {
        fs::read_to_string(self.path(rel)).unwrap()
    }

    fn package(&self, name: &str, src: &str, code: &str, deps: &str) {
        self.write(
            &format!("crates/{name}/Cargo.toml"),
            &format!(
                "[package]\nname = \"{name}\"\nversion = \"0.0.0\"\nedition = \"2024\"\npublish = false\n\n[dependencies]\n{deps}"
            ),
        );
        self.write(&format!("crates/{name}/{src}"), code);
    }

    fn cargo(&self, args: &[&str]) {
        let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
        let out = Command::new(cargo)
            .args(args)
            .current_dir(&self.root)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }

    fn run(&self, args: &[&str]) -> forgo_dev::Result<String> {
        let args: Vec<String> = args.iter().map(|a| a.to_string()).collect();
        forgo_dev::run(&args, &self.root)
    }

    fn ok(&self, args: &[&str]) -> String {
        self.run(args)
            .unwrap_or_else(|e| panic!("cargo x {}: {e}", args.join(" ")))
    }

    fn fails(&self, args: &[&str]) -> String {
        let before = self.snapshot();
        let e = self.run(args).expect_err("an error").to_string();
        assert!(self.snapshot() == before, "a failed command changed files");
        e
    }

    /// Every file (outside `.git`) and its bytes.
    fn snapshot(&self) -> BTreeMap<String, Vec<u8>> {
        fn walk(dir: &Path, root: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
            for e in fs::read_dir(dir).unwrap().flatten() {
                let p = e.path();
                if e.file_name() == ".git" {
                    continue;
                }
                if e.file_type().unwrap().is_dir() {
                    walk(&p, root, out);
                } else {
                    let rel = p
                        .strip_prefix(root)
                        .unwrap()
                        .to_string_lossy()
                        .replace('\\', "/");
                    out.insert(rel, fs::read(&p).unwrap());
                }
            }
        }
        let mut out = BTreeMap::new();
        walk(&self.root, &self.root, &mut out);
        out
    }

    /// Make the scratch workspace its own git repository, with everything
    /// committed. `None` if git isn't available.
    fn git_init(&self) -> Option<()> {
        let git = |args: &[&str]| {
            Command::new("git")
                .arg("-C")
                .arg(&self.root)
                .args(["-c", "user.name=test", "-c", "user.email=test@example.com"])
                .args(["-c", "commit.gpgsign=false", "-c", "core.autocrlf=false"])
                .args(args)
                .output()
                .ok()
                .filter(|o| o.status.success())
        };
        git(&["init", "-q"])?;
        git(&["add", "-A"])?;
        git(&["commit", "-q", "-m", "scratch"])?;
        Some(())
    }
}

#[test]
fn new_then_rm_restores_every_byte() {
    let s = Scratch::new("roundtrip");
    let before = s.snapshot();
    for (kind, name, public) in [
        ("lib", "forgo_lib_demo", true),
        ("bin", "forgo_demo", false),
        ("lib", "aaa_first", false),
    ] {
        if kind == "lib" && !name.starts_with("forgo_lib_") {
            // Not a valid lib name: refused, nothing changed.
            s.fails(&["new", kind, name]);
            continue;
        }
        let mut args = vec!["new", kind, name];
        if public {
            args.push("--public");
        }
        let report = s.ok(&args);
        let src = if kind == "lib" {
            "src/lib.rs"
        } else {
            "src/main.rs"
        };
        assert!(
            report.starts_with(&format!("Created crates/{name} ({kind}, ")),
            "{report}"
        );
        for line in [
            "  update Cargo.toml\n".to_string(),
            "  update .github/workflows/ci.yaml\n".to_string(),
            format!("  create crates/{name}/Cargo.toml\n"),
            format!("  create crates/{name}/README.md\n"),
            format!("  create crates/{name}/{src}\n"),
            "  update Cargo.lock\n".to_string(),
        ] {
            assert!(report.contains(&line), "{line:?} in {report}");
        }
        assert_eq!(
            report.contains("update .github/workflows/release.yaml"),
            public,
            "{report}"
        );
        assert!(
            s.read("Cargo.toml")
                .contains(&format!("    \"crates/{name}\",\n"))
        );
        assert!(s.read("Cargo.lock").contains(&format!("name = \"{name}\"")));
        assert!(
            s.read(&format!("crates/{name}/Cargo.toml"))
                .contains(&format!("publish = {public}\n"))
        );

        let report = s.ok(&["rm", name, "--yes"]);
        assert!(
            report.starts_with(&format!("Removed crates/{name}:\n")),
            "{report}"
        );
        assert!(s.snapshot() == before, "new + rm of {name} changed files");
    }
}

#[test]
fn rm_dry_run_changes_nothing() {
    let s = Scratch::new("dry_run");
    let before = s.snapshot();
    let report = s.ok(&["rm", "forgo_lib_b"]);
    assert_eq!(
        report,
        "\
Would remove crates/forgo_lib_b:
  update Cargo.toml
  update .github/workflows/ci.yaml
  update .github/workflows/release.yaml
  delete crates/forgo_lib_b/ (2 files)
  update Cargo.lock
Would still be mentioned (line numbers after the change):
  .github/workflows/ci.yaml:28: if: matrix.project == 'forgo_lib_b'
Nothing was changed. To remove it, run: cargo x rm forgo_lib_b --yes
"
        .replace(
            "Nothing was changed",
            &if s.root.join(".git").exists() || in_git_work_tree(&s.root) {
                "Nothing was changed".to_string()
            } else {
                "note: not in a git work tree; skipped the check for uncommitted changes\nNothing was changed".to_string()
            }
        )
    );
    assert!(s.snapshot() == before);
}

fn in_git_work_tree(dir: &Path) -> bool {
    Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["rev-parse", "--is-inside-work-tree"])
        .output()
        .is_ok_and(|o| o.status.success())
}

#[test]
fn rm_edits_exactly() {
    let s = Scratch::new("rm_exact");
    s.ok(&["rm", "forgo_lib_b", "--yes"]);
    assert_eq!(
        s.read("Cargo.toml"),
        ROOT_MANIFEST.replace("    \"crates/forgo_lib_b\",\n", "")
    );
    assert_eq!(
        s.read(".github/workflows/ci.yaml"),
        CI.replace("forgo_lib_a, forgo_lib_b, ", "forgo_lib_a, ")
            .replace("          - forgo_lib_b   # the one with fixtures\n", "")
    );
    // `default` named the removed crate: it moves to the first choice left.
    assert_eq!(
        s.read(".github/workflows/release.yaml"),
        RELEASE
            .replace("default: forgo_lib_b", "default: forgo_lib_a")
            .replace("          - forgo_lib_b\n", "")
    );
    assert!(!s.path("crates/forgo_lib_b").exists());
    assert!(!s.read("Cargo.lock").contains("forgo_lib_b"));
}

#[test]
fn rm_refusals_change_nothing() {
    let s = Scratch::new("rm_refusals");
    let e = s.fails(&["rm", "forgo_dev", "--yes"]);
    assert!(e.contains("can't remove itself"), "{e}");
    let e = s.fails(&["rm", "forgo_lib_c_bench", "--yes"]);
    assert!(e.contains("isn't a member of the workspace"), "{e}");
    let e = s.fails(&["rm", "forgo_lib_zzz", "--yes"]);
    assert!(e.contains("isn't a member of the workspace"), "{e}");
    let e = s.fails(&["rm", "forgo_lib_a", "--yes"]);
    assert_eq!(
        e,
        "forgo_tool depends on forgo_lib_a; remove that dependency first"
    );
    let e = s.fails(&["rm", "forgo_lib_c", "--yes"]);
    assert_eq!(
        e,
        "crates/forgo_lib_c_bench (outside the workspace) depends on forgo_lib_c; remove that dependency first"
    );
    let e = s.fails(&["rm", "bin", "forgo_lib_b", "--yes"]);
    assert!(e.contains("isn't a bin crate name"), "{e}");
    let e = s.fails(&["rm", "../forgo_lib_b", "--yes"]);
    assert!(e.contains("invalid crate name"), "{e}");
    // A member whose directory is gone.
    s.write(
        "Cargo.toml",
        &ROOT_MANIFEST.replace(
            "    \"crates/forgo_lib_a\",\n",
            "    \"crates/forgo_lib_a\",\n    \"crates/forgo_lib_gone\",\n",
        ),
    );
    let e = s.fails(&["rm", "forgo_lib_gone", "--yes"]);
    assert!(
        e.contains("crates/forgo_lib_gone/Cargo.toml doesn't exist"),
        "{e}"
    );
}

#[test]
fn rm_refuses_uncommitted_changes() {
    let s = Scratch::new("rm_uncommitted");
    if s.git_init().is_none() {
        eprintln!("git isn't available; skipped");
        return;
    }
    s.write("crates/forgo_lib_b/src/lib.rs", "pub fn mine() {}\n");
    let e = s.fails(&["rm", "forgo_lib_b", "--yes"]);
    assert_eq!(
        e,
        "crates/forgo_lib_b has uncommitted changes; commit or stash them first:\n   M crates/forgo_lib_b/src/lib.rs"
    );
    s.write("crates/forgo_lib_b/src/lib.rs", "");
    s.write("crates/forgo_lib_b/notes.txt", "keep me\n");
    let e = s.fails(&["rm", "forgo_lib_b", "--yes"]);
    assert!(e.ends_with("\n  ?? crates/forgo_lib_b/notes.txt"), "{e}");
    fs::remove_file(s.path("crates/forgo_lib_b/notes.txt")).unwrap();
    s.ok(&["rm", "forgo_lib_b", "--yes"]);
    assert!(!s.path("crates/forgo_lib_b").exists());

    // A crate just created (untracked, exactly as generated) can go at
    // once; after a change to one of its files, it can't.
    s.ok(&["new", "lib", "forgo_lib_new"]);
    s.ok(&["rm", "forgo_lib_new", "--yes"]);
    s.ok(&["new", "lib", "forgo_lib_new"]);
    s.write("crates/forgo_lib_new/README.md", "# mine\n");
    let e = s.fails(&["rm", "forgo_lib_new", "--yes"]);
    assert!(e.ends_with("\n  ?? crates/forgo_lib_new/README.md"), "{e}");
}

#[test]
fn new_refusals_change_nothing() {
    let s = Scratch::new("new_refusals");
    let e = s.fails(&["new", "lib", "forgo_lib_a"]);
    assert_eq!(e, "crates/forgo_lib_a already exists");
    let e = s.fails(&["new", "bin", "../escape"]);
    assert!(e.contains("invalid crate name"), "{e}");
    let e = s.fails(&["new", "lib", "forgo_tool2"]);
    assert!(e.contains("isn't a lib crate name"), "{e}");
    // An edit that can't be made exactly stops everything before any
    // file is written.
    s.write(
        ".github/workflows/ci.yaml",
        &CI.replace(
            "project: [forgo_dev, forgo_lib_a, forgo_lib_b, forgo_lib_c, forgo_tool]",
            "project: [forgo_dev, 42]",
        ),
    );
    let e = s.fails(&["new", "lib", "forgo_lib_new"]);
    assert!(e.contains("item 2 of the list is not a string"), "{e}");
    assert!(!s.path("crates/forgo_lib_new").exists());
}

#[test]
fn new_without_workflows() {
    let s = Scratch::new("no_workflows");
    fs::remove_dir_all(s.path(".github")).unwrap();
    let report = s.ok(&["new", "lib", "forgo_lib_new", "--public"]);
    assert!(
        report.contains("note: .github/workflows/ci.yaml doesn't exist; skipped it\n"),
        "{report}"
    );
    assert!(
        report.contains("note: .github/workflows/release.yaml doesn't exist; skipped it\n"),
        "{report}"
    );
    assert!(s.path("crates/forgo_lib_new/src/lib.rs").is_file());
}

#[test]
fn help() {
    let s = Scratch::new("help");
    assert!(s.ok(&["help"]).contains("Usage:\n  cargo x new"));
    let e = s.run(&[]).unwrap_err();
    assert!(e.is_usage());
}
