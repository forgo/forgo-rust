//! This repository's own workflows: `cargo x new` and `rm` can still edit
//! their lists of crates, and adding then removing a crate gives each file
//! back byte for byte.

use forgo_dev::workflows::{Change, edit_ci_matrix, edit_release_options};
use std::path::Path;

fn read(rel: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel);
    std::fs::read_to_string(path).unwrap()
}

#[test]
fn the_ci_matrices_can_be_edited() {
    let ci = read(".github/workflows/ci.yaml");
    let added = edit_ci_matrix(&ci, Change::Add("forgo_lib_zzz"))
        .unwrap()
        .expect("a change");
    let removed = edit_ci_matrix(&added, Change::Remove("forgo_lib_zzz"))
        .unwrap()
        .expect("a change");
    assert_eq!(removed, ci);
}

#[test]
fn the_release_choices_can_be_edited() {
    let release = read(".github/workflows/release.yaml");
    let added = edit_release_options(&release, Change::Add("forgo_lib_zzz"), true)
        .unwrap()
        .expect("a change");
    let removed = edit_release_options(&added, Change::Remove("forgo_lib_zzz"), true)
        .unwrap()
        .expect("a change");
    assert_eq!(removed, release);
}

#[test]
fn every_workflow_is_valid_yaml() {
    fn walk(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, out);
            } else if p.extension().is_some_and(|x| x == "yaml" || x == "yml") {
                out.push(p);
            }
        }
    }
    let mut files = Vec::new();
    walk(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.github"),
        &mut files,
    );
    assert!(files.len() >= 4, "{files:?}");
    for f in files {
        let text = std::fs::read_to_string(&f).unwrap();
        forgo_lib_yaml::Doc::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", f.display()));
    }
}
