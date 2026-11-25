//! The workflow list edits, checked against exact output text.

use forgo_dev::workflows::{Change, edit_ci_matrix, edit_release_options, mentions};

fn add(text: &str, name: &str) -> String {
    edit_ci_matrix(text, Change::Add(name))
        .expect("edit")
        .expect("a change")
}

fn remove(text: &str, name: &str) -> String {
    edit_ci_matrix(text, Change::Remove(name))
        .expect("edit")
        .expect("a change")
}

fn err(r: forgo_dev::Result<Option<String>>) -> String {
    r.expect_err("an error").to_string()
}

const CI: &str = "\
name: CI

on:
  push:
    branches: [main]

jobs:
  lint:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        project:
          - forgo
          - forgo_lib_yaml
    steps:
      - run: cargo make fmt-check-project ${{ matrix.project }}

  msrv:
    runs-on: ubuntu-latest
    steps:
      - run: cargo +1.88 check -p forgo_lib_yaml

  test:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        project:
          - forgo
          - forgo_lib_yaml
    steps:
      - run: cargo make test-project ${{ matrix.project }}
";

#[test]
fn adds_in_order_to_every_matrix_and_nothing_else() {
    let out = add(CI, "forgo_lib_cli");
    assert_eq!(
        out,
        CI.replace(
            "          - forgo\n          - forgo_lib_yaml\n",
            "          - forgo\n          - forgo_lib_cli\n          - forgo_lib_yaml\n",
        )
    );
    // At the start and the end too.
    let out = add(CI, "a_first");
    assert!(
        out.contains("          - a_first\n          - forgo\n"),
        "{out}"
    );
    let out = add(CI, "zz_last");
    assert!(
        out.contains("          - forgo_lib_yaml\n          - zz_last\n"),
        "{out}"
    );
}

#[test]
fn removes_from_every_matrix_and_nothing_else() {
    assert_eq!(
        remove(CI, "forgo"),
        CI.replace(
            "          - forgo\n          - forgo_lib_yaml\n",
            "          - forgo_lib_yaml\n"
        )
    );
}

#[test]
fn no_change_is_none() {
    assert_eq!(edit_ci_matrix(CI, Change::Add("forgo")).unwrap(), None);
    assert_eq!(edit_ci_matrix(CI, Change::Remove("absent")).unwrap(), None);
}

#[test]
fn add_then_remove_restores_the_file() {
    for name in ["a", "forgo_lib_cli", "zzz"] {
        assert_eq!(remove(&add(CI, name), name), CI);
    }
}

#[test]
fn inline_lists_stay_inline() {
    let text = "jobs:\n  t:\n    strategy:\n      matrix:\n        project: [forgo, forgo_lib_yaml]  # crates\n";
    assert_eq!(
        add(text, "forgo_lib_cli"),
        "jobs:\n  t:\n    strategy:\n      matrix:\n        project: [forgo, forgo_lib_cli, forgo_lib_yaml]  # crates\n"
    );
    assert_eq!(
        remove(text, "forgo"),
        "jobs:\n  t:\n    strategy:\n      matrix:\n        project: [forgo_lib_yaml]  # crates\n"
    );
}

#[test]
fn comments_around_the_list_are_kept() {
    let text = "\
jobs:
  t:
    strategy:
      matrix:
        # Every crate in the workspace.
        project:
          - forgo
          - forgo_lib_yaml
        # More axes could go here.
        os: [ubuntu-latest]
    steps: []
";
    assert_eq!(
        add(text, "forgo_lib_cli"),
        text.replace(
            "          - forgo\n",
            "          - forgo\n          - forgo_lib_cli\n"
        )
    );
}

#[test]
fn quoted_items_match_by_value() {
    let text = "jobs:\n  t:\n    strategy:\n      matrix:\n        project:\n          - \"forgo\"\n          - 'forgo_lib_cli'\n          - forgo_lib_yaml\n";
    let out = remove(text, "forgo_lib_cli");
    assert_eq!(
        out,
        "jobs:\n  t:\n    strategy:\n      matrix:\n        project:\n          - \"forgo\"\n          - forgo_lib_yaml\n"
    );
    assert_eq!(edit_ci_matrix(text, Change::Add("forgo")).unwrap(), None);
}

#[test]
fn crlf_line_endings_are_kept() {
    let text = CI.replace('\n', "\r\n");
    let out = add(&text, "forgo_lib_cli");
    assert_eq!(out, add(CI, "forgo_lib_cli").replace('\n', "\r\n"));
    let out = remove(&text, "forgo");
    assert_eq!(out, remove(CI, "forgo").replace('\n', "\r\n"));
}

#[test]
fn a_missing_final_newline_stays_missing() {
    let text = "jobs:\n  t:\n    strategy:\n      matrix:\n        project:\n          - forgo\n          - forgo_lib_yaml";
    assert_eq!(
        add(text, "forgo_lib_cli"),
        "jobs:\n  t:\n    strategy:\n      matrix:\n        project:\n          - forgo\n          - forgo_lib_cli\n          - forgo_lib_yaml"
    );
}

#[test]
fn only_matrix_project_lists_are_edited() {
    // A `project:` list elsewhere isn't a CI matrix.
    let text = "\
env:
  project:
    - forgo
jobs:
  t:
    with:
      project:
        - forgo
    strategy:
      matrix:
        project:
          - forgo
";
    assert_eq!(
        remove(text, "forgo"),
        text.replacen(
            "        project:\n          - forgo\n",
            "        project: []\n",
            1
        )
    );
}

#[test]
fn a_file_without_a_matrix_list_is_an_error() {
    let text = "jobs:\n  t:\n    runs-on: ubuntu-latest\n";
    assert!(
        err(edit_ci_matrix(text, Change::Add("x")))
            .contains("no `jobs.<job>.strategy.matrix.project` list")
    );
    // A computed matrix isn't a list to edit.
    let text = "jobs:\n  t:\n    strategy:\n      matrix:\n        project: ${{ fromJSON(needs.plan.outputs.crates) }}\n";
    assert!(
        err(edit_ci_matrix(text, Change::Add("x")))
            .contains("no `jobs.<job>.strategy.matrix.project` list")
    );
}

#[test]
fn items_that_are_not_plain_strings_are_errors() {
    let alias = "\
crates: &crates
  - forgo
jobs:
  t:
    strategy:
      matrix:
        project:
          - *crates
          - forgo_lib_yaml
";
    assert!(
        err(edit_ci_matrix(alias, Change::Add("x"))).contains("not a string"),
        "alias"
    );
    let number = "jobs:\n  t:\n    strategy:\n      matrix:\n        project: [forgo, 3]\n";
    assert!(
        err(edit_ci_matrix(number, Change::Add("x"))).contains("not a string"),
        "number"
    );
}

#[test]
fn invalid_yaml_is_an_error() {
    let text = "jobs:\n  t: [unclosed\n";
    assert!(err(edit_ci_matrix(text, Change::Add("x"))).starts_with(".github/workflows/ci.yaml: "));
}

const RELEASE: &str = "\
name: Release

on:
  workflow_dispatch:
    inputs:
      project:
        type: choice
        description: \"Select project to release\"
        required: true
        default: forgo
        options:
          - forgo
          - forgo_lib_yaml
      version:
        description: \"Version (e.g., 1.0.0)\"
        required: true
";

#[test]
fn release_options_add_keeps_default() {
    let out = edit_release_options(RELEASE, Change::Add("forgo_lib_cli"), true)
        .unwrap()
        .unwrap();
    assert_eq!(
        out,
        RELEASE.replace(
            "          - forgo\n",
            "          - forgo\n          - forgo_lib_cli\n"
        )
    );
}

#[test]
fn release_options_remove_moves_default() {
    let out = edit_release_options(RELEASE, Change::Remove("forgo"), true)
        .unwrap()
        .unwrap();
    assert_eq!(
        out,
        RELEASE
            .replace(
                "        default: forgo\n",
                "        default: forgo_lib_yaml\n"
            )
            .replace("          - forgo\n", "")
    );
    // Removing another choice leaves `default` alone.
    let out = edit_release_options(RELEASE, Change::Remove("forgo_lib_yaml"), true)
        .unwrap()
        .unwrap();
    assert_eq!(out, RELEASE.replace("          - forgo_lib_yaml\n", ""));
}

#[test]
fn release_options_cannot_lose_the_last_default() {
    let text = RELEASE.replace("          - forgo_lib_yaml\n", "");
    let e = err(edit_release_options(&text, Change::Remove("forgo"), true));
    assert!(e.contains("leaves no project for `default`"), "{e}");
}

#[test]
fn release_options_missing_list() {
    let text = "on:\n  push: {}\n";
    assert_eq!(
        edit_release_options(text, Change::Remove("x"), false).unwrap(),
        None
    );
    let e = err(edit_release_options(text, Change::Add("x"), true));
    assert!(
        e.contains("no `on.workflow_dispatch.inputs.project.options` list"),
        "{e}"
    );
}

#[test]
fn mentions_are_whole_words() {
    let text = "a: forgo_lib_yaml\nb: forgo_lib_yaml_bench\nc: crates/forgo_lib_yaml/x\nd: xforgo_lib_yaml\n";
    let found: Vec<usize> = mentions(text, "forgo_lib_yaml")
        .into_iter()
        .map(|(n, _)| n)
        .collect();
    assert_eq!(found, [1, 3]);
}

#[test]
fn bare_cr_line_breaks_are_an_error_not_a_rewrite() {
    // YAML reads a lone CR as a line break, but the layout can't be kept
    // line by line, so the edit is refused rather than re-rendered.
    let text = CI.replace('\n', "\r");
    let e = err(edit_ci_matrix(&text, Change::Add("forgo_lib_cli")));
    assert!(e.contains("edit it by hand"), "{e}");
}
