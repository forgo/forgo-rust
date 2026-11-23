# YAML 1.2.2 compliance

## Results

Against the official [yaml-test-suite](https://github.com/yaml/yaml-test-suite)
data release `data-2022-01-17` (commit `6e6c296a`, 402 cases: 308 valid, 94
invalid), every check passes and is gated in CI:

| check          | result  | passes when |
|----------------|---------|-------------|
| `error`        | 402/402 | the input is rejected exactly when the case has an `error` file |
| `events`       | 308/308 | the event stream equals `test.event`, character for character |
| `json`         | 279/279 | the composed documents equal `in.json` |
| `coverage`     | 308/308 | every input byte is inside a token or comment, or is white space: nothing is dropped |
| `roundtrip`    | 308/308 | parse → emit → parse keeps the content (normalized events: types, values, tags, anchors) |
| `presentation` | 308/308 | parse → emit → parse gives an equal `Doc`, including comments and styles |

Beyond the suite:

- **Spec gaps** (`tests/yaml_spec_gaps.rs`, 38 cases): YAML 1.2.2 rules the
  suite doesn't exercise. UTF-16LE/BE and UTF-32LE/BE input with and without
  a BOM, BOMs per document, CR and CRLF line breaks, NEL/LS/PS as content,
  spec examples 5.10, 5.13, 5.14, 6.17, 6.25, 6.27 and 7.22, `%YAML 1.3`
  vs `2.0`, the 1024-character implicit key limit, top-level block scalar
  indicators. Each valid case also passes the round-trip and presentation
  checks.
- **Robustness** (`tests/yaml_robustness.rs`): ~970,000 inputs derived from
  the suite by truncation and single-character mutation.
  - No panics or hangs on any of them.
  - The event parser and the composer always agree on validity.
  - Every input that parses is fully covered, and re-emits to YAML with the
    same content.
  - On these unusual inputs 48 differ only in presentation (comments in odd
    positions, such as between `?` and its key). That count is a ratchet
    and can only go down.
- **Self-authored tests**: 491 round-trip fixtures and about 500 API,
  feature, unit, integration and regression tests also check that
  re-parsing the output gives an equal document.

```sh
scripts/fetch-yaml-test-suite.sh
cargo test -p forgo_lib_yaml --test yaml_test_suite -- --nocapture
```

The report is written to `target/tmp/yaml-test-suite-report.md`. Checks
are gated by `tests/yaml_test_suite/known_failures.txt`, which is empty: a
new failure, or a listed failure that starts passing, fails the build.

## Decisions where the suite is silent

These choices are deliberate. Each is pinned by a test in
`tests/yaml_spec_gaps.rs`.

- **Closing bracket at column 0.** `key: [` … `]` with the closing bracket
  at column 0 is accepted, as yaml-rust2, saphyr and libyaml accept it.
  Read literally, the grammar requires the closing line to be indented past
  the enclosing block. Content lines must still be indented (`key: [a,\nb]`
  is an error, as in the suite).
- **Top-level indentation indicators.** An explicit indicator at the top
  level counts from column 0: `--- |2` means content at column 2, as in
  libyaml, yaml-rust2 and saphyr. The grammar read literally (parent
  indent −1) would give one less. Auto-detected top-level content may still
  start at column 0, which the suite requires.
- **Line breaks.** NEL, LS and PS are ordinary characters, not line breaks,
  as YAML 1.2 specifies (YAML 1.1 parsers break lines on them).
- **Byte order marks.** A BOM is allowed at the start of the stream and of
  each document, and inside quoted scalars. Anywhere else (plain or block
  scalars, comments, mid-document) it is an error.
- **`%YAML` versions.** `1.x` is accepted and `2.0` or higher is rejected
  (spec §6.8.1).
- **Duplicate keys.** They are kept in order, and lookups and path edits
  use the last one. `Doc::check_unique_keys()` enforces spec §3.2.1.1 on
  request, comparing keys by value, so `a` and `"a"` collide, as do `1` and
  `0x1`.
- **Aliases.** An alias to an undefined or later anchor is an error when
  composing documents (§7.1). The event stream reports it as written.
- **Nesting.** Composing stops at `ParseOptions::max_depth` (default 512)
  with an error, instead of risking a stack overflow in recursive tree
  code. The event parser has no depth limit, and its time is linear in
  nesting depth.
- **Surrogate escapes.** `"\uD83D\uDE00"` (a JSON-style surrogate pair)
  decodes to one character (😀). A lone surrogate escape is an error.

## Presentation

`parse(emit(doc)) == doc` holds for the suite, the spec gaps and the
fixtures.

Kept:
- comments, and anchors and tags as written;
- scalar style: plain, single or double quoted, or literal/folded with
  their chomping and indentation indicators;
- flow or block collections;
- how plain nulls and booleans were spelled (`~`, `Null`, `True`, or
  empty);
- number text such as `0x1F`;
- directives, `---` and `...`, and CRLF line breaks.

Normalized:
- indentation (two spaces per level);
- spacing inside lines;
- blank lines between entries;
- line folding of long flow and quoted scalars;
- a comment before `---` moves after it;
- a comment in an unusual position (between `?` and a key, or inside a
  flow pair) can move to the neighbouring node.

## History

Before the rebuild (measured on 2026-11-22 with the same harness):

| check  | legacy parser |
|--------|---------------|
| error  | 294/402 (73.1%) |
| events | not available (no event output) |
| json   | 109/279 (39.1%) |

The previously reported "306–316/363" came from a harness that checked only
accept/reject, mis-decoded the suite's visual markers, and found 363 of the
402 cases.
