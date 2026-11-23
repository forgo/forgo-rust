# YAML 1.2.2 compliance

## Method

Compliance is measured against the official
[yaml-test-suite](https://github.com/yaml/yaml-test-suite) data release
`data-2022-01-17` (commit `6e6c296a`), which has 402 cases: 308 valid and 94
that must be rejected. `tests/yaml_test_suite.rs` checks every case for:

| check  | passes when |
|--------|-------------|
| error  | the input is rejected exactly when the case has an `error` file |
| events | the event stream equals `test.event` (valid cases) |
| json   | the composed documents equal `in.json` (279 valid cases have one) |

Gated checks fail the build on any failure not listed in
`tests/yaml_test_suite/known_failures.txt`, and on any listed failure that
now passes. The full report is written to
`target/tmp/yaml-test-suite-report.md`.

```sh
scripts/fetch-yaml-test-suite.sh
cargo test -p forgo_lib_yaml --test yaml_test_suite -- --nocapture
```

## Baseline: legacy parser (before the rebuild)

Measured on 2026-11-22 with `Doc::from_stream`, report-only:

| check  | result |
|--------|--------|
| error  | 294/402 (73.1%): 82 valid inputs rejected, 26 invalid inputs accepted |
| events | not available (the legacy parser has no event output) |
| json   | 109/279 (39.1%): the parsed data is wrong or missing for 170 valid cases |

The previously reported "306–316/363" came from a harness that only checked
accept/reject, mis-decoded the suite's visual markers, and found 363 of the
402 cases.
