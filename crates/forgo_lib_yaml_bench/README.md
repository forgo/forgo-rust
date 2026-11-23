# forgo_lib_yaml_bench

Compares `forgo_lib_yaml` with the other Rust YAML libraries: compliance
with the official yaml-test-suite, and speed and memory on the same inputs.

This crate is outside the Cargo workspace and has its own `Cargo.lock`, so
the compared crates never reach the main lockfile, audit or release.

## Run

```sh
crates/forgo_lib_yaml/scripts/fetch-yaml-test-suite.sh   # suite data, once
cargo run --release --manifest-path crates/forgo_lib_yaml_bench/Cargo.toml \
  --bin report > report.md
```

The report is GitHub-flavored markdown and takes about 3 minutes.
[RESULTS.md](RESULTS.md) is its latest output.

| flag | effect |
|---|---|
| `--compliance` | compliance only (about 1 minute) |
| `--performance` | performance only |
| `--quick` | 3 timing runs instead of 11 |
| `--verbose` | every failing case, with the reason, on stderr |

`cargo bench --manifest-path crates/forgo_lib_yaml_bench/Cargo.toml` runs
the same parse and load measurements under criterion.

The report also checks popular parsers in other languages when they are
installed (compliance only; their speed isn't comparable in-process):

```sh
crates/forgo_lib_yaml_bench/other/setup.sh   # python3, npm, go; once
```

It installs pinned versions into `target/other/` only: PyYAML and
ruamel.yaml (Python venv), yaml and js-yaml (npm prefix), and go-yaml
(`go.yaml.in/yaml/v3`, built from `other/go/`). Missing runtimes are
skipped.

## Libraries

Every YAML parser or loader on crates.io that people use, at the exact
versions pinned in `Cargo.toml` (the report prints them):

| library | parser |
|---|---|
| forgo_lib_yaml | own |
| serde_yaml, serde_yaml_ng, serde_norway, yaml_serde | libyaml (C, transpiled) |
| libyaml (`unsafe-libyaml`) | libyaml (C, transpiled) |
| libyaml-safer | libyaml, ported to safe Rust |
| serde-saphyr, granit-parser | granit-parser (saphyr fork) |
| saphyr, saphyr-parser | saphyr-parser (yaml-rust2 fork) |
| yaml-rust2, yaml-rust | own |
| noyalib (`Value` and lossless `cst`), yaml-edit | own |
| rust-yaml, yaml-peg | own |

Not compared:

- **Wrappers of a compared parser:** serde_yml 0.0.13 (forwards to
  noyalib), serde_yaml2 (yaml-rust2), marked-yaml (yaml-rust2),
  yaml-spanned (libyaml-safer), libyaml-rs and unsafe-libyaml-norway
  (libyaml), serde_yaml_bw (granit-parser and libyaml).
- **Not loaders:** yaml_parser and pretty_yaml (formatter CST),
  tree-sitter-yaml, yamlpath and yamlpatch (queries and patches).
- **Single-product forks:** dbt-yaml, serde_yaml_with_quirks.
- **Little used or unmaintained:** located_yaml, yaml-rust-davvid, the
  `libyaml` C bindings, fyaml (libfyaml; its build script runs git).

## Compliance

Suite release `data-2022-01-17`: 402 cases, 308 valid and 94 invalid.
Every library runs with its default options.

| column | passes when |
|---|---|
| valid | a valid input parses without error |
| invalid | an invalid input is rejected |
| events | the event stream equals `test.event` exactly |
| data events | the same nodes, anchors, tags, scalar types and values |
| JSON | the loaded documents equal `in.json` (279 cases) |
| round trip | load, emit with the library's own emitter, load: same data |
| overall | every check that applies to the case passes, out of 402 |

- **events** needs anchor names, flow vs. block and explicit `---`/`...`
  from the event API; libraries without them show `—`. granit-parser
  reports anchor names with `emit_anchor_events`, and its explicit `...`
  is read from the event's span.
- **data events** ignore scalar styles, flow vs. block, document markers
  and how nulls, booleans and numbers are spelled. Scalars compare by
  their YAML 1.2 core-schema type and value, anchors by position.
- **JSON** ignores tags and compares numbers by value (`1` equals `1.0`).
- Libraries without their own tree type load into a minimal generic serde
  value defined here (`AnyValue`, via `deserialize_any`).
- `2JQS` and `X38W` repeat a mapping key. YAML 1.2.2 §3.2.1.1 makes that
  an error a loader may report, so a loader rejecting them passes.
- yaml-edit's parser is error-tolerant: an input counts as rejected when
  its `Validator` reports an error. Values are read the way yaml-edit's
  own suite harness reads them.
- Each library runs in worker processes, one case at a time with a 5 s
  limit, so a panic, hang or crash fails only that case. The report
  lists them.
- Other-language parsers get the loader checks through small drivers
  (`other/`) that print each case's documents as JSON. Values typed
  beyond the core schema (timestamps, binary, sets) count as mismatches.

## Performance

| measurement | unit | what is timed |
|---|---|---|
| parse to events | MB/s | parse and count events; no tree |
| load | MB/s | load into the library's own tree, then drop it |
| 1 KB load | µs | load one small config document |
| allocations | count | one load of the k8s input |
| peak heap | MB | most live heap during that load |
| emit | MB/s | write a loaded tree back to YAML (MB of output) |

- MB is 10⁶ bytes. Inputs are generated, so every run uses the same text.
- Each measurement runs in a fresh process: the median of 11 runs after a
  warm-up, batched so a sample lasts at least 2 ms. A run over 1 s is
  timed once, and a measurement is stopped after 60 s.
- Allocations are counted by a wrapping global allocator, switched off
  while timing.
- A loader must return the right number of documents; otherwise the cell
  says `wrong`.
- serde-saphyr and noyalib stop at 1,024 and 1,000 documents by default
  (serde-saphyr also at depth 64). The benchmarks lift those limits.
- Trees differ in what they keep: forgo_lib_yaml's `Doc` keeps comments,
  styles and source spans, yaml-edit and noyalib's `cst` keep every byte,
  and the serde crates keep data only.
- **emit** renders the whole tree: forgo_lib_yaml's documents are
  `reformat`ted first, because an untouched parsed `Doc` writes its source
  text back (reported separately, not compared). The lossless trees of
  yaml-edit and noyalib's `cst` always write their source back.

## Layout

| path | contents |
|---|---|
| `src/suite.rs` | case discovery, `test.event`, the event normal form |
| `src/data.rs` | the common data model and the `in.json` reader |
| `src/libs/` | one adapter per library |
| `src/inputs.rs` | the benchmark inputs |
| `src/bin/report.rs` | the report |
| `benches/parse.rs` | criterion benchmarks |
| `other/` | drivers and setup for the other-language parsers |
