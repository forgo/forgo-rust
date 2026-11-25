//! Editing a [`Document`] while keeping its text.
//!
//! An edit changes only the text it must: a value, a line, an item of a
//! list, a section. Everything else stays byte for byte: comments, blank
//! lines, indentation, quoting, key order, line endings, a missing final
//! newline. The edited text is then parsed again and compared with the
//! old document: it must hold exactly the change asked for, or the edit
//! fails and the document is left as it was. No edit re-renders the
//! document.

use crate::doc::{Document, Kind, NONE, Node, Spans, Table};
use crate::error::Error;
use crate::item::{self, Item};
use crate::{ParseOptions, Value};
use std::fmt::{self, Write as _};
use std::path::Path;

// ---------------------------------------------------------------------
// Paths
// ---------------------------------------------------------------------

/// One step of a path: a key of a table, or an index of an array.
///
/// An index also selects an element of an array of tables (`[[name]]`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Seg<'a> {
    /// The value of this key in a table.
    Key(&'a str),
    /// The item at this index in an array.
    Index(usize),
}

impl<'a> From<&'a str> for Seg<'a> {
    fn from(key: &'a str) -> Seg<'a> {
        Seg::Key(key)
    }
}

impl<'a> From<&'a String> for Seg<'a> {
    fn from(key: &'a String) -> Seg<'a> {
        Seg::Key(key)
    }
}

impl From<usize> for Seg<'_> {
    fn from(index: usize) -> Self {
        Seg::Index(index)
    }
}

/// A path from the root table: keys (`&["package", "version"]`), or keys
/// and indexes (`&[Seg::Key("package"), Seg::Index(0)]`).
pub trait IntoPath<'a> {
    /// The steps of the path.
    fn into_path(self) -> Vec<Seg<'a>>;
}

impl<'a> IntoPath<'a> for &[&'a str] {
    fn into_path(self) -> Vec<Seg<'a>> {
        self.iter().map(|&k| Seg::Key(k)).collect()
    }
}

impl<'a, const N: usize> IntoPath<'a> for &[&'a str; N] {
    fn into_path(self) -> Vec<Seg<'a>> {
        self.iter().map(|&k| Seg::Key(k)).collect()
    }
}

impl<'a, const N: usize> IntoPath<'a> for [&'a str; N] {
    fn into_path(self) -> Vec<Seg<'a>> {
        self.iter().map(|&k| Seg::Key(k)).collect()
    }
}

impl<'a> IntoPath<'a> for &Vec<&'a str> {
    fn into_path(self) -> Vec<Seg<'a>> {
        self.iter().map(|&k| Seg::Key(k)).collect()
    }
}

impl<'a> IntoPath<'a> for &'a [String] {
    fn into_path(self) -> Vec<Seg<'a>> {
        self.iter().map(|k| Seg::Key(k)).collect()
    }
}

impl<'a> IntoPath<'a> for &[Seg<'a>] {
    fn into_path(self) -> Vec<Seg<'a>> {
        self.to_vec()
    }
}

impl<'a, const N: usize> IntoPath<'a> for &[Seg<'a>; N] {
    fn into_path(self) -> Vec<Seg<'a>> {
        self.to_vec()
    }
}

impl<'a, const N: usize> IntoPath<'a> for [Seg<'a>; N] {
    fn into_path(self) -> Vec<Seg<'a>> {
        self.to_vec()
    }
}

impl<'a> IntoPath<'a> for Vec<Seg<'a>> {
    fn into_path(self) -> Vec<Seg<'a>> {
        self
    }
}

/// A path as TOML writes it: `package.metadata."a key"[2]`.
fn show(path: &[Seg]) -> String {
    let mut out = String::new();
    for seg in path {
        match seg {
            Seg::Key(k) => {
                if !out.is_empty() {
                    out.push('.');
                }
                item::key(&mut out, k);
            }
            Seg::Index(i) => {
                let _ = write!(out, "[{i}]");
            }
        }
    }
    if out.is_empty() {
        out.push_str("the root table");
    }
    out
}

// ---------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------

/// Why an edit wasn't made. The document is unchanged.
#[derive(Debug)]
#[non_exhaustive]
pub enum EditError {
    /// The path doesn't lead where the edit needs: a missing item, or a
    /// value of another type on the way.
    Path(String),
    /// The edit can't be made without rewriting the document's layout,
    /// for example replacing a table written as a `[section]` with a
    /// value, or reordering a list whose items share lines.
    Unsupported(String),
    /// The edited text didn't read back as the edit intended. This is a
    /// bug in the editor; nothing was changed.
    Verify(String),
    /// The file isn't valid TOML ([`edit_file_in_place`]).
    Parse(Error),
    /// Reading or writing the file failed ([`edit_file_in_place`]).
    Io(std::io::Error),
}

impl fmt::Display for EditError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EditError::Path(m) | EditError::Unsupported(m) | EditError::Verify(m) => f.write_str(m),
            EditError::Parse(e) => fmt::Display::fmt(e, f),
            EditError::Io(e) => fmt::Display::fmt(e, f),
        }
    }
}

impl std::error::Error for EditError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            EditError::Parse(e) => Some(e),
            EditError::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for EditError {
    fn from(e: std::io::Error) -> EditError {
        EditError::Io(e)
    }
}

// ---------------------------------------------------------------------
// The editing API
// ---------------------------------------------------------------------

impl Document {
    /// The value at `path`: through tables by key and arrays by index.
    ///
    /// ```
    /// use forgo_lib_toml::{Document, Seg};
    ///
    /// let doc = Document::parse("[[bin]]\nname = \"a\"\n[[bin]]\nname = \"b\"\n")?;
    /// let path = [Seg::Key("bin"), Seg::Index(1), Seg::Key("name")];
    /// assert_eq!(doc.get(&path).unwrap().as_str(), Some("b"));
    /// assert_eq!(doc.get(["bin"]).unwrap().as_array().unwrap().len(), 2);
    /// # Ok::<(), forgo_lib_toml::Error>(())
    /// ```
    pub fn get<'p>(&self, path: impl IntoPath<'p>) -> Option<Value<'_>> {
        let mut v = Value::Table(self.root());
        for seg in path.into_path() {
            v = match (seg, v) {
                (Seg::Key(k), Value::Table(t)) => t.get(k)?,
                (Seg::Index(i), Value::Array(a)) => a.get(i)?,
                _ => return None,
            };
        }
        Some(v)
    }

    /// Set the value at `path`, keeping the rest of the text.
    ///
    /// - An existing value is replaced where it is written; the key, the
    ///   spacing and any comment after it stay.
    /// - A new key goes after the last key/value line of its table's
    ///   section, with the same indentation, or into its inline table.
    /// - Missing tables are made with dotted keys in the nearest table
    ///   that exists, except under the root, where a new `[section]` is
    ///   added at the end of the document.
    ///
    /// New values are written in TOML 1.0's syntax, tables inline.
    ///
    /// ```
    /// use forgo_lib_toml::Document;
    ///
    /// let mut doc = Document::parse("\
    /// [package]
    /// name = \"demo\"
    /// version = \"0.1.0\"   # bumped by CI
    /// ")?;
    /// doc.set(["package", "version"], "0.2.0")?;
    /// doc.set(["package", "edition"], "2024")?;
    /// doc.set(["dependencies", "serde"], "1")?;
    /// assert_eq!(doc.to_string(), "\
    /// [package]
    /// name = \"demo\"
    /// version = \"0.2.0\"   # bumped by CI
    /// edition = \"2024\"
    ///
    /// [dependencies]
    /// serde = \"1\"
    /// ");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn set<'p>(
        &mut self,
        path: impl IntoPath<'p>,
        value: impl Into<Item>,
    ) -> Result<(), EditError> {
        let path = path.into_path();
        let value = value.into();
        if path.is_empty() {
            return Err(EditError::Path("can't replace the root table".into()));
        }
        self.ensure_spans()?;
        let w = self.walk(&path)?;
        let change = Ed::new(self).plan_set(&w, &path, &value)?;
        self.apply(change, &path, Expect::Value(&value))
    }

    /// Remove the value at `path` and everything that writes it: its
    /// key/value line (with the comment lines directly above it), its
    /// `[sections]`, or its item in a list. Returns whether it was there.
    ///
    /// Tables left empty that nothing else defines (made by dotted keys,
    /// or only named on the way to a `[section]`) go too, and so does an
    /// array of tables (`[[name]]`) when its last element goes.
    ///
    /// ```
    /// use forgo_lib_toml::Document;
    ///
    /// let mut doc = Document::parse("\
    /// [dependencies]
    /// serde = \"1\"
    /// ## For the CLI.
    /// clap = \"4\"
    ///
    /// [dev-dependencies]
    /// proptest = \"1\"
    /// ")?;
    /// assert!(doc.remove(["dependencies", "clap"])?);
    /// assert!(doc.remove(["dev-dependencies"])?);
    /// assert!(!doc.remove(["build-dependencies"])?);
    /// assert_eq!(doc.to_string(), "[dependencies]\nserde = \"1\"\n");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn remove<'p>(&mut self, path: impl IntoPath<'p>) -> Result<bool, EditError> {
        let path = path.into_path();
        if path.is_empty() {
            return Err(EditError::Path("can't remove the root table".into()));
        }
        self.ensure_spans()?;
        let w = self.walk(&path)?;
        if w.found() < path.len() {
            return Ok(false);
        }
        let gone = self.removal_root(&w, &path);
        let change = Ed::new(self).plan_remove(&w, &path, gone)?;
        self.apply(change, &path[..gone], Expect::Gone)?;
        Ok(true)
    }

    /// Change the array at `path` as a list of items. Items that stay
    /// keep their text, and the comments above them in an array written
    /// one item per line; new items are written in the array's style
    /// (indentation, trailing comma). Returns whether anything changed.
    ///
    /// When the array is missing and `f` adds items, it is created as by
    /// [`set`](Document::set).
    ///
    /// ```
    /// use forgo_lib_toml::{Document, Item};
    ///
    /// let mut doc = Document::parse("\
    /// features = [
    ///     \"std\",  # the default
    ///     \"serde\",
    /// ]
    /// ")?;
    /// doc.edit_array(["features"], |items| {
    ///     items.retain(|i| i != &Item::from("serde"));
    ///     items.push(Item::from("alloc"));
    /// })?;
    /// assert_eq!(doc.to_string(), "\
    /// features = [
    ///     \"std\",  # the default
    ///     \"alloc\",
    /// ]
    /// ");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn edit_array<'p>(
        &mut self,
        path: impl IntoPath<'p>,
        f: impl FnOnce(&mut Vec<Item>),
    ) -> Result<bool, EditError> {
        let path = path.into_path();
        self.ensure_spans()?;
        let w = self.walk(&path)?;
        if w.found() < path.len() {
            let mut items = Vec::new();
            f(&mut items);
            if items.is_empty() {
                return Ok(false);
            }
            self.set(path, Item::Array(items))?;
            return Ok(true);
        }
        let node = w.node();
        let Node::Array(a) = self.nodes[node as usize] else {
            return Err(EditError::Path(format!("`{}` isn't an array", show(&path))));
        };
        if self.arrays[a as usize].is_tables() {
            return Err(EditError::Unsupported(format!(
                "`{}` is an array of tables (`[[…]]` sections); edit its tables one by one",
                show(&path)
            )));
        }
        let old: Vec<Item> = self
            .array_items(a)
            .iter()
            .map(|&n| self.value(n).to_item())
            .collect();
        let mut items = old.clone();
        f(&mut items);
        if items.len() == old.len() && items.iter().zip(&old).all(|(x, y)| item_eq(x, y)) {
            return Ok(false);
        }
        // Items that stay keep their text.
        let mut used = vec![false; old.len()];
        let parts: Vec<Part> = items
            .iter()
            .map(
                |it| match (0..old.len()).find(|&j| !used[j] && item_eq(&old[j], it)) {
                    Some(j) => {
                        used[j] = true;
                        Part::Old(j)
                    }
                    None => Part::New(render(it)),
                },
            )
            .collect();
        let ed = Ed::new(self);
        let list = ed.array_list(node, a);
        let text = ed
            .rebuild(&list, &parts)
            .map_err(|why| unsupported(&path, why))?;
        let change = Change::Value {
            start: list.open,
            end: list.close + 1,
            text,
            slot: w.slot(w.found(), self, &path),
            depth: w.depth(w.found(), self),
            expect: Some(Item::Array(items.clone())),
        };
        self.apply(change, &path, Expect::Items(&items))?;
        Ok(true)
    }

    /// Change an array of strings as a sorted set: `f` edits the strings,
    /// then they are sorted and duplicates dropped, keeping each one's
    /// text and comments as [`edit_array`](Document::edit_array) does.
    /// An error if an item isn't a string. Returns whether anything
    /// changed.
    ///
    /// ```
    /// use forgo_lib_toml::Document;
    ///
    /// let mut doc = Document::parse("\
    /// [workspace]
    /// members = [
    ///     \"crates/b\",
    ///     \"crates/a\",
    /// ]
    /// ")?;
    /// doc.edit_string_set(["workspace", "members"], |m| m.push("crates/c".into()))?;
    /// assert_eq!(doc.to_string(), "\
    /// [workspace]
    /// members = [
    ///     \"crates/a\",
    ///     \"crates/b\",
    ///     \"crates/c\",
    /// ]
    /// ");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn edit_string_set<'p>(
        &mut self,
        path: impl IntoPath<'p>,
        f: impl FnOnce(&mut Vec<String>),
    ) -> Result<bool, EditError> {
        let path = path.into_path();
        let mut not_strings = false;
        let changed = self.edit_array(path.clone(), |items| {
            let Some(mut strings) = items
                .iter()
                .map(|i| match i {
                    Item::String(s) => Some(s.clone()),
                    _ => None,
                })
                .collect::<Option<Vec<String>>>()
            else {
                not_strings = true;
                return;
            };
            f(&mut strings);
            strings.sort();
            strings.dedup();
            *items = strings.into_iter().map(Item::String).collect();
        })?;
        if not_strings {
            return Err(EditError::Path(format!(
                "`{}` holds items that aren't strings",
                show(&path)
            )));
        }
        Ok(changed)
    }

    /// Parse again with positions, once, for edits.
    fn ensure_spans(&mut self) -> Result<(), EditError> {
        if self.spans.is_none() {
            crate::parse::reparse_spans(self).map_err(EditError::Parse)?;
        }
        Ok(())
    }

    /// Make a planned change to the text, and bring the document along.
    ///
    /// A value, or lines added to a section, are parsed where they are,
    /// in their context (which checks them), and the rest of the document
    /// only moves. Anything else is spliced and the whole text parsed
    /// again and compared with the old document ([`commit`]).
    fn apply(&mut self, change: Change, path: &[Seg], expect: Expect) -> Result<(), EditError> {
        let failed = |why: &str| {
            EditError::Verify(format!(
                "editing `{}` {why}; nothing was changed",
                show(path)
            ))
        };
        let (start, end, text) = match &change {
            Change::Reparse(_) => {
                let Change::Reparse(splices) = change else {
                    unreachable!()
                };
                return self.commit(splices, path, expect);
            }
            Change::Value {
                start, end, text, ..
            } => (*start, *end, text.as_str()),
            Change::Lines { at, text, .. } => (*at, *at, text.as_str()),
        };
        let mut new_text = String::with_capacity(self.src.len() + text.len());
        new_text.push_str(&self.src[..start]);
        new_text.push_str(text);
        new_text.push_str(&self.src[end..]);
        let at_end = start == self.src.len() && !self.src.is_empty() && !self.src.ends_with('\n');
        self.shift(start, end, text.len());
        if at_end && let Some(spans) = self.spans.as_deref_mut() {
            // Lines added after a last line without a newline end it: its
            // end moves past that newline.
            let nl = text.len() - text.trim_start_matches(['\r', '\n']).len();
            for t in &mut spans.table {
                for p in [&mut t.head_end, &mut t.end] {
                    if *p as usize == start {
                        *p += nl.min(if text.starts_with("\r\n") { 2 } else { 1 }) as u32;
                    }
                }
            }
        }
        let mark = self.entries.len() as u32;
        // Parse the new text where it is (the document's own text is
        // swapped in after).
        let parsed = match &change {
            Change::Value { depth, .. } => {
                match crate::parse::resume_value(self, &new_text, start, *depth) {
                    Ok((node, i)) if i == start + text.len() => Ok(Some(node)),
                    Ok(_) => Err("wrote a value that doesn't end where it should"),
                    Err(_) => Err("wrote a value that doesn't parse"),
                }
            }
            Change::Lines { section, .. } => {
                let headed = text.trim_start_matches(['\r', '\n']).starts_with('[');
                if !headed && self.section_at(start) != *section {
                    Err("would put the line in another section")
                } else {
                    match crate::parse::resume_lines(
                        self,
                        &new_text,
                        start,
                        *section,
                        start + text.len(),
                    ) {
                        Ok(()) => {
                            self.reorder_since(mark);
                            Ok(None)
                        }
                        Err(_) => Err("wrote lines that don't parse in their section"),
                    }
                }
            }
            Change::Reparse(_) => unreachable!(),
        };
        let old_text = std::mem::replace(&mut self.src, new_text.into_boxed_str());
        let done = parsed.and_then(|node| match (node, &change) {
            (
                Some(node),
                Change::Value {
                    slot, expect: want, ..
                },
            ) => {
                match *slot {
                    Slot::Entry(e) => self.entries[e as usize].node = node,
                    Slot::Item(a, i) => {
                        let at = self.arrays[a as usize].start as usize + i;
                        self.items[at] = node;
                    }
                }
                let want = match (want, &expect) {
                    (Some(item), _) => item,
                    (None, Expect::Value(item)) => *item,
                    (None, _) => unreachable!("a value change says what it must read as"),
                };
                if matches_item(self.value(node), want) {
                    Ok(())
                } else {
                    Err("didn't read back as intended")
                }
            }
            _ => Ok(()),
        });
        // A value checked itself; lines are checked by what the path holds.
        let checked = done.and_then(|()| {
            if !matches!(change, Change::Lines { .. }) {
                return Ok(());
            }
            let mut probe = Some(0u32);
            for seg in path {
                probe = probe.and_then(|n| match (*seg, self.nodes[n as usize]) {
                    (Seg::Key(k), Node::Table(t)) => self.find(&self.src, t, k.as_bytes()),
                    (Seg::Index(i), Node::Array(a)) => self.array_items(a).get(i).copied(),
                    _ => None,
                });
            }
            let ok = match &expect {
                Expect::Value(item) => probe.is_some_and(|n| matches_item(self.value(n), item)),
                Expect::Items(items) => {
                    probe.is_some_and(|n| matches_item(self.value(n), &Item::Array(items.to_vec())))
                }
                Expect::Gone => probe.is_none(),
            };
            if ok {
                Ok(())
            } else {
                Err("didn't read back as intended")
            }
        });
        if let Err(why) = checked {
            // Put the document back as it was.
            *self = crate::parse::parse_spans(&old_text, &self.options)
                .expect("the old text parsed before");
            return Err(failed(why));
        }
        #[cfg(debug_assertions)]
        self.assert_same_as_parse();
        Ok(())
    }

    /// The table whose section holds position `pos` (the root's, before
    /// any header).
    fn section_at(&self, pos: usize) -> u32 {
        let spans = self.spans.as_deref().expect("spans");
        let mut best = (0u32, 0u32);
        for (t, span) in spans.table.iter().enumerate() {
            if span.header != NONE && (span.header as usize) < pos && span.header >= best.1 {
                best = (t as u32, span.header);
            }
        }
        best.0
    }

    /// Check (in debug builds) that an edit made in place left the
    /// document just as parsing its text would.
    #[cfg(debug_assertions)]
    fn assert_same_as_parse(&self) {
        let fresh = crate::parse::parse_spans(&self.src, &self.options)
            .expect("an edit made text that doesn't parse");
        if let Err(why) = same_structure(self, 0, &fresh, 0, &mut String::new()) {
            panic!(
                "an edit left the document out of step with its text: {why}\n--- text\n{}",
                self.src
            );
        }
    }

    /// Apply `splices`, read the text back, and check it holds exactly
    /// the change: then it becomes the document.
    fn commit(
        &mut self,
        splices: Vec<Splice>,
        path: &[Seg],
        expect: Expect,
    ) -> Result<(), EditError> {
        let text = apply(&self.src, splices);
        let new = crate::parse::parse_spans(&text, &self.options).map_err(|e| {
            EditError::Verify(format!(
                "editing `{}` made text that doesn't parse ({e}); nothing was changed",
                show(path)
            ))
        })?;
        if !same_except(self, Some(0), &new, Some(0), path, &expect) {
            return Err(EditError::Verify(format!(
                "editing `{}` didn't read back as intended; nothing was changed",
                show(path)
            )));
        }
        *self = new;
        Ok(())
    }

    /// Follow `path` as far as it exists.
    fn walk(&self, path: &[Seg]) -> Result<Walk, EditError> {
        let mut w = Walk {
            nodes: vec![0],
            entries: Vec::new(),
        };
        for (n, seg) in path.iter().enumerate() {
            let next = match (*seg, self.nodes[w.node() as usize]) {
                (Seg::Key(k), Node::Table(t)) => {
                    match self.find_entry(&self.src, t, k.as_bytes()) {
                        Some(e) => {
                            w.entries.push(e);
                            self.entries[e as usize].node
                        }
                        None => break,
                    }
                }
                (Seg::Index(i), Node::Array(a)) => match self.array_items(a).get(i) {
                    Some(&item) => {
                        w.entries.push(NONE);
                        item
                    }
                    None => break,
                },
                (Seg::Key(_), _) => {
                    return Err(EditError::Path(format!(
                        "`{}` isn't a table",
                        show(&path[..n])
                    )));
                }
                (Seg::Index(_), _) => {
                    return Err(EditError::Path(format!(
                        "`{}` isn't an array",
                        show(&path[..n])
                    )));
                }
            };
            w.nodes.push(next);
        }
        Ok(w)
    }

    /// How much of `path` a removal takes away: the target, and the
    /// tables above it that it would leave empty and that nothing else
    /// defines.
    fn removal_root(&self, w: &Walk, path: &[Seg]) -> usize {
        let mut m = path.len();
        while m > 1 {
            let vanishes = match self.nodes[w.nodes[m - 1] as usize] {
                Node::Table(t) => {
                    (Table { doc: self, id: t }).len() == 1
                        && matches!(
                            self.tables[t as usize].kind,
                            Kind::Dotted(_) | Kind::Implicit
                        )
                }
                // `[[name]]` sections can't write an empty array.
                Node::Array(a) => {
                    self.arrays[a as usize].is_tables() && self.arrays[a as usize].len == 1
                }
                _ => false,
            };
            if !vanishes {
                break;
            }
            m -= 1;
        }
        m
    }
}

impl fmt::Display for Document {
    /// The document's text, with any edits.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.src)
    }
}

/// Read a TOML file, let `f` edit it, and write it back if it changed:
/// to a temporary file next to it, then renamed over it, keeping its
/// permissions. Returns whether the file changed.
///
/// ```no_run
/// use forgo_lib_toml::edit_file_in_place;
///
/// edit_file_in_place("Cargo.toml", |doc| {
///     doc.edit_string_set(["workspace", "members"], |m| m.push("crates/new".into()))?;
///     Ok(())
/// })?;
/// # Ok::<(), forgo_lib_toml::EditError>(())
/// ```
pub fn edit_file_in_place(
    path: impl AsRef<Path>,
    f: impl FnOnce(&mut Document) -> Result<(), EditError>,
) -> Result<bool, EditError> {
    let path = path.as_ref();
    let bytes = std::fs::read(path)?;
    let mut doc = Document::from_slice(&bytes, &ParseOptions::default().for_editing())
        .map_err(EditError::Parse)?;
    f(&mut doc)?;
    if doc.src.as_bytes() == bytes {
        return Ok(false);
    }
    write_atomically(path, doc.src.as_bytes())?;
    Ok(true)
}

fn write_atomically(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let tmp = path.with_file_name(format!(".{name}.{}.tmp", std::process::id()));
    let result = (|| {
        std::fs::write(&tmp, bytes)?;
        std::fs::set_permissions(&tmp, std::fs::metadata(path)?.permissions())?;
        std::fs::rename(&tmp, path)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
}

// ---------------------------------------------------------------------
// Checking an edit
// ---------------------------------------------------------------------

/// What the edited path must hold afterwards.
enum Expect<'a> {
    Value(&'a Item),
    Items(&'a [Item]),
    Gone,
}

/// Whether node `y` of document `b` is node `x` of document `a` with only
/// the change at `path`.
fn same_except(
    a: &Document,
    x: Option<u32>,
    b: &Document,
    y: Option<u32>,
    path: &[Seg],
    expect: &Expect,
) -> bool {
    let Some((seg, rest)) = path.split_first() else {
        return match expect {
            Expect::Value(item) => y.is_some_and(|n| matches_item(b.value(n), item)),
            Expect::Items(items) => match y.map(|n| b.value(n)) {
                Some(Value::Array(arr)) => {
                    arr.len() == items.len()
                        && arr.iter().zip(*items).all(|(v, i)| matches_item(v, i))
                }
                _ => false,
            },
            Expect::Gone => y.is_none(),
        };
    };
    match *seg {
        Seg::Key(k) => {
            let old = match x.map(|n| a.nodes[n as usize]) {
                Some(Node::Table(t)) => Some(t),
                None => None,
                Some(_) => return false,
            };
            let Some(Node::Table(new)) = y.map(|n| b.nodes[n as usize]) else {
                return false;
            };
            let k = k.as_bytes();
            let same_others = match old {
                Some(old) => same_entries(a, old, b, new, Some(k)),
                None => entry_from(b, b.first_entry(new), Some(k)) == NONE,
            };
            same_others
                && same_except(
                    a,
                    old.and_then(|t| a.find(&a.src, t, k)),
                    b,
                    b.find(&b.src, new, k),
                    rest,
                    expect,
                )
        }
        Seg::Index(i) => {
            let (Some(Node::Array(p)), Some(Node::Array(q))) = (
                x.map(|n| a.nodes[n as usize]),
                y.map(|n| b.nodes[n as usize]),
            ) else {
                return false;
            };
            let (u, v) = (a.array_items(p), b.array_items(q));
            if rest.is_empty() && matches!(expect, Expect::Gone) {
                return v.len() + 1 == u.len()
                    && v.iter()
                        .enumerate()
                        .all(|(j, &n)| same_node(a, u[if j < i { j } else { j + 1 }], b, n));
            }
            u.len() == v.len()
                && u.iter()
                    .zip(v)
                    .enumerate()
                    .all(|(j, (&m, &n))| j == i || same_node(a, m, b, n))
                && same_except(a, u.get(i).copied(), b, v.get(i).copied(), rest, expect)
        }
    }
}

/// Equal values (tables in any order, any NaN equal to any NaN).
fn same_node(a: &Document, x: u32, b: &Document, y: u32) -> bool {
    match (a.nodes[x as usize], b.nodes[y as usize]) {
        (Node::String(s), Node::String(t)) => a.text(s) == b.text(t),
        (Node::Integer(i), Node::Integer(j)) => i == j,
        (Node::Float(f), Node::Float(g)) => float_eq(f, g),
        (Node::Boolean(p), Node::Boolean(q)) => p == q,
        (Node::Datetime(d), Node::Datetime(e)) => {
            a.datetimes[d as usize] == b.datetimes[e as usize]
        }
        (Node::Array(p), Node::Array(q)) => {
            let (u, v) = (a.array_items(p), b.array_items(q));
            u.len() == v.len() && u.iter().zip(v).all(|(&m, &n)| same_node(a, m, b, n))
        }
        (Node::Table(p), Node::Table(q)) => same_entries(a, p, b, q, None),
        _ => false,
    }
}

fn entry_key(d: &Document, e: u32) -> &[u8] {
    d.key_bytes(&d.src, d.entries[e as usize].key)
}

/// The first entry from `e` on whose key isn't `skip`.
fn entry_from(d: &Document, mut e: u32, skip: Option<&[u8]>) -> u32 {
    while e != NONE && skip.is_some_and(|k| entry_key(d, e) == k) {
        e = d.entries[e as usize].next;
    }
    e
}

/// Tables `p` (of `a`) and `q` (of `b`) have the same keys with equal
/// values, leaving out the key `skip`. Keys are compared in order first
/// (edits keep the order of what they don't touch), else looked up.
fn same_entries(a: &Document, p: u32, b: &Document, q: u32, skip: Option<&[u8]>) -> bool {
    let mut i = entry_from(a, a.first_entry(p), skip);
    let mut j = entry_from(b, b.first_entry(q), skip);
    let mut count = 0;
    let mut in_order = true;
    while i != NONE {
        count += 1;
        let k = entry_key(a, i);
        let found = if in_order && j != NONE && entry_key(b, j) == k {
            let n = b.entries[j as usize].node;
            j = entry_from(b, b.entries[j as usize].next, skip);
            Some(n)
        } else {
            in_order = false;
            b.find(&b.src, q, k)
        };
        match found {
            Some(n) if same_node(a, a.entries[i as usize].node, b, n) => {}
            _ => return false,
        }
        i = entry_from(a, a.entries[i as usize].next, skip);
    }
    let skipped = skip.is_some_and(|k| b.find(&b.src, q, k).is_some());
    b.table_len(q) - usize::from(skipped) == count
}

/// The same document, as far as editing is concerned: values, key order,
/// how tables were made, and where everything is (debug builds check
/// edits made in place with it).
#[cfg(debug_assertions)]
fn same_structure(
    a: &Document,
    x: u32,
    b: &Document,
    y: u32,
    path: &mut String,
) -> Result<(), String> {
    let (sa, sb) = (
        a.spans.as_deref().expect("spans"),
        b.spans.as_deref().expect("spans"),
    );
    if sa.node[x as usize] != sb.node[y as usize] {
        return Err(format!(
            "{path}: written at {:?}, parsed at {:?}",
            sa.node[x as usize], sb.node[y as usize]
        ));
    }
    match (a.nodes[x as usize], b.nodes[y as usize]) {
        (Node::Table(p), Node::Table(q)) => {
            let kind = |d: &Document, t: u32| match d.tables[t as usize].kind {
                Kind::Dotted(_) => "dotted",
                Kind::Root => "root",
                Kind::Implicit => "implicit",
                Kind::Header => "header",
                Kind::Inline => "inline",
                Kind::Element => "element",
            };
            if kind(a, p) != kind(b, q) {
                return Err(format!(
                    "{path}: a {} table, parsed as {}",
                    kind(a, p),
                    kind(b, q)
                ));
            }
            if sa.table[p as usize] != sb.table[q as usize] {
                return Err(format!(
                    "{path}: section {:?}, parsed {:?}",
                    sa.table[p as usize], sb.table[q as usize]
                ));
            }
            let (ea, eb): (Vec<u32>, Vec<u32>) =
                (a.entry_ids(p).collect(), b.entry_ids(q).collect());
            if ea.len() != eb.len() {
                return Err(format!("{path}: {} keys, parsed {}", ea.len(), eb.len()));
            }
            for (&i, &j) in ea.iter().zip(&eb) {
                let (ki, kj) = (entry_key(a, i), entry_key(b, j));
                if ki != kj {
                    return Err(format!(
                        "{path}: key {:?} where parsing has {:?}",
                        String::from_utf8_lossy(ki),
                        String::from_utf8_lossy(kj)
                    ));
                }
                if sa.entry[i as usize] != sb.entry[j as usize] {
                    return Err(format!(
                        "{path}.{}: defined at {:?}, parsed {:?}",
                        String::from_utf8_lossy(ki),
                        sa.entry[i as usize],
                        sb.entry[j as usize]
                    ));
                }
                let len = path.len();
                path.push('.');
                path.push_str(&String::from_utf8_lossy(ki));
                same_structure(
                    a,
                    a.entries[i as usize].node,
                    b,
                    b.entries[j as usize].node,
                    path,
                )?;
                path.truncate(len);
            }
            Ok(())
        }
        (Node::Array(p), Node::Array(q)) => {
            let (u, v) = (a.array_items(p), b.array_items(q));
            if u.len() != v.len()
                || a.arrays[p as usize].is_tables() != b.arrays[q as usize].is_tables()
            {
                return Err(format!("{path}: a different array"));
            }
            for (k, (&m, &n)) in u.iter().zip(v).enumerate() {
                let len = path.len();
                path.push_str(&format!("[{k}]"));
                same_structure(a, m, b, n, path)?;
                path.truncate(len);
            }
            Ok(())
        }
        _ if same_node(a, x, b, y) => Ok(()),
        _ => Err(format!("{path}: a different value")),
    }
}

fn float_eq(a: f64, b: f64) -> bool {
    a.to_bits() == b.to_bits() || (a.is_nan() && b.is_nan())
}

fn matches_item(v: Value, item: &Item) -> bool {
    match (v, item) {
        (Value::String(x), Item::String(y)) => x == y,
        (Value::Integer(x), Item::Integer(y)) => x == *y,
        (Value::Float(x), Item::Float(y)) => float_eq(x, *y),
        (Value::Boolean(x), Item::Boolean(y)) => x == *y,
        (Value::Datetime(x), Item::Datetime(y)) => x == *y,
        (Value::Array(x), Item::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(p, q)| matches_item(p, q))
        }
        (Value::Table(x), Item::Table(y)) => {
            x.len() == y.len()
                && y.iter()
                    .all(|(k, i)| x.get(k).is_some_and(|v| matches_item(v, i)))
        }
        _ => false,
    }
}

/// Equal items, as TOML values.
fn item_eq(a: &Item, b: &Item) -> bool {
    match (a, b) {
        (Item::Float(x), Item::Float(y)) => float_eq(*x, *y),
        (Item::Array(x), Item::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(p, q)| item_eq(p, q))
        }
        (Item::Table(x), Item::Table(y)) => {
            x.len() == y.len()
                && x.iter()
                    .all(|(k, v)| y.iter().any(|(l, w)| k == l && item_eq(v, w)))
        }
        _ => a == b,
    }
}

// ---------------------------------------------------------------------
// Planning an edit
// ---------------------------------------------------------------------

/// A path followed from the root as far as it exists.
struct Walk {
    /// The node reached after each step; `nodes[0]` is the root.
    nodes: Vec<u32>,
    /// Per step: the entry a key found ([`NONE`] for an index).
    entries: Vec<u32>,
}

impl Walk {
    /// How many steps exist.
    fn found(&self) -> usize {
        self.nodes.len() - 1
    }

    /// The last node reached.
    fn node(&self) -> u32 {
        *self.nodes.last().expect("the root")
    }

    /// Where node `j` of the walk (after the root) is held.
    fn slot(&self, j: usize, doc: &Document, path: &[Seg]) -> Slot {
        match path[j - 1] {
            Seg::Key(_) => Slot::Entry(self.entries[j - 1]),
            Seg::Index(i) => match doc.nodes[self.nodes[j - 1] as usize] {
                Node::Array(a) => Slot::Item(a, i),
                _ => unreachable!("an index steps into an array"),
            },
        }
    }

    /// How many arrays and inline tables hold node `j` (for the nesting
    /// limit).
    fn depth(&self, j: usize, doc: &Document) -> usize {
        self.nodes[..j]
            .iter()
            .filter(|&&n| match doc.nodes[n as usize] {
                Node::Array(a) => !doc.arrays[a as usize].is_tables(),
                Node::Table(t) => doc.tables[t as usize].kind == Kind::Inline,
                _ => false,
            })
            .count()
    }
}

/// A planned change to the text.
enum Change {
    /// Replace the value written at `start..end` with `text`, which must
    /// read as `expect`; `slot` then holds it.
    Value {
        start: usize,
        end: usize,
        text: String,
        slot: Slot,
        depth: usize,
        /// What it must read as, if not the value being set.
        expect: Option<Item>,
    },
    /// Add whole lines at `at`, which is in the section of table
    /// `section` (or the lines start a section of their own).
    Lines {
        at: usize,
        text: String,
        section: u32,
    },
    /// Anything else: splice, then parse the whole text again.
    Reparse(Vec<Splice>),
}

/// Where a value is held: an entry of a table, or an item of an array.
#[derive(Clone, Copy)]
enum Slot {
    Entry(u32),
    Item(u32, usize),
}

/// Replace `start..end` of the text with `text`.
struct Splice {
    start: usize,
    end: usize,
    text: String,
}

fn apply(src: &str, mut splices: Vec<Splice>) -> String {
    splices.sort_by_key(|s| (s.start, s.end));
    let mut out =
        String::with_capacity(src.len() + splices.iter().map(|s| s.text.len()).sum::<usize>());
    let mut at = 0;
    for s in splices {
        out.push_str(&src[at..s.start]);
        out.push_str(&s.text);
        at = s.end;
    }
    out.push_str(&src[at..]);
    out
}

/// An item of a list being rebuilt: an old one (its text kept), or new
/// text.
enum Part {
    Old(usize),
    New(String),
}

/// A comma-separated list in the text: an array's items, or an inline
/// table's key/value pairs.
struct List {
    /// The `[` or `{`.
    open: usize,
    /// The `]` or `}`.
    close: usize,
    /// Each element's text.
    elems: Vec<(usize, usize)>,
    /// An inline table (`{ a = 1 }`), not an array.
    table: bool,
}

/// Add `value` at the dotted `keys` under the table `table`.
fn item_insert(table: &mut Item, keys: &[&str], value: Item) {
    let Item::Table(entries) = table else {
        unreachable!("a table")
    };
    let (first, rest) = keys.split_first().expect("a key");
    if rest.is_empty() {
        entries.push((first.to_string(), value));
        return;
    }
    match entries.iter_mut().find(|(k, _)| k == first) {
        Some((_, sub)) => item_insert(sub, rest, value),
        None => {
            let mut sub = Item::Table(Vec::new());
            item_insert(&mut sub, rest, value);
            entries.push((first.to_string(), sub));
        }
    }
}

/// Remove the value at the dotted `keys` under the table `table`.
fn item_remove(table: &mut Item, keys: &[&str]) {
    let Item::Table(entries) = table else {
        unreachable!("a table")
    };
    let (first, rest) = keys.split_first().expect("a key");
    if rest.is_empty() {
        entries.retain(|(k, _)| k != first);
    } else if let Some((_, sub)) = entries.iter_mut().find(|(k, _)| k == first) {
        item_remove(sub, rest);
    }
}

fn render(v: &Item) -> String {
    let mut s = String::new();
    item::inline(&mut s, v);
    s
}

fn dotted<'k>(keys: impl IntoIterator<Item = &'k str>) -> String {
    let mut s = String::new();
    for (i, k) in keys.into_iter().enumerate() {
        if i > 0 {
            s.push('.');
        }
        item::key(&mut s, k);
    }
    s
}

fn unsupported(path: &[Seg], why: String) -> EditError {
    EditError::Unsupported(format!("can't edit `{}` in place: {why}", show(path)))
}

fn line_start(src: &str, pos: usize) -> usize {
    src[..pos].rfind('\n').map_or(0, |i| i + 1)
}

/// The end of the line holding `pos`: after its newline, or the end of
/// the text.
fn line_end(src: &str, pos: usize) -> usize {
    src[pos..].find('\n').map_or(src.len(), |i| pos + i + 1)
}

fn is_blank(line: &str) -> bool {
    line.trim_matches([' ', '\t', '\r', '\n']).is_empty()
}

fn is_comment_line(line: &str) -> bool {
    line.trim_start_matches([' ', '\t']).starts_with('#')
}

/// The text and positions an edit is planned on.
struct Ed<'d> {
    doc: &'d Document,
    src: &'d str,
    spans: &'d Spans,
    nl: &'static str,
}

impl<'d> Ed<'d> {
    fn new(doc: &'d Document) -> Ed<'d> {
        let src: &str = &doc.src;
        let nl = match src.find('\n') {
            Some(i) if i > 0 && src.as_bytes()[i - 1] == b'\r' => "\r\n",
            _ => "\n",
        };
        Ed {
            doc,
            src,
            spans: doc.spans.as_deref().expect("spans"),
            nl,
        }
    }

    fn kind(&self, t: u32) -> Kind {
        self.doc.tables[t as usize].kind
    }

    // -- set ------------------------------------------------------------

    fn plan_set(&self, w: &Walk, path: &[Seg], value: &Item) -> Result<Change, EditError> {
        let found = w.found();
        if found == path.len() {
            let node = w.node();
            let (s, e) = self.spans.node[node as usize];
            if s == NONE {
                return Err(unsupported(
                    path,
                    "it is written as `[sections]` or dotted keys; set the keys inside it instead"
                        .into(),
                ));
            }
            return Ok(Change::Value {
                start: s as usize,
                end: e as usize,
                text: render(value),
                slot: w.slot(found, self.doc, path),
                depth: w.depth(found, self.doc),
                expect: None,
            });
        }
        let mut keys = Vec::new();
        for (n, seg) in path.iter().enumerate().skip(found) {
            match seg {
                Seg::Key(k) => keys.push(*k),
                Seg::Index(i) => {
                    return Err(EditError::Path(format!(
                        "`{}` has no item {i}",
                        show(&path[..n])
                    )));
                }
            }
        }
        let parent = w.node();
        let Node::Table(t) = self.doc.nodes[parent as usize] else {
            return Err(EditError::Path(format!(
                "`{}` isn't a table",
                show(&path[..found])
            )));
        };
        let pair = |prefix: &[&str]| {
            let mut s = dotted(prefix.iter().chain(&keys).copied());
            s.push_str(" = ");
            item::inline(&mut s, value);
            s
        };
        Ok(match self.kind(t) {
            Kind::Inline => self.append_pair(w, found, &keys, value, pair(&[]), path)?,
            Kind::Root if keys.len() > 1 => {
                let (last, tables) = keys.split_last().expect("keys");
                let pair = format!("{} = {}", dotted([*last]), render(value));
                self.append_section(&format!("[{}]", dotted(tables.iter().copied())), &pair)
            }
            Kind::Root | Kind::Header | Kind::Element => self.section_insert(t, &pair(&[])),
            Kind::Dotted(s) => {
                // The section's table is on the path; the keys after it
                // lead here.
                let j = (0..=found)
                    .rev()
                    .find(|&j| self.doc.nodes[w.nodes[j] as usize] == Node::Table(s))
                    .expect("a dotted table's section is above it");
                let prefix: Vec<&str> = path[j..found]
                    .iter()
                    .filter_map(|seg| match seg {
                        Seg::Key(k) => Some(*k),
                        Seg::Index(_) => None,
                    })
                    .collect();
                if self.kind(s) == Kind::Inline {
                    let mut all = prefix.clone();
                    all.extend(&keys);
                    self.append_pair(w, j, &all, value, pair(&prefix), path)?
                } else {
                    self.section_insert(s, &pair(&prefix))
                }
            }
            Kind::Implicit => {
                let head: Vec<&str> = path[..found]
                    .iter()
                    .filter_map(|seg| match seg {
                        Seg::Key(k) => Some(*k),
                        Seg::Index(_) => None,
                    })
                    .collect();
                let header = format!("[{}]", dotted(head));
                Change::Reparse(vec![self.implicit_section(parent, &header, &pair(&[]))])
            }
        })
    }

    /// A line of its own at `pos` (a line start, or the end of the text),
    /// in the section of table `section`.
    fn insert_line(&self, pos: usize, indent: &str, line: &str, section: u32) -> Change {
        let nl = self.nl;
        let mut text = String::with_capacity(nl.len() + indent.len() + line.len());
        // When the last line has no newline, keep it that way.
        let last = pos == self.src.len() && !self.src.is_empty() && !self.src.ends_with('\n');
        if last {
            text.push_str(nl);
        }
        text.push_str(indent);
        text.push_str(line);
        if !last {
            text.push_str(nl);
        }
        Change::Lines {
            at: pos,
            text,
            section,
        }
    }

    /// A key/value line in the section of table `t` (the root, or a table
    /// with a header): after its last key/value line, indented like it.
    fn section_insert(&self, t: u32, line: &str) -> Change {
        let span = self.spans.table[t as usize];
        if span.last_key != NONE {
            let key = span.last_key as usize;
            let indent = &self.src[line_start(self.src, key)..key];
            return self.insert_line(span.end as usize, indent, line, t);
        }
        if span.header != NONE {
            return self.insert_line(span.head_end as usize, "", line, t);
        }
        // The root, without keys: before the first section.
        match self
            .spans
            .table
            .iter()
            .map(|s| s.header)
            .filter(|&h| h != NONE)
            .min()
        {
            Some(h) => {
                let at = self.attached_start(line_start(self.src, h as usize));
                let nl = self.nl;
                Change::Lines {
                    at,
                    text: format!("{line}{nl}{nl}"),
                    section: t,
                }
            }
            None => self.insert_line(self.src.len(), "", line, t),
        }
    }

    /// A new section at the end of the text, after a blank line.
    fn append_section(&self, header: &str, line: &str) -> Change {
        let (src, nl) = (self.src, self.nl);
        let body = src.trim_end_matches(['\n', '\r', ' ', '\t']);
        let breaks = src[body.len()..].matches('\n').count();
        let lead = match (body.is_empty(), breaks) {
            (true, _) => String::new(),
            (false, 0) => format!("{nl}{nl}"),
            (false, 1) => nl.to_string(),
            _ => String::new(),
        };
        // A text without a final newline keeps it that way.
        let end = if breaks == 0 && !body.is_empty() {
            ""
        } else {
            nl
        };
        Change::Lines {
            at: src.len(),
            text: format!("{lead}{header}{nl}{line}{end}"),
            section: 0,
        }
    }

    /// A section for an implicit table (one only named on the way to
    /// other sections): before the first of those.
    fn implicit_section(&self, node: u32, header: &str, line: &str) -> Splice {
        let first = self
            .first_section(node)
            .expect("an implicit table has sections under it");
        let at = self.attached_start(line_start(self.src, first));
        let nl = self.nl;
        Splice {
            start: at,
            end: at,
            text: format!("{header}{nl}{line}{nl}{nl}"),
        }
    }

    /// Where the first section under `node` starts.
    fn first_section(&self, node: u32) -> Option<usize> {
        match self.doc.nodes[node as usize] {
            Node::Table(t) => {
                let own = self.spans.table[t as usize].header;
                let own = (own != NONE).then_some(own as usize);
                self.doc
                    .entry_ids(t)
                    .filter_map(|e| self.first_section(self.doc.entries[e as usize].node))
                    .chain(own)
                    .min()
            }
            Node::Array(a) if self.doc.arrays[a as usize].is_tables() => self
                .doc
                .array_items(a)
                .iter()
                .filter_map(|&n| self.first_section(n))
                .min(),
            _ => None,
        }
    }

    /// Add a key/value pair (`keys`, written as `pair`) to the inline table
    /// at node `j` of the walk.
    fn append_pair(
        &self,
        w: &Walk,
        j: usize,
        keys: &[&str],
        value: &Item,
        pair: String,
        path: &[Seg],
    ) -> Result<Change, EditError> {
        let node = w.nodes[j];
        let list = self.inline_list(node);
        let mut parts: Vec<Part> = (0..list.elems.len()).map(Part::Old).collect();
        parts.push(Part::New(pair));
        let text = self
            .rebuild(&list, &parts)
            .map_err(|why| unsupported(path, why))?;
        let mut expect = self.doc.value(node).to_item();
        item_insert(&mut expect, keys, value.clone());
        Ok(Change::Value {
            start: list.open,
            end: list.close + 1,
            text,
            slot: w.slot(j, self.doc, path),
            depth: w.depth(j, self.doc),
            expect: Some(expect),
        })
    }

    // -- remove ---------------------------------------------------------

    fn plan_remove(&self, w: &Walk, path: &[Seg], gone: usize) -> Result<Change, EditError> {
        let n = path.len();
        let target = w.nodes[n];
        let container = w.nodes[n - 1];
        if let (Node::Array(a), Seg::Index(i)) = (self.doc.nodes[container as usize], path[n - 1])
            && !self.doc.arrays[a as usize].is_tables()
        {
            let list = self.array_list(container, a);
            let parts: Vec<Part> = (0..list.elems.len())
                .filter(|&j| j != i)
                .map(Part::Old)
                .collect();
            let text = self
                .rebuild(&list, &parts)
                .map_err(|why| unsupported(path, why))?;
            let Item::Array(mut items) = self.doc.value(container).to_item() else {
                unreachable!("an array")
            };
            items.remove(i);
            return Ok(Change::Value {
                start: list.open,
                end: list.close + 1,
                text,
                slot: w.slot(n - 1, self.doc, path),
                depth: w.depth(n - 1, self.doc),
                expect: Some(Item::Array(items)),
            });
        }
        // Everything that writes the target: key/value pairs (with the
        // table holding each) and sections.
        let mut pairs = Vec::new();
        let mut sections = Vec::new();
        let entry = w.entries[n - 1];
        match self.doc.nodes[container as usize] {
            Node::Table(t) if entry != NONE && self.spans.entry[entry as usize].1 != NONE => {
                pairs.push((t, entry));
            }
            _ => self.collect(target, &mut pairs, &mut sections),
        }
        let mut lines = Vec::new();
        let mut lists: Vec<(u32, Vec<(usize, usize)>)> = Vec::new();
        for (t, e) in pairs {
            let (s, end) = self.spans.entry[e as usize];
            let span = (s as usize, end as usize);
            match self.inline_owner(t, w) {
                Some(owner) => match lists.iter_mut().find(|(o, _)| *o == owner) {
                    Some((_, v)) => v.push(span),
                    None => lists.push((owner, vec![span])),
                },
                None => lines.push(self.line_range(span)),
            }
        }
        for t in sections {
            lines.push(self.section_range(t));
        }
        // Pairs of one inline table only: rewrite that table where it is.
        if let ([(owner, dropped)], true) = (lists.as_slice(), lines.is_empty())
            && let Some(j) = w.nodes.iter().position(|&x| x == *owner)
        {
            let list = self.inline_list(*owner);
            let parts: Vec<Part> = (0..list.elems.len())
                .filter(|&k| !dropped.contains(&list.elems[k]))
                .map(Part::Old)
                .collect();
            let text = self
                .rebuild(&list, &parts)
                .map_err(|why| unsupported(path, why))?;
            let keys: Vec<&str> = path[j..gone]
                .iter()
                .map(|seg| match seg {
                    Seg::Key(k) => *k,
                    Seg::Index(_) => unreachable!("inline tables hold keys"),
                })
                .collect();
            let mut expect = self.doc.value(*owner).to_item();
            item_remove(&mut expect, &keys);
            return Ok(Change::Value {
                start: list.open,
                end: list.close + 1,
                text,
                slot: w.slot(j, self.doc, path),
                depth: w.depth(j, self.doc),
                expect: Some(expect),
            });
        }
        let mut splices = self.deletions(lines);
        for (owner, dropped) in lists {
            let list = self.inline_list(owner);
            let parts: Vec<Part> = (0..list.elems.len())
                .filter(|&k| !dropped.contains(&list.elems[k]))
                .map(Part::Old)
                .collect();
            let text = self
                .rebuild(&list, &parts)
                .map_err(|why| unsupported(path, why))?;
            splices.push(Splice {
                start: list.open,
                end: list.close + 1,
                text,
            });
        }
        Ok(Change::Reparse(splices))
    }

    /// The pairs and sections that write `node`.
    fn collect(&self, node: u32, pairs: &mut Vec<(u32, u32)>, sections: &mut Vec<u32>) {
        match self.doc.nodes[node as usize] {
            Node::Table(t) => {
                if self.spans.table[t as usize].header != NONE {
                    sections.push(t);
                }
                for e in self.doc.entry_ids(t) {
                    if self.spans.entry[e as usize].1 != NONE {
                        pairs.push((t, e));
                    } else {
                        self.collect(self.doc.entries[e as usize].node, pairs, sections);
                    }
                }
            }
            Node::Array(a) if self.doc.arrays[a as usize].is_tables() => {
                for &el in self.doc.array_items(a) {
                    self.collect(el, pairs, sections);
                }
            }
            _ => {}
        }
    }

    /// The node of the inline table that a pair of table `t` is written
    /// in, if any.
    fn inline_owner(&self, t: u32, w: &Walk) -> Option<u32> {
        let owner = match self.kind(t) {
            Kind::Inline => t,
            Kind::Dotted(s) if self.kind(s) == Kind::Inline => s,
            _ => return None,
        };
        let on_path = w
            .nodes
            .iter()
            .copied()
            .find(|&n| self.doc.nodes[n as usize] == Node::Table(owner));
        Some(on_path.unwrap_or_else(|| {
            let node = self.doc.nodes.iter().position(|n| *n == Node::Table(owner));
            node.expect("every table has a node") as u32
        }))
    }

    /// A key/value line, with the comment lines directly above it.
    fn line_range(&self, (key, value_end): (usize, usize)) -> (usize, usize) {
        (
            self.attached_start(line_start(self.src, key)),
            line_end(self.src, value_end),
        )
    }

    /// A section: its header (with the comment lines directly above it),
    /// its key/value lines, and the blank lines after them.
    fn section_range(&self, t: u32) -> (usize, usize) {
        let span = self.spans.table[t as usize];
        let start = self.attached_start(line_start(self.src, span.header as usize));
        let mut end = span.end as usize;
        while end < self.src.len() {
            let next = line_end(self.src, end);
            if !is_blank(&self.src[end..next]) {
                break;
            }
            end = next;
        }
        (start, end)
    }

    /// Deleting these ranges, merged. A deletion that reaches the end of
    /// the text also takes the blank lines before it, and keeps a missing
    /// final newline missing.
    fn deletions(&self, mut ranges: Vec<(usize, usize)>) -> Vec<Splice> {
        ranges.sort_unstable();
        let mut merged: Vec<(usize, usize)> = Vec::new();
        for (s, e) in ranges {
            match merged.last_mut() {
                Some(last) if s <= last.1 => last.1 = last.1.max(e),
                _ => merged.push((s, e)),
            }
        }
        let n = self.src.len();
        if let Some(last) = merged.last_mut()
            && last.1 == n
        {
            while last.0 > 0 {
                let prev = line_start(self.src, last.0 - 1);
                if !is_blank(&self.src[prev..last.0]) {
                    break;
                }
                last.0 = prev;
            }
            if !self.src.ends_with('\n') && last.0 > 0 {
                last.0 -= 1;
                if last.0 > 0 && self.src.as_bytes()[last.0 - 1] == b'\r' {
                    last.0 -= 1;
                }
            }
        }
        merged
            .into_iter()
            .map(|(start, end)| Splice {
                start,
                end,
                text: String::new(),
            })
            .collect()
    }

    /// The end of the last value, key/value line or header before `pos`.
    /// Comment lines above a line never reach back past it (a multi-line
    /// string can hold lines that look like comments).
    fn floor(&self, pos: usize) -> usize {
        let pos = pos as u32;
        let ends = self
            .spans
            .node
            .iter()
            .chain(&self.spans.entry)
            .map(|&(_, e)| e);
        let heads = self.spans.table.iter().map(|t| t.head_end);
        ends.chain(heads)
            .filter(|&e| e != NONE && e <= pos)
            .max()
            .unwrap_or(0) as usize
    }

    /// Where the comment lines directly above the line at `start` begin.
    fn attached_start(&self, start: usize) -> usize {
        let floor = self.floor(start);
        let mut s = start;
        while s > floor {
            let prev = line_start(self.src, s - 1);
            if prev < floor || !is_comment_line(&self.src[prev..s]) {
                break;
            }
            s = prev;
        }
        s
    }

    // -- lists ----------------------------------------------------------

    fn array_list(&self, node: u32, a: u32) -> List {
        let (open, end) = self.spans.node[node as usize];
        let elems = self
            .doc
            .array_items(a)
            .iter()
            .map(|&n| {
                let (s, e) = self.spans.node[n as usize];
                (s as usize, e as usize)
            })
            .collect();
        List {
            open: open as usize,
            close: end as usize - 1,
            elems,
            table: false,
        }
    }

    fn inline_list(&self, node: u32) -> List {
        let (open, end) = self.spans.node[node as usize];
        let (open, close) = (open as usize, end as usize - 1);
        let mut inside: Vec<(usize, usize)> = self
            .spans
            .entry
            .iter()
            .filter(|&&(s, e)| e != NONE && s as usize > open && e as usize <= close)
            .map(|&(s, e)| (s as usize, e as usize))
            .collect();
        inside.sort_unstable();
        // Only the pairs of this table, not those of tables inside it.
        let mut elems: Vec<(usize, usize)> = Vec::new();
        for (s, e) in inside {
            if elems.last().is_none_or(|&(_, pe)| s >= pe) {
                elems.push((s, e));
            }
        }
        List {
            open,
            close,
            elems,
            table: true,
        }
    }

    /// The list's new text (from its opening to its closing bracket),
    /// holding `parts` in its own style.
    ///
    /// A list on one line, or one whose items share lines, keeps every
    /// separator, comment and line break where it is, and items move
    /// between those places. A list written one item per line moves each
    /// item with its lines, and the comment lines above it.
    fn rebuild(&self, list: &List, parts: &[Part]) -> Result<String, String> {
        let src = self.src;
        let (open, close) = (list.open, list.close);
        let one_line = !src[open..close].contains('\n');
        if list.elems.is_empty() && one_line {
            let (o, c) = (&src[open..=open], &src[close..=close]);
            let texts: Vec<String> = parts.iter().map(|p| self.part_text(list, p)).collect();
            return Ok(match (texts.is_empty(), list.table) {
                (true, _) => format!("{o}{c}"),
                (false, true) => format!("{o} {} {c}", texts.join(", ")),
                (false, false) => format!("{o}{}{c}", texts.join(", ")),
            });
        }
        if one_line {
            return self.rebuild_in_place(list, parts);
        }
        self.rebuild_lines(list, parts)
            .or_else(|_| self.rebuild_in_place(list, parts))
    }

    fn part_text(&self, list: &List, p: &Part) -> String {
        match p {
            Part::Old(i) => self.src[list.elems[*i].0..list.elems[*i].1].to_string(),
            Part::New(t) => t.clone(),
        }
    }

    /// Items into the places of the old ones; more go after the last,
    /// separated as the first two are; fewer drop the last places.
    fn rebuild_in_place(&self, list: &List, parts: &[Part]) -> Result<String, String> {
        let src = self.src;
        let (open, close) = (list.open, list.close);
        let (k, m) = (list.elems.len(), parts.len());
        let (Some(&first), Some(&last)) = (list.elems.first(), list.elems.last()) else {
            return Err("it has no items to keep the places of".into());
        };
        // Whether the text between items in `from..to` holds a comment (a
        // `#` in an item is in a string).
        let comment_in = |from: usize, to: usize| {
            let mut at = from;
            for &(s, e) in &list.elems {
                if e <= at || s >= to {
                    continue;
                }
                if src[at..s.max(at)].contains('#') {
                    return true;
                }
                at = e.max(at);
            }
            at < to && src[at..to].contains('#')
        };
        if m == 0 {
            if comment_in(open + 1, close) {
                return Err("emptying it would delete its comments".into());
            }
            return Ok(format!("{}{}", &src[open..=open], &src[close..=close]));
        }
        // Only removals: each kept item keeps the separator before it.
        let olds: Option<Vec<usize>> = parts
            .iter()
            .map(|p| match p {
                Part::Old(i) => Some(*i),
                Part::New(_) => None,
            })
            .collect();
        if let Some(kept) = olds.filter(|o| m < k && o.windows(2).all(|w| w[0] < w[1])) {
            let e = |j: usize| list.elems[j].1;
            let s = |j: usize| list.elems[j].0;
            let mut dropped = vec![(first.0, s(kept[0]))];
            dropped.extend(kept.windows(2).map(|w| (e(w[0]), e(w[1] - 1))));
            dropped.push((e(kept[m - 1]), last.1));
            if dropped.iter().any(|&(a, b)| comment_in(a, b)) {
                return Err("removing its items would delete comments".into());
            }
            let mut out = src[open..first.0].to_string();
            for (t, &j) in kept.iter().enumerate() {
                if t > 0 {
                    out.push_str(&src[e(j - 1)..s(j)]);
                }
                out.push_str(&src[s(j)..e(j)]);
            }
            out.push_str(&src[last.1..=close]);
            return Ok(out);
        }
        let kept = m.min(k);
        if m < k && comment_in(list.elems[m - 1].1, last.1) {
            return Err("removing its items would delete comments between them".into());
        }
        let sep = match list.elems.get(1) {
            Some(&(s, _)) if !src[first.1..s].contains('#') => &src[first.1..s],
            _ => ", ",
        };
        let mut out = src[open..first.0].to_string();
        for (j, part) in parts.iter().enumerate() {
            if j > 0 {
                out.push_str(if j < kept {
                    &src[list.elems[j - 1].1..list.elems[j].0]
                } else {
                    sep
                });
            }
            out.push_str(&self.part_text(list, part));
        }
        out.push_str(&src[last.1..=close]);
        Ok(out)
    }

    /// Items with their lines, for a list written one item per line.
    fn rebuild_lines(&self, list: &List, parts: &[Part]) -> Result<String, String> {
        let src = self.src;
        let (open, close) = (list.open, list.close);
        // Over several lines: each element needs its lines to itself.
        let head_end = line_end(src, open);
        let close_line = line_start(src, close);
        if !src[close_line..close].trim_matches([' ', '\t']).is_empty() {
            return Err("its closing bracket shares a line with an item".into());
        }
        let mut chunks = Vec::with_capacity(list.elems.len());
        let mut at = head_end;
        for &(s, e) in &list.elems {
            let ls = line_start(src, s);
            let after_end = line_end(src, e);
            let after = src[e..after_end].trim_end_matches(['\n', '\r']);
            let after = after.trim_start_matches([' ', '\t']);
            let after = after
                .strip_prefix(',')
                .unwrap_or(after)
                .trim_start_matches([' ', '\t']);
            if ls < head_end
                || !src[ls..s].trim_matches([' ', '\t']).is_empty()
                || !(after.is_empty() || after.starts_with('#'))
                || after_end > close_line
            {
                return Err("its items share lines; write it with one item per line first".into());
            }
            chunks.push((at, after_end));
            at = after_end;
        }
        let tail = &src[at..=close];
        let has_comma = |i: usize| {
            src[list.elems[i].1..chunks[i].1]
                .trim_start_matches([' ', '\t'])
                .starts_with(',')
        };
        let (trailing, indent) = match list.elems.last() {
            Some(&(s, _)) => (
                has_comma(list.elems.len() - 1),
                src[line_start(src, s)..s].to_string(),
            ),
            None => (true, format!("{}    ", &src[close_line..close])),
        };
        let mut out = src[open..head_end].to_string();
        for (k, p) in parts.iter().enumerate() {
            let comma = k + 1 < parts.len() || trailing;
            match p {
                Part::Old(i) => {
                    let (cs, ce) = chunks[*i];
                    let e = list.elems[*i].1;
                    out.push_str(&src[cs..e]);
                    let after = &src[e..ce];
                    match (has_comma(*i), comma) {
                        (true, false) => {
                            let at = after.find(',').expect("a comma");
                            out.push_str(&after[..at]);
                            out.push_str(&after[at + 1..]);
                        }
                        (false, true) => {
                            out.push(',');
                            out.push_str(after);
                        }
                        _ => out.push_str(after),
                    }
                }
                Part::New(t) => {
                    out.push_str(&indent);
                    out.push_str(t);
                    if comma {
                        out.push(',');
                    }
                    out.push_str(self.nl);
                }
            }
        }
        out.push_str(tail);
        Ok(out)
    }
}
