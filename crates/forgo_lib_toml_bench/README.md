# forgo_lib_toml_bench

Compares forgo_lib_toml with the other Rust TOML crates. It isn't part of
the root workspace: it has its own `[workspace]` and `Cargo.lock`, so the
crates it compares against never enter the main lockfile.

```bash
cd crates/forgo_lib_toml_bench
cargo run --release --bin quick              # parse throughput, MB/s
cargo run --release --bin quick -- --edit    # edit and write, µs
cargo run --release --bin prof -- lockfile 5 # allocations per parse, then a 5 s loop
```

**`quick`** times each crate on each input, after checking that the crate
accepts the input, and reports the best of five runs of 300 ms. The
inputs come from `src/lib.rs`:

- **lockfile**: a Cargo.lock with 4,500 packages (1 MB);
- **mixed**: every kind of TOML value, about 1 MB;
- **manifest**: a typical Cargo.toml (1 KB).

With `--edit`, it times editing and writing instead: setting a version
and adding a dependency in the manifest, and setting one version in the
lock file, with forgo_lib_toml (parsed plainly, and for editing) and
`toml_edit`.

**`prof`** counts the allocations of one parse with forgo_lib_toml and
with toml-spanner, then parses in a loop for the given number of
seconds, for a sampling profiler such as macOS `sample`.

The crates compared, at fixed versions: toml 1.2.3 (`Table` and
`DeTable`), toml_edit 0.25.21, toml-spanner 1.0.3, toml-span 0.7.2,
basic-toml 0.1.10, boml 2.0.1 and taplo 0.14.0.
