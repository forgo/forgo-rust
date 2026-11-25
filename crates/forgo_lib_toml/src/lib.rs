//! TOML 1.1 and 1.0 for Rust.
//!
//! - [`Document`]: parse TOML into a document you can read through
//!   [`Table`], [`Array`] and [`Value`] views, and edit in place: each
//!   edit changes only the text it must, and is checked.
//! - [`Item`]: build TOML in code and encode it.
//!
//! ```
//! use forgo_lib_toml::Document;
//!
//! let doc = Document::parse("[package]\nname = \"demo\"\nversion = \"0.1.0\"\n")?;
//! assert_eq!(doc.get(["package", "version"]).unwrap().as_str(), Some("0.1.0"));
//! # Ok::<(), forgo_lib_toml::Error>(())
//! ```
//!
//! No dependencies, no `unsafe`. Passes all of toml-test, TOML's official
//! suite, for both versions. Compliance details and comparisons with other
//! crates are in the
//! [README](https://github.com/forgo/forgo-rust/tree/HEAD/crates/forgo_lib_toml#readme).

#![forbid(unsafe_code)]
#![warn(missing_docs)]

/// The README's examples, compiled and run as doctests.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
pub struct ReadmeDoctests;

mod datetime;
mod doc;
mod edit;
mod error;
mod item;
mod parse;

pub use datetime::{Date, Datetime, Offset, Time};
pub use doc::{Array, Document, ParseOptions, Table, TomlVersion, Value};
pub use edit::{EditError, IntoPath, Seg, edit_file_in_place};
pub use error::{Error, ErrorKind};
pub use item::{EncodeError, Item};
