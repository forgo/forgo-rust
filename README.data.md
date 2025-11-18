# Forgo Data Stack

A small, composable data system designed for **fast analytics**, **strong typing**, and **portability** (native + WebAssembly).
It's built from crates that each do one job well:

- **`forgo_lib_data_core`** - columnar data types (`Schema`, `Field`, `RecordBatch`)
- **`forgo_lib_data_expr`** - expression trees (`Expr`) for filters, projections, functions
- **`forgo_lib_data_plan`** - declarative logical plans (`LogicalPlan`) describing _what_ to compute
- **`forgo_lib_data_store`** - immutable columnar **segments** with schema + stats
- **`forgo_lib_data_exec`** - vectorized executor that runs plans over segments/batches
- **`forgo_lib_data_wasm`** - WASM bindings to run the same plans in the browser
- **`forgo_data_cli`** - a tiny CLI to load segments and execute plans for quick iteration

> Goal: a straightforward API that's easy to reason about, easy to test, and easy to port.

---

## Contents

- [Mental Model](#mental-model)
- [Crates and How They Fit](#crates-and-how-they-fit)
- [End-to-End Example (Rust)](#end-to-end-example-rust)
- [End-to-End Example (Browser via WASM)](#end-to-end-example-browser-via-wasm)
- [Design FAQ](#design-faq)
- [Roadmap Snapshot](#roadmap-snapshot)

---

## Mental Model

Think of the stack like a pipeline:

```
[Your Data]  ->  [Segments]  ->  [Scan] -> [Filter] -> [Project] -> [Aggregate] -> [Results]
   (CSV,            (immutable     (store-aware,        (Expr)        (Expr)         (batches
   logs, etc.)       columnar)      prunes work)                                   or tables)
```

- Data lives in **segments** (self-contained binary blobs).
- A **plan** says _what_ to do (not _how_).
- The **executor** runs the plan over batches, vectorized.
- The same plan can run natively or in the browser.

---

## Crates and How They Fit

```
forgo_lib_data_core   ─┐      types, schema, columns, batches
forgo_lib_data_expr    │      expression trees
forgo_lib_data_plan    │      logical plans
                       ├──► forgo_lib_data_exec  ───► results (batches)
forgo_lib_data_store   │        ▲
                       │        │ reads/writes segments
forgo_lib_data_wasm    └────────┘ browser surface (load segments, run plans)

forgo_data_cli  → examples + quick local iteration
```

- **Core/Expr/Plan** define the _language_ of data and queries.
- **Store** defines the _physical layout_ and I/O (segments).
- **Exec** turns plans into results (batch-by-batch).
- **WASM** exposes a tiny JS-facing API for browser use.
- **CLI** helps you iterate without bringing up a UI.

---

## End-to-End Example (Rust)

> Demonstrates: build data → persist segment → define plan → execute → consume results.

```rust
use std::time::Instant;

use forgo_lib_data_core::{Schema, Field, DataType, RecordBatch};
use forgo_lib_data_expr::{Expr, BinaryOp};
use forgo_lib_data_plan::{LogicalPlan, ScanNode, AggregateNode};
use forgo_lib_data_store::Segment;
use forgo_lib_data_exec::ExecutionContext;

// 1) Define a schema and build a small batch.
let schema = Schema::new(vec![
    Field::new("day", DataType::Utf8, false),
    Field::new("country", DataType::Utf8, false),
    Field::new("latency_ms", DataType::Int32, true),
]);

let batch = RecordBatch::builder(schema.clone())
    .with_utf8("day", ["2025-10-30", "2025-10-30", "2025-10-31"])
    .with_utf8("country", ["US", "CA", "US"])
    .with_i32_opt("latency_ms", [Some(120), None, Some(80)])
    .finish();

// 2) Persist as an immutable segment (in memory here; to_bytes() if you want a file).
let seg = Segment::from_batches("events", vec![batch]);

// 3) Describe a query: US-only, group by day, average latency.
let plan = LogicalPlan::Aggregate {
    input: Box::new(LogicalPlan::Filter {
        input: Box::new(LogicalPlan::Scan(ScanNode {
            source: "events".into(),
            projection: Some(vec!["day".into(), "country".into(), "latency_ms".into()]),
            predicate: None,
        })),
        predicate: Expr::Binary {
            left: Box::new(Expr::Column("country".into())),
            op: BinaryOp::Eq,
            right: Box::new(Expr::Literal("US".into())),
        },
    }),
    aggr: AggregateNode {
        group_by: vec![Expr::Column("day".into())],
        aggr: vec![Expr::Func {
            name: "avg".into(),
            args: vec![Expr::Column("latency_ms".into())],
        }],
    },
};

// 4) Execute.
let mut ctx = ExecutionContext::new();
ctx.register_source("events", vec![seg]);

let t0 = Instant::now();
let output_batches = ctx.execute(&plan);
let elapsed = t0.elapsed();

// 5) Inspect the result (for now, just row counts; you'll shape real outputs as needed).
let rows: usize = output_batches.iter().map(|b| b.len).sum();
println!("Batches: {}, Rows: {}, Took: {:?}", output_batches.len(), rows, elapsed);
```

> In practice you'll convert `RecordBatch`es to your UI needs (typed vectors, tables, JSON, etc.).

---

## End-to-End Example (Browser via WASM)

> Demonstrates: load a segment → submit a plan (as JSON) → get a quick JSON summary.

**Rust (already provided by the crate):**

```rust
// forgo_lib_data_wasm exports two functions:
//
// #[wasm_bindgen]
// pub fn load_segment(source: String, bytes: &[u8]) -> Result<(), JsValue>;
//
// #[wasm_bindgen]
// pub fn run_query(plan_json: &str) -> Result<JsValue, JsValue>;
```

**TypeScript (simplest possible use):**

```ts
import init, { load_segment, run_query } from "./pkg/forgo_lib_data_wasm.js";

// 1) Initialize WASM
await init();

// 2) Load data (e.g., fetched from your CDN or OPFS/IndexedDB)
const bytes = await (await fetch("/data/events.seg")).arrayBuffer();
await load_segment("events", new Uint8Array(bytes));

// 3) Describe the plan using the same logical shape as native
const plan = {
  Aggregate: {
    input: {
      Filter: {
        input: {
          Scan: {
            source: "events",
            projection: ["day", "country", "latency_ms"],
            predicate: null,
          },
        },
        predicate: {
          Binary: {
            left: { Column: "country" },
            op: "Eq",
            right: { Literal: "US" },
          },
        },
      },
    },
    aggr: {
      group_by: [{ Column: "day" }],
      aggr: [{ Func: { name: "avg", args: [{ Column: "latency_ms" }] } }],
    },
  },
};

// 4) Execute in the browser (ideally inside a Web Worker)
const summary = await run_query(JSON.stringify(plan));
console.log(summary); // e.g., { batches: 1, rows: 1 }
```

> You can later extend the WASM boundary to return typed arrays for charting with minimal copies.

---

## Design FAQ

### Why **columnar**?

Most analytics workloads touch a few columns across many rows (e.g., "filter by country, average latency"). Storing each column contiguously:

- speeds up scans and filters,
- minimizes cache misses,
- makes vectorized operations trivial,
- simplifies moving data between threads or across the JS/WASM boundary.

### What's a **segment**?

A self-contained, immutable blob with:

- the **schema**,
- one or more **record batches** (columnar data),
- **column statistics** (min, max, null_count) for fast pruning.

Segments let you:

- skip irrelevant data quickly,
- ship data over the wire as a single binary,
- cache slices of data in the browser or on disk.

### Why **immutable**?

Immutability makes systems **simpler** and **safer**:

- easy caching and deduplication,
- reproducible results (content-addressable if you want),
- lock-free read paths,
- append strategies are straightforward (new segment = new version).

### Where does **performance** come from?

- **Pruning**: stats let the scanner skip entire segments that can't match.
- **Projection**: only read columns you need.
- **Vectorized ops**: operators work on batches, not per-row overhead.
- **Tight layouts**: fixed-width columns are contiguous; var-width use offsets.

### How does this scale?

- Many small segments (partitioned by time or entity) → selective reads.
- A plan can stream batches; you never need all rows in memory.
- The same plan can run:

  - **natively** for heavy lifting,
  - **in the browser** for last-mile transforms and interactivity.

### How do I get **typed results** out?

There are a few options:

- Keep results as `RecordBatch`es and convert to typed vectors at the edge of your app.
- Add a small "materialize" layer tailored to your chart components.
- In WASM, return typed array views (zero/minimal-copy) for hot paths.

### What about **threading** and **SIMD**?

The base design is single-threaded and vectorized. You can layer in:

- threads (native) or Web Workers + SharedArrayBuffer (WASM),
- SIMD-backed kernels for hot operators,
  without changing public APIs.

---

## Roadmap Snapshot

Focused, incremental work that preserves API stability:

1. **Store**

   - solidify binary header/footer (magic, version, checksum)
   - column stats for basic types; pluggable encodings (e.g., RLE/delta later)

2. **Exec**

   - complete `Filter` + `Projection`
   - single-batch `Aggregate`, then multi-batch merge
   - `Sort` + `Limit` (bounded heaps)

3. **Interop**

   - typed export helpers (Rust → JSON/rows; WASM → typed arrays)
   - simple JSON plan builder helpers for TS/Rust

4. **Tooling**

   - `forgo_data_cli` commands: `scan`, `schema`, `stats`, `run --plan`
   - sample datasets + golden tests

5. **Perf**

   - batch size tuning
   - micro-benchmarks for kernels

---

## Where to Start

- Implement and test `Segment::{to_bytes, from_bytes}` with a tiny spec.
- Fill in `Filter` operator end-to-end (predicate → mask → compact columns).
- Add a `cli run --plan` path that prints small tables.
- Wire a minimal web demo with `forgo_lib_data_wasm`.

Once those are in place, you'll have a usable, portable "local-first analytics" backbone you fully own.
