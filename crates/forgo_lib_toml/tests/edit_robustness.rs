//! Edits on every valid toml-test document, at every path: replace the
//! value, add keys to every table (and under new tables), remove the
//! value, and reverse and extend every array.
//!
//! No edit may panic or fail its own check (`EditError::Verify` is a
//! bug). Each edit that succeeds is checked again here, independently of
//! the crate's check: the path holds what was asked, and every other
//! value is as it was.

use forgo_lib_toml::{Document, EditError, Item, ParseOptions, Seg, TomlVersion, Value};
use std::path::Path;

fn corpus() -> Vec<(String, String, TomlVersion)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/toml-test-data/tests");
    let mut out = Vec::new();
    for (list, version) in [
        ("files-toml-1.1.0", TomlVersion::V1_1),
        ("files-toml-1.0.0", TomlVersion::V1_0),
    ] {
        let Ok(names) = std::fs::read_to_string(dir.join(list)) else {
            assert!(
                std::env::var("TOML_TEST_SUITE_REQUIRED").as_deref() != Ok("1"),
                "toml-test data is missing; run scripts/fetch-toml-test.sh"
            );
            eprintln!("skipped: no toml-test data (scripts/fetch-toml-test.sh)");
            return Vec::new();
        };
        for name in names
            .lines()
            .filter(|l| l.starts_with("valid/") && l.ends_with(".toml"))
        {
            let bytes = std::fs::read(dir.join(name)).unwrap();
            // A byte order mark is part of the text; editing keeps it.
            out.push((name.to_string(), String::from_utf8(bytes).unwrap(), version));
        }
    }
    out
}

/// An owned path step.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum P {
    Key(String),
    Index(usize),
}

fn segs(path: &[P]) -> Vec<Seg<'_>> {
    path.iter()
        .map(|p| match p {
            P::Key(k) => Seg::Key(k),
            P::Index(i) => Seg::Index(*i),
        })
        .collect()
}

/// Every path in the document, parents before children.
fn paths(v: Value, prefix: &mut Vec<P>, out: &mut Vec<Vec<P>>) {
    out.push(prefix.clone());
    match v {
        Value::Table(t) => {
            for (k, v) in t.iter() {
                prefix.push(P::Key(k.to_string()));
                paths(v, prefix, out);
                prefix.pop();
            }
        }
        Value::Array(a) => {
            for (i, v) in a.iter().enumerate() {
                prefix.push(P::Index(i));
                paths(v, prefix, out);
                prefix.pop();
            }
        }
        _ => {}
    }
}

/// The values at the ends of paths (including empty tables and arrays),
/// sorted by path.
fn leaves(v: Value, prefix: &mut Vec<P>, out: &mut Vec<(Vec<P>, Item)>) {
    match v {
        Value::Table(t) if !t.is_empty() => {
            for (k, v) in t.iter() {
                prefix.push(P::Key(k.to_string()));
                leaves(v, prefix, out);
                prefix.pop();
            }
        }
        Value::Array(a) if !a.is_empty() => {
            for (i, v) in a.iter().enumerate() {
                prefix.push(P::Index(i));
                leaves(v, prefix, out);
                prefix.pop();
            }
        }
        _ => out.push((prefix.clone(), v.to_item())),
    }
}

fn all_leaves(doc: &Document) -> Vec<(Vec<P>, Item)> {
    let mut out = Vec::new();
    leaves(Value::Table(doc.root()), &mut Vec::new(), &mut out);
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

/// Equal as TOML values (any NaN equal to any NaN, tables in any order).
fn same(a: &Item, b: &Item) -> bool {
    match (a, b) {
        (Item::Float(x), Item::Float(y)) => {
            (x.is_nan() && y.is_nan()) || x.to_bits() == y.to_bits()
        }
        (Item::Array(x), Item::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(p, q)| same(p, q))
        }
        (Item::Table(x), Item::Table(y)) => {
            x.len() == y.len()
                && x.iter()
                    .all(|(k, v)| y.iter().any(|(l, w)| k == l && same(v, w)))
        }
        _ => a == b,
    }
}

fn under(path: &[P], prefix: &[P]) -> bool {
    path.len() >= prefix.len() && &path[..prefix.len()] == prefix
}

/// Which document may hold empty tables above the edited path that the
/// other doesn't: the old one when an edit adds under an empty table, the
/// new one when a removal empties a table that stays.
#[derive(Clone, Copy, PartialEq)]
enum Empty {
    Neither,
    Old,
    New,
}

/// The leaves outside `prefix` are the same in both documents.
fn same_outside(old: &Document, new: &Document, prefix: &[P], empty: Empty) -> Result<(), String> {
    let keep = |l: &Vec<(Vec<P>, Item)>, ancestors_ok: bool| -> Vec<(Vec<P>, Item)> {
        l.iter()
            .filter(|(p, item)| {
                !under(p, prefix)
                    && !(ancestors_ok
                        && under(prefix, p)
                        && matches!(item, Item::Table(t) if t.is_empty()))
            })
            .cloned()
            .collect()
    };
    let a = keep(&all_leaves(old), empty == Empty::Old);
    let b = keep(&all_leaves(new), empty == Empty::New);
    if a.len() != b.len()
        || a.iter()
            .zip(&b)
            .any(|(x, y)| x.0 != y.0 || !same(&x.1, &y.1))
    {
        return Err(format!("values outside {prefix:?} changed"));
    }
    Ok(())
}

#[derive(Default)]
struct Stats {
    ok: usize,
    unsupported: usize,
    path: usize,
}

fn run(
    name: &str,
    doc: &Document,
    what: &str,
    stats: &mut Stats,
    f: impl FnOnce(&mut Document) -> Result<(), EditError>,
    check: impl FnOnce(&Document) -> Result<(), String>,
) {
    let mut edited = doc.clone();
    match f(&mut edited) {
        Ok(()) => {
            stats.ok += 1;
            if let Err(why) = check(&edited) {
                panic!("{name}: {what}: {why}\n--- before\n{doc}\n--- after\n{edited}");
            }
            // The text the edit wrote reads back the same from scratch.
            let again = Document::parse_with(
                &edited.to_string(),
                &ParseOptions::default().with_version(TomlVersion::V1_1),
            )
            .unwrap_or_else(|e| panic!("{name}: {what}: {e}"));
            assert!(same(&again.to_item(), &edited.to_item()), "{name}: {what}");
        }
        Err(EditError::Unsupported(why)) => {
            stats.unsupported += 1;
            if std::env::var("SHOW_UNSUPPORTED").is_ok() {
                let why = why.split(": ").last().unwrap_or("").to_string();
                eprintln!(
                    "UNSUPPORTED\t{}\t{}\t{name}",
                    what.split(' ').next().unwrap_or(""),
                    why
                );
            }
            assert_eq!(
                edited.to_string(),
                doc.to_string(),
                "{name}: {what}: a failed edit changed the text"
            );
        }
        Err(EditError::Path(_)) => {
            stats.path += 1;
            assert_eq!(
                edited.to_string(),
                doc.to_string(),
                "{name}: {what}: a failed edit changed the text"
            );
        }
        Err(e) => panic!("{name}: {what}: {e}\n--- document\n{doc}"),
    }
}

#[test]
fn every_edit_on_every_toml_test_document() {
    let corpus = corpus();
    if corpus.is_empty() {
        return;
    }
    let mut stats = Stats::default();
    let mark = Item::from("edited");
    for (name, text, version) in &corpus {
        let doc = Document::parse_with(text, &ParseOptions::default().with_version(*version))
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        let mut all = Vec::new();
        paths(Value::Table(doc.root()), &mut Vec::new(), &mut all);
        for path in &all {
            let target = doc.get(segs(path)).unwrap();
            // Replace the value.
            if !path.is_empty() {
                run(
                    name,
                    &doc,
                    &format!("set {path:?}"),
                    &mut stats,
                    |d| d.set(segs(path), mark.clone()),
                    |new| {
                        let got = new.get(segs(path)).ok_or("the path is gone")?.to_item();
                        if !same(&got, &mark) {
                            return Err(format!("the path holds {got:?}"));
                        }
                        same_outside(&doc, new, path, Empty::Neither)
                    },
                );
            }
            // A new key in every table, and one under two new tables.
            if let Value::Table(t) = target {
                for (key, depth) in [("new-key", 1), ("new table", 3)] {
                    let mut new_path = path.clone();
                    for n in 0..depth {
                        new_path.push(P::Key(if n == depth - 1 {
                            key.to_string()
                        } else {
                            format!("t{n}")
                        }));
                    }
                    if t.contains_key("t0") || t.contains_key(key) {
                        continue;
                    }
                    run(
                        name,
                        &doc,
                        &format!("add {new_path:?}"),
                        &mut stats,
                        |d| d.set(segs(&new_path), 42),
                        |new| {
                            let got = new
                                .get(segs(&new_path))
                                .ok_or("the new key is missing")?
                                .to_item();
                            if !same(&got, &Item::from(42)) {
                                return Err(format!("the new key holds {got:?}"));
                            }
                            same_outside(&doc, new, &new_path[..path.len() + 1], Empty::Old)
                        },
                    );
                }
            }
            // Remove it.
            if !path.is_empty() {
                let (parent, last) = path.split_at(path.len() - 1);
                run(
                    name,
                    &doc,
                    &format!("remove {path:?}"),
                    &mut stats,
                    |d| d.remove(segs(path)).map(|was| assert!(was)),
                    |new| match last[0] {
                        P::Index(_)
                            if doc
                                .get(segs(parent))
                                .unwrap()
                                .as_array()
                                .unwrap()
                                .is_array_of_tables()
                                && doc.get(segs(parent)).unwrap().as_array().unwrap().len()
                                    == 1 =>
                        {
                            // The last `[[name]]` section: the key goes.
                            if new.get(segs(parent)).is_some() {
                                return Err("the array of tables is still there".into());
                            }
                            same_outside(&doc, new, parent, Empty::New)
                        }
                        P::Index(i) => {
                            let Some(Item::Array(old)) = doc.get(segs(parent)).map(|v| v.to_item())
                            else {
                                return Err("not an array".into());
                            };
                            let mut want = old.clone();
                            want.remove(i);
                            let got = new.get(segs(parent)).map(|v| v.to_item());
                            if !got.is_some_and(|g| same(&g, &Item::Array(want))) {
                                return Err("the array isn't the old one without the item".into());
                            }
                            same_outside(&doc, new, parent, Empty::Neither)
                        }
                        P::Key(_) => {
                            if new.get(segs(path)).is_some() {
                                return Err("the path is still there".into());
                            }
                            // Tables left empty above it may stay (or go,
                            // when nothing else defined them).
                            let mut top = path.clone();
                            while top.len() > 1 && new.get(segs(&top[..top.len() - 1])).is_none() {
                                top.pop();
                            }
                            same_outside(&doc, new, &top, Empty::New)
                        }
                    },
                );
            }
            // Reverse and extend arrays.
            if let Value::Array(a) = target
                && !a.is_array_of_tables()
                && !path.is_empty()
            {
                let old: Vec<Item> = a.iter().map(|v| v.to_item()).collect();
                let mut reversed = old.clone();
                reversed.reverse();
                let mut pushed = old.clone();
                pushed.push(mark.clone());
                for (what, want) in [("reverse", reversed), ("push", pushed)] {
                    run(
                        name,
                        &doc,
                        &format!("{what} {path:?}"),
                        &mut stats,
                        |d| {
                            d.edit_array(segs(path), |items| *items = want.clone())
                                .map(drop)
                        },
                        |new| {
                            let got = new.get(segs(path)).ok_or("the array is gone")?.to_item();
                            if !same(&got, &Item::Array(want.clone())) {
                                return Err(format!("the array holds {got:?}"));
                            }
                            same_outside(&doc, new, path, Empty::Neither)
                        },
                    );
                }
            }
        }
    }
    println!(
        "edits: {} made and checked, {} refused as unsupported, {} refused for their path, over {} documents",
        stats.ok,
        stats.unsupported,
        stats.path,
        corpus.len()
    );
    assert!(stats.ok > 5_000, "{}", stats.ok);
}

/// Many edits in a row on one document: positions move with every edit,
/// so later edits find their places in text earlier edits changed. (In
/// debug builds every edit made in place is also compared with a fresh
/// parse of its text.)
#[test]
fn chains_of_edits_on_one_document() {
    let corpus = corpus();
    if corpus.is_empty() {
        return;
    }
    let mut made = 0;
    for (name, text, version) in &corpus {
        let options = ParseOptions::default().with_version(*version);
        let mut doc = Document::parse_with(text, &options).unwrap();
        let mut all = Vec::new();
        paths(Value::Table(doc.root()), &mut Vec::new(), &mut all);
        for (n, path) in all.iter().enumerate().skip(1) {
            let before = doc.clone();
            // Alternate: replace a value, add a key next to it, extend an
            // array, remove something.
            let result = match n % 4 {
                0 => doc.set(segs(path), Item::from(n as i64)),
                1 => {
                    let mut new_path = path[..path.len() - 1].to_vec();
                    new_path.push(P::Key(format!("added-{n}")));
                    doc.set(segs(&new_path), Item::from("new"))
                }
                2 => doc
                    .edit_array(segs(path), |items| items.push(Item::from(true)))
                    .map(drop),
                _ => doc.remove(segs(path)).map(drop),
            };
            match result {
                Ok(()) => {
                    made += 1;
                    // The document is what its text says.
                    let again = Document::parse_with(&doc.to_string(), &options).unwrap();
                    assert!(
                        same(&again.to_item(), &doc.to_item()),
                        "{name}: edit {n} at {path:?}"
                    );
                }
                Err(EditError::Verify(e)) => panic!("{name}: edit {n} at {path:?}: {e}\n{before}"),
                Err(_) => assert_eq!(
                    doc.to_string(),
                    before.to_string(),
                    "{name}: a failed edit changed the text"
                ),
            }
        }
    }
    println!("chains: {made} edits made in sequence");
    assert!(made > 1_000, "{made}");
}
