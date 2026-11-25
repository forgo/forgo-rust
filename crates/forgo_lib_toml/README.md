# forgo_lib_toml

[TOML](https://toml.io) 1.1 and 1.0 for Rust: read documents, build them in
code, and write them. No dependencies, no `unsafe`.

- **Complete.** Passes every case of
  [toml-test](https://github.com/toml-lang/toml-test), TOML's official
  suite, for both versions. Every valid document decodes to the expected
  values, every invalid one is rejected, and every expected value encodes
  to TOML that reads back the same ([COMPLIANCE.md](COMPLIANCE.md)).
- **Fast.** About 1.1 GB/s on a Cargo.lock: 5 times the `toml` crate, 12
  times `toml_edit`, and within 15% of the fastest, toml-spanner, which
  uses `unsafe` ([How it compares](#how-it-compares)).
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
let serde = doc.get(&["dependencies", "serde"]).unwrap().as_table().unwrap();
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

[`Document`]: https://docs.rs/forgo_lib_toml/latest/forgo_lib_toml/struct.Document.html
[`Table`]: https://docs.rs/forgo_lib_toml/latest/forgo_lib_toml/struct.Table.html
[`Array`]: https://docs.rs/forgo_lib_toml/latest/forgo_lib_toml/struct.Array.html
[`Value`]: https://docs.rs/forgo_lib_toml/latest/forgo_lib_toml/enum.Value.html
[`Item`]: https://docs.rs/forgo_lib_toml/latest/forgo_lib_toml/enum.Item.html
