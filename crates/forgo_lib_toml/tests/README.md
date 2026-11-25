# Tests

```bash
scripts/fetch-toml-test.sh          # once
cargo test -p forgo_lib_toml
```

| Target | Inputs | Checks |
|---|---|---|
| `toml_test` | toml-test at a pinned commit (fetched): its 1.1.0 and 1.0.0 lists | valid documents decode to the expected values; invalid ones are rejected; expected values encode and read back the same |
| `toml_robustness` | 1.8 million truncations and one-character edits of every toml-test document | no panic or hang; everything that parses round-trips through the encoder; deep nesting is an error |
| `api` | | error kinds and positions, large tables, key order, version differences, date-times, the encoder's output |
| `editing` | manifests, a Cargo.lock, this workspace's Cargo.toml; each also with CRLF | the exact text after each edit; failed edits change nothing |
| `edit_robustness` | every path of every valid toml-test document | 8,160 edits: each made one checked independently, the rest refused cleanly; 1,659 more in chains on one document |
| unit (`src/parse.rs`) | | the eight-bytes-at-a-time scanner against a byte loop |
| doctests | README and API docs | the examples |

**`toml_test`**, **`toml_robustness`** and **`edit_robustness`** need
`tests/toml-test-data/`,
made by `scripts/fetch-toml-test.sh` (not committed). Without it they are
skipped, unless `TOML_TEST_SUITE_REQUIRED=1` (as in CI), when they fail.
The script downloads the suite's archive at the pinned commit, checks the
number of cases for each version, and keeps only its `tests/` directory
and its license. The suite is MIT-licensed.

`toml_test` reads the expected JSON with forgo_lib_yaml (JSON is YAML
1.2), a dev-dependency.
