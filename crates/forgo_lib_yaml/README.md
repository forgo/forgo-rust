# forgo_lib_yaml

A zero-dependency YAML 1.2.2 parser, emitter and editor for Rust.

- **Verified YAML 1.2.2.** Passes the whole official
  [yaml-test-suite](https://github.com/yaml/yaml-test-suite) (402 cases):
  error expectations, exact event streams, JSON values, and lossless round
  trips, all gated in CI. See [COMPLIANCE.md](COMPLIANCE.md).
- **Edits that keep the file.** Comments, quoting and block scalar styles,
  flow and block collections, anchors, tags, how nulls and booleans were
  spelled, document markers and CRLF line endings all survive
  parse → edit → emit.
- **Fast.** Ahead of libyaml, yaml-rust2, saphyr and serde_yaml on every
  benchmark input, for both event parsing and loading documents, with far
  fewer allocations (see [Performance](#performance)).
- **Small and safe.** No dependencies and no `unsafe`.

## Editing a file

```rust
use forgo_lib_yaml::{normalize_string_list, Doc, Elem};

let mut doc = Doc::from_str("\
jobs:
  build:
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest] # keep sorted
")?;

doc.visit_sequences_mut(&["jobs", "build", "strategy", "matrix", "os"], |os| {
    os.push(Elem::string("windows-latest"));
    normalize_string_list(os) // sort and dedup
});

// Only the edited list changes; its comment and flow style stay.
assert_eq!(doc.to_string()?, "\
jobs:
  build:
    strategy:
      matrix:
        os: [macos-latest, ubuntu-latest, windows-latest] # keep sorted
");
# Ok::<(), forgo_lib_yaml::Error>(())
```

Paths can use `Seg::Wildcard` to visit every entry at a level, and
`edit_file_in_place` reads, edits and writes a file in one call.

## Values

Plain scalars resolve with the YAML 1.2 core schema; a quoted or block
scalar is always a string, and tags like `!!str` override.

```rust
use forgo_lib_yaml::{Doc, Node, Scalar};

let doc = Doc::from_str("port: 0x1F90\nratio: .5\nenabled: True\nname: '8080'\nempty:\n")?;
let map = doc.root().node().as_map().unwrap();
assert_eq!(map[0].1.node().as_i64(), Some(8080));
assert_eq!(map[1].1.node().as_f64(), Some(0.5));
assert_eq!(map[2].1.node().as_bool(), Some(true));
assert_eq!(map[3].1.node().as_str(), Some("8080"));
assert!(matches!(map[4].1.node(), Node::Scalar(Scalar::Null)));

// Emitting keeps every spelling: `0x1F90`, `.5`, `True`, `'8080'`, empty.
assert_eq!(doc.to_string()?, "port: 0x1F90\nratio: .5\nenabled: True\nname: '8080'\nempty:\n");
# Ok::<(), forgo_lib_yaml::Error>(())
```

Duplicate keys are kept in order, and lookups use the last one.
`Doc::check_unique_keys()` enforces the spec's uniqueness rule when you want
it.

## Events

For streaming or tooling, `events` yields parse events with source spans.
Scalar values borrow from the input unless escapes or folding change them.

```rust
use forgo_lib_yaml::{events, Event};

let mut scalars = Vec::new();
for item in events("a: [1, 'two']\n") {
    let (event, span) = item?;
    if let Event::Scalar { value, .. } = event {
        scalars.push((value.into_owned(), span.start.col));
    }
}
assert_eq!(scalars, [("a".to_string(), 0), ("1".into(), 4), ("two".into(), 7)]);
# Ok::<(), forgo_lib_yaml::Error>(())
```

`syntax::test_event::to_test_event` prints events in the yaml-test-suite
notation (also `cargo run --example events -- file.yaml`). Comments are
available from `Events::comments`.

## Errors, encodings and limits

Syntax errors carry a 1-based line and column:

```rust
use forgo_lib_yaml::Doc;

let err = Doc::from_str("a: b: c\n").unwrap_err();
assert_eq!(err.to_string(), "Parse error at 1:5: mapping values are not allowed here");
```

- `Doc::from_slice` and `events_from_bytes` take bytes and detect UTF-8,
  UTF-16 or UTF-32, with or without a byte order mark (spec §5.2).
- `ParseOptions::max_depth` (default 512) bounds nesting when composing
  documents, so hostile input gets an error instead of a stack overflow.
  The event parser itself is iterative and linear in nesting depth.

## What a round trip keeps

`parse(emit(doc)) == doc`, comparing everything including comments and
styles. This is tested on the whole official suite and about 1,000 more
cases.

| Kept | Normalized |
|------|------------|
| comments (leading, trailing, after the last node) | indentation (2 spaces per level) |
| plain / `'single'` / `"double"` / `\|` / `>` styles, with chomping and indentation indicators | spacing inside lines |
| flow `[a, b]` vs block collections | blank lines between entries |
| anchors, aliases, tags as written | folding of long quoted scalars |
| `~`, `Null`, `True`, empty values; number text like `0x1F` | a comment before `---` moves after it |
| `%YAML` / `%TAG`, `---`, `...`; CRLF line breaks | |

When a node's recorded style can't represent an edited value, the emitter
picks the nearest style that can. For example, a plain value that gains
`: ` is written double-quoted. Nodes built in code (`Elem::string(..)`) get
the natural style: plain when safe, a literal block for multi-line text,
otherwise double quotes.

## Performance

Throughput in MB/s and allocation counts on an Apple M2 Max, Rust 1.97,
release build, median of 11 runs. Reproduce with
`cargo run --release --manifest-path crates/forgo_lib_yaml_bench/Cargo.toml --bin report`.

Event parsing:

| input (size) | forgo_lib_yaml | libyaml | yaml-rust2 | saphyr-parser |
|---|---:|---:|---:|---:|
| Kubernetes manifests (0.7 MB) | **84.8** (820 allocs) | 57.1 (221k) | 58.1 (201k) | 51.9 (103k) |
| CI workflows (0.8 MB) | **97.2** (11k) | 62.3 (203k) | 68.5 (188k) | 60.4 (110k) |
| deep nesting (0.6 MB) | **292.3** (36) | 126.9 (20k) | 207.8 (20k) | 120.3 (10k) |
| long scalars (1.5 MB) | **511.5** (1.3k) | 154.3 (2.7k) | 373.2 (2.6k) | 400.3 (2.2k) |
| flow-heavy (0.8 MB) | **43.7** (8k) | 34.5 (544k) | 31.3 (536k) | 28.6 (280k) |

Loading documents (forgo_lib_yaml also keeps comments, styles and spans):

| input | forgo_lib_yaml | serde_yaml | yaml-rust2 | saphyr |
|---|---:|---:|---:|---:|
| Kubernetes manifests | **51.0** | 32.1 | 39.4 | 40.6 |
| CI workflows | **61.3** | 35.9 | 44.8 | 43.3 |
| deep nesting | **192.4** | 94.2 | 147.8 | 146.4 |
| long scalars | **498.4** | 150.3 | 376.0 | 306.4 |
| flow-heavy | **27.1** | 18.7 | 22.8 | 23.6 |

Agreement with the official suite on accept/reject: forgo_lib_yaml 402/402,
yaml-rust2 and saphyr-parser 402/402 as event parsers (400/402 as loaders,
which reject duplicate keys), serde_yaml 333/402, libyaml 335/402.

## Testing

```sh
scripts/fetch-yaml-test-suite.sh   # pinned official suite data, once
cargo test -p forgo_lib_yaml
```

See [tests/README.md](tests/README.md) for how the tests are organized.
