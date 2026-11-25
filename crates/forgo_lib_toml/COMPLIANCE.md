# Compliance

forgo_lib_toml implements [TOML 1.1.0](https://toml.io/en/v1.1.0) (the
default) and [TOML 1.0.0](https://toml.io/en/v1.0.0).

| Check | Inputs | Result |
|---|---|---|
| toml-test, TOML 1.1.0 | 218 valid and 502 invalid documents | all decoded exactly / all rejected |
| toml-test, TOML 1.0.0 | 208 valid and 509 invalid documents | all decoded exactly / all rejected |
| Encoder | the expected values of every valid case, both versions | all read back the same |
| Robustness | 1.8 million truncated and edited documents | no panics or hangs; all round-trip |
| Editing | 8,160 edits at every path of every valid toml-test document | 7,508 made, each checked independently; 652 refused, none wrong |
| toml-test's own runner | the Go runner, built at the pinned commit, on a decoder and encoder | all pass, both versions |
| Other parsers | 50,958 mutated documents, against `toml`, `toml_edit`, toml-spanner | same values wherever both accept; every other difference is theirs (below) |

## toml-test

[toml-test](https://github.com/toml-lang/toml-test) is TOML's official
test suite. `tests/toml_test.rs` runs it at commit `96fd088` (pinned in
`scripts/fetch-toml-test.sh`), taking the cases from the suite's own list
for each version (`files-toml-1.1.0` and `files-toml-1.0.0`).

| Version | Valid: decoded exactly | Invalid: rejected | Encoder: read back the same |
|---|---:|---:|---:|
| 1.1.0 | 218 / 218 | 502 / 502 | 218 / 218 |
| 1.0.0 | 208 / 208 | 509 / 509 | 208 / 208 |

- **Valid.** The document must parse, and its values must equal the
  expected tagged JSON under toml-test's own comparison rules (its
  `json.go`):
  - strings and integers exactly;
  - floats as numbers, with any NaN equal to any NaN;
  - offset date-times as the same instant, and local date-times, dates
    and times field by field.

  Tables must have exactly the expected keys, and arrays the expected
  items in order. The test reads the expected date-times with its own
  RFC 3339 reader, not the crate's parser.
- **Invalid.** The document must be rejected.
- **Encoder.** The expected values of each valid case are built as an
  `Item`, encoded, and parsed back as TOML 1.0 (so the output suits
  either version). The values must be equal again under the same rules.

## toml-test's own runner

The tests above read the suite themselves. As an independent check, the
bench crate builds toml-test's own runner (Go) from the same commit and
runs it on two small programs around forgo_lib_toml: a decoder (TOML in,
tagged JSON out) and an encoder (tagged JSON in, TOML out). The runner
reads the encoder's output with BurntSushi/toml, the Go reference
implementation, so the encoder check doesn't rely on forgo_lib_toml's
parser.

| TOML | Valid | Invalid | Encoder |
|---|---:|---:|---:|
| 1.1 | 218 / 218 | 502 / 502 | 218 / 218 |
| 1.0 | 208 / 208 | 509 / 509 | 208 / 208 |

`crates/forgo_lib_toml_bench/scripts/official-toml-test.sh` reproduces
it.

## Agreement with other parsers

`crates/forgo_lib_toml_bench/tests/differential.rs` makes 50,958 documents
from the valid toml-test cases (each truncated, and with one of 17
characters inserted, at a dozen places). Each is read by forgo_lib_toml
and by the three other Rust crates that parse TOML 1.1 into values:
`toml` 1.2.3, `toml_edit` 0.25.21 and toml-spanner 1.0.3.

- **Values agree.** Wherever forgo_lib_toml and another crate both accept
  a document, they read the same values: 20,021 of 20,021 for each.
- **forgo_lib_toml accepts nothing they reject.**
- **They accept 46 documents that forgo_lib_toml rejects.** All three
  accept the same 46, and each one has a line break inside a key/value
  pair of an inline table, such as `t = { a` on one line and `= 1 }` on
  the next. TOML 1.1 allows line breaks *between* an inline table's
  pairs, but not within one: its ABNF has `keyval = key keyval-sep val`
  and `keyval-sep = ws %x3D ws`, where `ws` is spaces and tabs only. Each
  of the 46 reads the same as theirs once the pair is on one line. The
  test checks exactly this, and fails on any other kind of difference.

## Where the versions differ

The two lists hold the same cases except for these. They come from
comparing the files, not from a list written by hand.

| Cases | TOML 1.0 | TOML 1.1 |
|---|---|---|
| `inline-table/linebreak-01` to `-04`, `inline-table/trailing-comma`; `inline-table/newline`, `inline-table/newline-comment`, `key/empty-05` | invalid | valid |
| `string/basic-byte-escapes`; `string/hex-escape`, `string/escape-esc` | invalid | valid |
| `datetime/no-secs`, `local-datetime/no-secs`, `local-time/no-secs`; `datetime/no-seconds` | invalid | valid |
| `control/multi-cr`, `control/rawmulti-cr` (a lone carriage return in a multi-line string) | not tested | invalid |
| the specification's own examples (`spec-1.0.0/*`, `spec-1.1.0/*`) | 48 valid, 8 invalid | 52 valid, 8 invalid |

**A lone carriage return in a multi-line string.** TOML 1.0's text lists
carriage return among the characters a multi-line string may contain, but
its ABNF allows one only before a line feed. TOML 1.1 forbids it, and
toml-test checks it only for 1.1. forgo_lib_toml follows the ABNF and
rejects it in both versions.

## What the specification leaves to implementations

| Question | Answer |
|---|---|
| Integer range | 64-bit, as TOML requires. Out of range is an error, never a rounded value. |
| Floats | IEEE 754 binary64. `-0.0` and the sign of `nan` are kept. |
| Fractional seconds | Nine digits (nanoseconds). Further digits are dropped, not rounded, as TOML requires. |
| Leap seconds | `:60` is accepted, as RFC 3339 allows. |
| Years and offsets | Years 0000 to 9999 (RFC 3339), offsets up to ±23:59. |
| Nesting | Arrays and inline tables nest at most 128 deep by default; deeper input is an error, not a stack overflow. |
| Size | A document must be smaller than 4 GiB. |
| Key order | Tables keep keys in the order they were defined. |
| Byte order mark | One at the start is skipped. |

## Beyond toml-test

- **Robustness** (`tests/toml_robustness.rs`). Every toml-test document,
  valid and invalid, is cut at every byte. It is also edited at about 20
  places: a deletion, and 27 different characters each replacing a byte
  and inserted before it. Both versions parse every input: 1.8 million in
  all. None may panic or take longer than 250 ms; the slowest takes tens of
  microseconds. Every input that parses must encode to TOML that parses
  back to the same values. Nesting 100,000 deep must be an error.
- **The API** (`tests/api.rs`): error kinds and positions, a table of
  5,000 keys (the hash index, and duplicates found through it), key order,
  the version differences, date-times, and the encoder's exact output.
- **Doctests**: the README's examples and the API documentation's.

## Editing

Edits change the text in place, so they're checked against TOML itself
rather than against a re-rendering.

**At run time,** each edit is checked one of two ways.

- **Values and new lines are parsed in place.** That covers a replaced
  value, a rewritten array or inline table, and a key/value line added to
  a section. The new text is parsed by the same parser, where it sits,
  in its context. That enforces TOML's syntax, duplicate keys and table
  rules, and the result must read back as the value asked for.
- **Other changes re-parse the whole document.** That covers removing
  lines or sections, and adding a section for a table that so far exists
  only as part of a longer header. The new text is parsed from scratch
  and compared with the old document, and it must differ by exactly the
  change.

An edit that fails a check returns an error and leaves the document as it
was. An edit that can't keep the layout returns
`EditError::Unsupported` instead of re-rendering anything.

**Tests.**

- **Every edit on every document** (`tests/edit_robustness.rs`). Each
  valid toml-test document, under its own version (426 in all), gets
  these edits at every path:
  - replace the value;
  - add a key to every table, and a key under two new tables;
  - remove the value;
  - reverse and extend every array.

  Of the 8,160 edits:
  - 7,508 are made. The test then checks each one with its own code,
    which doesn't share the crate's checks: the path holds what was
    asked, every other value is unchanged, and the text parses back from
    scratch to the same values.
  - 637 are refused because they would replace a table written as
    `[sections]` or dotted keys with a value.
  - 15 are refused because they would delete a comment. They are in the
    suite's two deliberately tricky comment files.
- **Edits in a row** (also `tests/edit_robustness.rs`). Chains of edits
  on one document, alternating replace, add, extend and remove: 1,659
  made in sequence, each later edit working on text the earlier ones
  moved.
- **Positions match a fresh parse.** In debug builds, so in all of
  these tests, every edit made in place is compared with a fresh parse of
  its text: values, key order, how each table was made, and where
  everything is.
- **Exact text** (`tests/editing.rs`). 31 cases check the exact output
  of edits on manifests, a Cargo.lock and this workspace's own
  Cargo.toml. Every case also runs with CRLF line endings.

## Reproducing

```bash
crates/forgo_lib_toml/scripts/fetch-toml-test.sh
TOML_TEST_SUITE_REQUIRED=1 cargo test -p forgo_lib_toml -- --nocapture
```
