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
- **Fast.** About 1.1 GB/s on a Cargo.lock: 5 times the `toml` crate, 12
  times `toml_edit`, and within 15% of the fastest, toml-spanner, which
  uses `unsafe`. Editing and writing a manifest is 5 to 7 times faster
  than with `toml_edit` ([How it compares](#how-it-compares)).
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

Parse throughput in MB/s, higher is better. Measured on an Apple M-series
Mac with Rust 1.97.1, on November 25, 2026, by
`cargo run --release --bin quick` in
[`crates/forgo_lib_toml_bench`](../forgo_lib_toml_bench):

| Crate | Cargo.lock (1 MB) | mixed (1 MB) | Cargo.toml (1 KB) |
|---|---:|---:|---:|
| **forgo_lib_toml** | **1,168** | **395** | **566** |
| toml-spanner 1.0.3 | 1,365 | 399 | 674 |
| boml 2.0.1 | 295 | 134 | 146 |
| toml 1.2.3 (`DeTable`) | 233 | 75 | 141 |
| toml 1.2.3 (`Table`) | 158 | 48 | 84 |
| taplo 0.14.0 (syntax tree only) | 117 | 65 | 57 |
| toml_edit 0.25.21 | 98 | 46 | 56 |
| toml-span 0.7.2 | 89 | no date-times | 48 |
| basic-toml 0.1.10 | 44 | no date-times | 40 |

The inputs:

- **Cargo.lock**: 4,500 packages, as Cargo writes them.
- **mixed**: every kind of value, including escapes, multi-line strings,
  inline tables, dotted keys and arrays of tables.
- **Cargo.toml**: a typical package manifest.

toml-spanner is faster on the lock file and the manifest by about 15%.
It uses `unsafe` code and returns values that borrow the input, while
forgo_lib_toml has no `unsafe` and copies the input into the document.

Editing and writing, in µs, lower is better (`--bin quick -- --edit`):

| Task | forgo_lib_toml | parsed for editing | toml_edit 0.25.21 |
|---|---:|---:|---:|
| Cargo.toml: set a version, add a dependency, write | **3.4** (4.9×) | **2.4** (7.1×) | 16.7 |
| Cargo.lock (1 MB): set one version, write | **2,149** (6.2×) | **1,204** (11.1×) | 13,419 |

Every forgo_lib_toml edit is checked as described in [Edit](#edit);
`toml_edit` re-renders its document model.

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
