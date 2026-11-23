# YAML libraries for Rust: compliance and performance

Generated 2026-11-23 by `crates/forgo_lib_yaml_bench` on Apple M2 Max, macOS 26.6.2; rustc 1.97.1 (8bab26f4f 2026-07-14).

| library | version | parser | events | tree |
|---|---|---|:-:|---|
| forgo_lib_yaml | 0.0.0 | own | ✓ | `Doc` (with comments, styles, spans) |
| serde_yaml | 0.9.34+deprecated | libyaml (C, transpiled) |  | `serde_yaml::Value` |
| serde_yaml_ng | 0.10.0 | libyaml (C, transpiled) |  | `serde_yaml_ng::Value` |
| serde_norway | 0.9.42 | libyaml (C, transpiled) |  | `serde_norway::Value` |
| yaml_serde | 0.10.7 | libyaml (C, transpiled) |  | `yaml_serde::Value` |
| serde-saphyr | 2.2.0 | granit-parser (saphyr fork) |  | none (a generic serde value) |
| noyalib | 0.0.63 | own |  | `noyalib::Value` |
| noyalib (CST) | 0.0.63 | own |  | lossless syntax tree |
| saphyr | 0.4.0 | saphyr-parser (yaml-rust2 fork) | ✓ | `saphyr::Yaml` |
| yaml-rust2 | 0.13.2 | own (yaml-rust fork) | ✓ | `yaml_rust2::Yaml` |
| yaml-rust | 0.4.5 | own | ✓ | `yaml_rust::Yaml` |
| yaml-edit | 0.3.6 | own |  | lossless syntax tree |
| rust-yaml | 1.1.0 | own |  | `rust_yaml::Value` |
| yaml-peg | 1.0.9 | own |  | `yaml_peg::Node` |
| libyaml | 0.2.11 | libyaml (C, transpiled) | ✓ | |
| libyaml-safer | 0.3.0 | libyaml, ported to safe Rust | ✓ | |
| granit-parser | 2.2.0 | granit-parser (saphyr fork) | ✓ | |

## Compliance

Official [yaml-test-suite](https://github.com/yaml/yaml-test-suite) release `data-2022-01-17`: 402 cases, 308 valid and 94 invalid. Every library runs with its default options.

### Event parsers

| library | valid | invalid | events | data events | overall |
|---|--:|--:|--:|--:|--:|
| forgo_lib_yaml | **308/308** | **94/94** | **308/308** | **308/308** | **100.0%** |
| libyaml | 257/308 | 78/94 | 252/308 | 252/308 | 82.1% |
| libyaml-safer | 255/308 | 78/94 | 251/308 | 251/308 | 81.8% |
| granit-parser | **308/308** | 91/94 | 269/308 | **308/308** | 99.3% |
| saphyr-parser | **308/308** | **94/94** | — | **308/308** | **100.0%** |
| yaml-rust2 | **308/308** | **94/94** | — | **308/308** | **100.0%** |
| yaml-rust | 279/308 | 66/94 | — | 258/308 | 80.6% |

- **valid**: valid inputs parsed without error. **invalid**: invalid inputs rejected.
- **events**: the event stream equals `test.event` exactly. **—**: the event API doesn't report anchor names, flow vs. block, or explicit `---`/`...`.
- **data events**: the same nodes, anchors, tags, scalar types and values; styles and document markers are ignored.
- **overall**: share of the 402 cases where every applicable check (valid or invalid, then data events) passes.
- granit-parser runs with `emit_anchor_events` on, and its explicit `...` is read from the event's span.
- Panics, hangs (> 5s) and crashes count as failures: libyaml-safer (2 panics).

### Loaders

| library | valid | invalid | JSON | round trip | overall |
|---|--:|--:|--:|--:|--:|
| forgo_lib_yaml | **308/308** | **94/94** | **279/279** | **308/308** | **100.0%** |
| serde_yaml | 257/308 | 78/94 | 230/279 | 254/308 | 80.3% |
| serde_yaml_ng | 257/308 | 78/94 | 230/279 | 254/308 | 80.3% |
| serde_norway | 257/308 | 78/94 | 230/279 | 254/308 | 80.3% |
| yaml_serde | 257/308 | 78/94 | 230/279 | 254/308 | 80.3% |
| serde-saphyr | 307/308 | 91/94 | 265/279 | 307/308 | 95.8% |
| noyalib | **308/308** | **94/94** | 278/279 | **308/308** | 99.8% |
| noyalib (CST) | **308/308** | **94/94** | 273/279 | **308/308** | 98.5% |
| saphyr | **308/308** | **94/94** | 278/279 | 293/308 | 96.3% |
| yaml-rust2 | **308/308** | **94/94** | **279/279** | 307/308 | 99.8% |
| yaml-rust | 279/308 | 66/94 | 241/279 | 277/308 | 82.8% |
| yaml-edit | **308/308** | **94/94** | 278/279 | **308/308** | 99.8% |
| rust-yaml | **308/308** | 32/94 | 270/279 | **308/308** | 82.3% |
| yaml-peg | 186/308 | 41/94 | 124/279 | 159/308 | 41.8% |

- **JSON**: the loaded documents equal `in.json` (279 valid cases have one; tags are ignored, numbers compare by value).
- **round trip**: load, write with the library's own emitter, load again: the data is unchanged.
- **overall**: share of the 402 cases where every applicable check passes.
- `2JQS` and `X38W` repeat a mapping key, which YAML 1.2.2 §3.2.1.1 makes an error a loader may report: rejecting them counts as a pass.
- serde-saphyr skips null documents in multi-document streams (documented), and reads `y` as `true`; granit-parser and serde-saphyr accept three under-indented inputs unless `strict_indentation` is set.
- noyalib (CST) is noyalib's lossless, format-preserving tree (`noyalib::cst`); it returns a document for streams that hold none (comments only), which fails the JSON check.
- Panics, hangs (> 5s) and crashes count as failures: yaml-peg (12 hangs).

### Other languages

| library | version | valid | invalid | JSON | round trip | overall |
|---|---|--:|--:|--:|--:|--:|
| PyYAML (Python) | 6.0.4 | 227/308 | 82/94 | 210/279 | 227/308 | 74.1% |
| ruamel.yaml (Python) | 0.19.1 | 269/308 | 80/94 | 229/279 | 257/308 | 80.3% |
| yaml (JavaScript) | 2.9.1 | **308/308** | 92/94 | **273/279** | **301/308** | **96.5%** |
| js-yaml (JavaScript) | 5.5.0 | 278/308 | **94/94** | 263/279 | 278/308 | 92.5% |
| go-yaml (Go) | 3.0.6 | 238/308 | 79/94 | 223/279 | 235/308 | 76.4% |

- The same cases and checks as the Rust loaders, through each library's usual API: PyYAML `safe_load_all` and `safe_dump_all`; ruamel.yaml `YAML()` (round-trip mode); yaml `parseAllDocuments` and `stringify`; js-yaml `loadAll` and `dump`; go-yaml (`go.yaml.in/yaml/v3`) `Decoder` and `Encoder` into `any`.
- Installed by `other/setup.sh` into `target/other/`. Values typed beyond the YAML 1.2 core schema (timestamps, binary, sets, ordered maps, YAML 1.1 booleans) count as JSON mismatches.

## Performance

The same inputs for every library, each measurement in a fresh process with the system allocator; release build. Each number is the median of 11 runs after a warm-up (a single run when one takes over 1 s; cut off after 60 s).

| input | size | content |
|---|--:|---|
| k8s | 0.70 MB | 800 Kubernetes Deployments (block maps, flow maps, quoted strings) |
| CI | 0.78 MB | 1,500 CI workflows (anchors, merge keys, block scalars, comments) |
| deep | 0.64 MB | 40 × 120-level nested mappings and flow sequences |
| long | 1.45 MB | long literal, folded, double-quoted and plain scalars |
| flow | 0.84 MB | 8,000 flow mappings with nested flow collections |
| 1 KB | 0.9 KB | one Kubernetes Deployment |

### Parse to events, MB/s (higher is better)

| library | k8s | CI | deep | long | flow |
|---|--:|--:|--:|--:|--:|
| forgo_lib_yaml | **85.5** | **98.6** | **289** | **511** | 43.8 |
| libyaml | 59.3 | 66.4 | 127 | 159 | 35.5 |
| libyaml-safer | 71.3 | 78.7 | 130 | 161 | **47.2** |
| granit-parser | 39.3 | 45.3 | 141 | 469 | 23.0 |
| saphyr-parser | 54.4 | 60.5 | 120 | 393 | 29.7 |
| yaml-rust2 | 61.4 | 67.1 | 200 | 360 | 31.8 |
| yaml-rust | 62.3 | 67.6 | 140 | 235 | 41.1 |

### Load to the library's tree, MB/s (higher is better)

| library | k8s | CI | deep | long | flow |
|---|--:|--:|--:|--:|--:|
| forgo_lib_yaml | **50.8** | **56.9** | **194** | **493** | **27.5** |
| serde_yaml | 31.0 | 34.3 | 92.9 | 152 | 18.4 |
| serde_yaml_ng | 31.4 | 34.5 | 93.9 | 152 | 18.4 |
| serde_norway | 31.6 | 33.4 | 93.1 | 154 | 18.4 |
| yaml_serde | 30.9 | 34.4 | 93.0 | 152 | 17.8 |
| serde-saphyr | 19.7 | 19.8 | 75.8 | 478 | 11.8 |
| noyalib | 41.0 | 43.3 | 157 | 208 | 24.9 |
| noyalib (CST) | 17.4 | 18.6 | 70.4 | 68.4 | 12.0 |
| saphyr | 40.2 | 43.4 | 143 | 304 | 23.9 |
| yaml-rust2 | 39.8 | 43.6 | 144 | 364 | 23.5 |
| yaml-rust | 39.1 | 44.0 | 106 | 228 | 25.0 |
| yaml-edit | 31.7 | 33.5 | 142 | 90.4 | 16.6 |
| rust-yaml | 0.1 | wrong | 1.7 | 213 | 0.1 |
| yaml-peg | 29.7 | 35.7 | 82.1 | 180 | 18.2 |

- rust-yaml, CI: loaded 1 of 1500 documents
- serde-saphyr and noyalib stop at 1,024 and 1,000 documents by default (serde-saphyr also at depth 64); these runs lift those limits.

### Small documents, memory and emitting

| library | 1 KB load, µs | allocations | peak heap, MB | emit, MB/s |
|---|--:|--:|--:|--:|
| forgo_lib_yaml | **17.3** | **73k** | 8.7 | **330** |
| serde_yaml | 30.5 | 366k | 10.2 | 63.3 |
| serde_yaml_ng | 30.5 | 366k | 10.2 | 64.4 |
| serde_norway | 31.3 | 366k | 10.2 | 65.2 |
| yaml_serde | 30.5 | 366k | 10.2 | 64.7 |
| serde-saphyr | 46.2 | 86k | **4.7** | 131 |
| noyalib | 23.8 | 130k | 12.3 | 135 |
| noyalib (CST) | 52.9 | 365k | 18.3 | — |
| saphyr | 23.2 | 123k | 11.6 | 198 |
| yaml-rust2 | 25.7 | 314k | 8.7 | 167 |
| yaml-rust | 24.2 | 195k | 10.1 | 211 |
| yaml-edit | 38.1 | 271k | 17.5 | — |
| rust-yaml | 53.9 | 656k | 41.1 | 24.9 |
| yaml-peg | 28.8 | 328k | 18.7 | > 60 s |

- **1 KB load**: microseconds to load the 1 KB document (lower is better).
- **allocations** and **peak heap**: one load of the 0.70 MB k8s input, counted by a wrapping global allocator (lower is better).
- **emit**: rendering the loaded k8s tree as YAML with the library's own emitter, in MB of output per second (higher is better). forgo_lib_yaml's documents are reformatted first so that it renders every node; **—**: a lossless tree, which can only write its source back.
- **Write back** an untouched loaded tree as its source text (not a render, so not in the emit column): forgo_lib_yaml 3190 MB/s, noyalib (CST) 950 MB/s, yaml-edit 29.6 MB/s.
- Trees differ: forgo_lib_yaml's `Doc` also keeps comments, styles and source spans; serde-saphyr has no tree type and loads into a minimal generic serde value.

