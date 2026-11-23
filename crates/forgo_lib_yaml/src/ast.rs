// crates/forgo_lib_yaml/src/ast.rs
use std::fmt;
use std::fs;
use std::path::Path;
use std::sync::Arc;

use crate::emitter::Emitter;
pub use crate::syntax::{BlockHeader, Chomping};

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

    /// The value as a signed integer (decimal, `0o` octal or `0x` hex).
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Scalar::Num { text } => crate::schema::int_value(text)?.try_into().ok(),
            _ => None,
        }
    }

    /// The value as a float, including integers and `.inf`/`.nan`.
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Scalar::Num { text } => crate::schema::float_value(text),
            _ => None,
        }
    }

    /// The value as an unsigned integer (decimal, `0o` octal or `0x` hex).
    pub fn as_u64(&self) -> Option<u64> {
        match self {
            Scalar::Num { text } => crate::schema::int_value(text)?.try_into().ok(),
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
    // Public API predating `Display`; kept for compatibility.
    #[allow(clippy::inherent_to_string)]
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

/// How a node is written.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Style {
    /// A plain scalar.
    Plain,
    /// `'single quoted'`
    SingleQuoted,
    /// `"double quoted"`
    DoubleQuoted,
    /// A `|` block scalar and its header indicators.
    Literal(BlockHeader),
    /// A `>` block scalar and its header indicators.
    Folded(BlockHeader),
    /// A block collection (indented entries, `- ` items).
    Block,
    /// A flow collection (`[a, b]`, `{a: 1}`).
    Flow,
}

/// The byte range a node had in its source, when it was parsed. Always
/// equal to any other `SourceSpan`, so documents compare by content and
/// presentation, not by origin. (Byte offsets only, to keep nodes small;
/// line and column can be computed from the source text.)
#[derive(Clone, Copy, Debug, Default)]
pub struct SourceSpan(Option<(u32, u32)>);

impl SourceSpan {
    pub(crate) fn from_span(span: crate::syntax::Span) -> Self {
        match (
            u32::try_from(span.start.index),
            u32::try_from(span.end.index),
        ) {
            (Ok(s), Ok(e)) => SourceSpan(Some((s, e))),
            _ => SourceSpan(None),
        }
    }

    pub(crate) fn set_end(&mut self, end: usize) {
        if let (Some((_, e)), Ok(end)) = (&mut self.0, u32::try_from(end)) {
            *e = end;
        }
    }

    /// The node's byte range in the source, if it was parsed.
    pub fn range(&self) -> Option<std::ops::Range<usize>> {
        self.0.map(|(s, e)| s as usize..e as usize)
    }
}

impl PartialEq for SourceSpan {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

/// Where a parsed document came from: the source text, the document's
/// byte range in it, and its document-level fields as parsed. Writing a
/// document back copies the source text of everything that didn't change
/// (see `preserve`). Ignored by `==`.
#[derive(Clone, Debug, Default)]
pub(crate) struct Origin {
    /// The whole source stream (shared by its documents).
    pub(crate) source: Option<Arc<str>>,
    /// This document's byte range in `source`.
    pub(crate) range: (usize, usize),
    /// A mutable borrow of the tree was handed out, so it may differ from
    /// the source.
    pub(crate) touched: bool,
    pub(crate) yaml_version: Option<(u8, u8)>,
    pub(crate) tag_handles: Vec<(String, String)>,
    pub(crate) explicit_start: bool,
    pub(crate) explicit_end: bool,
}

impl PartialEq for Origin {
    fn eq(&self, _: &Self) -> bool {
        true
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
    /// Optional tag, as written: `!str`, `!!str`, `!<tag:example.com,2000:app/thing>`, etc.
    pub tag: Option<String>,
    /// How the node was written. `None` (nodes built in code) lets the
    /// emitter choose. The emitter honors a style when the value allows it.
    pub style: Option<Style>,
    /// The source text of a plain null or boolean (`~`, `Null`, `True`, or
    /// empty) when it differs from the default `null`, `true` or `false`.
    pub spelling: Option<String>,
    /// Where the node was in the source (ignored by `==`).
    pub span: SourceSpan,
}

impl Meta {
    /// Single- or double-quoted.
    pub fn is_quoted(&self) -> bool {
        matches!(self.style, Some(Style::SingleQuoted | Style::DoubleQuoted))
    }

    /// A literal or folded block scalar, or a block collection.
    pub fn is_block(&self) -> bool {
        matches!(
            self.style,
            Some(Style::Literal(_) | Style::Folded(_) | Style::Block)
        )
    }

    /// A flow collection.
    pub fn is_flow(&self) -> bool {
        self.style == Some(Style::Flow)
    }
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

    /// Emit with block style: a literal block scalar for strings, block
    /// layout for collections.
    pub fn prefer_block(mut self) -> Self {
        self.meta.style = Some(match self.node {
            Node::Map(_) | Node::Seq(_) => Style::Block,
            _ => Style::Literal(BlockHeader::default()),
        });
        self
    }

    /// Emit strings double-quoted.
    pub fn prefer_quoted(mut self) -> Self {
        self.meta.style = Some(Style::DoubleQuoted);
        self
    }

    /// Emit with the given style (when the value allows it).
    pub fn with_style(mut self, style: Style) -> Self {
        self.meta.style = Some(style);
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
    // Public API (takes `impl Into<String>`, so it isn't `FromStr`).
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: impl Into<String>) -> Self {
        MapKey::Str(s.into())
    }

    /// Create a complex key
    pub fn from_elem(elem: Elem) -> Self {
        MapKey::Complex(Box::new(elem))
    }

    /// The key's text: a string key, or a scalar key without a string
    /// type (`200`, `true`) as written. `None` for null, alias and
    /// collection keys.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            MapKey::Str(s) => Some(s),
            MapKey::Complex(e) => match &e.node {
                Node::Scalar(Scalar::Str(s)) => Some(s),
                Node::Scalar(Scalar::Num { text }) => Some(text),
                Node::Scalar(Scalar::Bool(true)) => Some("true"),
                Node::Scalar(Scalar::Bool(false)) => Some("false"),
                _ => None,
            },
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

impl MapKey {
    /// Does this key match `text`? String and scalar keys compare by their
    /// text; a null key matches any null spelling (`null`, `~`, ...).
    fn matches_text(&self, text: &str) -> bool {
        if self.as_str() == Some(text) {
            return true;
        }
        matches!(self, MapKey::Complex(e) if matches!(e.node, Node::Scalar(Scalar::Null)))
            && crate::schema::is_null(text)
    }

    /// A canonical form for comparing keys by value (YAML 1.2.2 §3.2.1.1):
    /// `a` and `"a"` are equal, as are `0x1` and `1`.
    pub(crate) fn canonical(&self) -> String {
        match self {
            MapKey::Str(s) => format!("s:{s}"),
            MapKey::Complex(e) => canonical_elem(e),
        }
    }
}

fn canonical_elem(e: &Elem) -> String {
    match &e.node {
        Node::Scalar(Scalar::Str(s)) => format!("s:{s}"),
        Node::Scalar(Scalar::Bool(b)) => format!("b:{b}"),
        Node::Scalar(Scalar::Null) => "n".into(),
        Node::Scalar(Scalar::Num { text }) => match crate::schema::int_value(text) {
            Some(i) if crate::schema::is_int(text) => format!("i:{i}"),
            _ => format!(
                "f:{}",
                crate::schema::float_value(text).map_or_else(|| text.clone(), |f| format!("{f:?}"))
            ),
        },
        Node::Alias(a) => format!("*{a}"),
        Node::Seq(items) => {
            let parts: Vec<String> = items.iter().map(canonical_elem).collect();
            format!("[{}]", parts.join(","))
        }
        Node::Map(entries) => {
            let mut parts: Vec<String> = entries
                .iter()
                .map(|(k, v)| format!("{}={}", k.canonical(), canonical_elem(v)))
                .collect();
            parts.sort();
            format!("{{{}}}", parts.join(","))
        }
    }
}

// Compare a key with text: string keys and scalar keys by their text.
impl PartialEq<str> for MapKey {
    fn eq(&self, other: &str) -> bool {
        self.matches_text(other)
    }
}

impl PartialEq<String> for MapKey {
    fn eq(&self, other: &String) -> bool {
        self.matches_text(other)
    }
}

impl PartialEq<&str> for MapKey {
    fn eq(&self, other: &&str) -> bool {
        self.matches_text(other)
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
    /// Comments after the document's last node.
    pub(crate) footer_comments: Vec<String>,
    /// The source stream had no document at all (only comments or nothing).
    pub(crate) no_content: bool,
    /// The source text, for format-preserving output (ignored by `==`).
    pub(crate) origin: Origin,
}

/// Error type (no deps).
#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Parse(String),
    /// A syntax error at a 1-based line and column.
    Syntax {
        line: usize,
        col: usize,
        msg: String,
    },
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
            Error::Syntax { line, col, msg } => write!(f, "Parse error at {line}:{col}: {msg}"),
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

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn from_parts(
        root: Elem,
        had_leading_blank: bool,
        saw_carriage_return: bool,
        yaml_version: Option<(u8, u8)>,
        tag_handles: Vec<(String, String)>,
        explicit_start: bool,
        explicit_end: bool,
        footer_comments: Vec<String>,
        source: Option<(Arc<str>, (usize, usize))>,
    ) -> Self {
        let origin = match source {
            Some((source, range)) => Origin {
                source: Some(source),
                range,
                touched: false,
                yaml_version,
                tag_handles: tag_handles.clone(),
                explicit_start,
                explicit_end,
            },
            None => Origin::default(),
        };
        Doc {
            root,
            had_leading_blank,
            saw_carriage_return,
            yaml_version,
            tag_handles,
            explicit_start,
            explicit_end,
            footer_comments,
            no_content: false,
            origin,
        }
    }

    /// A document for a stream that holds no document, keeping its comments.
    pub(crate) fn empty_with_comments(
        comments: Vec<String>,
        saw_carriage_return: bool,
        source: Option<(Arc<str>, (usize, usize))>,
    ) -> Self {
        let mut d = Doc::from_parts(
            Elem::new(Node::Scalar(Scalar::Null)),
            false,
            saw_carriage_return,
            None,
            Vec::new(),
            false,
            false,
            comments,
            source,
        );
        d.no_content = true;
        d
    }

    /// A document with `root` and no directives or markers, for building
    /// YAML in code.
    ///
    /// ```
    /// use forgo_lib_yaml::{Doc, Elem, MapKey};
    ///
    /// let mut root = Elem::map();
    /// root.node_mut()
    ///     .as_map_mut()
    ///     .unwrap()
    ///     .push((MapKey::from("name"), Elem::string("demo")));
    /// assert_eq!(Doc::new(root).to_string()?, "name: demo\n");
    /// # Ok::<(), forgo_lib_yaml::Error>(())
    /// ```
    pub fn new(root: Elem) -> Self {
        Doc::from_parts(
            root,
            false,
            false,
            None,
            Vec::new(),
            false,
            false,
            Vec::new(),
            None,
        )
    }

    /// This document stands in for a stream that has no documents, only
    /// comments (or nothing): it keeps those comments, as footer comments,
    /// so writing it back preserves them. Its root is null. It stops
    /// being a placeholder once its root is set.
    ///
    /// ```
    /// use forgo_lib_yaml::Doc;
    ///
    /// let docs = Doc::from_stream("# nothing here yet\n")?;
    /// assert!(docs[0].is_placeholder());
    /// assert_eq!(Doc::to_stream_string(&docs)?, "# nothing here yet\n");
    /// # Ok::<(), forgo_lib_yaml::Error>(())
    /// ```
    pub fn is_placeholder(&self) -> bool {
        self.no_content && self.root == Elem::new(Node::Scalar(Scalar::Null))
    }

    /// Forget the source layout: from now on [`Doc::to_string`] writes the
    /// whole document in the standard layout (two-space indentation,
    /// single spaces, no blank lines) instead of keeping the source text
    /// of unchanged parts. Comments, styles and markers are still kept.
    pub fn reformat(&mut self) {
        self.origin = Origin::default();
    }

    /// Comments after the document's last node.
    pub fn footer_comments(&self) -> &[String] {
        &self.footer_comments
    }

    /// Parse the first document of a YAML stream. The whole stream must be
    /// valid. A stream without documents gives a
    /// [placeholder](Doc::is_placeholder) that keeps its comments.
    // Public API; `str::parse::<Doc>()` works through the `FromStr` impl below.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(input: &str) -> Result<Self, Error> {
        crate::compose::compose_first(input, &crate::ParseOptions::default())
    }

    /// Parse every document of a YAML stream. A stream with no documents
    /// (only comments, or `...`) gives one [placeholder](Doc::is_placeholder)
    /// that keeps its text; empty input gives no documents.
    pub fn from_stream(input: &str) -> Result<Vec<Self>, Error> {
        crate::compose::compose_stream(input)
    }

    /// [`Doc::from_str`] with explicit limits.
    pub fn from_str_with(input: &str, options: &crate::ParseOptions) -> Result<Self, Error> {
        crate::compose::compose_first(input, options)
    }

    /// [`Doc::from_stream`] with explicit limits.
    pub fn from_stream_with(
        input: &str,
        options: &crate::ParseOptions,
    ) -> Result<Vec<Self>, Error> {
        crate::compose::compose_stream_with(input, options)
    }

    /// Nesting depth of the tree (collections), computed without recursion.
    fn depth(&self) -> usize {
        let mut max = 0;
        let mut stack = vec![(&self.root, 0usize)];
        while let Some((e, d)) = stack.pop() {
            match &e.node {
                Node::Seq(items) => {
                    max = max.max(d + 1);
                    stack.extend(items.iter().map(|i| (i, d + 1)));
                }
                Node::Map(entries) => {
                    max = max.max(d + 1);
                    for (k, v) in entries {
                        stack.push((v, d + 1));
                        if let MapKey::Complex(ke) = k {
                            stack.push((ke, d + 1));
                        }
                    }
                }
                _ => {}
            }
        }
        max
    }

    /// The emitter recurses over the tree; refuse trees deeper than the
    /// default parse limit allows (for trees built in code).
    fn check_emit_depth(&self) -> Result<(), Error> {
        const MAX: usize = 512;
        if self.depth() > MAX {
            return Err(Error::Emit(format!(
                "collections are nested more than {MAX} levels deep"
            )));
        }
        Ok(())
    }

    /// Decode a byte stream (UTF-8, UTF-16 or UTF-32, detected per YAML
    /// 1.2.2 §5.2) and parse its first document.
    pub fn from_slice(bytes: &[u8]) -> Result<Self, Error> {
        let text = crate::syntax::input::decode(bytes)?;
        Self::from_str(&text)
    }

    /// Write the document as YAML.
    ///
    /// A parsed document keeps its source text: untouched, it is written
    /// back byte for byte, and after edits only the changed entries are
    /// rewritten (see [`Doc::reformat`] to rewrite everything). Documents
    /// built in code, and parts that changed, use the emitter's layout:
    /// directives, `---` and `...` when the document has them, and CRLF
    /// line breaks when the source used them.
    pub fn to_string(&self) -> Result<String, Error> {
        self.check_emit_depth()?;
        if let Some(text) = crate::preserve::doc_text(self) {
            return Ok(text);
        }
        self.render()
    }

    /// The document in the emitter's layout, ignoring its source text.
    pub(crate) fn render(&self) -> Result<String, Error> {
        let mut out = String::new();
        // If the first thing we'll emit is a comment, don't add the preserved
        // leading blank line: the comment should come first.
        let begins_with_comment = !self.root.meta.leading_comments.is_empty()
            || match &self.root.node {
                crate::ast::Node::Map(entries) => entries
                    .first()
                    .is_some_and(|(_, v)| !v.meta.leading_comments.is_empty()),
                crate::ast::Node::Seq(items) => items
                    .first()
                    .is_some_and(|e| !e.meta.leading_comments.is_empty()),
                _ => false,
            };
        if self.had_leading_blank && !begins_with_comment && !self.is_placeholder() {
            out.push('\n');
        }
        self.emit_one_into(&mut out, false)?;
        Ok(self.line_breaks(out))
    }

    /// CR line breaks in the source become CRLF in the output.
    fn line_breaks(&self, out: String) -> String {
        if self.saw_carriage_return {
            out.replace('\n', "\r\n")
        } else {
            out
        }
    }

    fn emit_directives(&self, out: &mut String) -> Result<(), Error> {
        use std::fmt::Write;
        if let Some((maj, min)) = self.yaml_version {
            writeln!(out, "%YAML {maj}.{min}")?;
        }
        for (handle, prefix) in &self.tag_handles {
            writeln!(out, "%TAG {handle} {prefix}")?;
        }
        Ok(())
    }

    fn emit_body_with(&self, out: &mut String, header_written: bool) -> Result<(), Error> {
        if !self.is_placeholder() {
            Emitter::default().emit_root(
                &self.root,
                !self.footer_comments.is_empty(),
                header_written,
                out,
            )?;
            if header_written && !out.ends_with('\n') {
                out.push('\n');
            }
        }
        for c in &self.footer_comments {
            out.push('#');
            if !c.is_empty() {
                out.push(' ');
                out.push_str(c);
            }
            out.push('\n');
        }
        Ok(())
    }

    /// Write a stream of documents, separated by `---` (and `...` where
    /// directives need it). Parsed documents keep their source text as in
    /// [`Doc::to_string`].
    pub fn to_stream_string(docs: &[Self]) -> Result<String, Error> {
        for d in docs {
            d.check_emit_depth()?;
        }
        if let Some(text) = crate::preserve::stream_text(docs) {
            return Ok(text);
        }
        Self::render_stream(docs)
    }

    /// A stream in the emitter's layout, ignoring source text.
    pub(crate) fn render_stream(docs: &[Self]) -> Result<String, Error> {
        let mut out = String::new();
        if let Some(first) = docs.first()
            && first.had_leading_blank
            && !first.is_placeholder()
        {
            out.push('\n');
        }
        for (i, d) in docs.iter().enumerate() {
            if i > 0 && !out.ends_with('\n') {
                out.push('\n');
            }
            // A document must start with `---` unless the previous one
            // ended with `...` (or it is the first).
            let needs_start = i > 0 && !docs[i - 1].explicit_end;
            d.emit_one_into(&mut out, needs_start)?;
        }
        Ok(match docs.first() {
            Some(d) => d.line_breaks(out),
            None => out,
        })
    }

    fn emit_one_into(&self, out: &mut String, needs_start: bool) -> Result<(), Error> {
        // Directives must precede `---`; after another document they also
        // need that document to end with `...`.
        let has_directives = self.yaml_version.is_some() || !self.tag_handles.is_empty();
        if has_directives && needs_start && !out.ends_with("...\n") {
            out.push_str("...\n");
        }
        self.emit_directives(out)?;
        let header_written = false;
        if self.explicit_start || has_directives || needs_start {
            out.push_str("---");
            // A block root's properties and trailing comment (or an empty
            // root's trailing comment) go on the `---` line.
            if !self.is_placeholder()
                && let Some(h) = crate::emitter::root_header(&self.root)
            {
                out.push(' ');
                out.push_str(&h);
                out.push('\n');
                self.emit_body_with(out, true)?;
                if self.explicit_end {
                    out.push_str("...\n");
                }
                return Ok(());
            }
            // Keep a scalar or flow root on the `---` line, as written.
            let inline = !self.is_placeholder()
                && self.root.meta.leading_comments.is_empty()
                && match &self.root.node {
                    Node::Scalar(_) | Node::Alias(_) => true,
                    Node::Map(m) => m.is_empty() || self.root.meta.is_flow(),
                    Node::Seq(s) => s.is_empty() || self.root.meta.is_flow(),
                };
            if inline {
                out.push(' ');
            } else {
                out.push('\n');
            }
        }
        self.emit_body_with(out, header_written)?;
        if self.explicit_end {
            out.push_str("...\n");
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
        self.origin.touched = true;
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
                        .rfind(|(kk, _)| kk == k)
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
        self.origin.touched = true;
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
                    if let Some((_, v)) = entries.iter_mut().rfind(|(kk, _)| kk == k) {
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
        self.origin.touched = true;
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
                    if let Some((_, v)) = entries.iter_mut().rfind(|(kk, _)| kk == k) {
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

    /// Check that no mapping has two equal keys (YAML 1.2.2 §3.2.1.1).
    /// Keys compare by value: `a`/`"a"` and `1`/`0x1` are duplicates.
    /// Parsing keeps duplicates (lookups use the last one); call this to
    /// enforce the spec's uniqueness rule.
    pub fn check_unique_keys(&self) -> Result<(), Error> {
        fn go(e: &Elem, path: &mut Vec<String>) -> Result<(), Error> {
            match &e.node {
                Node::Map(entries) => {
                    let mut seen = std::collections::HashSet::new();
                    for (k, v) in entries {
                        if !seen.insert(k.canonical()) {
                            let at = if path.is_empty() {
                                String::from("the root mapping")
                            } else {
                                format!("`{}`", path.join("."))
                            };
                            return Err(Error::Parse(format!(
                                "duplicate mapping key `{k}` in {at}"
                            )));
                        }
                        path.push(k.to_string());
                        go(v, path)?;
                        path.pop();
                    }
                    Ok(())
                }
                Node::Seq(items) => {
                    for (i, it) in items.iter().enumerate() {
                        path.push(i.to_string());
                        go(it, path)?;
                        path.pop();
                    }
                    Ok(())
                }
                _ => Ok(()),
            }
        }
        go(&self.root, &mut Vec::new())
    }

    /// Borrow the root element.
    pub fn root(&self) -> &Elem {
        &self.root
    }

    /// Mutably borrow the root element.
    pub fn root_mut(&mut self) -> &mut Elem {
        self.origin.touched = true;
        &mut self.root
    }
}

impl std::str::FromStr for Doc {
    type Err = Error;

    fn from_str(input: &str) -> Result<Self, Error> {
        crate::compose::compose_first(input, &crate::ParseOptions::default())
    }
}
