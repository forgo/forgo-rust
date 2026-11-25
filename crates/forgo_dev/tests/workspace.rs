//! `[workspace].members` edits, checked against exact output text.

use forgo_dev::workspace::{add_member, members, remove_member};

const MANIFEST: &str = "\
[workspace]
resolver = \"3\"
members = [
    \"crates/forgo\",
    \"crates/forgo_dev\",
    \"crates/forgo_lib_yaml\",
]
# Standalone benchmark crate with its own lockfile.
exclude = [\"crates/forgo_lib_yaml_bench\"]
";

#[test]
fn adds_in_order() {
    assert_eq!(
        add_member(MANIFEST, "crates/forgo_lib_cli")
            .unwrap()
            .unwrap(),
        MANIFEST.replace(
            "    \"crates/forgo_dev\",\n",
            "    \"crates/forgo_dev\",\n    \"crates/forgo_lib_cli\",\n"
        )
    );
    assert_eq!(add_member(MANIFEST, "crates/forgo").unwrap(), None);
}

#[test]
fn removes() {
    assert_eq!(
        remove_member(MANIFEST, "crates/forgo_dev")
            .unwrap()
            .unwrap(),
        MANIFEST.replace("    \"crates/forgo_dev\",\n", "")
    );
    assert_eq!(remove_member(MANIFEST, "crates/absent").unwrap(), None);
}

#[test]
fn lists_members() {
    assert_eq!(
        members(MANIFEST).unwrap(),
        ["crates/forgo", "crates/forgo_dev", "crates/forgo_lib_yaml"]
    );
}

#[test]
fn keeps_the_arrays_style() {
    // On one line, unsorted, single-quoted: sorted, still on one line, each
    // entry quoted as it was; everything outside the array is kept.
    let text = "\
# The workspace.
[workspace]
members = ['crates/b', \"crates/a\"]  # all crates
default-members = [\"crates/b\"]

[workspace.package]
edition = \"2024\"
";
    assert_eq!(
        add_member(text, "crates/c").unwrap().unwrap(),
        "\
# The workspace.
[workspace]
members = [\"crates/a\", 'crates/b', \"crates/c\"]  # all crates
default-members = [\"crates/b\"]

[workspace.package]
edition = \"2024\"
"
    );
}

#[test]
fn comments_and_brackets_inside_the_array() {
    let text = "[workspace]\nmembers = [\n    \"crates/a\", # the [first] one\n    # \"crates/old\",\n    \"crates/b\",\n]\n";
    assert_eq!(members(text).unwrap(), ["crates/a", "crates/b"]);
    // The entry goes with the comment on its line; the comment line above
    // `crates/b` stays with it.
    assert_eq!(
        remove_member(text, "crates/a").unwrap().unwrap(),
        "[workspace]\nmembers = [\n    # \"crates/old\",\n    \"crates/b\",\n]\n"
    );
}

#[test]
fn default_members_is_not_members() {
    let text = "[workspace]\ndefault-members = [\"crates/a\"]\nmembers = [\"crates/a\"]\n";
    assert_eq!(
        add_member(text, "crates/b").unwrap().unwrap(),
        "[workspace]\ndefault-members = [\"crates/a\"]\nmembers = [\"crates/a\", \"crates/b\"]\n"
    );
}

#[test]
fn only_the_workspace_table() {
    // `members` under another table isn't the workspace's.
    let text = "[package.metadata]\nmembers = [\"x\"]\n\n[workspace]\nmembers = [\"crates/a\"]\n";
    assert_eq!(
        add_member(text, "crates/b").unwrap().unwrap(),
        "[package.metadata]\nmembers = [\"x\"]\n\n[workspace]\nmembers = [\"crates/a\", \"crates/b\"]\n"
    );
}

#[test]
fn crlf_is_kept() {
    let text = MANIFEST.replace('\n', "\r\n");
    assert_eq!(
        add_member(&text, "crates/forgo_lib_cli").unwrap().unwrap(),
        add_member(MANIFEST, "crates/forgo_lib_cli")
            .unwrap()
            .unwrap()
            .replace('\n', "\r\n")
    );
}

#[test]
fn errors_instead_of_guessing() {
    let no_table = "[package]\nname = \"x\"\n";
    assert!(add_member(no_table, "crates/a").is_err());
    // A second `[workspace]` table is never appended.
    let no_members = "[workspace]\nresolver = \"3\"\n";
    assert!(add_member(no_members, "crates/a").is_err());
    let not_array = "[workspace]\nmembers = \"crates/a\"\n";
    assert!(add_member(not_array, "crates/b").is_err());
    let unclosed = "[workspace]\nmembers = [\"crates/a\",\n";
    assert!(add_member(unclosed, "crates/b").is_err());
    let not_strings = "[workspace]\nmembers = [1]\n";
    assert!(add_member(not_strings, "crates/b").is_err());
}

#[test]
fn escapes_are_read_and_kept() {
    // `\\` in a basic string is one backslash; literal strings keep theirs.
    let escapes = "[workspace]\nmembers = [\"crates\\\\a\"]\n";
    assert_eq!(members(escapes).unwrap(), ["crates\\a"]);
    assert_eq!(
        add_member(escapes, "crates/b").unwrap().unwrap(),
        "[workspace]\nmembers = [\"crates/b\", \"crates\\\\a\"]\n"
    );
    let literal = "[workspace]\nmembers = ['crates\\a']\n";
    assert_eq!(members(literal).unwrap(), ["crates\\a"]);
}
