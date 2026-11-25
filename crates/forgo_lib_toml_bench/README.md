# forgo_lib_toml_bench

Compares forgo_lib_toml with the other Rust TOML crates: the report in
[RESULTS.md](RESULTS.md). It isn't part of the root workspace: it has its
own `[workspace]` and `Cargo.lock`, so the crates it compares against never
enter the main lockfile.

```bash
cd crates/forgo_lib_toml_bench
../forgo_lib_toml/scripts/fetch-toml-test.sh  # toml-test, once
scripts/fetch-inputs.sh                       # real-world files, once
scripts/official-toml-test.sh                 # toml-test's own runner (needs Go)
cargo run --release --bin report > RESULTS.md # the report (--quick: fewer rounds)
cargo test --release                          # agreement with toml, toml_edit, toml-spanner
```

## The report

`src/bin/report.rs` writes RESULTS.md:

- **Compliance**: every toml-test case, for TOML 1.1 and 1.0, through each
  crate. Then toml-test's own runner on forgo_lib_toml, when
  `scripts/official-toml-test.sh` has built it.
- **Agreement**: about 51,000 documents made from the valid toml-test cases
  (truncated, or with a character inserted), read by forgo_lib_toml,
  `toml`, `toml_edit` and toml-spanner. Differences are classified; any
  unexplained one is flagged.
- **Speed**: parse, then walk every value, on real files and generated
  ones. Also the latency of a 1 KB manifest.
- **Memory**: allocations and peak heap.
- **Writing TOML**: from each crate's value tree.
- **Editing**: parse, edit, write, keeping the rest of the text. The
  crates' outputs are checked against each other.

## Inputs

`src/lib.rs` generates a Cargo.lock (4,500 packages), a document with every
kind of value (1 MB and 5 MiB) and a 1 KB manifest. `scripts/fetch-inputs.sh`
fetches zed's Cargo.toml and Cargo.lock and ruff's pyproject.toml at pinned
commits, checked against `scripts/inputs.sha256`, into `inputs/` (not
committed).

## toml-test's own runner

`scripts/official-toml-test.sh` downloads toml-test at the pinned commit,
builds its runner with Go (the runner embeds that commit's cases), and runs
it on forgo_lib_toml:

- `src/bin/toml-test-decoder.rs`: TOML on stdin, tagged JSON on stdout,
  exit code 1 for invalid input; `--toml-1.0` for TOML 1.0.
- `src/bin/toml-test-encoder.rs`: tagged JSON on stdin, TOML on stdout.
  The runner reads it back with BurntSushi/toml, the Go reference
  implementation.

## Tools

```bash
cargo run --release --bin quick              # parse throughput, MB/s
cargo run --release --bin quick -- --edit    # edit and write, µs
cargo run --release --bin prof -- lockfile 5 # allocations per parse, then a 5 s loop
```

**`quick`** times each crate on each generated input, after checking that
the crate accepts it, and reports the best of five runs of 300 ms. With
`--edit`, it times editing and writing instead.

**`prof`** counts the allocations of one parse with forgo_lib_toml and
with toml-spanner, then loops for the given number of seconds, for a
sampling profiler such as macOS `sample` (`prof edit` and `prof walk` loop
over an edit, and over parsing zed's Cargo.lock and reading every value).

The crates compared, at fixed versions: toml 1.2.3 (`Table` and
`DeTable`), toml_edit 0.25.21, toml-spanner 1.0.3 (with its `to-toml`
feature, for writing and editing), toml-span 0.7.2, basic-toml 0.1.10,
boml 2.0.1 and taplo 0.14.0.
