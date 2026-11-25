//! The files `cargo x new` writes for a crate.

use crate::cli::Kind;

/// The Rust edition of new crates.
pub const EDITION: &str = "2024";

/// `Cargo.toml` for a new crate.
pub fn manifest(kind: Kind, name: &str, public: bool) -> String {
    let template = match kind {
        Kind::Lib => LIB_MANIFEST,
        Kind::Bin => BIN_MANIFEST,
        Kind::Wasm => WASM_MANIFEST,
    };
    template
        .replace("{NAME}", name)
        .replace("{EDITION}", EDITION)
        .replace("{PUBLISH}", if public { "true" } else { "false" })
        .replace("{DESCRIPTION}", &description(kind, name))
}

fn description(kind: Kind, name: &str) -> String {
    match kind {
        Kind::Lib => format!("Library crate: {name}"),
        Kind::Bin => format!("Binary crate: {name}"),
        Kind::Wasm => format!("WASM library crate: {name}"),
    }
}

/// `README.md` for a new crate.
pub fn readme(kind: Kind, name: &str) -> String {
    let (tagline, example) = match kind {
        Kind::Lib => (
            "A library crate in the Forgo monorepo.",
            format!("cargo build -p {name}"),
        ),
        Kind::Bin => (
            "A binary crate in the Forgo monorepo.",
            format!("cargo run -p {name} -- --help"),
        ),
        Kind::Wasm => (
            "A WebAssembly library crate (cdylib + wasm-bindgen) in the Forgo monorepo.",
            format!("cargo build -p {name} --features wasm"),
        ),
    };
    format!("# {name}\n\n{tagline}\n\n## Quick start\n\n```bash\n{example}\n```\n")
}

/// The source file of a new crate, as (path within the crate, contents).
pub fn source(kind: Kind) -> (&'static str, &'static str) {
    match kind {
        Kind::Lib => ("src/lib.rs", LIB_RS),
        Kind::Bin => ("src/main.rs", MAIN_RS),
        Kind::Wasm => ("src/lib.rs", WASM_RS),
    }
}

const LIB_MANIFEST: &str = r#"[package]
name = "{NAME}"
version = "0.0.0"
edition = "{EDITION}"
publish = {PUBLISH}
license = "MIT"
description = "{DESCRIPTION}"
repository = "https://github.com/forgo/forgo-rust"
readme = "README.md"

[dependencies]
"#;

const BIN_MANIFEST: &str = r#"[package]
name = "{NAME}"
version = "0.0.0"
edition = "{EDITION}"
publish = {PUBLISH}
license = "MIT"
description = "{DESCRIPTION}"
repository = "https://github.com/forgo/forgo-rust"
readme = "README.md"

[dependencies]

[[bin]]
name = "{NAME}"
path = "src/main.rs"
"#;

const WASM_MANIFEST: &str = r#"[package]
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

const LIB_RS: &str = r#"pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
"#;

const MAIN_RS: &str = r#"fn main() {
    println!("Hello, world!");
}
"#;

const WASM_RS: &str = r#"#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

/// Adds two numbers; exported to JavaScript with the `wasm` feature.
#[cfg_attr(feature = "wasm", wasm_bindgen)]
pub fn add(left: u32, right: u32) -> u32 {
    left + right
}
"#;
