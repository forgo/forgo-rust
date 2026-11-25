# forgo_lib_yaml tests

```sh
scripts/fetch-yaml-test-suite.sh   # official suite data, pinned (once)
cargo test -p forgo_lib_yaml
```

Without the suite data, the suite-based tests skip with a warning. They
fail instead when `CI` or `YAML_TEST_SUITE_REQUIRED` is set.

## Compliance

| Target | Inputs | Result |
|---|---|---|
| `yaml_test_suite` | 402 official cases | 7 checks, all 100% |
| `yaml_spec_gaps` | 38 hand-made cases | all pass |
| `yaml_robustness` | 970,514 mutated inputs | no failures |
| `yaml_editing` | 320 files: 1,741 edits, 211 emptied collections | all pass |

**`yaml_test_suite`** runs the official
[yaml-test-suite](https://github.com/yaml/yaml-test-suite), release
`data-2022-01-17`. For each case:

- `error`: the input is rejected exactly when the case is invalid.
- `events`: the event stream equals `test.event`, character for character.
- `json`: the loaded data equals `in.json`.
- `coverage`: every input byte is inside a token or a comment, or is
  white space, so nothing is silently dropped.
- `source`: untouched documents write back byte for byte.
- `roundtrip`: rendered by the emitter and parsed again, the content is
  unchanged (types, values, tags, anchors).
- `presentation`: rendered and parsed again, the documents are equal,
  comments and styles included.

A failure must be listed in `yaml_test_suite/known_failures.txt`, which
is empty, so any regression fails the build. The full report goes to
`target/tmp/yaml-test-suite-report.md`.

**`yaml_spec_gaps`** covers YAML 1.2.2 rules the suite doesn't test:
UTF-16 and UTF-32 input, byte order marks, CR and CRLF line breaks, NEL,
LS and PS, spec examples missing from the suite, `%YAML` versions, the
1024-character key limit, and the choices listed in
[COMPLIANCE.md](../COMPLIANCE.md). Expected events are derived from the
spec by hand.

**`yaml_robustness`** derives inputs from every suite case: each prefix,
and at each character a replacement or an insertion of one of 25
YAML-significant characters (`:`, `-`, `#`, quotes, tabs, line breaks, a
byte order mark, …) or a deletion. On every result it checks:

- it returns `Ok` or `Err`, never panics or hangs;
- the event parser and the document loader agree on validity;
- each input that parses is fully covered by tokens, comments and white
  space;
- each one writes back byte for byte, and its rendering reads back as
  equal documents;
- nesting deeper than the limit is an error, not a stack overflow.

**`yaml_editing`** edits each document in many ways, one edit at a time,
and compares the output with the exact expected text: only the edited
entry may change. It also empties each nested block collection (one
without anchors), which must be written `[]` or `{}` in its place by the
strict writer (`Doc::to_stream_string_preserving`). The inputs are the
files in `editing/` (also with CRLF line breaks and without a final line
break) and every valid suite case.

## Behaviour

| Target | Tests | Covers |
|---|---:|---|
| `api_tests` | 161 | constructors, accessors, paths |
| `feature_tests` | 45 | comments, block scalars, flow, tags |
| `integration_tests` | 16 | editing workflows |
| `regression_tests` | 13 | past bugs, property round trip |
| `unit_tests` | 58 | scanner errors, markers, emitter |
| `yaml_spec_1_2_2` | 671 | cases by spec chapter (`spec/`) |

The `spec/` cases are written for this crate, and each one also checks
that its input writes back byte for byte and that its rendering reads
back equal (`common/`). The official suite above is the compliance
reference.
