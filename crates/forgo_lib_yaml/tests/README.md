# forgo_lib_yaml tests

```sh
scripts/fetch-yaml-test-suite.sh   # official suite data (pinned), once
cargo test -p forgo_lib_yaml
```

## Compliance

| target | what it checks |
|--------|----------------|
| `yaml_test_suite` | The official [yaml-test-suite](https://github.com/yaml/yaml-test-suite) data release `data-2022-01-17` (402 cases). For each case it checks the error expectation, the exact event stream (`test.event`), the JSON value (`in.json`), byte coverage, the normalized round trip, and presentation (the re-parsed `Doc` is equal). All six checks are gated by `yaml_test_suite/known_failures.txt` (empty). The report goes to `target/tmp/yaml-test-suite-report.md`. |
| `yaml_spec_gaps` | YAML 1.2.2 rules the suite doesn't test: encodings and BOMs, CR/CRLF, NEL/LS/PS, spec examples missing from the suite, version directives, implicit key length, plus the decisions listed in [COMPLIANCE.md](../COMPLIANCE.md). Expected events are hand-derived from the spec. |
| `yaml_robustness` | ~970k truncated and mutated suite inputs. Checks: no panics or hangs, the parser and composer agree, full coverage, content-preserving re-emission (presentation as a ratchet), and the nesting-depth limit. |

Without the suite data these tests skip loudly, or fail when `CI` or
`YAML_TEST_SUITE_REQUIRED` is set. The `yaml_test_suite/` directory holds the
harness's helpers: a JSON reader, `Doc` → JSON conversion, and the
style-independent event normalizer.

## Behaviour

| target | tests | scope |
|--------|------:|-------|
| `api_tests` | 161 | constructors, accessors, builders, paths |
| `feature_tests` | 45 | anchors, block scalars, comments, document streams, flow style, quoting, schemas, tags |
| `integration_tests` | 16 | editing workflows and AST round trips |
| `regression_tests` | 13 | reproductions and a property-based round trip |
| `unit_tests` | 58 | scanner errors, document markers, emitter details |
| `yaml_spec_1_2_2` | 671 | self-authored tests organized by spec chapter (`spec/ch_*.rs`); `Fixture`s check that parse → emit → parse is stable and gives an equal document |

`common/` holds the `Fixture` helper. The `spec/` tests are written for this
crate; the official suite above is the compliance reference.
