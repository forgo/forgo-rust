//! Templates for new crate manifests and README scaffolding, plus helpers.

use std::fs;
use std::path::Path;

use crate::util::EDITION;

pub fn write_manifest_from_template(crate_path: &Path, kind: &str, name: &str, make_public: bool) {
    let publish_val = if make_public { "true" } else { "false" };
    let description = default_description(kind, name);

    let body = match kind {
        "lib" => TEMPLATE_LIB
            .replace("{NAME}", name)
            .replace("{EDITION}", EDITION)
            .replace("{PUBLISH}", publish_val)
            .replace("{DESCRIPTION}", &description),
        "bin" => TEMPLATE_BIN
            .replace("{NAME}", name)
            .replace("{EDITION}", EDITION)
            .replace("{PUBLISH}", publish_val)
            .replace("{DESCRIPTION}", &description),
        "wasm" => TEMPLATE_WASM
            .replace("{NAME}", name)
            .replace("{EDITION}", EDITION)
            .replace("{PUBLISH}", publish_val)
            .replace("{DESCRIPTION}", &description),
        _ => unreachable!(),
    };

    fs::write(crate_path.join("Cargo.toml"), body).expect("write templated Cargo.toml");
}

pub fn default_description(kind: &str, name: &str) -> String {
    match kind {
        "lib" => format!("Library crate: {}", name),
        "bin" => format!("Binary crate: {}", name),
        "wasm" => format!("WASM library crate: {}", name),
        _ => name.to_string(),
    }
}

pub fn scaffold_readme(crate_path: &Path, kind: &str, name: &str) {
    let readme = crate_path.join("README.md");
    if readme.exists() {
        return;
    }
    let (tag, example_cmd) = match kind {
        "lib" => (
            "A library crate in the Forgo monorepo.",
            format!("cargo build -p {name}"),
        ),
        "bin" => (
            "A binary crate in the Forgo monorepo.",
            format!("cargo run -p {name} -- --help"),
        ),
        "wasm" => (
            "A WebAssembly library crate (cdylib + wasm-bindgen) in the Forgo monorepo.",
            format!("cargo build -p {name} --features wasm"),
        ),
        _ => (
            "A crate in the Forgo monorepo.",
            format!("cargo build -p {name}"),
        ),
    };

    let body = TEMPLATE_README
        .replace("{NAME}", name)
        .replace("{TAGLINE}", tag)
        .replace("{EXAMPLE_CMD}", &example_cmd);

    let _ = fs::write(readme, body);
}

/* =============================== Templates =============================== */

const TEMPLATE_LIB: &str = r#"[package]
name = "{NAME}"
version = "0.0.0"
edition = "{EDITION}"
publish = {PUBLISH}
license = "MIT"
description = "{DESCRIPTION}"
repository = "https://github.com/forgo/forgo-rust"
readme = "README.md"

[features]
default = []
# serde = ["dep:serde"]

[dependencies]
# serde = { version = "1", features = ["derive"], optional = true }
"#;

const TEMPLATE_BIN: &str = r#"[package]
name = "{NAME}"
version = "0.0.0"
edition = "{EDITION}"
publish = {PUBLISH}
license = "MIT"
description = "{DESCRIPTION}"
repository = "https://github.com/forgo/forgo-rust"
readme = "README.md"

[dependencies]
# forgo_lib_something = { path = "../forgo_lib_something" }

[[bin]]
name = "{NAME}"
path = "src/main.rs"
"#;

const TEMPLATE_WASM: &str = r#"[package]
name = "{NAME}"
version = "0.0.0"
edition = "{EDITION}"
publish = {PUBLISH}
license = "MIT"
description = "{DESCRIPTION}"
repository = "https://github.com/forgo/forgo-rust"
readme = "README.md"

[lib]
crate-type = ["cdylib", "rlib"]

[features]
default = ["wasm"]
wasm = ["dep:wasm-bindgen"]

[dependencies]
wasm-bindgen = { version = "0.2", optional = true }
"#;

const TEMPLATE_README: &str = r#"# {NAME}

{TAGLINE}

## Quick start

```bash
{EXAMPLE_CMD}
"#;
