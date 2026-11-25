//! A YAML 1.2.2 parser, emitter and editor that keeps your files as they
//! are. No dependencies, no `unsafe`.
//!
//! - [`Doc`] is a parsed document: read it ([`Doc::root`]), edit it
//!   ([`Doc::root_mut`], [`Doc::visit_values_mut`] and friends) and write
//!   it back ([`Doc::to_string`]). Untouched text stays byte for byte;
//!   edits rewrite only what changed.
//! - [`events`] is a streaming parser that yields [`Event`]s with source
//!   [`Span`]s.
//! - Values follow the YAML 1.2 core schema ([`Scalar`]); comments, styles,
//!   anchors and tags are kept in each node's [`Meta`].
//!
//! ```
//! use forgo_lib_yaml::{Doc, Node};
//!
//! let mut doc = Doc::from_str("version: 1.4.2   # bump on release\nname: app\n")?;
//! doc.visit_values_mut(&["version"], |v| {
//!     *v.node_mut() = Node::string("1.5.0");
//!     true
//! });
//! assert_eq!(doc.to_string()?, "version: 1.5.0   # bump on release\nname: app\n");
//! # Ok::<(), forgo_lib_yaml::Error>(())
//! ```
//!
//! Compliance (all 402 cases of the official yaml-test-suite), benchmarks
//! and comparisons with other crates are in the
//! [README](https://github.com/forgo/forgo-rust/tree/HEAD/crates/forgo_lib_yaml#readme).

#![forbid(unsafe_code)]
#![warn(missing_docs)]

/// The README's examples, compiled and run as doctests.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
pub struct ReadmeDoctests;

mod ast;
mod compose;
mod editor;
mod emitter;
mod emitter_helpers;
mod preserve;
mod schema;
pub mod syntax;
mod util;

pub use ast::{Doc, Elem, Error, IntoPath, MapKey, Meta, Node, Scalar, Seg, SourceSpan, Style};
pub use editor::{edit_file_in_place, normalize_string_list, try_normalize_string_list};
pub use syntax::{
    BlockHeader, Chomping, Event, Events, Mark, ScalarStyle, Span, Tag, events, events_from_bytes,
};
pub use util::needs_quotes;

/// Limits applied while composing documents.
///
/// ```
/// use forgo_lib_yaml::{Doc, ParseOptions};
///
/// let options = ParseOptions::default().with_max_depth(64);
/// assert!(Doc::from_str_with("a: [[1]]\n", &options).is_ok());
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct ParseOptions {
    /// Deepest allowed nesting of collections. Deeper input is an error
    /// instead of risking a stack overflow in code that walks the tree
    /// recursively (dropping, cloning, comparing and emitting it). The
    /// event parser itself has no limit.
    pub max_depth: usize,
}

impl Default for ParseOptions {
    fn default() -> Self {
        ParseOptions { max_depth: 512 }
    }
}

impl ParseOptions {
    /// These options with `max_depth` set.
    pub fn with_max_depth(mut self, max_depth: usize) -> Self {
        self.max_depth = max_depth;
        self
    }
}

// ============================================================================
// Layer 1: Simple "Just Works" API
// ============================================================================

/// Parse the first document of a YAML stream (same as [`Doc::from_str`]).
///
/// # Examples
///
/// ```
/// use forgo_lib_yaml::parse;
///
/// let doc = parse("name: test").unwrap();
/// assert_eq!(doc.root().node().as_map().unwrap()[0].0.as_str(), Some("name"));
/// ```
pub fn parse(input: &str) -> Result<Doc, Error> {
    Doc::from_str(input)
}

/// Write a document as YAML (same as [`Doc::to_string`]).
///
/// # Examples
///
/// ```
/// use forgo_lib_yaml::{parse, stringify};
///
/// let doc = parse("name: test").unwrap();
/// let yaml = stringify(&doc).unwrap();
/// assert!(yaml.contains("name:"));
/// ```
pub fn stringify(doc: &Doc) -> Result<String, Error> {
    doc.to_string()
}

/// Parse the first document of a YAML byte stream: UTF-8, UTF-16 or UTF-32,
/// detected per YAML 1.2.2 §5.2, with or without a byte order mark (same as
/// [`Doc::from_slice`]).
///
/// # Examples
///
/// ```
/// use forgo_lib_yaml::parse_bytes;
///
/// // "a: 1" in UTF-16LE with a byte order mark
/// let bytes: Vec<u8> = "\u{FEFF}a: 1".encode_utf16().flat_map(u16::to_le_bytes).collect();
/// let doc = parse_bytes(&bytes).unwrap();
/// assert_eq!(doc.root().node().as_map().unwrap()[0].1.node().as_i64(), Some(1));
/// ```
pub fn parse_bytes(bytes: &[u8]) -> Result<Doc, Error> {
    Doc::from_slice(bytes)
}

/// Parse every document of a YAML stream (same as [`Doc::from_stream`]).
///
/// # Examples
///
/// ```
/// use forgo_lib_yaml::parse_all;
///
/// let yaml = "---\ndoc: first\n---\ndoc: second";
/// let docs = parse_all(yaml).unwrap();
/// assert_eq!(docs.len(), 2);
/// ```
pub fn parse_all(input: &str) -> Result<Vec<Doc>, Error> {
    Doc::from_stream(input)
}
