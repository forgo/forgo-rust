# forgo_lib_yaml API Design Plan

## Overview

This document outlines the complete API surface for `forgo_lib_yaml`, designed to be:
- **Ergonomic**: Rust-idiomatic with intuitive naming
- **Layered**: Simple API for common cases, powerful API for advanced use
- **Round-trip preserving**: Maintains comments, formatting, and structure
- **Zero-dependency**: Clean, auditable codebase

## Design Philosophy

### Unique Value Proposition
- ✅ **Zero dependencies** (unlike serde_yaml)
- ✅ **Round-trip preservation** (unlike serde_yaml, like ruamel.yaml)
- ✅ **Editor-focused** with path wildcards (unique!)
- ✅ **Simple, auditable** (unlike bloated alternatives)
- 🎯 **Rust-idiomatic ergonomics** (what this API design achieves)

**Positioning:** "The ruamel.yaml for Rust" - round-trip preserving, editor-friendly, with clean Rust idioms.

---

## 1. Layered API Design

### Layer 1: Simple "Just Works" API

For users who just want to parse/stringify YAML:

```rust
/// Parse a single YAML document from a string
pub fn parse(input: &str) -> Result<Doc, Error>

/// Serialize a document to a YAML string
pub fn stringify(doc: &Doc) -> Result<String, Error>

/// Parse multiple YAML documents from a stream (separated by --- or ...)
pub fn parse_all(input: &str) -> Result<Vec<Doc>, Error>
```

### Layer 2: Power User API

Detailed in sections below. Includes:
- Document construction & I/O
- Direct AST access
- Path-based traversal (visitor pattern with wildcards)
- High-level mutation helpers
- Node construction helpers
- Debug utilities

---

## 2. Document Construction & I/O

```rust
impl Doc {
    // === Parsing (multiple input sources) ===

    /// Parse a single YAML document from a string
    pub fn from_str(input: &str) -> Result<Self, Error>

    /// Parse a single YAML document from UTF-8 bytes
    pub fn from_slice(bytes: &[u8]) -> Result<Self, Error>

    /// Parse a single YAML document from a reader
    pub fn from_reader<R: std::io::Read>(reader: R) -> Result<Self, Error>

    /// Parse multiple YAML documents from a stream
    pub fn from_stream(input: &str) -> Result<Vec<Self>, Error>

    // === File I/O ===

    /// Read and parse YAML from a file
    pub fn read_file(path: &Path) -> Result<Self, Error>

    /// Write document to a file (overwrite)
    pub fn write_file(&self, path: &Path) -> Result<(), Error>

    // === Serialization ===

    /// Serialize document to a YAML string
    pub fn to_string(&self) -> Result<String, Error>

    /// Serialize multiple documents to a stream string
    pub fn to_stream_string(docs: &[Self]) -> Result<String, Error>

    /// Write document to a writer
    pub fn to_writer<W: std::io::Write>(&self, writer: W) -> Result<(), Error>
}
```

---

## 3. Direct AST Access

```rust
impl Doc {
    /// Get immutable reference to root element
    pub fn root(&self) -> &Elem

    /// Get mutable reference to root element for direct manipulation
    pub fn root_mut(&mut self) -> &mut Elem

    /// Access YAML version directive (e.g., Some((1, 2)) for %YAML 1.2)
    pub fn yaml_version(&self) -> Option<(u8, u8)>

    /// Access TAG directives as (handle, prefix) pairs
    pub fn tag_handles(&self) -> &[(String, String)]

    /// Check if document started with explicit ---
    pub fn explicit_start(&self) -> bool

    /// Check if document ended with explicit ...
    pub fn explicit_end(&self) -> bool
}
```

---

## 4. Path-Based Traversal (Visitor Pattern with Wildcards)

**Path Syntax:**
- `["jobs", "build", "steps"]` - exact path
- `["jobs", "*", "steps"]` - wildcard matches all jobs' steps
- `["*"]` - matches all top-level keys

### Path Representation

```rust
/// A segment in a path: either a key or a wildcard
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

/// Trait for types that can be converted into a path
pub trait IntoPath {
    fn into_path(self) -> Vec<Seg>;
}

// Implementations:
// - &[Seg]
// - Vec<Seg>
// - &[&str; N]  (array literals like &["jobs", "*"])
// - &[&str]
// - Vec<&str>
```

### Visitor Methods

```rust
impl Doc {
    /// Visit all sequences matching path pattern, apply function to each.
    /// Returns true if any sequences were found and modified.
    ///
    /// # Example
    /// ```
    /// doc.visit_sequences_mut(&["jobs", "*", "steps"], |seq| {
    ///     seq.push(Elem::string("new step"));
    ///     true
    /// })
    /// ```
    pub fn visit_sequences_mut<P, F>(&mut self, path: P, f: F) -> bool
    where
        P: IntoPath,
        F: FnMut(&mut Vec<Elem>) -> bool

    /// Visit all maps matching path pattern, apply function to each.
    /// Returns true if any maps were found and modified.
    pub fn visit_maps_mut<P, F>(&mut self, path: P, f: F) -> bool
    where
        P: IntoPath,
        F: FnMut(&mut Vec<(String, Elem)>) -> bool

    /// Visit all elements matching path pattern, apply function to each.
    /// Returns true if any elements were found and modified.
    /// Most flexible - works with any node type.
    pub fn visit_values_mut<P, F>(&mut self, path: P, f: F) -> bool
    where
        P: IntoPath,
        F: FnMut(&mut Elem) -> bool
}
```

**What visitor methods do:** They traverse the document tree following the path, including wildcards. For each matching node, they call your function. The wildcard `*` matches all keys in a map or all items in a sequence.

---

## 5. High-Level Mutation Helpers

Simpler API for common operations (vs. verbose visitor pattern):

```rust
impl Doc {
    // === Direct Access ===

    /// Get element at exact path (no wildcards)
    pub fn get_at<P: IntoPath>(&self, path: P) -> Option<&Elem>

    /// Get mutable element at exact path (no wildcards)
    pub fn get_at_mut<P: IntoPath>(&mut self, path: P) -> Option<&mut Elem>

    // === Type-Specific Getters ===

    /// Get string value at path (None if not found or wrong type)
    pub fn get_str_at<P: IntoPath>(&self, path: P) -> Option<&str>

    /// Get boolean value at path
    pub fn get_bool_at<P: IntoPath>(&self, path: P) -> Option<bool>

    /// Get integer value at path
    pub fn get_i64_at<P: IntoPath>(&self, path: P) -> Option<i64>

    /// Get float value at path
    pub fn get_f64_at<P: IntoPath>(&self, path: P) -> Option<f64>

    /// Get map at path
    pub fn get_map_at<P: IntoPath>(&self, path: P) -> Option<&Vec<(String, Elem)>>

    /// Get sequence at path
    pub fn get_seq_at<P: IntoPath>(&self, path: P) -> Option<&Vec<Elem>>

    // === Mutation ===

    /// Set value at path, creating intermediate maps as needed
    /// Errors if path crosses through non-map nodes
    pub fn set<P: IntoPath>(&mut self, path: P, value: Elem) -> Result<(), Error>

    /// Set value only if path exists (don't create intermediate nodes)
    /// Returns true if value was set
    pub fn set_existing<P: IntoPath>(&mut self, path: P, value: Elem) -> Result<bool, Error>

    /// Remove element at path, returns the removed element if found
    pub fn delete<P: IntoPath>(&mut self, path: P) -> Option<Elem>

    // === Sequence Operations ===

    /// Append to sequence at path
    /// Errors if path doesn't point to a sequence
    pub fn push_at<P: IntoPath>(&mut self, path: P, value: Elem) -> Result<(), Error>

    /// Insert into sequence at path and index
    pub fn insert_at<P: IntoPath>(&mut self, path: P, index: usize, value: Elem) -> Result<(), Error>

    /// Prepend to sequence at path
    pub fn unshift_at<P: IntoPath>(&mut self, path: P, value: Elem) -> Result<(), Error>

    // === Map Operations ===

    /// Insert key-value into map at path
    /// Returns the old value if key already existed
    pub fn insert_into<P: IntoPath>(&mut self, path: P, key: String, value: Elem) -> Result<Option<Elem>, Error>

    /// Remove key from map at path
    pub fn remove_from<P: IntoPath>(&mut self, path: P, key: &str) -> Result<Option<Elem>, Error>

    /// Merge maps: combine map at path with provided map
    /// Existing keys are overwritten by new values
    pub fn merge_at<P: IntoPath>(&mut self, path: P, other: Vec<(String, Elem)>) -> Result<(), Error>

    /// Rename key in map at path
    /// Returns true if key was found and renamed
    pub fn rename_key<P: IntoPath>(&mut self, path: P, old_key: &str, new_key: String) -> Result<bool, Error>

    // === Advanced Operations ===

    /// Update value at path using a function
    /// Creates the path if it doesn't exist (with default value)
    pub fn update_or_insert<P: IntoPath, F>(&mut self, path: P, default: Elem, f: F) -> Result<(), Error>
    where F: FnOnce(&mut Elem)

    /// Move element from one path to another
    pub fn move_elem<P1: IntoPath, P2: IntoPath>(&mut self, from: P1, to: P2) -> Result<(), Error>

    /// Copy element from one path to another
    pub fn copy_elem<P1: IntoPath, P2: IntoPath>(&mut self, from: P1, to: P2) -> Result<(), Error>
}
```

### Examples

```rust
// set: create nested structure if needed
doc.set(&["jobs", "build", "runs-on"], Elem::string("ubuntu-latest"))?;

// get typed value
if let Some(version) = doc.get_str_at(&["version"]) {
    println!("Version: {}", version);
}

// push_at: add to array
doc.push_at(&["jobs", "build", "steps"], Elem::string("run: cargo test"))?;

// merge_at: combine maps
doc.merge_at(&["env"], vec![
    ("NODE_ENV".into(), Elem::string("production")),
    ("DEBUG".into(), Elem::boolean(false)),
])?;

// update_or_insert: increment a counter
doc.update_or_insert(&["build", "count"], Elem::number("0"), |elem| {
    if let Some(n) = elem.node().as_i64() {
        *elem = Elem::number((n + 1).to_string());
    }
})?;
```

---

## 6. Node Construction & Type Helpers

### Node Constructors

```rust
impl Node {
    // === Type Constructors ===

    /// Create an empty map
    pub fn map() -> Self

    /// Create an empty sequence
    pub fn seq() -> Self

    /// Create a string scalar
    pub fn string(s: impl Into<String>) -> Self

    /// Create a number scalar
    pub fn number(n: impl ToString) -> Self

    /// Create a boolean scalar
    pub fn boolean(b: bool) -> Self

    /// Create a null scalar
    pub fn null() -> Self
}
```

### Node Type Guards

```rust
impl Node {
    // === Type Guards ===

    /// Check if node is a map
    pub fn is_map(&self) -> bool

    /// Check if node is a sequence
    pub fn is_seq(&self) -> bool

    /// Check if node is a scalar
    pub fn is_scalar(&self) -> bool

    /// Check if node is an alias
    pub fn is_alias(&self) -> bool

    /// Check if node is null
    pub fn is_null(&self) -> bool
}
```

### Node Type Conversions

```rust
impl Node {
    // === Safe Downcasts ===

    /// Get map if node is a map
    pub fn as_map(&self) -> Option<&Vec<(String, Elem)>>

    /// Get mutable map if node is a map
    pub fn as_map_mut(&mut self) -> Option<&mut Vec<(String, Elem)>>

    /// Get sequence if node is a sequence
    pub fn as_seq(&self) -> Option<&Vec<Elem>>

    /// Get mutable sequence if node is a sequence
    pub fn as_seq_mut(&mut self) -> Option<&mut Vec<Elem>>

    /// Get scalar if node is a scalar
    pub fn as_scalar(&self) -> Option<&Scalar>

    /// Get mutable scalar if node is a scalar
    pub fn as_scalar_mut(&mut self) -> Option<&mut Scalar>

    // === Scalar Convenience (delegate to Scalar if node is scalar) ===

    /// Get string value if node is a string scalar
    pub fn as_str(&self) -> Option<&str>

    /// Get boolean value if node is a boolean scalar
    pub fn as_bool(&self) -> Option<bool>

    /// Get integer value if node is a number scalar (parses text)
    pub fn as_i64(&self) -> Option<i64>

    /// Get float value if node is a number scalar (parses text)
    pub fn as_f64(&self) -> Option<f64>

    /// Get unsigned integer value if node is a number scalar (parses text)
    pub fn as_u64(&self) -> Option<u64>
}
```

### Node Map Operations

```rust
impl Node {
    // === Map Operations (work only if node is Map) ===

    /// Get value for key (O(n) lookup)
    pub fn get(&self, key: &str) -> Option<&Elem>

    /// Get mutable value for key
    pub fn get_mut(&mut self, key: &str) -> Option<&mut Elem>

    /// Insert key-value pair, returns old value if key existed
    pub fn insert(&mut self, key: String, value: Elem) -> Result<Option<Elem>, Error>

    /// Remove key, returns value if found
    pub fn remove(&mut self, key: &str) -> Option<Elem>

    /// Check if map contains key
    pub fn contains_key(&self, key: &str) -> bool

    /// Iterate over keys
    pub fn keys(&self) -> Box<dyn Iterator<Item = &str> + '_>

    /// Iterate over values
    pub fn values(&self) -> Box<dyn Iterator<Item = &Elem> + '_>

    /// Iterate over key-value pairs
    pub fn iter(&self) -> Box<dyn Iterator<Item = (&str, &Elem)> + '_>

    /// Iterate mutably over key-value pairs
    pub fn iter_mut(&mut self) -> Box<dyn Iterator<Item = (&str, &mut Elem)> + '_>

    /// Retain only entries matching predicate
    pub fn retain<F>(&mut self, f: F) where F: FnMut(&str, &mut Elem) -> bool

    /// Remove all entries
    pub fn clear(&mut self)

    /// Get number of entries (works for Map or Seq)
    pub fn len(&self) -> usize

    /// Check if map/sequence is empty
    pub fn is_empty(&self) -> bool
}
```

**Note on performance:** Map operations are O(n) because `Node::Map` uses `Vec<(String, Elem)>` to preserve insertion order. This is acceptable for typical YAML files (<50 keys per map). We may optimize with indexmap in the future if needed.

### Node Sequence Operations

```rust
impl Node {
    // === Sequence Operations (work only if node is Seq) ===

    /// Append element to sequence
    pub fn push(&mut self, elem: Elem) -> Result<(), Error>

    /// Remove and return last element
    pub fn pop(&mut self) -> Option<Elem>

    /// Insert element at index
    pub fn insert(&mut self, index: usize, elem: Elem) -> Result<(), Error>

    /// Remove element at index
    pub fn remove(&mut self, index: usize) -> Option<Elem>

    /// Get element at index
    pub fn get(&self, index: usize) -> Option<&Elem>

    /// Get mutable element at index
    pub fn get_mut(&mut self, index: usize) -> Option<&mut Elem>

    /// Iterate over elements
    pub fn iter(&self) -> impl Iterator<Item = &Elem>

    /// Iterate mutably over elements
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Elem>

    /// Retain only elements matching predicate
    pub fn retain<F>(&mut self, f: F) where F: FnMut(&Elem) -> bool

    /// Remove all elements
    pub fn clear(&mut self)

    /// Sort elements by comparator
    pub fn sort_by<F>(&mut self, compare: F) where F: FnMut(&Elem, &Elem) -> Ordering

    /// Remove consecutive duplicate elements
    pub fn dedup(&mut self)

    // len() and is_empty() also work (shared with Map)
}
```

---

## 7. Elem (Element with Metadata)

```rust
impl Elem {
    // === Constructors ===

    /// Create element from node with default metadata
    pub fn new(node: Node) -> Self

    /// Create element with empty map
    pub fn map() -> Self

    /// Create element with empty sequence
    pub fn seq() -> Self

    /// Create element with string scalar
    pub fn string(s: impl Into<String>) -> Self

    /// Create element with number scalar
    pub fn number(n: impl ToString) -> Self

    /// Create element with boolean scalar
    pub fn boolean(b: bool) -> Self

    /// Create element with null scalar
    pub fn null() -> Self

    // === Fluent Metadata Builders ===

    /// Add trailing comment (same line as value)
    pub fn with_comment(mut self, comment: impl Into<String>) -> Self

    /// Add leading comments (full lines before value)
    pub fn with_leading_comments(mut self, comments: Vec<String>) -> Self

    /// Add anchor reference (&name)
    pub fn with_anchor(mut self, anchor: impl Into<String>) -> Self

    /// Prefer block style when emitting
    pub fn prefer_block(mut self) -> Self

    /// Prefer quoted style for string scalars
    pub fn prefer_quoted(mut self) -> Self

    // === Accessors ===

    /// Get immutable reference to underlying node
    pub fn node(&self) -> &Node

    /// Get mutable reference to underlying node
    pub fn node_mut(&mut self) -> &mut Node
}
```

### Example

```rust
let elem = Elem::string("ubuntu-latest")
    .with_comment("Use latest Ubuntu LTS")
    .prefer_quoted();

let job = Elem::map()
    .with_leading_comments(vec!["Build job".into()])
    .with_anchor("build-job");
```

---

## 8. Scalar Type System

```rust
#[derive(Clone, Debug, PartialEq)]
pub enum Scalar {
    Str(String),
    Bool(bool),
    /// Numeric literal; we keep the original text for stable re-emission.
    Num { text: String },
    Null,
}

impl Scalar {
    // === Type Guards ===

    /// Check if scalar is a string
    pub fn is_str(&self) -> bool

    /// Check if scalar is a boolean
    pub fn is_bool(&self) -> bool

    /// Check if scalar is a number
    pub fn is_num(&self) -> bool

    /// Check if scalar is null
    pub fn is_null(&self) -> bool

    // === Type Conversions ===

    /// Get string value if scalar is a string
    pub fn as_str(&self) -> Option<&str>

    /// Get boolean value if scalar is a boolean
    pub fn as_bool(&self) -> Option<bool>

    /// Parse as signed integer if scalar is a number
    pub fn as_i64(&self) -> Option<i64>

    /// Parse as float if scalar is a number
    pub fn as_f64(&self) -> Option<f64>

    /// Parse as unsigned integer if scalar is a number
    pub fn as_u64(&self) -> Option<u64>

    /// Get raw number text without parsing
    pub fn num_text(&self) -> Option<&str>

    /// Convert scalar to string representation
    pub fn to_string(&self) -> String
}
```

**Design note:** Numbers are stored as text (`Num { text: String }`) to preserve exact formatting during round-trips. Parsing happens on-demand via `as_i64()`, `as_f64()`, etc.

---

## 9. Error Types

```rust
#[derive(Debug)]
pub enum Error {
    /// I/O error (file operations)
    Io(std::io::Error),

    /// Parse error with location context
    Parse {
        msg: String,
        line: usize,   // 1-indexed
        col: usize,    // 1-indexed
        input: String, // Short snippet of problematic input
    },

    /// Emit/serialization error
    Emit {
        msg: String,
        context: Option<String>,  // e.g., "while emitting map key 'foo'"
    },

    /// Path not found during traversal
    PathNotFound {
        path: String,  // e.g., "jobs.build.steps"
        reason: String, // e.g., "key 'build' not found in map"
    },

    /// Type mismatch (expected one type, found another)
    TypeMismatch {
        expected: String,  // e.g., "sequence"
        found: String,     // e.g., "scalar"
        context: Option<String>,  // e.g., "at path jobs.build.steps"
    },

    /// Invalid operation for node type
    InvalidOperation {
        operation: String,  // e.g., "push"
        node_type: String,  // e.g., "scalar"
    },

    /// UTF-8 encoding error
    Utf8(std::str::Utf8Error),

    /// Format error (from std::fmt)
    Fmt(std::fmt::Error),
}

impl Error {
    // === Convenience Constructors ===

    pub fn parse(msg: impl Into<String>, line: usize, col: usize) -> Self
    pub fn type_mismatch(expected: &str, found: &str) -> Self
    pub fn path_not_found(path: &str, reason: &str) -> Self
}

impl std::error::Error for Error {}
impl std::fmt::Display for Error { /* ... */ }
impl From<std::io::Error> for Error { /* ... */ }
impl From<std::str::Utf8Error> for Error { /* ... */ }
impl From<std::fmt::Error> for Error { /* ... */ }
```

**TODO:** Add position tracking to lexer/parser if not already present.

---

## 10. Editor Utilities

```rust
/// Normalize a sequence: convert all elements to strings, sort, deduplicate
/// Returns true if modified
pub fn normalize_string_list(seq: &mut Vec<Elem>) -> bool

/// Edit file in place: load, visit sequences at path, normalize, save if changed
/// Returns true if file was modified
pub fn edit_file_in_place<P, F>(path: P, seg_path: &[Seg], updater: F) -> Result<bool, Error>
where
    P: AsRef<Path>,
    F: FnMut(&mut Vec<Elem>) -> bool
```

---

## 11. Debug Utilities

```rust
/// Dump all tokens from input with position information
/// Useful for understanding how the lexer tokenizes input
pub fn debug_token_dump(input: &str) -> String

/// Pretty-print the AST structure (not YAML, but the tree itself)
/// Shows node types, metadata, nesting
pub fn debug_ast_dump(doc: &Doc) -> String

/// Validate document structure and return detailed warnings
/// (e.g., unused anchors, duplicate keys, etc.)
pub fn debug_validate(doc: &Doc) -> Vec<String>

/// Show parse trace: step-by-step what the parser is doing
/// Expensive but invaluable for debugging parser issues
pub fn debug_parse_trace(input: &str) -> String

/// Compare two documents and show structural differences
/// Useful for testing round-trips
pub fn debug_diff(a: &Doc, b: &Doc) -> String
```

**Purpose:**
- `debug_token_dump()` - understand lexer behavior, debug tokenization issues
- `debug_ast_dump()` - visualize document structure, understand nesting
- `debug_validate()` - catch YAML anti-patterns, unused features
- `debug_parse_trace()` - deep debugging of parser state machine
- `debug_diff()` - verify round-trip preservation, test comparisons

**Implementation status:** `debug_token_dump()` already exists. Others to be implemented.

---

## 12. Public API Surface

### What to Export

```rust
// crates/forgo_lib_yaml/src/lib.rs

// Layer 1: Simple API
pub use parse;
pub use stringify;
pub use parse_all;

// Layer 2: Core types
pub use ast::{Doc, Elem, Node, Scalar, Seg, Error};
pub use ast::{Meta, BlockStyle};  // Advanced users

// Layer 2: Editor utilities
pub use editor::{normalize_string_list, edit_file_in_place};

// Layer 2: Debug utilities
pub use debug_token_dump;
pub use debug_ast_dump;
pub use debug_validate;
pub use debug_parse_trace;
pub use debug_diff;

// Advanced: Path trait (used in generic bounds)
pub use ast::IntoPath;
```

### What NOT to Export

Make these `pub(crate)`:
- `Lexer`, `Tok` - internal implementation details
- `Parser` - internal implementation details
- `Emitter` - internal implementation details
- Helper modules: `char_validator`, `flow_validator`, `emitter_helpers`, `parser_helpers`

Users can access lexer functionality via `debug_token_dump()` without exposing internals.

---

## 13. Implementation Priority

### Phase 1: Core Ergonomics (High Priority)
1. ✅ Type guards and basic conversions (`Node`, `Scalar`)
2. ✅ Node construction helpers (`Node::string()`, etc.)
3. ✅ `Elem` builders with fluent API
4. ✅ Top-level `parse()`/`stringify()` convenience functions
5. ✅ `IntoPath` trait for `&[&str]` support

### Phase 2: Enhanced Functionality (Medium Priority)
6. ✅ Enhanced error types with position tracking
7. ✅ `from_slice()`, `from_reader()`, `to_writer()` for I/O variety
8. ✅ Map operations: `get()`, `insert()`, `remove()`, `keys()`, `values()`, `iter()`
9. ✅ Sequence operations: `push()`, `pop()`, `insert()`, `remove()`, `get()`, `iter()`
10. ✅ Basic mutation helpers: `set()`, `delete()`, `push_at()`, `get_at()`

### Phase 3: Advanced Features (Lower Priority)
11. ✅ Type-specific getters: `get_str_at()`, `get_bool_at()`, etc.
12. ✅ Advanced mutation: `merge_at()`, `rename_key()`, `move_elem()`, `copy_elem()`
13. ✅ Debug utilities: `debug_ast_dump()`, `debug_validate()`, `debug_diff()`, `debug_parse_trace()`

### Phase 4: Polish & Examples
14. ✅ Comprehensive documentation with examples
15. ✅ Create real educational examples (replace current debug examples)
16. ✅ Integration tests for all new APIs

---

## 14. Data Structure Considerations

### Current: `Node::Map(Vec<(String, Elem)>)`

**Pros:**
- Preserves insertion order (critical for round-tripping)
- Zero dependencies
- Simple implementation

**Cons:**
- O(n) lookup/insert/remove operations

**Decision:** Keep `Vec<(String, Elem)>` for now. YAML maps are typically small (<50 keys), so O(n) is acceptable. We can optimize later with `indexmap` crate if users have performance issues with large maps. This is an internal implementation detail and won't break the API.

---

## 15. Example Usage Patterns

### Simple Parsing
```rust
use forgo_lib_yaml::parse;

let doc = parse("version: 1.2\nname: myapp")?;
println!("{}", doc.get_str_at(&["name"]).unwrap());
```

### Building Documents
```rust
use forgo_lib_yaml::{Doc, Elem, Node};

let mut doc = Doc::from_str("")?;
doc.set(&["jobs", "build", "runs-on"], Elem::string("ubuntu-latest"))?;
doc.push_at(&["jobs", "build", "steps"],
    Elem::string("run: cargo build").with_comment("Build the project"))?;

println!("{}", doc.to_string()?);
```

### Editing with Wildcards
```rust
// Add a timeout to all jobs
doc.visit_maps_mut(&["jobs", "*"], |job| {
    job.push(("timeout-minutes".into(), Elem::number("30")));
    true
});
```

### Type-Safe Access
```rust
if let Some(version) = doc.get_i64_at(&["version"]) {
    println!("Version: {}", version);
}

if let Some(deps) = doc.get_seq_at(&["dependencies"]) {
    for dep in deps {
        if let Some(name) = dep.node().as_str() {
            println!("Dependency: {}", name);
        }
    }
}
```

### Metadata Preservation
```rust
let elem = Elem::map()
    .with_leading_comments(vec!["This is a build configuration".into()])
    .with_anchor("build-config");

doc.set(&["config"], elem)?;
```

---

## 16. Open Questions & Future Work

### Open Questions
- [ ] Should we provide `IndexMap` as an optional feature flag for performance?
- [ ] Should `debug_parse_trace()` be behind a feature flag (compile-time overhead)?
- [ ] Do we need `async` variants of I/O operations?

### Future Enhancements
- Schema validation support
- Anchor/alias resolution helpers
- Query language (more powerful than path wildcards)
- Streaming parser for very large documents
- Custom scalar type handlers

---

## 17. Migration Notes

### Changes from Current API

**Removed from public API:**
- `Lexer`, `Tok` - use `debug_token_dump()` instead

**Added:**
- Top-level `parse()`, `stringify()`, `parse_all()`
- All type guards, builders, and helpers
- Path-based mutation helpers
- Enhanced error types
- Debug utilities

**Breaking Changes:**
- None for existing users who use `Doc::from_str()` / `Doc::to_string()`
- `Lexer`/`Tok` users need to migrate to debug utilities

---

## Conclusion

This API design provides:
1. **Simplicity** for basic use cases (parse/stringify)
2. **Power** for advanced editing (visitor pattern + mutation helpers)
3. **Ergonomics** through Rust idioms (type guards, builders, fluent API)
4. **Safety** through strong typing and clear error messages

Ready for implementation!
