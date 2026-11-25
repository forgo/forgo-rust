//! [`Item`]: an owned TOML value, for building documents in code, and the
//! encoder that writes them as TOML.

use crate::datetime::Datetime;
use crate::doc::{Document, Value};
use std::fmt::Write;

/// An owned TOML value.
///
/// Build one in code and [encode](Item::to_toml_string) it, or copy a
/// parsed value out of a [`Document`] with [`Value::to_item`].
///
/// ```
/// use forgo_lib_toml::Item;
///
/// let manifest = Item::table([
///     ("package", Item::table([
///         ("name", Item::from("demo")),
///         ("version", Item::from("0.1.0")),
///     ])),
///     ("dependencies", Item::table([
///         ("forgo_lib_toml", Item::table([("path", Item::from("../forgo_lib_toml"))])),
///     ])),
/// ]);
/// assert_eq!(manifest.to_toml_string().unwrap(), "\
/// [package]
/// name = \"demo\"
/// version = \"0.1.0\"
///
/// [dependencies]
///
/// [dependencies.forgo_lib_toml]
/// path = \"../forgo_lib_toml\"
/// ");
/// ```
#[derive(Clone, Debug, PartialEq)]
pub enum Item {
    /// A string.
    String(String),
    /// A 64-bit integer.
    Integer(i64),
    /// A 64-bit float.
    Float(f64),
    /// `true` or `false`.
    Boolean(bool),
    /// A date and/or time.
    Datetime(Datetime),
    /// An array.
    Array(Vec<Item>),
    /// A table: keys and values, in order.
    Table(Vec<(String, Item)>),
}

impl Item {
    /// A table from key/value pairs.
    pub fn table<K: Into<String>>(entries: impl IntoIterator<Item = (K, Item)>) -> Item {
        Item::Table(entries.into_iter().map(|(k, v)| (k.into(), v)).collect())
    }

    /// An array.
    pub fn array(items: impl IntoIterator<Item = Item>) -> Item {
        Item::Array(items.into_iter().collect())
    }

    /// Encode as a TOML document (this must be a table). The output uses
    /// TOML 1.0 syntax, so any TOML parser reads it.
    pub fn to_toml_string(&self) -> Result<String, EncodeError> {
        let Item::Table(entries) = self else {
            return Err(EncodeError("only a table can be a TOML document"));
        };
        let mut out = String::new();
        let mut path = Vec::new();
        table(&mut out, &mut path, entries, false);
        Ok(out)
    }
}

impl std::fmt::Display for Item {
    /// The value as TOML writes it inline: `"text"`, `1.5`,
    /// `[1, 2]`, `{ a = 1 }` (TOML 1.0 syntax).
    ///
    /// ```
    /// use forgo_lib_toml::Item;
    ///
    /// let item = Item::table([("tags", Item::array([Item::from("a"), Item::from("b\"c")]))]);
    /// assert_eq!(item.to_string(), r#"{ tags = ["a", "b\"c"] }"#);
    /// ```
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut out = String::new();
        inline(&mut out, self);
        f.write_str(&out)
    }
}

/// Why an [`Item`] can't be encoded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EncodeError(&'static str);

impl std::fmt::Display for EncodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}

impl std::error::Error for EncodeError {}

macro_rules! item_from {
    ($($t:ty => $v:ident),* $(,)?) => {$(
        impl From<$t> for Item {
            fn from(x: $t) -> Item {
                Item::$v(x.into())
            }
        }
    )*};
}

item_from!(&str => String, String => String, i64 => Integer, i32 => Integer, u32 => Integer, f64 => Float, bool => Boolean, Datetime => Datetime);

impl Value<'_> {
    /// An owned copy of the value.
    pub fn to_item(&self) -> Item {
        match self {
            Value::String(s) => Item::String((*s).to_string()),
            Value::Integer(i) => Item::Integer(*i),
            Value::Float(f) => Item::Float(*f),
            Value::Boolean(b) => Item::Boolean(*b),
            Value::Datetime(d) => Item::Datetime(*d),
            Value::Array(a) => Item::Array(a.iter().map(|v| v.to_item()).collect()),
            Value::Table(t) => Item::Table(
                t.iter()
                    .map(|(k, v)| (k.to_string(), v.to_item()))
                    .collect(),
            ),
        }
    }
}

impl Document {
    /// An owned copy of the whole document (its root table).
    pub fn to_item(&self) -> Item {
        Value::Table(self.root()).to_item()
    }
}

/// A key: bare when it can be, else a basic string.
pub(crate) fn key(out: &mut String, k: &str) {
    if !k.is_empty()
        && k.bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
    {
        out.push_str(k);
    } else {
        string(out, k);
    }
}

/// A basic string with the escapes TOML 1.0 has.
pub(crate) fn string(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{8}' => out.push_str("\\b"),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\u{c}' => out.push_str("\\f"),
            '\r' => out.push_str("\\r"),
            c if (c as u32) < 0x20 || c == '\u{7f}' => {
                let _ = write!(out, "\\u{:04X}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

fn float(out: &mut String, f: f64) {
    if f.is_nan() {
        out.push_str(if f.is_sign_negative() { "-nan" } else { "nan" });
    } else if f.is_infinite() {
        out.push_str(if f < 0.0 { "-inf" } else { "inf" });
    } else {
        // `{:?}` is the shortest text that reads back exactly, and keeps
        // a `.0` or an exponent, so it stays a float.
        let _ = write!(out, "{f:?}");
    }
}

fn is_table(v: &Item) -> bool {
    matches!(v, Item::Table(_))
}

/// An array written as `[[key]]` headers: non-empty, all tables.
fn is_array_of_tables(v: &Item) -> bool {
    matches!(v, Item::Array(items) if !items.is_empty() && items.iter().all(is_table))
}

/// A value on one line (inline tables and arrays).
pub(crate) fn inline(out: &mut String, v: &Item) {
    match v {
        Item::String(s) => string(out, s),
        Item::Integer(i) => {
            let _ = write!(out, "{i}");
        }
        Item::Float(f) => float(out, *f),
        Item::Boolean(b) => out.push_str(if *b { "true" } else { "false" }),
        Item::Datetime(d) => {
            let _ = write!(out, "{d}");
        }
        Item::Array(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                inline(out, item);
            }
            out.push(']');
        }
        Item::Table(entries) => {
            if entries.is_empty() {
                out.push_str("{}");
                return;
            }
            out.push_str("{ ");
            for (i, (k, v)) in entries.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                key(out, k);
                out.push_str(" = ");
                inline(out, v);
            }
            out.push_str(" }");
        }
    }
}

fn header(out: &mut String, path: &[&str], array: bool) {
    if !out.is_empty() {
        out.push('\n');
    }
    out.push_str(if array { "[[" } else { "[" });
    for (i, k) in path.iter().enumerate() {
        if i > 0 {
            out.push('.');
        }
        key(out, k);
    }
    out.push_str(if array { "]]\n" } else { "]\n" });
}

/// A table's keys and values, then its sub-tables and arrays of tables
/// under headers.
fn table<'a>(
    out: &mut String,
    path: &mut Vec<&'a str>,
    entries: &'a [(String, Item)],
    _element: bool,
) {
    for (k, v) in entries {
        if is_table(v) || is_array_of_tables(v) {
            continue;
        }
        key(out, k);
        out.push_str(" = ");
        inline(out, v);
        out.push('\n');
    }
    // Sub-tables and arrays of tables, in their own order.
    for (k, v) in entries {
        match v {
            Item::Table(sub) => {
                path.push(k);
                header(out, path, false);
                table(out, path, sub, false);
                path.pop();
            }
            Item::Array(items) if is_array_of_tables(v) => {
                path.push(k);
                for item in items {
                    let Item::Table(sub) = item else {
                        unreachable!()
                    };
                    header(out, path, true);
                    table(out, path, sub, true);
                }
                path.pop();
            }
            _ => {}
        }
    }
}
