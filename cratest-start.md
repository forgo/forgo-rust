Got it — thanks for calling that out. Easy fix.

We’ll just rename every **library** crate we added to follow your existing pattern:

* ✅ library → `forgo_lib_*`
* ✅ binary / tool → `forgo_*`
* keep everything under `crates/`

Here’s the revised lineup.

---

## 1. Final crate names

**Libraries (new):**

* `crates/forgo_lib_data_core` – core data types, schema, record batches
* `crates/forgo_lib_data_expr` – expression AST
* `crates/forgo_lib_data_plan` – logical plan
* `crates/forgo_lib_data_store` – segment format + metadata
* `crates/forgo_lib_data_exec` – execution context + operators
* `crates/forgo_lib_data_wasm` – wasm-bindgen surface (feature-gated)

**Binaries / tools (new):**

* `crates/forgo_data_cli` – tiny CLI to test the engine

**Existing:**

* `crates/forgo` (bin)
* `crates/forgo_lib_cli` (lib)

So your `crates/` dir will look like:

```text
crates/
  forgo/                   # bin (existing)
  forgo_lib_cli/           # lib (existing)

  forgo_lib_data_core/     # lib
  forgo_lib_data_expr/     # lib
  forgo_lib_data_plan/     # lib
  forgo_lib_data_store/    # lib
  forgo_lib_data_exec/     # lib
  forgo_lib_data_wasm/     # lib (wasm)

  forgo_data_cli/          # bin
```

---

## 2. Workspace (root `Cargo.toml`)

Update your root to:

```toml
[workspace]
resolver = "3"
members = [
    "crates/forgo_lib_cli",
    "crates/forgo",

    # data stack (all libraries start with forgo_lib_)
    "crates/forgo_lib_data_core",
    "crates/forgo_lib_data_expr",
    "crates/forgo_lib_data_plan",
    "crates/forgo_lib_data_store",
    "crates/forgo_lib_data_exec",
    "crates/forgo_lib_data_wasm",

    # tooling for it
    "crates/forgo_data_cli",
]
```

That keeps your naming story completely consistent.

---

## 3. Per-crate Cargo.toml adjustments

I’ll just show the name changes + deps; structure is same as before.

### 3.1 `crates/forgo_lib_data_core/Cargo.toml`

```toml
[package]
name = "forgo_lib_data_core"
version = "0.0.0"
edition = "2024"
publish = true
description = "Core data structures (schema, column buffers, record batches) for the Forgo data stack."
license = "MIT"

[features]
default = ["std"]
std = []
serde = ["dep:serde"]

[dependencies]
serde = { version = "1", features = ["derive"], optional = true }
```

### 3.2 `crates/forgo_lib_data_expr/Cargo.toml`

```toml
[package]
name = "forgo_lib_data_expr"
version = "0.0.0"
edition = "2024"
publish = true
description = "Expression AST for the Forgo data stack."
license = "MIT"

[features]
default = ["serde"]
serde = ["dep:serde"]

[dependencies]
forgo_lib_data_core = { path = "../forgo_lib_data_core" }
serde = { version = "1", features = ["derive"], optional = true }
serde_json = { version = "1", optional = true }
```

### 3.3 `crates/forgo_lib_data_plan/Cargo.toml`

```toml
[package]
name = "forgo_lib_data_plan"
version = "0.0.0"
edition = "2024"
publish = true
description = "Logical query plan representation for the Forgo data stack."
license = "MIT"

[features]
default = ["serde"]
serde = ["dep:serde"]

[dependencies]
forgo_lib_data_expr = { path = "../forgo_lib_data_expr" }
serde = { version = "1", features = ["derive"], optional = true }
```

### 3.4 `crates/forgo_lib_data_store/Cargo.toml`

```toml
[package]
name = "forgo_lib_data_store"
version = "0.0.0"
edition = "2024"
publish = true
description = "Segmented columnar storage for the Forgo data stack."
license = "MIT"

[features]
default = []
serde = ["dep:serde", "forgo_lib_data_core/serde"]

[dependencies]
forgo_lib_data_core = { path = "../forgo_lib_data_core" }
serde = { version = "1", features = ["derive"], optional = true }
```

### 3.5 `crates/forgo_lib_data_exec/Cargo.toml`

```toml
[package]
name = "forgo_lib_data_exec"
version = "0.0.0"
edition = "2024"
publish = true
description = "Execution engine for the Forgo data stack."
license = "MIT"

[dependencies]
forgo_lib_data_core = { path = "../forgo_lib_data_core" }
forgo_lib_data_plan = { path = "../forgo_lib_data_plan" }
forgo_lib_data_expr = { path = "../forgo_lib_data_expr" }
forgo_lib_data_store = { path = "../forgo_lib_data_store" }
```

### 3.6 `crates/forgo_lib_data_wasm/Cargo.toml`

```toml
[package]
name = "forgo_lib_data_wasm"
version = "0.0.0"
edition = "2024"
publish = true
description = "WASM bindings for the Forgo data stack."
license = "MIT"

[lib]
crate-type = ["cdylib", "rlib"]

[features]
default = ["wasm"]
wasm = ["dep:wasm-bindgen"]

[dependencies]
wasm-bindgen = { version = "0.2", optional = true }
serde_json = "1"
forgo_lib_data_plan = { path = "../forgo_lib_data_plan" }
forgo_lib_data_exec = { path = "../forgo_lib_data_exec" }
forgo_lib_data_store = { path = "../forgo_lib_data_store" }
```

### 3.7 `crates/forgo_data_cli/Cargo.toml`

```toml
[package]
name = "forgo_data_cli"
version = "0.0.0"
edition = "2024"
publish = false
description = "CLI utilities for inspecting/working with Forgo data segments."
license = "MIT"

[dependencies]
forgo_lib_data_plan = { path = "../forgo_lib_data_plan" }
forgo_lib_data_exec = { path = "../forgo_lib_data_exec" }
forgo_lib_data_store = { path = "../forgo_lib_data_store" }
serde_json = "1"
anyhow = "1"
```

---

## 4. Sample wasm export (renamed)

In `crates/forgo_lib_data_wasm/src/lib.rs`:

```rust
#![cfg(feature = "wasm")]

use wasm_bindgen::prelude::*;
use std::cell::RefCell;
use std::collections::HashMap;

use forgo_lib_data_plan::LogicalPlan;
use forgo_lib_data_exec::ExecutionContext;
use forgo_lib_data_store::Segment;

thread_local! {
    static SEGMENTS: RefCell<HashMap<String, Vec<Segment>>> =
        RefCell::new(HashMap::new());
}

#[wasm_bindgen]
pub fn load_segment(source: String, bytes: &[u8]) -> Result<(), JsValue> {
    let seg = Segment::from_bytes(bytes)
        .map_err(|e| JsValue::from_str(&e))?;

    SEGMENTS.with(|map| {
        let mut map = map.borrow_mut();
        map.entry(source).or_default().push(seg);
    });

    Ok(())
}

#[wasm_bindgen]
pub fn run_query(plan_json: &str) -> Result<JsValue, JsValue> {
    let plan: LogicalPlan =
        serde_json::from_str(plan_json).map_err(|e| JsValue::from_str(&e.to_string()))?;

    let ctx = SEGMENTS.with(|map| {
        let map = map.borrow();
        let mut ctx = ExecutionContext::new();
        for (name, segs) in map.iter() {
            ctx.register_source(name.clone(), segs.clone());
        }
        ctx
    });

    let batches = ctx.execute(&plan);
    let summary = serde_json::json!({
        "batches": batches.len(),
        "rows": batches.iter().map(|b| b.len).sum::<usize>(),
    });

    JsValue::from_serde(&summary).map_err(|e| JsValue::from_str(&e.to_string()))
}
```

Everything lines up with your naming rule.

---

## 5. CI note

In `.github/workflows/ci.yaml`, when you expand the matrix, just use the final crate names:

```yaml
strategy:
  matrix:
    project: [
      "forgo_lib_cli",
      "forgo",
      "forgo_lib_data_core",
      "forgo_lib_data_expr",
      "forgo_lib_data_plan",
      "forgo_lib_data_store",
      "forgo_lib_data_exec",
      "forgo_data_cli"
      # (optional) "forgo_lib_data_wasm" in a job that has wasm-bindgen/wasm-pack
    ]
```

---

That should keep the repo visually obvious:

* “is this a lib?” → it starts with `forgo_lib_`
* “is this runnable?” → it doesn’t 😎

If you want to go one step further, we can also rename modules inside each crate to match (`forgo_lib_data_core::schema`, etc.), but that’s cosmetic.
