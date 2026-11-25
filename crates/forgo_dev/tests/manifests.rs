//! `cargo x version`, `toml`, `ci` and `deps` on scratch workspaces, with
//! the real `cargo` for lockfiles. Edits are checked as exact text.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const ROOT: &str = "\
[workspace]
resolver = \"3\"
members = [
    \"crates/forgo_lib_a\",
    \"crates/forgo_lib_b\",
    \"crates/forgo_lib_w\",
    \"crates/forgo_tool\",
]

[workspace.package]
version = \"0.3.0\"   # shared
rust-version = \"1.85\"
";

const LIB_A: &str = "\
[package]
name = \"forgo_lib_a\"
version = \"0.1.0\"   # the version
edition = \"2024\"
rust-version = \"1.88\"
";

const LIB_B: &str = "\
[package]
name = \"forgo_lib_b\"
version = \"0.2.0\"
edition = \"2024\"

[dev-dependencies]
# Pinned exactly.
forgo_lib_a = { path = \"../forgo_lib_a\", version = \"=0.1.0\" }
";

const LIB_W: &str = "\
[package]
name = \"forgo_lib_w\"
version.workspace = true
rust-version.workspace = true
edition = \"2024\"
";

const TOOL: &str = "\
[package]
name = \"forgo_tool\"
version = \"0.0.1\"
edition = \"2024\"

[dependencies]
forgo_lib_a = { path = \"../forgo_lib_a\", version = \"0.1\" }
forgo_lib_w = { path = \"../forgo_lib_w\" }
";

struct Scratch {
    root: PathBuf,
}

impl Scratch {
    fn new(test: &str) -> Scratch {
        let root = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join("forgo_dev_manifests")
            .join(test);
        let _ = fs::remove_dir_all(&root);
        let s = Scratch { root };
        s.write("Cargo.toml", ROOT);
        for (name, manifest, src) in [
            ("forgo_lib_a", LIB_A, "src/lib.rs"),
            ("forgo_lib_b", LIB_B, "src/lib.rs"),
            ("forgo_lib_w", LIB_W, "src/lib.rs"),
            ("forgo_tool", TOOL, "src/main.rs"),
        ] {
            s.write(&format!("crates/{name}/Cargo.toml"), manifest);
            s.write(
                &format!("crates/{name}/{src}"),
                if src.ends_with("main.rs") {
                    "fn main() {}\n"
                } else {
                    ""
                },
            );
        }
        s.cargo(&["generate-lockfile", "--offline"]);
        s
    }

    fn write(&self, rel: &str, text: &str) {
        let p = self.root.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, text).unwrap();
    }

    fn read(&self, rel: &str) -> String {
        fs::read_to_string(self.root.join(rel)).unwrap()
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

    fn fails(&self, args: &[&str]) -> forgo_dev::Error {
        let before = self.files();
        let e = self.run(args).expect_err("an error");
        assert_eq!(self.files(), before, "a failed command changed files");
        e
    }

    /// The manifests and the lockfile.
    fn files(&self) -> Vec<String> {
        [
            "Cargo.toml",
            "Cargo.lock",
            "crates/forgo_lib_a/Cargo.toml",
            "crates/forgo_lib_b/Cargo.toml",
            "crates/forgo_tool/Cargo.toml",
        ]
        .iter()
        .map(|f| self.read(f))
        .collect()
    }

    /// The version Cargo.lock records for a package.
    fn locked(&self, name: &str) -> String {
        let lock = self.read("Cargo.lock");
        let at = lock
            .find(&format!("name = \"{name}\"\nversion = \""))
            .expect("in the lockfile");
        let rest = &lock[at..];
        let v = rest.split("version = \"").nth(1).unwrap();
        v[..v.find('"').unwrap()].to_string()
    }
}

#[test]
fn version_get() {
    let s = Scratch::new("get");
    assert_eq!(s.ok(&["version", "get", "forgo_lib_a"]), "0.1.0\n");
    assert_eq!(s.ok(&["version", "get", "forgo_lib_w"]), "0.3.0\n");
    assert!(
        s.fails(&["version", "get", "forgo_nope"])
            .to_string()
            .contains("isn't a workspace member")
    );
}

#[test]
fn version_set_updates_requirements_it_breaks_and_the_lockfile() {
    let s = Scratch::new("set");
    let report = s.ok(&["version", "set", "forgo_lib_a", "0.1.5"]);
    assert!(!report.contains("warning"), "{report}");
    assert_eq!(
        s.read("crates/forgo_lib_a/Cargo.toml"),
        LIB_A.replace("\"0.1.0\"", "\"0.1.5\"")
    );
    // `0.1` still matches 0.1.5; `=0.1.0` doesn't.
    assert_eq!(s.read("crates/forgo_tool/Cargo.toml"), TOOL);
    assert_eq!(
        s.read("crates/forgo_lib_b/Cargo.toml"),
        LIB_B.replace("\"=0.1.0\"", "\"0.1.5\"")
    );
    assert_eq!(s.locked("forgo_lib_a"), "0.1.5");
    assert!(
        report.contains("dev-dependencies.forgo_lib_a.version: \"=0.1.0\" → \"0.1.5\""),
        "{report}"
    );
    // Cargo agrees the lockfile is current.
    s.cargo(&["metadata", "--locked", "--offline", "--format-version", "1"]);

    // A major bump breaks `0.1` too.
    s.ok(&["version", "bump", "forgo_lib_a", "major"]);
    assert_eq!(s.ok(&["version", "get", "forgo_lib_a"]), "1.0.0\n");
    assert_eq!(
        s.read("crates/forgo_tool/Cargo.toml"),
        TOOL.replace("\"0.1\"", "\"1.0.0\"")
    );
    assert_eq!(s.locked("forgo_lib_a"), "1.0.0");
    s.cargo(&["metadata", "--locked", "--offline", "--format-version", "1"]);
}

#[test]
fn version_set_on_an_inherited_version_sets_the_workspaces() {
    let s = Scratch::new("inherited");
    let report = s.ok(&["version", "set", "forgo_lib_w", "0.4.0-rc.1"]);
    assert!(
        report.contains("in [workspace.package], for forgo_lib_w"),
        "{report}"
    );
    assert_eq!(
        s.read("Cargo.toml"),
        ROOT.replace("\"0.3.0\"", "\"0.4.0-rc.1\"")
    );
    assert_eq!(s.read("crates/forgo_lib_w/Cargo.toml"), LIB_W);
    assert_eq!(s.locked("forgo_lib_w"), "0.4.0-rc.1");
    s.cargo(&["metadata", "--locked", "--offline", "--format-version", "1"]);
}

#[test]
fn version_bump_kinds() {
    let s = Scratch::new("bump");
    for (args, want) in [
        (&["minor"][..], "0.2.0"),
        (&["prerelease", "rc"][..], "0.2.1-rc.0"),
        (&["prerelease"][..], "0.2.1-rc.1"),
        (&["release"][..], "0.2.1"),
        (&["premajor", "beta"][..], "1.0.0-beta.0"),
    ] {
        let mut cmd = vec!["version", "bump", "forgo_lib_a"];
        cmd.extend(args);
        s.ok(&cmd);
        assert_eq!(
            s.ok(&["version", "get", "forgo_lib_a"]),
            format!("{want}\n"),
            "{args:?}"
        );
    }
}

#[test]
fn versions_are_strict_and_dry_runs_change_nothing() {
    let s = Scratch::new("strict");
    for bad in ["v1.2.3", "01.2.3", "1.2", "1.2.3-", "1.2.3 "] {
        let e = s.fails(&["version", "set", "forgo_lib_a", bad]);
        assert!(e.to_string().contains("isn't a version"), "{bad}: {e}");
    }
    let report = s.ok(&[
        "version",
        "set",
        "forgo_lib_a",
        "1.2.3-rc.1+build.5",
        "--dry-run",
    ]);
    assert!(
        report.contains("0.1.0 → 1.2.3-rc.1+build.5") && report.contains("dry run"),
        "{report}"
    );
    assert_eq!(s.ok(&["version", "get", "forgo_lib_a"]), "0.1.0\n");
    assert!(
        s.fails(&["version", "bump", "forgo_lib_a", "sideways"])
            .is_usage()
    );
    assert!(
        s.fails(&["version", "bump", "forgo_lib_a", "major", "rc"])
            .is_usage()
    );
}

#[test]
fn toml_get_and_set() {
    let s = Scratch::new("toml");
    let f = "crates/forgo_tool/Cargo.toml";
    assert_eq!(s.ok(&["toml", "get", f, "package.name"]), "forgo_tool\n");
    assert_eq!(
        s.ok(&["toml", "get", f, "dependencies.forgo_lib_a"]),
        "{ path = \"../forgo_lib_a\", version = \"0.1\" }\n"
    );
    assert_eq!(
        s.ok(&["toml", "get", f, "dependencies.\"forgo_lib_a\".version"]),
        "0.1\n"
    );
    assert_eq!(
        s.ok(&["toml", "get", "Cargo.toml", "workspace.members[1]"]),
        "crates/forgo_lib_b\n"
    );
    assert_eq!(s.ok(&["toml", "get", "Cargo.lock", "version"]), "4\n");
    let missing = s.fails(&["toml", "get", f, "package.license"]);
    assert!(matches!(missing, forgo_dev::Error::Unmet(_)), "{missing}");

    s.ok(&["toml", "set", f, "package.edition", "\"2021\""]);
    s.ok(&["toml", "set", "--string", f, "package.license", "MIT"]);
    s.ok(&[
        "toml",
        "set",
        f,
        "package.metadata.docs.all-features",
        "true",
    ]);
    assert_eq!(
        s.read(f),
        TOOL.replace(
            "edition = \"2024\"",
            "edition = \"2021\"\nlicense = \"MIT\"\nmetadata.docs.all-features = true"
        )
    );
    assert!(
        s.fails(&["toml", "set", f, "package.edition", "2021-x"])
            .is_usage()
    );
    assert!(s.fails(&["toml", "get", f, "package..name"]).is_usage());
}

#[test]
fn ci_matrix() {
    let s = Scratch::new("ci");
    assert_eq!(
        s.ok(&["ci", "matrix"]),
        "{\"include\":[{\"crate\":\"forgo_lib_a\",\"msrv\":\"1.88\"},{\"crate\":\"forgo_lib_w\",\"msrv\":\"1.85\"}]}\n"
    );
}

#[test]
fn deps_check() {
    let s = Scratch::new("deps");
    s.write(
        ".github/allowed-dependencies.txt",
        "# Allowed third-party crates.\nwasm-bindgen  # for wasm crates\n",
    );
    assert!(s.ok(&["deps", "check"]).contains("0 third-party crates"));
    let lock = s.read("Cargo.lock");
    s.write(
        "Cargo.lock",
        &format!("{lock}\n[[package]]\nname = \"serde\"\nversion = \"1.0.219\"\nsource = \"registry+https://github.com/rust-lang/crates.io-index\"\n"),
    );
    let e = s.fails(&["deps", "check"]);
    assert!(
        matches!(&e, forgo_dev::Error::Unmet(r) if r.contains("serde 1.0.219")),
        "{e}"
    );
}
