// crates/forgo_lib_yaml/src/lib.rs
#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]

mod ast;
mod compose;
mod editor;
mod emitter;
mod emitter_helpers;
mod schema;
pub mod syntax;
mod util;

pub use ast::{Doc, Elem, Error, IntoPath, MapKey, Meta, Node, Scalar, Seg, SourceSpan, Style};
pub use editor::{edit_file_in_place, normalize_string_list};
pub use syntax::{
    BlockHeader, Chomping, Event, Events, Mark, ScalarStyle, Span, Tag, events, events_from_bytes,
};
pub use util::needs_quotes;

/// Limits applied while composing documents.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

// ============================================================================
// Layer 1: Simple "Just Works" API
// ============================================================================

/// Parse a single YAML document from a string
///
/// This is a convenience wrapper around [`Doc::from_str`].
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

/// Serialize a document to a YAML string
///
/// This is a convenience wrapper around [`Doc::to_string`].
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

/// Parse multiple YAML documents from a stream (separated by --- or ...)
///
/// This is a convenience wrapper around [`Doc::from_stream`].
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
