# YAML 1.2.2 compliance

forgo_lib_yaml passes the whole official YAML test suite with exact
results, and the YAML 1.2.2 rules that the suite doesn't test. Every
check below runs in CI, and any regression fails the build.

## Official test suite

[yaml-test-suite](https://github.com/yaml/yaml-test-suite), release
`data-2022-01-17` (commit `6e6c296a`): 402 cases, 308 valid and 94
invalid.

| Check | Result | Passes when |
|---|---:|---|
| `error` | 402/402 | invalid input is rejected, valid accepted |
| `events` | 308/308 | the events equal `test.event` exactly |
| `json` | 279/279 | the loaded data equals `in.json` |
| `coverage` | 308/308 | no input byte is silently skipped |
| `source` | 308/308 | untouched documents write back unchanged |
| `roundtrip` | 308/308 | rendered output reads back as the same data |
| `presentation` | 308/308 | it also keeps every comment and style |

```sh
scripts/fetch-yaml-test-suite.sh
cargo test -p forgo_lib_yaml --test yaml_test_suite -- --nocapture
```

The report goes to `target/tmp/yaml-test-suite-report.md`.
`tests/yaml_test_suite/known_failures.txt` lists accepted failures; it is
empty.

## Other libraries

The same suite run through every Rust YAML library in use, with default
options, by [forgo_lib_yaml_bench](../forgo_lib_yaml_bench) on November
23, 2026 ([full results](../forgo_lib_yaml_bench/RESULTS.md)).

Loaders, out of 308 valid cases, 94 invalid cases and 279 with
expected JSON:

| Library | Valid | Invalid | JSON | Round trip | Overall |
|---|--:|--:|--:|--:|--:|
| **forgo_lib_yaml** | **308** | **94** | **279** | **308** | **100%** |
| yaml-rust2 0.13.2 | 308 | 94 | 279 | 307 | 99.8% |
| noyalib 0.0.63 | 308 | 94 | 278 | 308 | 99.8% |
| yaml-edit 0.3.6 | 308 | 94 | 278 | 308 | 99.8% |
| saphyr 0.4.0 | 308 | 94 | 278 | 293 | 96.3% |
| serde-saphyr 2.2.0 | 307 | 91 | 265 | 307 | 95.8% |
| yaml-rust 0.4.5 | 279 | 66 | 241 | 277 | 82.8% |
| rust-yaml 1.1.0 | 308 | 32 | 270 | 308 | 82.3% |
| serde_yaml 0.9.34 ¹ | 257 | 78 | 230 | 254 | 80.3% |
| yaml-peg 1.0.9 | 186 | 41 | 124 | 159 | 41.8% |

¹ serde_yaml_ng, serde_norway and yaml_serde score the same (libyaml).

Event parsers, against the suite's expected events:

| Library | Valid | Invalid | Exact events | Same data |
|---|--:|--:|--:|--:|
| **forgo_lib_yaml** | **308** | **94** | **308** | **308** |
| yaml-rust2 0.13.2 | 308 | 94 | — | 308 |
| saphyr-parser 0.4.0 | 308 | 94 | — | 308 |
| granit-parser 2.2.0 | 308 | 91 | 269 | 308 |
| libyaml 0.2.11 | 257 | 78 | 252 | 252 |
| libyaml-safer 0.3.0 | 255 | 78 | 251 | 251 |
| yaml-rust 0.4.5 | 279 | 66 | — | 258 |

- **Overall** counts the cases where every check that applies passes.
- **—**: the event API doesn't report anchor names, flow versus block
  style, or explicit `---` and `...`.
- `2JQS` and `X38W` repeat a mapping key, which a loader may report as
  an error (§3.2.1.1), so rejecting them counts as a pass.
- Other languages, same loader checks: yaml (JavaScript) 96.5%, js-yaml
  92.5%, ruamel.yaml 80.3%, go-yaml 76.4%, PyYAML 74.1%.

## Beyond the suite

| Test | Inputs | Result |
|---|---|---|
| Spec gaps | 38 cases | all pass |
| Robustness | 970,514 mutated inputs | no failures |
| Editing | 1,741 edits and 211 emptied collections on 320 files | all pass |

- **Spec gaps** (`tests/yaml_spec_gaps.rs`) cover rules the suite skips:
  UTF-16 and UTF-32 input, byte order marks, CR and CRLF line breaks,
  NEL, LS and PS as content, spec examples 5.10, 5.13, 5.14, 6.17, 6.25,
  6.27 and 7.22, `%YAML 1.3` versus `2.0`, the 1024-character key limit,
  and top-level block scalar indicators. Each also passes the round-trip
  checks.
- **Robustness** (`tests/yaml_robustness.rs`) derives inputs from every
  suite case by truncation and one-character edits. None panics or hangs.
  The event parser and the loader always agree on validity. Every input
  that parses writes back byte for byte, an edit to it reads back as the
  edited document, and its rendering keeps all content, comments and
  styles.
- **Editing** (`tests/yaml_editing.rs`) changes, deletes, appends and
  renames entries one at a time, in realistic files and in every valid
  suite case, and compares the output with the exact expected text. It
  also empties each nested block collection without anchors: the strict
  writer must keep the layout around it.

## Choices where the spec is open

Each choice is pinned by a test in `tests/yaml_spec_gaps.rs`.

- **Closing bracket at column 0.** `key: [` … `]` with `]` at column 0
  is accepted, as libyaml, yaml-rust2 and saphyr accept it. Read
  literally, the grammar wants it indented past the parent. Content lines
  must still be indented: `key: [a,\nb]` is an error, as in the suite.
- **Top-level indentation indicators.** `--- |2` puts content at column
  2, counting from column 0 as libyaml, yaml-rust2 and saphyr do. The
  grammar read literally (parent indentation −1) gives one less.
- **Line breaks.** NEL, LS and PS are content, not line breaks, as YAML
  1.2 specifies. YAML 1.1 parsers break lines on them.
- **Byte order marks.** Allowed at the start of the stream, at the start
  of each document, and inside quoted scalars. Anywhere else they are an
  error.
- **`%YAML` versions.** `1.x` is accepted; `2.0` and later are rejected
  (§6.8.1).
- **Duplicate keys.** Kept in order; lookups and path edits use the last
  one. `Doc::check_unique_keys()` enforces uniqueness (§3.2.1.1) on
  request, comparing keys by value, so `a` and `"a"` collide, as do `1`
  and `0x1`.
- **Aliases.** An alias to an undefined or later anchor is an error when
  loading documents (§7.1). The event stream reports it as written.
- **Nesting.** Loading stops at `ParseOptions::max_depth` (512 by
  default) with an error rather than risking a stack overflow. The event
  parser has no depth limit and runs in time linear in the depth.
- **Surrogate escapes.** `"\uD83D\uDE00"`, a JSON-style surrogate pair,
  decodes to one character (😀). A lone surrogate escape is an error.

## History

The parser was rebuilt in November 2026. Measured with the same harness,
the previous one scored:

| Check | Before | Now |
|---|---:|---:|
| `error` | 294/402 (73.1%) | 402/402 |
| `events` | not available | 308/308 |
| `json` | 109/279 (39.1%) | 279/279 |

Its earlier "306–316/363" figure came from a harness that checked only
accept/reject, mis-decoded the suite's visual markers and found 363 of
the 402 cases.
