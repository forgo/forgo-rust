//! Editing: exact text after each edit. Every case also runs with CRLF
//! line endings, where the result must be the same text with CRLF.

use forgo_lib_toml::{Document, EditError, Item, Seg};

type Edit = fn(&mut Document) -> Result<(), EditError>;

fn edit(src: &str, f: Edit) -> String {
    let mut doc = Document::parse(src).unwrap_or_else(|e| panic!("{src:?}: {e}"));
    f(&mut doc).unwrap_or_else(|e| panic!("{src:?}: {e}"));
    doc.to_string()
}

/// `f` turns `src` into `want`, with LF and with CRLF. (Text without a
/// line break has no style to keep: new lines get LF.)
fn check(src: &str, f: Edit, want: &str) {
    assert_eq!(edit(src, f), want, "LF");
    if src.contains('\n') {
        let crlf = |s: &str| s.replace('\n', "\r\n");
        assert_eq!(edit(&crlf(src), f), crlf(want), "CRLF");
    }
}

fn fails(src: &str, f: Edit) -> EditError {
    let mut doc = Document::parse(src).unwrap();
    let err = f(&mut doc).expect_err("the edit should fail");
    assert_eq!(doc.to_string(), src, "a failed edit changes nothing");
    err
}

const MANIFEST: &str = r#"[package]
name = "demo"
version = "0.1.0"     # bumped by CI
edition = "2021"

# Runtime.
[dependencies]
serde = { version = "1.0", features = ["derive"] }
tokio = "1"

[dev-dependencies]
proptest = "1"

[features]
default = ["std"]
std = []
"#;

// ---------------------------------------------------------------------
// Untouched documents
// ---------------------------------------------------------------------

#[test]
fn untouched_documents_write_back_exactly() {
    for src in [
        MANIFEST,
        "",
        "a = 1",
        "\u{feff}a = 1\r\n",
        "# only a comment",
        "a = \"\"\"\nx\r\n\"\"\"\n",
    ] {
        assert_eq!(Document::parse(src).unwrap().to_string(), src);
    }
}

// ---------------------------------------------------------------------
// Replacing values
// ---------------------------------------------------------------------

#[test]
fn replace_keeps_the_key_spacing_and_comment() {
    check(
        MANIFEST,
        |d| d.set(["package", "version"], "0.2.0"),
        &MANIFEST.replace("\"0.1.0\"     # bumped", "\"0.2.0\"     # bumped"),
    );
    check("a = 1 # one\n", |d| d.set(["a"], "x"), "a = \"x\" # one\n");
    check("a=1\n", |d| d.set(["a"], 2.5), "a=2.5\n");
}

#[test]
fn replace_inside_inline_tables_arrays_and_dotted_keys() {
    check(
        MANIFEST,
        |d| d.set(["dependencies", "serde", "version"], "1.0.219"),
        &MANIFEST.replace("version = \"1.0\"", "version = \"1.0.219\""),
    );
    check(
        "a = [1, 2, 3]\n",
        |d| d.set([Seg::Key("a"), Seg::Index(1)], 20),
        "a = [1, 20, 3]\n",
    );
    check(
        "a.b.c = 1\n",
        |d| d.set(["a", "b", "c"], true),
        "a.b.c = true\n",
    );
    check(
        "x = { y = { z = 1 } }\n",
        |d| d.set(["x", "y"], Item::table([("w", Item::from(2))])),
        "x = { y = { w = 2 } }\n",
    );
    check(
        "s = \"\"\"\nmulti\nline\n\"\"\" # c\n",
        |d| d.set(["s"], "one"),
        "s = \"one\" # c\n",
    );
}

#[test]
fn replace_in_an_array_of_tables() {
    let lock = "version = 4\n\n[[package]]\nname = \"a\"\nversion = \"1.0.0\"\n\n[[package]]\nname = \"b\"\nversion = \"2.0.0\"\n";
    check(
        lock,
        |d| {
            d.set(
                [Seg::Key("package"), Seg::Index(1), Seg::Key("version")],
                "2.1.0",
            )
        },
        &lock.replace("2.0.0", "2.1.0"),
    );
}

#[test]
fn values_are_written_in_toml_1_0_syntax() {
    check(
        "x = 0\n",
        |d| {
            let item = Item::table([
                ("s", Item::from("tab\there \"q\"")),
                ("f", Item::from(1.0)),
                ("list", Item::array([Item::from(1), Item::from("two")])),
                ("needs quotes", Item::from(true)),
            ]);
            d.set(["x"], item)
        },
        "x = { s = \"tab\\there \\\"q\\\"\", f = 1.0, list = [1, \"two\"], \"needs quotes\" = true }\n",
    );
}

// ---------------------------------------------------------------------
// Adding keys
// ---------------------------------------------------------------------

#[test]
fn a_new_key_follows_the_last_line_of_its_section() {
    check(
        MANIFEST,
        |d| d.set(["package", "license"], "MIT"),
        &MANIFEST.replace(
            "edition = \"2021\"\n",
            "edition = \"2021\"\nlicense = \"MIT\"\n",
        ),
    );
    // Indented like the line before it; after the last key even when
    // comments and blank lines follow.
    check(
        "[a]\n  x = 1\n  # about y\n\n[b]\n",
        |d| d.set(["a", "y"], 2),
        "[a]\n  x = 1\n  y = 2\n  # about y\n\n[b]\n",
    );
    // A section without keys: right after its header.
    check(
        "[a]\n\n[b]\nx = 1\n",
        |d| d.set(["a", "k"], "v"),
        "[a]\nk = \"v\"\n\n[b]\nx = 1\n",
    );
    // Keys that need quotes.
    check("[t]\n", |d| d.set(["t", "a b"], 1), "[t]\n\"a b\" = 1\n");
}

#[test]
fn a_new_root_key_goes_before_the_first_section() {
    check(
        "a = 1\n\n[t]\nx = 1\n",
        |d| d.set(["b"], 2),
        "a = 1\nb = 2\n\n[t]\nx = 1\n",
    );
    check(
        "# file\n\n[t]\nx = 1\n",
        |d| d.set(["b"], 2),
        "# file\n\nb = 2\n\n[t]\nx = 1\n",
    );
    check(
        "# about t\n[t]\n",
        |d| d.set(["b"], 2),
        "b = 2\n\n# about t\n[t]\n",
    );
    check("", |d| d.set(["b"], 2), "b = 2\n");
    check(
        "# just a comment\n",
        |d| d.set(["b"], 2),
        "# just a comment\nb = 2\n",
    );
}

#[test]
fn a_missing_final_newline_stays_missing() {
    check("a = 1", |d| d.set(["b"], 2), "a = 1\nb = 2");
    check("[t]\nx = 1", |d| d.set(["t", "y"], 2), "[t]\nx = 1\ny = 2");
    check("[t]", |d| d.set(["t", "y"], 2), "[t]\ny = 2");
    check("a = 1", |d| d.set(["t", "y"], 2), "a = 1\n\n[t]\ny = 2");
    check("a = 1\nb = 2", |d| d.remove(["b"]).map(drop), "a = 1");
}

#[test]
fn new_keys_in_inline_tables() {
    check(
        "x = { a = 1 }\n",
        |d| d.set(["x", "b"], 2),
        "x = { a = 1, b = 2 }\n",
    );
    check(
        "x = {a=1}\n",
        |d| d.set(["x", "b"], 2),
        "x = {a=1, b = 2}\n",
    );
    check("x = {}\n", |d| d.set(["x", "b"], 2), "x = { b = 2 }\n");
    check(
        "x = { a.b = 1 }\n",
        |d| d.set(["x", "a", "c"], 2),
        "x = { a.b = 1, a.c = 2 }\n",
    );
}

#[test]
fn new_keys_in_tables_made_by_dotted_keys() {
    check(
        "a.x = 1\nb = 2\n",
        |d| d.set(["a", "y"], 3),
        "a.x = 1\nb = 2\na.y = 3\n",
    );
    check(
        "[t]\na.b.x = 1\n",
        |d| d.set(["t", "a", "b", "y"], 2),
        "[t]\na.b.x = 1\na.b.y = 2\n",
    );
}

#[test]
fn new_keys_in_tables_only_named_by_headers() {
    check(
        "x = 1\n\n# the b table\n[a.b]\ny = 2\n",
        |d| d.set(["a", "k"], 3),
        "x = 1\n\n[a]\nk = 3\n\n# the b table\n[a.b]\ny = 2\n",
    );
}

#[test]
fn missing_tables_are_created() {
    // Under the root: a new section at the end.
    check(
        MANIFEST,
        |d| d.set(["build-dependencies", "cc"], "1"),
        &format!("{MANIFEST}\n[build-dependencies]\ncc = \"1\"\n"),
    );
    check(
        "a = 1\n\n",
        |d| d.set(["t", "u", "k"], 1),
        "a = 1\n\n[t.u]\nk = 1\n",
    );
    // Under a table: dotted keys in its section.
    check(
        "[t]\nx = 1\n",
        |d| d.set(["t", "u", "v", "k"], 1),
        "[t]\nx = 1\nu.v.k = 1\n",
    );
    // Under an inline table: dotted keys inside it.
    check(
        "x = { a = 1 }\n",
        |d| d.set(["x", "b", "c"], 2),
        "x = { a = 1, b.c = 2 }\n",
    );
}

#[test]
fn new_keys_in_array_of_tables_elements() {
    check(
        "[[bin]]\nname = \"a\"\n\n[[bin]]\nname = \"b\"\n",
        |d| {
            d.set(
                [Seg::Key("bin"), Seg::Index(0), Seg::Key("path")],
                "src/a.rs",
            )
        },
        "[[bin]]\nname = \"a\"\npath = \"src/a.rs\"\n\n[[bin]]\nname = \"b\"\n",
    );
}

// ---------------------------------------------------------------------
// Removing
// ---------------------------------------------------------------------

#[test]
fn removing_a_key_takes_its_line_and_the_comments_above_it() {
    check(
        "[d]\na = 1\n# about b\n# more\nb = 2 # trailing\nc = 3\n",
        |d| d.remove(["d", "b"]).map(drop),
        "[d]\na = 1\nc = 3\n",
    );
    // A blank line separates a comment from the key below it.
    check(
        "# heading\n\nb = 2\nc = 3\n",
        |d| d.remove(["b"]).map(drop),
        "# heading\n\nc = 3\n",
    );
    // A multi-line string's lines aren't comments.
    check(
        "s = \"\"\"\n# not a comment\n\"\"\"\nt = 1\n",
        |d| d.remove(["t"]).map(drop),
        "s = \"\"\"\n# not a comment\n\"\"\"\n",
    );
    check(
        "a = [\n  1,\n  2,\n]\nb = 1\n",
        |d| d.remove(["a"]).map(drop),
        "b = 1\n",
    );
}

#[test]
fn removing_sections() {
    check(
        MANIFEST,
        |d| d.remove(["dev-dependencies"]).map(drop),
        &MANIFEST.replace("[dev-dependencies]\nproptest = \"1\"\n\n", ""),
    );
    // The last section: no blank lines left at the end.
    check(
        MANIFEST,
        |d| d.remove(["features"]).map(drop),
        &MANIFEST.replace("\n[features]\ndefault = [\"std\"]\nstd = []\n", ""),
    );
    // With its sub-sections, wherever they are, and comments above them.
    check(
        "[a]\nx = 1\n\n[b]\ny = 2\n\n# a's c\n[a.c]\nz = 3\n",
        |d| d.remove(["a"]).map(drop),
        "[b]\ny = 2\n",
    );
    // A table only named by headers goes with its last section.
    check(
        "[a.b]\nx = 1\n\n[c]\n",
        |d| d.remove(["a", "b"]).map(drop),
        "[c]\n",
    );
}

#[test]
fn removing_from_inline_tables() {
    check(
        "x = { a = 1, b = 2, c = 3 }\n",
        |d| d.remove(["x", "a"]).map(drop),
        "x = { b = 2, c = 3 }\n",
    );
    check(
        "x = { a = 1, b = 2, c = 3 }\n",
        |d| d.remove(["x", "b"]).map(drop),
        "x = { a = 1, c = 3 }\n",
    );
    check(
        "x = { a = 1, b = 2, c = 3 }\n",
        |d| d.remove(["x", "c"]).map(drop),
        "x = { a = 1, b = 2 }\n",
    );
    check(
        "x = { a = 1 }\n",
        |d| d.remove(["x", "a"]).map(drop),
        "x = {}\n",
    );
    check(
        "x = { a.b = 1, a.c = 2, d = 3 }\n",
        |d| d.remove(["x", "a"]).map(drop),
        "x = { d = 3 }\n",
    );
    // Emptied dotted tables go too.
    check(
        "x = { a.b = 1, d = 3 }\n",
        |d| d.remove(["x", "a", "b"]).map(drop),
        "x = { d = 3 }\n",
    );
}

#[test]
fn removing_dotted_keys_and_array_items() {
    check(
        "a.b = 1\na.c = 2\nd = 3\n",
        |d| d.remove(["a"]).map(drop),
        "d = 3\n",
    );
    check(
        "a.b = 1\nd = 3\n",
        |d| d.remove(["a", "b"]).map(drop),
        "d = 3\n",
    );
    check(
        "a = [1, 2, 3]\n",
        |d| d.remove([Seg::Key("a"), Seg::Index(0)]).map(drop),
        "a = [2, 3]\n",
    );
    check(
        "a = [1, 2, 3]\n",
        |d| d.remove([Seg::Key("a"), Seg::Index(2)]).map(drop),
        "a = [1, 2]\n",
    );
    check(
        "a = [ 1 ]\n",
        |d| d.remove([Seg::Key("a"), Seg::Index(0)]).map(drop),
        "a = []\n",
    );
}

#[test]
fn removing_array_of_tables_elements() {
    let src = "[[p]]\nn = 1\n\n[[p]]\nn = 2\n\n[[p]]\nn = 3\n";
    check(
        src,
        |d| d.remove([Seg::Key("p"), Seg::Index(1)]).map(drop),
        "[[p]]\nn = 1\n\n[[p]]\nn = 3\n",
    );
    check(src, |d| d.remove(["p"]).map(drop), "");
}

#[test]
fn removing_what_isnt_there() {
    let mut doc = Document::parse(MANIFEST).unwrap();
    assert!(!doc.remove(["nope"]).unwrap());
    assert!(!doc.remove(["package", "nope"]).unwrap());
    assert!(
        !doc.remove([Seg::Key("features"), Seg::Key("default"), Seg::Index(5)])
            .unwrap()
    );
    assert_eq!(doc.to_string(), MANIFEST);
}

// ---------------------------------------------------------------------
// Arrays
// ---------------------------------------------------------------------

#[test]
fn arrays_on_one_line_keep_their_style() {
    let push3: Edit = |d| d.edit_array(["a"], |v| v.push(Item::from(3))).map(drop);
    check("a = [1, 2]\n", push3, "a = [1, 2, 3]\n");
    check("a = [ 1, 2 ]\n", push3, "a = [ 1, 2, 3 ]\n");
    check("a = [1,2]\n", push3, "a = [1,2,3]\n");
    check("a = [1, 2,]\n", push3, "a = [1, 2, 3,]\n");
    check("a = []\n", push3, "a = [3]\n");
    check(
        "a = [3, 1, 2]\n",
        |d| d.edit_array(["a"], |v| v.reverse()).map(drop),
        "a = [2, 1, 3]\n",
    );
}

#[test]
fn arrays_one_item_per_line_keep_comments_with_their_items() {
    let src =
        "members = [\n    # tools\n    \"tools/x\",\n    \"crates/b\", # b\n    \"crates/a\",\n]\n";
    check(
        src,
        |d| {
            d.edit_string_set(["members"], |m| m.push("crates/c".into()))
                .map(drop)
        },
        "members = [\n    \"crates/a\",\n    \"crates/b\", # b\n    \"crates/c\",\n    # tools\n    \"tools/x\",\n]\n",
    );
    check(
        src,
        |d| {
            d.edit_string_set(["members"], |m| m.retain(|x| x != "tools/x"))
                .map(drop)
        },
        "members = [\n    \"crates/a\",\n    \"crates/b\", # b\n]\n",
    );
}

#[test]
fn arrays_without_a_trailing_comma_keep_it_that_way() {
    let src = "a = [\n  1,\n  2\n]\n";
    check(
        src,
        |d| d.edit_array(["a"], |v| v.push(Item::from(3))).map(drop),
        "a = [\n  1,\n  2,\n  3\n]\n",
    );
    check(
        src,
        |d| d.edit_array(["a"], |v| v.truncate(1)).map(drop),
        "a = [\n  1\n]\n",
    );
    check(
        src,
        |d| d.edit_array(["a"], |v| v.clear()).map(drop),
        "a = [\n]\n",
    );
    check(
        "a = [\n]\n",
        |d| d.edit_array(["a"], |v| v.push(Item::from(1))).map(drop),
        "a = [\n    1,\n]\n",
    );
}

#[test]
fn arrays_whose_items_share_lines_keep_their_layout() {
    // Items move between the places of the old ones; separators, line
    // breaks and comments stay where they are.
    let src = "a = [1, 2,  # two\n     3]\n";
    check(
        src,
        |d| d.edit_array(["a"], |v| v.reverse()).map(drop),
        "a = [3, 2,  # two\n     1]\n",
    );
    check(
        src,
        |d| d.edit_array(["a"], |v| v.push(Item::from(4))).map(drop),
        "a = [1, 2,  # two\n     3, 4]\n",
    );
    check(
        src,
        |d| d.remove([Seg::Key("a"), Seg::Index(0)]).map(drop),
        "a = [2,  # two\n     3]\n",
    );
    check(
        "a = [1,\n  2, 3]\n",
        |d| d.remove([Seg::Key("a"), Seg::Index(1)]).map(drop),
        "a = [1, 3]\n",
    );
    // Unless that would delete a comment.
    let err = fails(src, |d| d.remove([Seg::Key("a"), Seg::Index(2)]).map(drop));
    assert!(matches!(err, EditError::Unsupported(_)), "{err}");
}

#[test]
fn missing_arrays_are_created() {
    check(
        "[w]\n",
        |d| {
            d.edit_string_set(["w", "members"], |m| m.push("x".into()))
                .map(drop)
        },
        "[w]\nmembers = [\"x\"]\n",
    );
    let mut doc = Document::parse("[w]\n").unwrap();
    assert!(!doc.edit_array(["w", "members"], |_| {}).unwrap());
}

#[test]
fn unchanged_arrays_are_untouched() {
    let src = "a = [ 1,2 ,3 ]\n";
    let mut doc = Document::parse(src).unwrap();
    assert!(!doc.edit_array(["a"], |_| {}).unwrap());
    assert!(
        !Document::parse("a = [\"x\", \"y\"]")
            .unwrap()
            .edit_string_set(["a"], |_| {})
            .unwrap()
    );
    assert_eq!(doc.to_string(), src);
}

// ---------------------------------------------------------------------
// Real files
// ---------------------------------------------------------------------

#[test]
fn a_manifest() {
    check(
        MANIFEST,
        |d| {
            d.set(["package", "version"], "0.2.0")?;
            d.set(["dependencies", "anyhow"], "1")?;
            d.remove(["dependencies", "tokio"])?;
            d.edit_array(["features", "default"], |f| f.push(Item::from("serde")))?;
            d.set(
                ["features", "serde"],
                Item::array([Item::from("dep:serde")]),
            )?;
            Ok(())
        },
        r#"[package]
name = "demo"
version = "0.2.0"     # bumped by CI
edition = "2021"

# Runtime.
[dependencies]
serde = { version = "1.0", features = ["derive"] }
anyhow = "1"

[dev-dependencies]
proptest = "1"

[features]
default = ["std", "serde"]
std = []
serde = ["dep:serde"]
"#,
    );
}

#[test]
fn a_lock_file() {
    let lock = r#"# This file is automatically @generated by Cargo.
# It is not intended for manual editing.
version = 4

[[package]]
name = "demo"
version = "0.1.0"
dependencies = [
 "serde",
]

[[package]]
name = "serde"
version = "1.0.219"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "5f0e2c6ed6606019b4e29e69dbaba95b11854410e5347d525002456dbbb786b6"
"#;
    check(
        lock,
        |d| {
            let packages = d.get(["package"]).unwrap().as_array().unwrap();
            let i = packages
                .iter()
                .position(|p| p.as_table().unwrap().get("name").unwrap().as_str() == Some("demo"))
                .unwrap();
            d.set(
                [Seg::Key("package"), Seg::Index(i), Seg::Key("version")],
                "0.2.0",
            )
        },
        &lock.replace("\"0.1.0\"", "\"0.2.0\""),
    );
}

#[test]
fn this_workspace_manifest() {
    let root = include_str!("../../../Cargo.toml");
    let mut doc = Document::parse(root).unwrap();
    assert!(
        doc.edit_string_set(["workspace", "members"], |m| m
            .push("crates/forgo_lib_aaa".into()))
            .unwrap()
    );
    assert_eq!(
        doc.to_string(),
        root.replace(
            "    \"crates/forgo_dev\",\n",
            "    \"crates/forgo_dev\",\n    \"crates/forgo_lib_aaa\",\n"
        )
    );
    assert!(
        doc.edit_string_set(["workspace", "members"], |m| m
            .retain(|x| x != "crates/forgo_lib_aaa"))
            .unwrap()
    );
    assert_eq!(doc.to_string(), root);
}

// ---------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------

#[test]
fn edits_that_cant_keep_the_layout_fail_and_change_nothing() {
    let err = fails(MANIFEST, |d| d.set(["package"], 1));
    assert!(matches!(err, EditError::Unsupported(_)), "{err}");
    let err = fails("[[p]]\n", |d| d.edit_array(["p"], |v| v.clear()).map(drop));
    assert!(matches!(err, EditError::Unsupported(_)), "{err}");
}

#[test]
fn paths_that_lead_nowhere() {
    let err = fails(MANIFEST, |d| d.set(["package", "name", "x"], 1));
    assert_eq!(err.to_string(), "`package.name` isn't a table");
    let err = fails("a = [1]\n", |d| d.set([Seg::Key("a"), Seg::Index(3)], 1));
    assert_eq!(err.to_string(), "`a` has no item 3");
    let err = fails("a = 1\n", |d| d.edit_array(["a"], |_| {}).map(drop));
    assert_eq!(err.to_string(), "`a` isn't an array");
    let err = fails("a = [1]\n", |d| d.edit_string_set(["a"], |_| {}).map(drop));
    assert_eq!(err.to_string(), "`a` holds items that aren't strings");
}

// ---------------------------------------------------------------------
// Files
// ---------------------------------------------------------------------

#[test]
fn files_are_replaced_only_when_they_change() {
    let dir = std::env::temp_dir().join(format!("forgo_lib_toml_edit_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("Cargo.toml");
    std::fs::write(&file, MANIFEST).unwrap();
    let changed =
        forgo_lib_toml::edit_file_in_place(&file, |d| d.set(["package", "version"], "0.1.0"))
            .unwrap();
    assert!(!changed);
    let changed =
        forgo_lib_toml::edit_file_in_place(&file, |d| d.set(["package", "version"], "0.3.0"))
            .unwrap();
    assert!(changed);
    assert_eq!(
        std::fs::read_to_string(&file).unwrap(),
        MANIFEST.replace("0.1.0", "0.3.0")
    );
    std::fs::write(&file, "not = [toml").unwrap();
    assert!(matches!(
        forgo_lib_toml::edit_file_in_place(&file, |_| Ok(())),
        Err(EditError::Parse(_))
    ));
    std::fs::remove_dir_all(&dir).unwrap();
}
