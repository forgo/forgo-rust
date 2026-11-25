# forgo_lib_yaml

A YAML 1.2.2 parser, emitter and editor for Rust. No dependencies, no
`unsafe`.

- **Complete.** Passes all 402 cases of the official
  [yaml-test-suite](https://github.com/yaml/yaml-test-suite) with exact
  results, plus the YAML 1.2.2 rules the suite skips
  ([COMPLIANCE.md](COMPLIANCE.md)).
- **Edits keep the file.** Untouched documents write back byte for byte,
  and an edit rewrites only what changed. Comments, blank lines,
  indentation and quoting everywhere else stay as they were.
- **Fast.** The fastest Rust YAML loader on every input tested, with the
  fewest allocations ([How it compares](#how-it-compares)).

## Edit a file

```rust
use forgo_lib_yaml::{Doc, Elem, Node};

let mut doc = Doc::from_str("\
image: nginx:1.27    # pinned
replicas: 2

env:
  - MODE=production
")?;

doc.visit_values_mut(&["image"], |v| {
    *v.node_mut() = Node::string("nginx:1.28");
    true
});
doc.visit_sequences_mut(&["env"], |env| {
    env.push(Elem::string("LOG=info"));
    true
});

// Only the changed value and the new item differ from the input.
assert_eq!(doc.to_string()?, "\
image: nginx:1.28    # pinned
replicas: 2

env:
  - MODE=production
  - LOG=info
");
# Ok::<(), forgo_lib_yaml::Error>(())
```

- Paths take `*` to match every entry at a level.
- `edit_file_in_place` updates the lists at a path in every document of
  a file, sorted and deduplicated, in one call.
- `Doc::from_stream` and `Doc::to_stream_string` handle multi-document
  files.
- `Doc::to_string_preserving` and `Doc::to_stream_string_preserving`
  return an error rather than re-render a document whose layout an edit
  can't keep, for tools that must never reformat a file.
  `try_normalize_string_list` likewise refuses a list with items that
  aren't strings, rather than converting them.

## Read values

Plain scalars are typed by the YAML 1.2 core schema. Quoted and block
scalars are always strings, and tags such as `!!str` override.

```rust
use forgo_lib_yaml::{Doc, Node, Scalar};

let doc = Doc::from_str("port: 0x1F90\nratio: .5\nenabled: True\nname: '8080'\nempty:\n")?;
let map = doc.root().node().as_map().unwrap();
assert_eq!(map[0].1.node().as_i64(), Some(8080));
assert_eq!(map[1].1.node().as_f64(), Some(0.5));
assert_eq!(map[2].1.node().as_bool(), Some(true));
assert_eq!(map[3].1.node().as_str(), Some("8080"));
assert!(matches!(map[4].1.node(), Node::Scalar(Scalar::Null)));
# Ok::<(), forgo_lib_yaml::Error>(())
```

Duplicate keys are kept in order, and lookups use the last one.
`Doc::check_unique_keys()` enforces the spec's uniqueness rule when you
want it.

## Build YAML in code

```rust
use forgo_lib_yaml::{Doc, Elem, MapKey, Node};

let root = Elem::new(Node::Map(vec![
    (MapKey::from("name"), Elem::string("demo")),
    (MapKey::from("ports"), Elem::new(Node::Seq(vec![Elem::number(80), Elem::number(443)]))),
    (MapKey::from("motd"), Elem::string("Hello,\nworld!\n").with_comment("shown at login")),
]));
assert_eq!(Doc::new(root).to_string()?, "\
name: demo
ports:
  - 80
  - 443
motd: | # shown at login
  Hello,
  world!
");
# Ok::<(), forgo_lib_yaml::Error>(())
```

The emitter quotes a string only when it has to, and picks a block
scalar for multi-line text. `Elem::with_style` asks for a specific style.

## Stream events

`events` parses without building a tree, with the source span of each
event. Values borrow from the input unless escapes or folding change
them.

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

`Events::comments` gives the comments. `syntax::test_event::to_test_event`
prints events in the yaml-test-suite notation (`cargo run --example
events -- file.yaml`).

## Errors, encodings and limits

```rust
use forgo_lib_yaml::Doc;

let err = Doc::from_str("a: b: c\n").unwrap_err();
assert_eq!(err.to_string(), "Parse error at 1:5: mapping values are not allowed here");
```

- Errors carry a 1-based line and column.
- `Doc::from_slice` and `events_from_bytes` read UTF-8, UTF-16 or
  UTF-32, with or without a byte order mark (spec §5.2).
- `ParseOptions::max_depth` (512 by default; set it with
  `ParseOptions::default().with_max_depth(n)`) turns hostile nesting into
  an error instead of a stack overflow.

## How output is written

| Part of the document | Written as |
|---|---|
| untouched document | its source text, byte for byte |
| unchanged entry | its source text, with comments and blank lines |
| changed scalar | new text in place; the rest of the line stays |
| new or rewritten entry | in the file's indentation style |
| emptied block list or mapping | `[]` or `{}` in its place |
| built in code, or `reformat()` | the emitter's layout (below) |

The emitter indents by two spaces, writes `- key: value` items compactly
and keeps each node's comments, quoting, block style, anchors, tags and
spellings (`~`, `True`, `0x1F`). Every edit is checked: the output is
parsed again and must equal the edited document, or the whole document
is rendered instead (the `*_preserving` methods return an error).

## How it compares

Every Rust YAML library in use on crates.io, measured on November 23,
2026 by [forgo_lib_yaml_bench](../forgo_lib_yaml_bench). The full tables
are in [RESULTS.md](../forgo_lib_yaml_bench/RESULTS.md).

### Compliance

Share of the 402 official test cases passed, counting every check that
applies: accept or reject, the loaded data, and a write and re-read.

| Library | Version | Suite | Keeps comments | Edits keep layout |
|---|---|--:|:-:|:-:|
| **forgo_lib_yaml** | 0.0.0 | **100%** | ✓ | ✓ |
| yaml-rust2 | 0.13.2 | 99.8% | — | — |
| noyalib | 0.0.63 | 99.8% | `cst` | `cst` |
| yaml-edit | 0.3.6 | 99.8% | ✓ | ✓ |
| saphyr | 0.4.0 | 96.3% | — | — |
| serde-saphyr | 2.2.0 | 95.8% | per value | — |
| yaml-rust | 0.4.5 | 82.8% | — | — |
| rust-yaml | 1.1.0 | 82.3% | partly | — |
| serde_yaml ¹ | 0.9.34 | 80.3% | — | — |
| yaml-peg | 1.0.9 | 41.8% | — | — |

¹ serde_yaml_ng 0.10.0, serde_norway 0.9.42 and yaml_serde 0.10.7 share
its parser (libyaml) and score the same.

- Only forgo_lib_yaml reproduces the suite's expected parse events
  exactly: 308 of 308, against 269 for granit-parser and 252 for libyaml.
  yaml-rust2 and saphyr-parser match at the data level, but their event
  APIs omit anchor names and styles.
- Other languages, same checks: yaml (JavaScript) 96.5%, js-yaml 92.5%,
  ruamel.yaml 80.3%, go-yaml 76.4%, PyYAML 74.1%.

### Performance

The same inputs for every library, release builds, each number the
median of 11 runs in a fresh process, on an Apple M2 Max with Rust
1.97.1. MB is 10⁶ bytes.

Loading 800 Kubernetes manifests (0.70 MB) into each library's tree:

| Library | Load MB/s | 1 KB µs | Allocs | Heap MB | Emit MB/s |
|---|--:|--:|--:|--:|--:|
| **forgo_lib_yaml** | **50.8** | **17.3** | **73k** | 8.7 | **330** |
| noyalib | 41.0 | 23.8 | 130k | 12.3 | 135 |
| saphyr | 40.2 | 23.2 | 123k | 11.6 | 198 |
| yaml-rust2 | 39.8 | 25.7 | 314k | 8.7 | 167 |
| yaml-rust | 39.1 | 24.2 | 195k | 10.1 | 211 |
| yaml-edit | 31.7 | 38.1 | 271k | 17.5 | — |
| serde_yaml ¹ | 31.0 | 30.5 | 366k | 10.2 | 63.3 |
| serde-saphyr | 19.7 | 46.2 | 86k | **4.7** | 131 |

- **Load**: parse into the library's tree. forgo_lib_yaml is also
  fastest on the other inputs: CI workflows 56.9, deep nesting 194, long
  scalars 493, flow-heavy 27.5 MB/s.
- **1 KB**: microseconds to load one small manifest.
- **Allocs** and **Heap**: allocations and peak heap during one load.
  serde-saphyr keeps data only; forgo_lib_yaml's tree also keeps every
  comment, style and source position.
- **Emit**: rendering the tree as YAML, in MB of output per second.
  Writing back an untouched parsed document copies its source, at 3,190
  MB/s.

Parsing to events (no tree), MB/s:

| Library | k8s | CI | deep | long | flow |
|---|--:|--:|--:|--:|--:|
| **forgo_lib_yaml** | **85.5** | **98.6** | **289** | **511** | 43.8 |
| libyaml-safer | 71.3 | 78.7 | 130 | 161 | **47.2** |
| yaml-rust | 62.3 | 67.6 | 140 | 235 | 41.1 |
| yaml-rust2 | 61.4 | 67.1 | 200 | 360 | 31.8 |
| libyaml | 59.3 | 66.4 | 127 | 159 | 35.5 |
| saphyr-parser | 54.4 | 60.5 | 120 | 393 | 29.7 |
| granit-parser | 39.3 | 45.3 | 141 | 469 | 23.0 |

Inputs: 800 Kubernetes manifests (0.70 MB), 1,500 CI workflows (0.78
MB), 40 deeply nested blocks (0.64 MB), long scalars of every style (1.45
MB), and 8,000 flow mappings (0.84 MB).

## Testing

```sh
scripts/fetch-yaml-test-suite.sh   # official suite data, pinned (once)
cargo test -p forgo_lib_yaml
```

[tests/README.md](tests/README.md) describes each test.
