// crates/forgo_lib_yaml/src/ast.rs
use std::fmt;
use std::fs;
use std::path::Path;

use crate::emitter::Emitter;
use crate::parser::Parser;

/// Path segment for traversal. `*` is a wildcard.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Seg {
    Key(String),
    Wildcard,
}

impl From<&str> for Seg {
    fn from(s: &str) -> Self {
        if s == "*" {
            Seg::Wildcard
        } else {
            Seg::Key(s.to_string())
        }
    }
}

/// Trait for types that can be converted into a path for traversal
///
/// This trait allows multiple convenient syntaxes for specifying paths:
/// - `&["jobs", "*", "steps"]` - array literal of string slices
/// - `&[Seg::Key("jobs".into()), Seg::Wildcard]` - explicit Seg types
/// - `vec!["jobs", "*"]` - vector of string slices
///
/// # Examples
///
/// ```
/// use forgo_lib_yaml::{Doc, Elem};
///
/// # let mut doc = Doc::from_str("jobs:\n  build:\n    steps: []").unwrap();
/// // All of these work:
/// doc.visit_sequences_mut(&["jobs", "build", "steps"], |s| { s.len(); false });
/// doc.visit_sequences_mut(&["jobs", "*", "steps"], |s| { s.len(); false });
/// # let path = vec!["jobs", "build", "steps"];
/// doc.visit_sequences_mut(&path[..], |s| { s.len(); false });
/// ```
pub trait IntoPath {
    fn into_path(self) -> Vec<Seg>;
}

// Support &[Seg] directly
impl IntoPath for &[Seg] {
    fn into_path(self) -> Vec<Seg> {
        self.to_vec()
    }
}

// Support &[Seg; N] - array literals of Seg
impl<const N: usize> IntoPath for &[Seg; N] {
    fn into_path(self) -> Vec<Seg> {
        self.to_vec()
    }
}

// Support Vec<Seg>
impl IntoPath for Vec<Seg> {
    fn into_path(self) -> Vec<Seg> {
        self
    }
}

// Support &[&str; N] - array literals like &["jobs", "*", "steps"]
impl<const N: usize> IntoPath for &[&str; N] {
    fn into_path(self) -> Vec<Seg> {
        self.iter().map(|&s| Seg::from(s)).collect()
    }
}

// Support &[&str] - slices from Vec<&str>
impl IntoPath for &[&str] {
    fn into_path(self) -> Vec<Seg> {
        self.iter().map(|&s| Seg::from(s)).collect()
    }
}

// Support Vec<&str>
impl IntoPath for Vec<&str> {
    fn into_path(self) -> Vec<Seg> {
        self.iter().map(|&s| Seg::from(s)).collect()
    }
}

/// Scalar with typing. If the source was quoted, we always store it as `Str`.
#[derive(Clone, Debug, PartialEq)]
pub enum Scalar {
    Str(String),
    Bool(bool),
    /// Numeric literal; we keep the original text for stable re-emission.
    Num {
        text: String,
    },
    Null,
}

impl Scalar {
    // ========================================================================
    // Type Guards
    // ========================================================================

    /// Check if this scalar is a string
    pub fn is_str(&self) -> bool {
        matches!(self, Scalar::Str(_))
    }

    /// Check if this scalar is a boolean
    pub fn is_bool(&self) -> bool {
        matches!(self, Scalar::Bool(_))
    }

    /// Check if this scalar is a number
    pub fn is_num(&self) -> bool {
        matches!(self, Scalar::Num { .. })
    }

    /// Check if this scalar is null
    pub fn is_null(&self) -> bool {
        matches!(self, Scalar::Null)
    }

    // ========================================================================
    // Type Conversions
    // ========================================================================

    /// Get string value if this scalar is a string
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Scalar::Str(s) => Some(s),
            _ => None,
        }
    }

    /// Get boolean value if this scalar is a boolean
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Scalar::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Parse as signed integer if this scalar is a number
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Scalar::Num { text } => text.parse().ok(),
            _ => None,
        }
    }

    /// Parse as float if this scalar is a number
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Scalar::Num { text } => text.parse().ok(),
            _ => None,
        }
    }

    /// Parse as unsigned integer if this scalar is a number
    pub fn as_u64(&self) -> Option<u64> {
        match self {
            Scalar::Num { text } => text.parse().ok(),
            _ => None,
        }
    }

    /// Get raw number text without parsing
    pub fn num_text(&self) -> Option<&str> {
        match self {
            Scalar::Num { text } => Some(text),
            _ => None,
        }
    }

    /// Convert scalar to string representation
    pub fn to_string(&self) -> String {
        match self {
            Scalar::Str(s) => s.clone(),
            Scalar::Bool(true) => "true".into(),
            Scalar::Bool(false) => "false".into(),
            Scalar::Num { text } => text.clone(),
            Scalar::Null => "null".into(),
        }
    }
}

/// Node metadata we preserve and re-emit.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Meta {
    /// Full-line comments immediately preceding this node (without the '#', trimmed).
    pub leading_comments: Vec<String>,
    /// Same-line comment trailing the node/value (without the '#', trimmed).
    pub trailing_comment: Option<String>,
    /// Optional anchor name attached to this node: `&name`.
    pub anchor: Option<String>,
    /// Optional tag attached to this node: `!str`, `!!str`, `!<tag:example.com,2000:app/thing>`, etc.
    pub tag: Option<String>,
    /// Remember the source style so emitter can force block style when desired
    pub prefer_block: bool,
    /// Preserving quotes for an originally quoted scalar ("abc" at top level).
    pub prefer_quoted: bool,
    /// If present, preserve original block scalar style instead of canonicalizing.
    pub block_style: Option<BlockStyle>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockStyle {
    /// Literal `|` with optional chomping (+/-/None).
    Literal(Option<char>),
    /// Folded `>` with optional chomping (+/-/None).
    Folded(Option<char>),
}

/// An element pairs a node with metadata (comments, anchor).
#[derive(Clone, Debug, PartialEq)]
pub struct Elem {
    pub node: Node,
    pub meta: Meta,
}
impl Elem {
    // ========================================================================
    // Constructors
    // ========================================================================

    /// Create element from node with default metadata
    pub fn new(node: Node) -> Self {
        Self {
            node,
            meta: Meta::default(),
        }
    }

    /// Create element with empty map
    pub fn map() -> Self {
        Self::new(Node::map())
    }

    /// Create element with empty sequence
    pub fn seq() -> Self {
        Self::new(Node::seq())
    }

    /// Create element with string scalar
    pub fn string(s: impl Into<String>) -> Self {
        Self::new(Node::string(s))
    }

    /// Create element with number scalar
    pub fn number(n: impl ToString) -> Self {
        Self::new(Node::number(n))
    }

    /// Create element with boolean scalar
    pub fn boolean(b: bool) -> Self {
        Self::new(Node::boolean(b))
    }

    /// Create element with null scalar
    pub fn null() -> Self {
        Self::new(Node::null())
    }

    // ========================================================================
    // Fluent Metadata Builders
    // ========================================================================

    /// Add trailing comment (same line as value)
    pub fn with_comment(mut self, comment: impl Into<String>) -> Self {
        self.meta.trailing_comment = Some(comment.into());
        self
    }

    /// Add leading comments (full lines before value)
    pub fn with_leading_comments(mut self, comments: Vec<String>) -> Self {
        self.meta.leading_comments = comments;
        self
    }

    /// Add anchor reference (&name)
    pub fn with_anchor(mut self, anchor: impl Into<String>) -> Self {
        self.meta.anchor = Some(anchor.into());
        self
    }

    /// Prefer block style when emitting
    pub fn prefer_block(mut self) -> Self {
        self.meta.prefer_block = true;
        self
    }

    /// Prefer quoted style for string scalars
    pub fn prefer_quoted(mut self) -> Self {
        self.meta.prefer_quoted = true;
        self
    }

    // ========================================================================
    // Accessors
    // ========================================================================

    /// Convenience: borrow the underlying node.
    pub fn node(&self) -> &Node {
        &self.node
    }

    /// Convenience: mutably borrow the underlying node.
    pub fn node_mut(&mut self) -> &mut Node {
        &mut self.node
    }
}

/// Map key - can be a simple string or a complex element (collection)
#[derive(Clone, Debug, PartialEq)]
pub enum MapKey {
    /// Simple scalar key (most common case)
    Str(String),
    /// Complex key (sequence or mapping as a key)
    Complex(Box<Elem>),
}

impl MapKey {
    /// Create a string key
    pub fn from_str(s: impl Into<String>) -> Self {
        MapKey::Str(s.into())
    }

    /// Create a complex key
    pub fn from_elem(elem: Elem) -> Self {
        MapKey::Complex(Box::new(elem))
    }

    /// Get as string if this is a string key
    pub fn as_str(&self) -> Option<&str> {
        match self {
            MapKey::Str(s) => Some(s),
            MapKey::Complex(_) => None,
        }
    }

    /// Get as element if this is a complex key
    pub fn as_elem(&self) -> Option<&Elem> {
        match self {
            MapKey::Str(_) => None,
            MapKey::Complex(e) => Some(e),
        }
    }

    /// Check if this is a string key
    pub fn is_str(&self) -> bool {
        matches!(self, MapKey::Str(_))
    }

    /// Check if this is a complex key
    pub fn is_complex(&self) -> bool {
        matches!(self, MapKey::Complex(_))
    }
}

// Allow comparing MapKey with &str for convenience
impl PartialEq<str> for MapKey {
    fn eq(&self, other: &str) -> bool {
        match self {
            MapKey::Str(s) => s == other,
            MapKey::Complex(_) => false,
        }
    }
}

impl PartialEq<String> for MapKey {
    fn eq(&self, other: &String) -> bool {
        match self {
            MapKey::Str(s) => s == other,
            MapKey::Complex(_) => false,
        }
    }
}

impl PartialEq<&str> for MapKey {
    fn eq(&self, other: &&str) -> bool {
        match self {
            MapKey::Str(s) => s == *other,
            MapKey::Complex(_) => false,
        }
    }
}

// Display for MapKey
impl fmt::Display for MapKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MapKey::Str(s) => write!(f, "{}", s),
            MapKey::Complex(elem) => {
                // For complex keys, we need to serialize them
                // This is a simplified representation
                match &elem.node {
                    Node::Seq(_) => write!(f, "[...]"),
                    Node::Map(_) => write!(f, "{{...}}"),
                    Node::Scalar(s) => write!(f, "{}", s.to_string()),
                    Node::Alias(a) => write!(f, "*{}", a),
                }
            }
        }
    }
}

// Allow creating MapKey from String
impl From<String> for MapKey {
    fn from(s: String) -> Self {
        MapKey::Str(s)
    }
}

impl From<&str> for MapKey {
    fn from(s: &str) -> Self {
        MapKey::Str(s.to_string())
    }
}

/// AST node for the YAML subset.
#[derive(Clone, Debug, PartialEq)]
pub enum Node {
    Map(Vec<(MapKey, Elem)>),
    Seq(Vec<Elem>),
    Scalar(Scalar),
    /// Alias reference: `*name`
    Alias(String),
}
impl Node {
    // ========================================================================
    // Type Guards
    // ========================================================================

    /// Check if this node is a map
    pub fn is_map(&self) -> bool {
        matches!(self, Node::Map(_))
    }

    /// Check if this node is a sequence
    pub fn is_seq(&self) -> bool {
        matches!(self, Node::Seq(_))
    }

    /// Check if this node is a scalar
    pub fn is_scalar(&self) -> bool {
        matches!(self, Node::Scalar(_))
    }

    /// Check if this node is an alias
    pub fn is_alias(&self) -> bool {
        matches!(self, Node::Alias(_))
    }

    /// Check if this node is a null scalar
    pub fn is_null(&self) -> bool {
        matches!(self, Node::Scalar(Scalar::Null))
    }

    // ========================================================================
    // Type Conversions (Safe Downcasts)
    // ========================================================================

    /// Get map if this node is a map
    pub fn as_map(&self) -> Option<&Vec<(MapKey, Elem)>> {
        match self {
            Node::Map(m) => Some(m),
            _ => None,
        }
    }

    /// Get mutable map if this node is a map
    pub fn as_map_mut(&mut self) -> Option<&mut Vec<(MapKey, Elem)>> {
        match self {
            Node::Map(m) => Some(m),
            _ => None,
        }
    }

    /// Get sequence if this node is a sequence
    pub fn as_seq(&self) -> Option<&Vec<Elem>> {
        match self {
            Node::Seq(v) => Some(v),
            _ => None,
        }
    }

    /// Get mutable sequence if this node is a sequence (alias for seq_mut)
    pub fn as_seq_mut(&mut self) -> Option<&mut Vec<Elem>> {
        self.seq_mut()
    }

    /// Get mutable sequence (legacy method, prefer as_seq_mut)
    pub fn seq_mut(&mut self) -> Option<&mut Vec<Elem>> {
        match self {
            Node::Seq(v) => Some(v),
            _ => None,
        }
    }

    /// Get scalar if this node is a scalar
    pub fn as_scalar(&self) -> Option<&Scalar> {
        match self {
            Node::Scalar(s) => Some(s),
            _ => None,
        }
    }

    /// Get mutable scalar if this node is a scalar
    pub fn as_scalar_mut(&mut self) -> Option<&mut Scalar> {
        match self {
            Node::Scalar(s) => Some(s),
            _ => None,
        }
    }

    // ========================================================================
    // Scalar Convenience Methods (delegate to Scalar)
    // ========================================================================

    /// Get string value if this node is a string scalar
    pub fn as_str(&self) -> Option<&str> {
        self.as_scalar().and_then(|s| s.as_str())
    }

    /// Get boolean value if this node is a boolean scalar
    pub fn as_bool(&self) -> Option<bool> {
        self.as_scalar().and_then(|s| s.as_bool())
    }

    /// Get integer value if this node is a number scalar (parses text)
    pub fn as_i64(&self) -> Option<i64> {
        self.as_scalar().and_then(|s| s.as_i64())
    }

    /// Get float value if this node is a number scalar (parses text)
    pub fn as_f64(&self) -> Option<f64> {
        self.as_scalar().and_then(|s| s.as_f64())
    }

    /// Get unsigned integer value if this node is a number scalar (parses text)
    pub fn as_u64(&self) -> Option<u64> {
        self.as_scalar().and_then(|s| s.as_u64())
    }

    // ========================================================================
    // Node Constructors
    // ========================================================================

    /// Create an empty map node
    pub fn map() -> Self {
        Node::Map(vec![])
    }

    /// Create an empty sequence node
    pub fn seq() -> Self {
        Node::Seq(vec![])
    }

    /// Create a string scalar node
    pub fn string(s: impl Into<String>) -> Self {
        Node::Scalar(Scalar::Str(s.into()))
    }

    /// Create a number scalar node
    pub fn number(n: impl ToString) -> Self {
        Node::Scalar(Scalar::Num {
            text: n.to_string(),
        })
    }

    /// Create a boolean scalar node
    pub fn boolean(b: bool) -> Self {
        Node::Scalar(Scalar::Bool(b))
    }

    /// Create a null scalar node
    pub fn null() -> Self {
        Node::Scalar(Scalar::Null)
    }
}

/// Document root wrapper.
#[derive(Clone, Debug, PartialEq)]
pub struct Doc {
    pub(crate) root: Elem,
    // True if the original input began with at least one blank line (CR, LF, or CRLF).
    pub(crate) had_leading_blank: bool,
    pub(crate) saw_carriage_return: bool,

    /// Optional YAML version directive, e.g. (1,2) for `%YAML 1.2`
    pub yaml_version: Option<(u8, u8)>,
    /// Collected TAG directives as (handle, prefix), e.g. ("!e!", "tag:example.com,2000:app/")
    pub tag_handles: Vec<(String, String)>,
    /// Was this document explicitly started with `---`?
    pub explicit_start: bool,
    /// Was this document explicitly ended with `...`?
    pub explicit_end: bool,
}

/// Error type (no deps).
#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Parse(String),
    Emit(String),
    Fmt(std::fmt::Error),
}

impl From<std::fmt::Error> for Error {
    fn from(e: std::fmt::Error) -> Self {
        Error::Fmt(e)
    }
}

// If you need std::io::Error conversions elsewhere:
impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

impl std::error::Error for Error {}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(e) => write!(f, "I/O error: {e}"),
            Error::Parse(s) => write!(f, "Parse error: {s}"),
            Error::Emit(s) => write!(f, "Emit error: {s}"),
            Error::Fmt(e) => write!(f, "Format error: ${e}"),
        }
    }
}

impl Doc {
    // Accessors for tests / users
    pub fn yaml_version(&self) -> Option<(u8, u8)> {
        self.yaml_version
    }
    pub fn tag_handles(&self) -> &[(String, String)] {
        &self.tag_handles
    }
    pub fn explicit_start(&self) -> bool {
        self.explicit_start
    }
    pub fn explicit_end(&self) -> bool {
        self.explicit_end
    }

    /// Parse from UTF-8 string.
    /// If input contains multiple documents, only the first is returned.
    /// Use from_stream() to parse all documents in a multi-document stream.
    pub fn from_str(input: &str) -> Result<Self, Error> {
        let mut p = Parser::new(input);
        p.parse_document()
    }

    /// Parse a full YAML stream (possibly multiple documents separated by `---`/`...`).
    pub fn from_stream(input: &str) -> Result<Vec<Self>, Error> {
        let mut p = Parser::new(input);
        p.parse_stream()
    }

    /// Emit as canonical string (dash-lists, stable indentation, comments).
    pub fn to_string(&self) -> Result<String, Error> {
        let mut out = String::new();
        // If the first thing we’ll emit is a comment (either on the root
        // or on the first child of a map/seq), don’t add the preserved
        // leading blank line — tests expect the comment to be first.
        let begins_with_comment = !self.root.meta.leading_comments.is_empty()
            || match &self.root.node {
                crate::ast::Node::Map(entries) => entries
                    .first()
                    .map_or(false, |(_, v)| !v.meta.leading_comments.is_empty()),
                crate::ast::Node::Seq(items) => items
                    .first()
                    .map_or(false, |e| !e.meta.leading_comments.is_empty()),
                _ => false,
            };

        // Preserve original leading blank if present and not overshadowed by banner comments.
        if self.had_leading_blank && !begins_with_comment {
            out.push('\n');
        }

        Emitter::emit_elem(&self.root, 0, &mut out)?;
        Ok(out)
    }

    /// Emit a full stream.
    pub fn to_stream_string(docs: &[Self]) -> Result<String, Error> {
        let mut out = String::new();
        let mut first = true;
        for d in docs {
            if !first {
                // separate docs with newline if needed
                if !out.ends_with('\n') {
                    out.push('\n');
                }
            }
            first = false;
            d.emit_one_into(&mut out)?;
        }
        Ok(out)
    }

    fn emit_one_into(&self, out: &mut String) -> Result<(), Error> {
        use std::fmt::Write;
        // Directives must precede `---` and the doc content.
        if let Some((maj, min)) = self.yaml_version {
            writeln!(out, "%YAML {maj}.{min}").map_err(Error::from)?;
        }
        for (handle, prefix) in &self.tag_handles {
            writeln!(out, "%TAG {} {}", handle, prefix).map_err(Error::from)?;
        }
        if self.explicit_start || self.yaml_version.is_some() || !self.tag_handles.is_empty() {
            writeln!(out, "---").map_err(Error::from)?;
        }
        Emitter::emit_elem(&self.root, 0, out)?;
        if self.explicit_end {
            writeln!(out, "...").map_err(Error::from)?;
        }
        Ok(())
    }

    /// Read a file and parse.
    pub fn read_file(path: &Path) -> Result<Self, Error> {
        let s = fs::read_to_string(path).map_err(Error::Io)?;
        Self::from_str(&s)
    }

    /// Write to file (overwrite).
    pub fn write_file(&self, path: &Path) -> Result<(), Error> {
        let s = self.to_string()?;
        fs::write(path, s).map_err(Error::Io)
    }

    /// Visit all nodes at `path`, applying `f` to sequences.
    pub fn visit_sequences_mut<P, F>(&mut self, path: P, mut f: F) -> bool
    where
        P: IntoPath,
        F: FnMut(&mut Vec<Elem>) -> bool,
    {
        let path = path.into_path();
        fn go<F>(elem: &mut Elem, path: &[Seg], f: &mut F) -> bool
        where
            F: FnMut(&mut Vec<Elem>) -> bool,
        {
            if path.is_empty() {
                if let Node::Seq(ref mut v) = elem.node {
                    return f(v);
                }
                return false;
            }
            match &mut elem.node {
                Node::Map(entries) => match &path[0] {
                    Seg::Key(k) => entries
                        .iter_mut()
                        .find(|(kk, _)| kk == k)
                        .map(|(_, child)| go(child, &path[1..], f))
                        .unwrap_or(false),
                    Seg::Wildcard => {
                        let mut changed = false;
                        for (_, child) in entries.iter_mut() {
                            changed |= go(child, &path[1..], f);
                        }
                        changed
                    }
                },
                Node::Seq(items) => match &path[0] {
                    Seg::Wildcard => {
                        let mut changed = false;
                        for child in items.iter_mut() {
                            changed |= go(child, &path[1..], f);
                        }
                        changed
                    }
                    Seg::Key(_) => false,
                },
                _ => false,
            }
        }
        go(&mut self.root, &path, &mut f)
    }

    pub fn visit_maps_mut<P, F>(&mut self, path: P, mut f: F) -> bool
    where
        P: IntoPath,
        F: FnMut(&mut Vec<(MapKey, Elem)>) -> bool,
    {
        let path = path.into_path();
        fn go<F>(node: &mut Elem, path: &[Seg], hit: &mut bool, f: &mut F)
        where
            F: FnMut(&mut Vec<(MapKey, Elem)>) -> bool,
        {
            if path.is_empty() {
                if let Node::Map(ref mut m) = node.node {
                    *hit = f(m) || *hit;
                }
                return;
            }
            match (&mut node.node, &path[0]) {
                (Node::Map(entries), Seg::Key(k)) => {
                    if let Some((_, v)) = entries.iter_mut().find(|(kk, _)| kk == k) {
                        go(v, &path[1..], hit, f);
                    }
                }
                (Node::Map(entries), Seg::Wildcard) => {
                    for (_, v) in entries.iter_mut() {
                        go(v, &path[1..], hit, f);
                    }
                }
                (Node::Seq(items), _) => {
                    for it in items.iter_mut() {
                        go(it, path, hit, f);
                    }
                }
                _ => {}
            }
        }
        let mut hit = false;
        go(&mut self.root, &path, &mut hit, &mut f);
        hit
    }

    pub fn visit_values_mut<P, F>(&mut self, path: P, mut f: F) -> bool
    where
        P: IntoPath,
        F: FnMut(&mut Elem) -> bool,
    {
        let path = path.into_path();
        fn go<F>(node: &mut Elem, path: &[Seg], hit: &mut bool, f: &mut F)
        where
            F: FnMut(&mut Elem) -> bool,
        {
            if path.is_empty() {
                *hit = f(node) || *hit;
                return;
            }
            match (&mut node.node, &path[0]) {
                (Node::Map(entries), Seg::Key(k)) => {
                    if let Some((_, v)) = entries.iter_mut().find(|(kk, _)| kk == k) {
                        go(v, &path[1..], hit, f);
                    }
                }
                (Node::Map(entries), Seg::Wildcard) => {
                    for (_, v) in entries.iter_mut() {
                        go(v, &path[1..], hit, f);
                    }
                }
                (Node::Seq(items), _) => {
                    for it in items.iter_mut() {
                        go(it, path, hit, f);
                    }
                }
                _ => {}
            }
        }
        let mut hit = false;
        go(&mut self.root, &path, &mut hit, &mut f);
        hit
    }

    /// Borrow the root element.
    pub fn root(&self) -> &Elem {
        &self.root
    }

    /// Mutably borrow the root element.
    pub fn root_mut(&mut self) -> &mut Elem {
        &mut self.root
    }
}
