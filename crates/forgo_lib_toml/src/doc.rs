//! [`Document`]: a parsed TOML document, stored as an arena.
//!
//! Values, tables and arrays live in vectors and refer to each other by
//! index. Strings and keys are byte ranges into the source text, or into
//! a side buffer when escapes made them differ from it. Large tables get
//! a hash index (seeded per process); small ones are searched in order.
//! [`Table`], [`Array`] and [`Value`] are borrowed views.

use crate::datetime::Datetime;
use crate::error::Error;
use std::fmt;
use std::sync::OnceLock;

/// Text in the document: a range of the source, or of the side buffer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Text {
    pub(crate) start: u32,
    pub(crate) end: u32,
    pub(crate) side: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Node {
    String(Text),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    /// An index into [`Document::datetimes`] (kept out of line, so nodes
    /// stay 16 bytes).
    Datetime(u32),
    Array(u32),
    Table(u32),
}

/// How a table came to be, for TOML's rules on defining tables.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    Root,
    /// Created by a header path (`[a.b.c]` creates `a` and `a.b`) and not
    /// yet defined.
    Implicit,
    /// Defined by a `[header]`.
    Header,
    /// Created by dotted keys in the given section (a header's table, an
    /// array element, the root, or an inline table).
    Dotted(u32),
    /// An inline table: closed to any additions.
    Inline,
    /// An element of an array of tables (`[[header]]`).
    Element,
}

/// No entry (the end of a table's list).
pub(crate) const NONE: u32 = u32::MAX;

/// A key and its value, in [`Document::entries`]; each table's entries are
/// linked in the order they were defined.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Entry {
    pub(crate) key: Text,
    pub(crate) node: u32,
    pub(crate) next: u32,
}

#[derive(Clone, Debug)]
pub(crate) struct TableData {
    first: u32,
    last: u32,
    len: u32,
    /// For an element of an array of tables, while parsing: the next
    /// element's node.
    pub(crate) next: u32,
    pub(crate) kind: Kind,
    /// The table's hash index in [`Document::indexes`], once it is large
    /// ([`NONE`] before).
    index: u32,
}

/// An array: items `start..start + len` of [`Document::items`].
///
/// An array of tables (`[[header]]`) grows as headers add elements, so
/// while parsing its elements are a list instead: `start` is the first
/// element's node, `last` the last one's, linked through
/// [`TableData::next`]. [`Document::finish`] moves them to `items`.
#[derive(Clone, Debug)]
pub(crate) struct ArrayData {
    pub(crate) start: u32,
    pub(crate) len: u32,
    /// The last element of an array of tables; [`NONE`] for other arrays.
    pub(crate) last: u32,
}

impl ArrayData {
    pub(crate) fn is_tables(&self) -> bool {
        self.last != NONE
    }
}

/// A parsed TOML document.
///
/// ```
/// use forgo_lib_toml::Document;
///
/// let doc = Document::parse(r#"
/// [package]
/// name = "demo"
/// version = "1.2.3"
///
/// [dependencies]
/// serde = { version = "1", features = ["derive"] }
/// "#)?;
/// assert_eq!(doc.get(&["package", "version"]).unwrap().as_str(), Some("1.2.3"));
/// let serde = doc.get(&["dependencies", "serde"]).unwrap().as_table().unwrap();
/// assert_eq!(serde.get("features").unwrap().as_array().unwrap().len(), 1);
/// # Ok::<(), forgo_lib_toml::Error>(())
/// ```
#[derive(Clone)]
pub struct Document {
    pub(crate) src: Box<str>,
    pub(crate) side: String,
    pub(crate) nodes: Vec<Node>,
    pub(crate) tables: Vec<TableData>,
    pub(crate) arrays: Vec<ArrayData>,
    /// Every table's entries.
    pub(crate) entries: Vec<Entry>,
    /// Every static array's items.
    pub(crate) items: Vec<u32>,
    pub(crate) datetimes: Vec<Datetime>,
    /// Hash indexes of large tables: open addressing over a table's
    /// entries (index in [`Document::entries`] + 1; 0 is empty).
    indexes: Vec<Vec<u32>>,
}

/// The TOML version to parse.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TomlVersion {
    /// TOML 1.0.0 (2021).
    V1_0,
    /// TOML 1.1.0 (2025): newlines and trailing commas in inline tables,
    /// `\e` and `\xHH` escapes, optional seconds.
    #[default]
    V1_1,
}

/// Parsing options.
///
/// ```
/// use forgo_lib_toml::{Document, ParseOptions, TomlVersion};
///
/// let v1_0 = ParseOptions::default().with_version(TomlVersion::V1_0);
/// assert!(Document::parse_with("t = 07:32", &v1_0).is_err()); // seconds required in 1.0
/// assert!(Document::parse("t = 07:32").is_ok());
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct ParseOptions {
    /// The TOML version (default 1.1).
    pub version: TomlVersion,
    /// The deepest nesting of arrays and inline tables (default 128).
    pub max_depth: usize,
}

impl Default for ParseOptions {
    fn default() -> ParseOptions {
        ParseOptions {
            version: TomlVersion::V1_1,
            max_depth: 128,
        }
    }
}

impl ParseOptions {
    /// These options with `version`.
    pub fn with_version(mut self, version: TomlVersion) -> ParseOptions {
        self.version = version;
        self
    }

    /// These options with `max_depth`.
    pub fn with_max_depth(mut self, max_depth: usize) -> ParseOptions {
        self.max_depth = max_depth;
        self
    }
}

/// Tables with more entries than this get a hash index.
const INDEX_FROM: usize = 12;

fn seed() -> u64 {
    static SEED: OnceLock<u64> = OnceLock::new();
    *SEED.get_or_init(|| {
        use std::hash::{BuildHasher, Hasher};
        let mut h = std::collections::hash_map::RandomState::new().build_hasher();
        h.write_u64(0x5eed);
        h.finish() | 1
    })
}

fn hash(s: &[u8]) -> u64 {
    let mut h = seed();
    let mut chunks = s.chunks_exact(8);
    for c in &mut chunks {
        let w = u64::from_le_bytes(c.try_into().expect("8 bytes"));
        h = (h.rotate_left(23) ^ w).wrapping_mul(0x9e37_79b9_7f4a_7c15);
    }
    let mut last = [0u8; 8];
    last[..chunks.remainder().len()].copy_from_slice(chunks.remainder());
    h = (h.rotate_left(23) ^ u64::from_le_bytes(last) ^ s.len() as u64)
        .wrapping_mul(0x9e37_79b9_7f4a_7c15);
    h ^ (h >> 29)
}

/// Equal keys. Keys are short, so a loop beats calling `memcmp`.
#[inline(always)]
fn same(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    for i in 0..a.len() {
        if a[i] != b[i] {
            return false;
        }
    }
    true
}

impl TableData {
    pub(crate) fn new(kind: Kind) -> TableData {
        TableData {
            first: NONE,
            last: NONE,
            len: 0,
            next: NONE,
            kind,
            index: NONE,
        }
    }
}

impl Document {
    pub(crate) fn empty(src: &str) -> Document {
        // Room for what typical TOML holds per byte (manifests, lock
        // files, configuration), so most documents never regrow.
        let per = |bytes: usize, min: usize| (src.len() / bytes).clamp(min, 1 << 20);
        let mut nodes = Vec::with_capacity(per(16, 64));
        nodes.push(Node::Table(0));
        let mut tables = Vec::with_capacity(per(64, 8));
        tables.push(TableData::new(Kind::Root));
        Document {
            src: Box::from(""),
            side: String::new(),
            nodes,
            tables,
            arrays: Vec::with_capacity(per(128, 8)),
            entries: Vec::with_capacity(per(20, 32)),
            items: Vec::with_capacity(per(64, 16)),
            datetimes: Vec::new(),
            indexes: Vec::new(),
        }
    }

    /// After parsing: move the elements of arrays of tables to `items`.
    pub(crate) fn finish(&mut self) {
        for a in &mut self.arrays {
            if a.last == NONE {
                continue;
            }
            let start = self.items.len() as u32;
            let mut node = a.start;
            while node != NONE {
                self.items.push(node);
                let Node::Table(id) = self.nodes[node as usize] else {
                    unreachable!("arrays of tables hold tables")
                };
                node = self.tables[id as usize].next;
            }
            a.start = start;
        }
    }

    /// Parse TOML 1.1 text.
    pub fn parse(text: &str) -> Result<Document, Error> {
        crate::parse::parse(text, &ParseOptions::default())
    }

    /// Parse with options (the TOML version, the nesting limit).
    pub fn parse_with(text: &str, options: &ParseOptions) -> Result<Document, Error> {
        crate::parse::parse(text, options)
    }

    /// Parse bytes: UTF-8, with or without a byte order mark (one; the
    /// parser skips it).
    pub fn from_slice(bytes: &[u8], options: &ParseOptions) -> Result<Document, Error> {
        match std::str::from_utf8(bytes) {
            Ok(text) => crate::parse::parse(text, options),
            Err(e) => {
                let valid = std::str::from_utf8(&bytes[..e.valid_up_to()]).expect("valid prefix");
                Err(Error::at(
                    valid,
                    valid.len(),
                    crate::ErrorKind::Encoding,
                    "invalid UTF-8",
                ))
            }
        }
    }

    pub(crate) fn text(&self, t: Text) -> &str {
        let range = t.start as usize..t.end as usize;
        if t.side {
            &self.side[range]
        } else {
            &self.src[range]
        }
    }

    /// The root table.
    pub fn root(&self) -> Table<'_> {
        Table { doc: self, id: 0 }
    }

    /// The value at a path of keys from the root, through tables (and
    /// not into arrays).
    pub fn get(&self, path: &[&str]) -> Option<Value<'_>> {
        let mut v = Value::Table(self.root());
        for key in path {
            v = v.as_table()?.get(key)?;
        }
        Some(v)
    }

    /// The source text the document was parsed from.
    pub fn source(&self) -> &str {
        &self.src
    }

    pub(crate) fn push_node(&mut self, n: Node) -> u32 {
        self.nodes.push(n);
        (self.nodes.len() - 1) as u32
    }

    pub(crate) fn new_table(&mut self, kind: Kind) -> (u32, u32) {
        self.tables.push(TableData::new(kind));
        let id = (self.tables.len() - 1) as u32;
        (id, self.push_node(Node::Table(id)))
    }

    /// The node for `key` in table `t`.
    #[inline]
    pub(crate) fn find(&self, src: &str, t: u32, key: &[u8]) -> Option<u32> {
        let table = &self.tables[t as usize];
        if table.index == NONE {
            let mut e = table.first;
            while e != NONE {
                let entry = &self.entries[e as usize];
                if (entry.key.end - entry.key.start) as usize == key.len()
                    && same(self.key_bytes(src, entry.key), key)
                {
                    return Some(entry.node);
                }
                e = entry.next;
            }
            return None;
        }
        let index = &self.indexes[table.index as usize];
        let mask = index.len() - 1;
        let mut slot = hash(key) as usize & mask;
        loop {
            match index[slot] {
                0 => return None,
                n => {
                    let e = &self.entries[n as usize - 1];
                    if same(self.key_bytes(src, e.key), key) {
                        return Some(e.node);
                    }
                }
            }
            slot = (slot + 1) & mask;
        }
    }

    /// The node for the key `k` (text of this document) in table `t`.
    #[inline]
    pub(crate) fn find_key(&self, src: &str, t: u32, k: Text) -> Option<u32> {
        self.find(src, t, self.key_bytes(src, k))
    }

    /// Add `key` (not already in `t`) for `node`.
    pub(crate) fn insert(&mut self, src: &str, t: u32, key: Text, node: u32) {
        let idx = self.entries.len() as u32;
        self.entries.push(Entry {
            key,
            node,
            next: NONE,
        });
        let table = &mut self.tables[t as usize];
        let last = table.last;
        if last == NONE {
            table.first = idx;
        }
        table.last = idx;
        table.len += 1;
        let len = table.len as usize;
        let index = table.index;
        if last != NONE {
            self.entries[last as usize].next = idx;
        }
        if len <= INDEX_FROM {
            return;
        }
        if index == NONE || 2 * len > self.indexes[index as usize].len() {
            self.rebuild_index(src, t);
        } else {
            let h = hash(self.key_bytes(src, key));
            let index = &mut self.indexes[index as usize];
            let mask = index.len() - 1;
            let mut slot = h as usize & mask;
            while index[slot] != 0 {
                slot = (slot + 1) & mask;
            }
            index[slot] = idx + 1;
        }
    }

    #[inline]
    pub(crate) fn key_bytes<'s>(&'s self, src: &'s str, k: Text) -> &'s [u8] {
        let range = k.start as usize..k.end as usize;
        if k.side {
            &self.side.as_bytes()[range]
        } else {
            &src.as_bytes()[range]
        }
    }

    fn rebuild_index(&mut self, src: &str, t: u32) {
        let n = self.tables[t as usize].len as usize;
        let cap = (4 * n).next_power_of_two();
        let mut index = vec![0u32; cap];
        let mut e = self.tables[t as usize].first;
        while e != NONE {
            let entry = self.entries[e as usize];
            let mut slot = hash(self.key_bytes(src, entry.key)) as usize & (cap - 1);
            while index[slot] != 0 {
                slot = (slot + 1) & (cap - 1);
            }
            index[slot] = e + 1;
            e = entry.next;
        }
        match self.tables[t as usize].index {
            NONE => {
                self.tables[t as usize].index = self.indexes.len() as u32;
                self.indexes.push(index);
            }
            i => self.indexes[i as usize] = index,
        }
    }

    /// An array's item nodes.
    pub(crate) fn array_items(&self, a: u32) -> &[u32] {
        let a = &self.arrays[a as usize];
        &self.items[a.start as usize..(a.start + a.len) as usize]
    }

    fn value(&self, node: u32) -> Value<'_> {
        match self.nodes[node as usize] {
            Node::String(t) => Value::String(self.text(t)),
            Node::Integer(i) => Value::Integer(i),
            Node::Float(f) => Value::Float(f),
            Node::Boolean(b) => Value::Boolean(b),
            Node::Datetime(d) => Value::Datetime(self.datetimes[d as usize]),
            Node::Array(id) => Value::Array(Array { doc: self, id }),
            Node::Table(id) => Value::Table(Table { doc: self, id }),
        }
    }
}

impl fmt::Debug for Document {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.root(), f)
    }
}

/// A value in a [`Document`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Value<'a> {
    /// A string (escapes decoded).
    String(&'a str),
    /// A 64-bit integer.
    Integer(i64),
    /// A 64-bit float.
    Float(f64),
    /// `true` or `false`.
    Boolean(bool),
    /// A date and/or time.
    Datetime(Datetime),
    /// An array.
    Array(Array<'a>),
    /// A table (a `[header]`, an inline table, or one made by dotted
    /// keys).
    Table(Table<'a>),
}

impl<'a> Value<'a> {
    /// The string, if this is one.
    pub fn as_str(&self) -> Option<&'a str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    /// The integer, if this is one.
    pub fn as_integer(&self) -> Option<i64> {
        match self {
            Value::Integer(i) => Some(*i),
            _ => None,
        }
    }

    /// The float, if this is one.
    pub fn as_float(&self) -> Option<f64> {
        match self {
            Value::Float(f) => Some(*f),
            _ => None,
        }
    }

    /// The boolean, if this is one.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Boolean(b) => Some(*b),
            _ => None,
        }
    }

    /// The date-time, if this is one.
    pub fn as_datetime(&self) -> Option<Datetime> {
        match self {
            Value::Datetime(d) => Some(*d),
            _ => None,
        }
    }

    /// The array, if this is one.
    pub fn as_array(&self) -> Option<Array<'a>> {
        match self {
            Value::Array(a) => Some(*a),
            _ => None,
        }
    }

    /// The table, if this is one.
    pub fn as_table(&self) -> Option<Table<'a>> {
        match self {
            Value::Table(t) => Some(*t),
            _ => None,
        }
    }

    /// The type, as TOML names it: `string`, `integer`, `float`,
    /// `boolean`, `datetime`, `array` or `table`.
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::String(_) => "string",
            Value::Integer(_) => "integer",
            Value::Float(_) => "float",
            Value::Boolean(_) => "boolean",
            Value::Datetime(_) => "datetime",
            Value::Array(_) => "array",
            Value::Table(_) => "table",
        }
    }
}

/// A table in a [`Document`]: its keys in the order they were defined.
#[derive(Clone, Copy)]
pub struct Table<'a> {
    doc: &'a Document,
    id: u32,
}

impl<'a> Table<'a> {
    fn data(&self) -> &'a TableData {
        &self.doc.tables[self.id as usize]
    }

    /// The number of keys.
    pub fn len(&self) -> usize {
        self.data().len as usize
    }

    /// Whether there are no keys.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The value for `key`.
    pub fn get(&self, key: &str) -> Option<Value<'a>> {
        self.doc
            .find(&self.doc.src, self.id, key.as_bytes())
            .map(|n| self.doc.value(n))
    }

    /// Whether `key` is defined.
    pub fn contains_key(&self, key: &str) -> bool {
        self.get(key).is_some()
    }

    /// The keys and values, in the order they were defined.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = (&'a str, Value<'a>)> + 'a {
        TableIter {
            doc: self.doc,
            next: self.data().first,
            left: self.len(),
        }
    }

    /// The keys, in the order they were defined.
    pub fn keys(&self) -> impl ExactSizeIterator<Item = &'a str> + 'a {
        self.iter().map(|(k, _)| k)
    }

    /// Whether the table was written inline (`{ … }`).
    pub fn is_inline(&self) -> bool {
        self.data().kind == Kind::Inline
    }
}

struct TableIter<'a> {
    doc: &'a Document,
    next: u32,
    left: usize,
}

impl<'a> Iterator for TableIter<'a> {
    type Item = (&'a str, Value<'a>);

    fn next(&mut self) -> Option<(&'a str, Value<'a>)> {
        if self.next == NONE {
            return None;
        }
        let e = &self.doc.entries[self.next as usize];
        self.next = e.next;
        self.left -= 1;
        Some((self.doc.text(e.key), self.doc.value(e.node)))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.left, Some(self.left))
    }
}

impl ExactSizeIterator for TableIter<'_> {}

impl PartialEq for Table<'_> {
    /// Equal keys with equal values, in any order.
    fn eq(&self, other: &Table<'_>) -> bool {
        self.len() == other.len() && self.iter().all(|(k, v)| other.get(k) == Some(v))
    }
}

impl fmt::Debug for Table<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}

/// An array in a [`Document`].
#[derive(Clone, Copy)]
pub struct Array<'a> {
    doc: &'a Document,
    id: u32,
}

impl<'a> Array<'a> {
    fn items(&self) -> &'a [u32] {
        self.doc.array_items(self.id)
    }

    /// The number of items.
    pub fn len(&self) -> usize {
        self.items().len()
    }

    /// Whether there are no items.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The item at `index`.
    pub fn get(&self, index: usize) -> Option<Value<'a>> {
        self.items().get(index).map(|&n| self.doc.value(n))
    }

    /// The items, in order.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = Value<'a>> + 'a {
        let doc = self.doc;
        self.items().iter().map(move |&n| doc.value(n))
    }

    /// Whether this is an array of tables (`[[header]]`).
    pub fn is_array_of_tables(&self) -> bool {
        self.doc.arrays[self.id as usize].is_tables()
    }
}

impl PartialEq for Array<'_> {
    fn eq(&self, other: &Array<'_>) -> bool {
        self.len() == other.len() && self.iter().zip(other.iter()).all(|(a, b)| a == b)
    }
}

impl fmt::Debug for Array<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}
