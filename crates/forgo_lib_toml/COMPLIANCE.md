# Compliance

forgo_lib_toml implements [TOML 1.1.0](https://toml.io/en/v1.1.0) (the
default) and [TOML 1.0.0](https://toml.io/en/v1.0.0).

| Check | Inputs | Result |
|---|---|---|
| toml-test, TOML 1.1.0 | 218 valid and 502 invalid documents | all decoded exactly / all rejected |
| toml-test, TOML 1.0.0 | 208 valid and 509 invalid documents | all decoded exactly / all rejected |
| Encoder | the expected values of every valid case, both versions | all read back the same |
| Robustness | 1.8 million truncated and edited documents | no panics or hangs; all round-trip |

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

## Reproducing

```bash
crates/forgo_lib_toml/scripts/fetch-toml-test.sh
TOML_TEST_SUITE_REQUIRED=1 cargo test -p forgo_lib_toml -- --nocapture
```
