# forgo_lib_toml

[TOML](https://toml.io) 1.1 and 1.0 for Rust: read documents, edit them
in place, build them in code, and write them. No dependencies, no
`unsafe`.

- **Complete.** Passes every case of
  [toml-test](https://github.com/toml-lang/toml-test), TOML's official
  suite, for both versions. Every valid document decodes to the expected
  values, every invalid one is rejected, and every expected value encodes
  to TOML that reads back the same ([COMPLIANCE.md](COMPLIANCE.md)).
- **Edits keep the file.** An edit changes only the text it must:
  comments, blank lines, indentation, quoting, key order, line endings
  and a missing final newline all stay. Each edit is checked, and one
  that can't keep the layout is refused rather than re-rendered.
- **Fast.** Reads zed's Cargo.lock in full at about 700 MB/s: nearly 4
  times the `toml` crate and 8 times `toml_edit`. Only toml-spanner, which
  uses `unsafe`, reads faster (by 10 to 40%). Editing is the fastest
  measured: 5 to 10 times `toml_edit` ([How it compares](#how-it-compares)).
- **Errors say what and where:** a kind, a message, and a line and
  column.
- **Hardened.** 1.8 million mutated documents parse without a panic or a
  hang. Nesting is limited, and large tables are indexed with a hash
  seeded per process, so crafted keys can't slow lookups down.

## Read

```rust
use forgo_lib_toml::Document;

let doc = Document::parse(r#"
[package]
name = "demo"
version = "0.4.2"
keywords = ["toml", "parser"]

[dependencies]
serde = { version = "1", features = ["derive"] }
"#)?;

let package = doc.root().get("package").unwrap().as_table().unwrap();
assert_eq!(package.get("name").unwrap().as_str(), Some("demo"));
assert_eq!(package.get("keywords").unwrap().as_array().unwrap().len(), 2);

// A path of keys from the root.
let serde = doc.get(["dependencies", "serde"]).unwrap().as_table().unwrap();
assert!(serde.is_inline());

// Tables keep their keys in the order they were defined.
assert_eq!(doc.root().keys().collect::<Vec<_>>(), ["package", "dependencies"]);
# Ok::<(), forgo_lib_toml::Error>(())
```

A [`Document`] owns its data; [`Table`], [`Array`] and [`Value`] are
borrowed views of it, so reading allocates nothing. Strings without
escapes point into the document's copy of the source text.

```rust
use forgo_lib_toml::{Document, Offset, Value};

let doc = Document::parse("
released = 1979-05-27T07:32:00.5-07:00
ratio = 0.25
sizes = [1, 2, 3]
")?;

let released = doc.root().get("released").unwrap().as_datetime().unwrap();
assert_eq!(released.date.unwrap().year, 1979);
assert_eq!(released.time.unwrap().nanosecond, 500_000_000);
assert_eq!(released.offset, Some(Offset::Minutes(-420)));
assert_eq!(released.to_string(), "1979-05-27T07:32:00.5-07:00");

let total: i64 = doc.root().get("sizes").unwrap().as_array().unwrap()
    .iter()
    .filter_map(|v| v.as_integer())
    .sum();
assert_eq!(total, 6);

for (key, value) in doc.root().iter() {
    match value {
        Value::Float(f) => assert_eq!((key, f), ("ratio", 0.25)),
        other => assert!(["released", "sizes"].contains(&key), "{}", other.type_name()),
    }
}
# Ok::<(), forgo_lib_toml::Error>(())
```

## Edit

```rust
use forgo_lib_toml::{Document, Item};

let mut doc = Document::parse("\
[package]
name = \"demo\"
version = \"0.1.0\"   # bumped by CI

[dependencies]
serde = { version = \"1\", features = [\"derive\"] }
tokio = \"1\"

[features]
default = [
    \"std\",
]
")?;

doc.set(["package", "version"], "0.2.0")?;
doc.set(["dependencies", "serde", "version"], "1.0.219")?;
doc.set(["dependencies", "anyhow"], "1")?;
doc.remove(["dependencies", "tokio"])?;
doc.edit_array(["features", "default"], |items| items.push(Item::from("serde")))?;

assert_eq!(doc.to_string(), "\
[package]
name = \"demo\"
version = \"0.2.0\"   # bumped by CI

[dependencies]
serde = { version = \"1.0.219\", features = [\"derive\"] }
anyhow = \"1\"

[features]
default = [
    \"std\",
    \"serde\",
]
");
# Ok::<(), Box<dyn std::error::Error>>(())
```

- [`Document::set`] replaces a value where it is written, or adds a key
  after the last line of its table's section (indented like it), or into
  its inline table. Missing tables are made with dotted keys in the
  nearest table that exists, or as a new `[section]` under the root.
- [`Document::remove`] removes the key/value line and the comment lines
  directly above it, a table's `[sections]`, or an item of a list.
- [`Document::edit_array`] changes an array as a list of items: the items
  that stay keep their text, and in an array written one item per line,
  their comments move with them. [`Document::edit_string_set`] keeps an
  array of strings sorted and without duplicates, as for
  `[workspace].members`.
- Paths take keys and indexes: `[Seg::Key("package"), Seg::Index(3),
  Seg::Key("version")]` reaches into a `[[package]]` array of tables.
- [`edit_file_in_place`] reads a file, edits it, and replaces it
  atomically (a temporary file renamed over it), only if it changed.

**How edits are checked.** The new text of a value, or a new line in its
section, is parsed where it is, in its context, by the same parser:
syntax, duplicate keys and TOML's rules on tables are all enforced, and
the value must read back as the one set. Removing lines or sections,
and adding a section for a table that so far exists only as part of a
longer `[header]`, re-parse the whole document and compare it with the
old one: exactly the change, nothing else. An edit that fails leaves the document as it
was. An edit that can't keep the layout returns
`EditError::Unsupported`: for example replacing a table written as a
`[section]` with a value, or a removal from a list that would delete a
comment.

Parse with `ParseOptions::default().for_editing()` when you'll edit: it
records where things are while parsing (5 to 15% slower), so the first
edit doesn't parse again. `edit_file_in_place` does this.

## Write

An [`Item`] is an owned value to build in code and encode:

```rust
use forgo_lib_toml::Item;

let manifest = Item::table([
    ("package", Item::table([
        ("name", Item::from("demo")),
        ("version", Item::from("0.1.0")),
    ])),
    ("features", Item::table([
        ("default", Item::array([Item::from("std")])),
    ])),
]);
assert_eq!(manifest.to_toml_string()?, "\
[package]
name = \"demo\"
version = \"0.1.0\"

[features]
default = [\"std\"]
");
# Ok::<(), forgo_lib_toml::EncodeError>(())
```

The output uses TOML 1.0's syntax, so any TOML parser reads it.
`Document::to_item` and `Value::to_item` copy parsed values into items.

## Versions

TOML 1.1 is the default. TOML 1.0 is an option:

```rust
use forgo_lib_toml::{Document, ParseOptions, TomlVersion};

let v1_0 = ParseOptions::default().with_version(TomlVersion::V1_0);
assert!(Document::parse_with("point = { x = 1, y = 2, }", &v1_0).is_err());
assert!(Document::parse("point = { x = 1, y = 2, }").is_ok());
```

| | TOML 1.0 | TOML 1.1 |
|---|---|---|
| inline tables over several lines, with comments and a trailing comma | error | allowed |
| the `\e` and `\xHH` escapes | error | allowed |
| times without seconds (`07:32`) | error | allowed (`07:32:00`) |

## Errors

```rust
use forgo_lib_toml::{Document, ErrorKind};

let err = Document::parse("[server]\nport = 8080\nport = 8081\n").unwrap_err();
assert_eq!(err.kind(), ErrorKind::Duplicate);
assert_eq!((err.line(), err.column()), (3, 1));
assert_eq!(err.to_string(), "the key `port` is already defined at line 3, column 1");
```

The kinds: `Syntax`, `String`, `Number`, `Datetime`, `Duplicate`, `Table`
(defined twice, or extended where TOML doesn't allow it), `Depth`,
`TooLarge` and `Encoding` (input that isn't UTF-8, from
`Document::from_slice`).

## Limits

- **Integers** are 64-bit. A value out of range is an error, never a
  rounded number.
- **Floats** are 64-bit. `-0.0` and the sign of `nan` are kept.
- **Date-times**: years 0000 to 9999, offsets up to ±23:59, seconds up to
  60 (a leap second). Fractions of a second keep nine digits; the rest
  are dropped, not rounded, as TOML requires.
- **Nesting**: arrays and inline tables nest at most 128 deep
  (`ParseOptions::with_max_depth` changes it). Deeper input is an error,
  not a stack overflow.
- **Size**: a document must be smaller than 4 GiB.

## How it compares

Against the other Rust TOML crates, measured on November 25, 2026 (Apple
M-series, Rust 1.97.1). The full report, and how to reproduce it:
[RESULTS.md](../forgo_lib_toml_bench/RESULTS.md).

**Compliance.** toml-test, TOML 1.1: share of valid documents accepted and
invalid ones rejected. TOML 1.0 and the other crates are in the report.

| Crate | Valid (218) | Invalid (493) |
|---|---:|---:|
| **forgo_lib_toml** | **100%** | **100%** |
| toml-spanner 1.0.3 | 100% | 100% |
| toml 1.2.3 | 100% | 100% |
| toml_edit 0.25.21 | 100% | 100% |
| boml 2.0.1 | 100% | 71.2% |
| taplo 0.14.0 | 91.7% | 99.2% |
| toml-span 0.7.2 | 89.4% | 99.2% |
| basic-toml 0.1.10 | 85.3% | 99.2% |

toml-test's own runner passes forgo_lib_toml on every case too, for both
versions. On 50,958 mutated documents, forgo_lib_toml, `toml`,
`toml_edit` and toml-spanner read the same values wherever they all
accept a document. The other three also accept 46 documents that TOML
1.1 forbids (a line break inside an inline table's key/value pair);
forgo_lib_toml rejects them ([COMPLIANCE.md](COMPLIANCE.md)).

**Speed.** Parse, then read every key and value, in MB/s (higher is
better):

| Crate | zed Cargo.lock | zed Cargo.toml | ruff pyproject.toml | Cargo.toml 1 KB |
|---|---:|---:|---:|---:|
| **forgo_lib_toml** | **690** | **497** | **1,162** | **437** |
| toml-spanner 1.0.3 | 953 | 664 | 1,495 | 587 |
| boml 2.0.1 | 290 | 169 | 455 | 149 |
| toml 1.2.3 (`DeTable`) | 186 | 141 | 248 | 139 |
| toml 1.2.3 (`Table`) | 130 | 88 | 181 | 84 |
| toml_edit 0.25.21 | 87 | 71 | 135 | 55 |

forgo_lib_toml is 3.1 to 4.7 times as fast as `toml`'s fastest interface
and 7 to 9 times `toml_edit`, on every input measured. toml-spanner is
1.1 to 1.4 times as fast as forgo_lib_toml. It uses `unsafe` code, and
its values borrow the input, while forgo_lib_toml copies the input into
the document.

**Memory.** forgo_lib_toml holds the least at its peak of all the crates
measured: 1.7 MiB for zed's 489 KiB Cargo.lock, text included. It makes
15 allocations for that parse; toml-spanner makes 11, and the others
thousands.

**Writing.** Writing TOML from a value tree is the fastest measured: zed's
Cargo.lock in 786 µs, against 955 for `toml` and 1,965 for toml-spanner.

**Editing.** Parse, edit, write, in µs (lower is better). The crates write
identical text:

| Task | forgo_lib_toml | parsed for editing | toml-spanner | toml_edit |
|---|---:|---:|---:|---:|
| 1 KB Cargo.toml: set a version, add a dependency | **3.3** | **2.2** | 4.1 | 16.2 |
| zed's Cargo.lock: set one version | **1,291** | **709** | 1,960 | 7,208 |

Every forgo_lib_toml edit is checked, as described in [Edit](#edit).

[`Document`]: https://docs.rs/forgo_lib_toml/latest/forgo_lib_toml/struct.Document.html
[`Table`]: https://docs.rs/forgo_lib_toml/latest/forgo_lib_toml/struct.Table.html
[`Array`]: https://docs.rs/forgo_lib_toml/latest/forgo_lib_toml/struct.Array.html
[`Value`]: https://docs.rs/forgo_lib_toml/latest/forgo_lib_toml/enum.Value.html
[`Item`]: https://docs.rs/forgo_lib_toml/latest/forgo_lib_toml/enum.Item.html
[`Document::set`]: https://docs.rs/forgo_lib_toml/latest/forgo_lib_toml/struct.Document.html#method.set
[`Document::remove`]: https://docs.rs/forgo_lib_toml/latest/forgo_lib_toml/struct.Document.html#method.remove
[`Document::edit_array`]: https://docs.rs/forgo_lib_toml/latest/forgo_lib_toml/struct.Document.html#method.edit_array
[`Document::edit_string_set`]: https://docs.rs/forgo_lib_toml/latest/forgo_lib_toml/struct.Document.html#method.edit_string_set
[`edit_file_in_place`]: https://docs.rs/forgo_lib_toml/latest/forgo_lib_toml/fn.edit_file_in_place.html
