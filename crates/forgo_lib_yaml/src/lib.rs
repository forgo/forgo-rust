// crates/forgo_lib_yaml/src/lib.rs
//! forgo_lib_yaml: A tiny, zero-dependency YAML subset parser/editor tailored for CI/config editing.
//!
//! Goals:
//! - Deterministic round-trips for supported subset
//! - Small, auditable code
//! - Editor API: visit by path (with wildcards) + normalize sequences (sort/dedup)
//!
//! Non-goals (initially): full YAML 1.2, anchors/tags, block scalars (|/>), comments preservation.
//! Extend incrementally in lexer/parser/emitter where marked by `// EXTEND:`.

mod ast;
mod char_validator;
mod editor;
mod emitter;
mod emitter_helpers;
mod flow_validator;
mod lexer;
mod parser;
mod parser_helpers;
mod util;

pub use crate::lexer::{Lexer, Tok};
pub use ast::{Doc, Elem, Error, IntoPath, MapKey, Node, Scalar, Seg};
pub use editor::{edit_file_in_place, normalize_string_list};
pub use util::needs_quotes;

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

// ============================================================================
// Debug Utilities
// ============================================================================

pub fn debug_token_dump(input: &str) -> String {
    let mut lx = crate::lexer::Lexer::new(input);
    let mut out = String::new();
    loop {
        let t = lx.next_token();
        use std::fmt::Write;
        let _ = writeln!(out, "{t:?}");
        if matches!(t, crate::lexer::Tok::Eof) {
            break;
        }
    }
    out
}
