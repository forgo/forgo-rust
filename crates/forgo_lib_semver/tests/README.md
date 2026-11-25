# Tests

```bash
scripts/fetch-node-semver.sh          # once; needs Node.js
cargo test -p forgo_lib_semver
```

| Target | Inputs | Checks |
|---|---|---|
| `spec` | semver.org's lists, the specification's examples, 80,000 generated versions | parsing against the BNF, printing back, precedence, `==`/`Ord`/`Hash` |
| `npm` | node-semver 7.8.6 on its own test suite (fetched) | 5,723 calls and 442,040 cross-product results, identical |
| `cargo` | the `semver` crate's requirement tests | matching, canonical text, errors |
| `sets` | 12,000 random pairs of requirements, a universe of 1,088 versions | set operations against direct matching |
| unit (`src/npm/regex.rs`) | | the regular expression engine against JavaScript |
| doctests | README and API docs | 32 examples |

**`npm`** needs `tests/node-semver-data/`, made by
`scripts/fetch-node-semver.sh` (not committed). Without it the test is
skipped, unless `SEMVER_NODE_REQUIRED=1` (as in CI), when it fails.

**`sets`** proves each "yes" with a witness version that must match
directly, and checks each "no" against the universe; see
[COMPLIANCE.md](../COMPLIANCE.md).

The scripts:

| Script | Does |
|---|---|
| `fetch-node-semver.sh` | downloads node-semver at its commit and records its behaviour |
| `capture-node-semver.js` | runs node-semver's tests with a stand-in for `tap`, recording each call and its result |
| `gen-npm-patterns.js` | regenerates `src/npm/patterns.rs` from node-semver's source |
