//! Comparison of forgo_lib_yaml with the other Rust YAML libraries:
//! compliance with the official yaml-test-suite, and performance on the
//! same inputs in the same process.
//!
//! `cargo run --release --bin report` prints the whole comparison as
//! GitHub-flavored markdown; see README.md.

pub mod data;
pub mod inputs;
pub mod libs;
pub mod suite;

pub use inputs::{Input, small_document, suite_valid, synthetic_inputs};
pub use libs::{EVENT_LIBS, EventLib, LOAD_LIBS, LoadLib};

/// Exact versions of the compared crates, read from Cargo.lock at build
/// time: `(package, version)`.
pub const VERSIONS: &[(&str, &str)] = include!(concat!(env!("OUT_DIR"), "/versions.rs"));

/// `rustc -V` of the compiler that built the report.
pub const RUSTC: &str = env!("BENCH_RUSTC_VERSION");

/// The locked version of a package, or `?`.
pub fn version(krate: &str) -> &'static str {
    VERSIONS
        .iter()
        .find(|(k, _)| *k == krate)
        .map_or("?", |(_, v)| v)
}
