//! Format-preserving editing.
//!
//! Untouched documents write back byte for byte (`yaml_test_suite` checks
//! that on the whole official suite). Here each input is edited many ways,
//! one edit at a time, and the output must be exactly the source with only
//! the edited entry changed:
//!
//! - `replace`: a scalar value of a block collection gets a new value:
//!   only its text changes;
//! - `delete`: an entry (not the first) of a block collection is removed:
//!   exactly its lines go, with the comments and blank lines above it;
//! - `append`: a block collection gets a new last entry: exactly one new
//!   line, right after the last entry;
//! - `rename`: a plain key whose value is on its line is renamed: only the
//!   key's text changes.
//!
//! Every output must also read back as the edited documents. Inputs: the
//! files in `tests/editing/` (also with CRLF line breaks, and without a
//! final line break), and every valid case of the official suite when its
//! data is present.

use forgo_lib_yaml::{Doc, Elem, MapKey, Node, Scalar, Style};
use std::path::Path;

#[derive(Clone, Copy, Debug)]
enum Step {
    /// The value of a mapping entry.
    Value(usize),
    /// A sequence item.
    Item(usize),
}

fn get<'a>(mut e: &'a Elem, path: &[Step]) -> &'a Elem {
    for s in path {
        e = match (s, &e.node) {
            (Step::Value(i), Node::Map(m)) => &m[*i].1,
            (Step::Item(i), Node::Seq(v)) => &v[*i],
            _ => unreachable!("path follows the tree"),
        };
    }
    e
}

fn get_mut<'a>(mut e: &'a mut Elem, path: &[Step]) -> &'a mut Elem {
    for s in path {
        e = match (s, &mut e.node) {
            (Step::Value(i), Node::Map(m)) => &mut m[*i].1,
            (Step::Item(i), Node::Seq(v)) => &mut v[*i],
            _ => unreachable!("path follows the tree"),
        };
    }
    e
}

fn block(e: &Elem) -> bool {
    let non_empty = match &e.node {
        Node::Map(m) => !m.is_empty(),
        Node::Seq(s) => !s.is_empty(),
        _ => false,
    };
    non_empty && e.meta.style != Some(Style::Flow)
}

/// Paths of the block collections reachable through block collections.
fn collections(e: &Elem, path: &mut Vec<Step>, out: &mut Vec<Vec<Step>>) {
    if !block(e) {
        return;
    }
    out.push(path.clone());
    let children: Vec<(Step, &Elem)> = match &e.node {
        Node::Map(m) => m
            .iter()
            .enumerate()
            .map(|(i, (_, v))| (Step::Value(i), v))
            .collect(),
        Node::Seq(s) => s
            .iter()
            .enumerate()
            .map(|(i, v)| (Step::Item(i), v))
            .collect(),
        _ => Vec::new(),
    };
    for (step, child) in children {
        path.push(step);
        collections(child, path, out);
        path.pop();
    }
}

fn children(e: &Elem) -> Vec<&Elem> {
    match &e.node {
        Node::Map(m) => m.iter().map(|(_, v)| v).collect(),
        Node::Seq(s) => s.iter().collect(),
        _ => Vec::new(),
    }
}

/// Position after the line break of the line holding the byte before `pos`.
fn line_end_after(text: &str, pos: usize) -> usize {
    if pos == 0 || text.as_bytes()[pos - 1] == b'\n' {
        return pos;
    }
    text[pos..].find('\n').map_or(text.len(), |i| pos + i + 1)
}

fn line_start(text: &str, pos: usize) -> usize {
    text[..pos].rfind('\n').map_or(0, |i| i + 1)
}

/// The node's source range, if it has text.
fn range(e: &Elem) -> Option<std::ops::Range<usize>> {
    e.meta.span.range().filter(|r| r.end > r.start)
}

/// Ends in a `|+` or `>+` block scalar (whose trailing blank lines are
/// content, so they belong to it).
fn ends_with_keep(e: &Elem) -> bool {
    match &e.node {
        Node::Map(m) if block(e) => m.last().is_some_and(|(_, v)| ends_with_keep(v)),
        Node::Seq(s) if block(e) => s.last().is_some_and(ends_with_keep),
        _ => matches!(
            e.meta.style,
            Some(Style::Literal(h) | Style::Folded(h)) if h.chomping == forgo_lib_yaml::Chomping::Keep
        ),
    }
}

/// Removing an anchor would leave its aliases dangling.
fn has_anchor(e: &Elem) -> bool {
    e.meta.anchor.is_some()
        || match &e.node {
            Node::Map(m) => m
                .iter()
                .any(|(k, v)| has_anchor(v) || matches!(k, MapKey::Complex(k) if has_anchor(k))),
            Node::Seq(s) => s.iter().any(has_anchor),
            _ => false,
        }
}

struct Run<'a> {
    name: &'a str,
    docs: Vec<Doc>,
    edits: usize,
    failures: Vec<String>,
}

impl Run<'_> {
    /// Apply `edit` to a copy of document `d` and compare the output with
    /// `expected`.
    fn check(
        &mut self,
        kind: &str,
        d: usize,
        path: &[Step],
        expected: String,
        edit: impl FnOnce(&mut Doc),
    ) {
        self.edits += 1;
        let mut docs = self.docs.clone();
        edit(&mut docs[d]);
        let out = match Doc::to_stream_string(&docs) {
            Ok(o) => o,
            Err(e) => {
                self.failures.push(format!(
                    "{} {kind} doc {d} {path:?}: emit failed: {e}",
                    self.name
                ));
                return;
            }
        };
        if out != expected {
            self.failures.push(format!(
                "{} {kind} doc {d} {path:?}:\n  expected {expected:?}\n  got      {out:?}",
                self.name
            ));
            return;
        }
        match Doc::from_stream(&out) {
            Ok(back) if back == docs => {}
            Ok(_) => self.failures.push(format!(
                "{} {kind} doc {d} {path:?}: reads back differently",
                self.name
            )),
            Err(e) => self.failures.push(format!(
                "{} {kind} doc {d} {path:?}: output doesn't parse: {e}",
                self.name
            )),
        }
    }
}

fn edit_all(name: &str, text: &str) -> (usize, Vec<String>) {
    let docs = match Doc::from_stream(text) {
        Ok(d) => d,
        Err(_) => return (0, Vec::new()),
    };
    let mut run = Run {
        name,
        docs,
        edits: 0,
        failures: Vec::new(),
    };
    let nl = if text.contains("\r\n") { "\r\n" } else { "\n" };
    for d in 0..run.docs.len() {
        let mut paths = Vec::new();
        collections(run.docs[d].root(), &mut Vec::new(), &mut paths);
        for cpath in paths {
            let c = get(run.docs[d].root(), &cpath).clone();
            let kids = children(&c);
            let step = |i: usize| match c.node {
                Node::Map(_) => Step::Value(i),
                _ => Step::Item(i),
            };

            // replace
            for (i, v) in kids.iter().enumerate() {
                let (Node::Scalar(_), Some(r)) = (&v.node, range(v)) else {
                    continue;
                };
                let quoted = match v.meta.style {
                    Some(Style::Plain) => "edited",
                    Some(Style::SingleQuoted) => "'edited'",
                    Some(Style::DoubleQuoted) => "\"edited\"",
                    _ => continue,
                };
                if v.meta.anchor.is_some() || v.meta.tag.is_some() {
                    continue;
                }
                let mut path = cpath.clone();
                path.push(step(i));
                let expected = format!("{}{quoted}{}", &text[..r.start], &text[r.end..]);
                let p = path.clone();
                run.check("replace", d, &path, expected, move |doc| {
                    let e = get_mut(doc.root_mut(), &p);
                    e.node = Node::Scalar(Scalar::Str("edited".into()));
                    e.meta.spelling = None;
                });
            }

            // delete
            for i in 1..kids.len() {
                let (prev, cur) = (kids[i - 1], kids[i]);
                let (Some(a), Some(b)) = (range(prev), range(cur)) else {
                    continue;
                };
                if ends_with_keep(prev) || ends_with_keep(cur) || has_anchor(cur) {
                    continue;
                }
                if let Node::Map(m) = &c.node
                    && matches!(&m[i].0, MapKey::Complex(k) if k.meta.span.range().is_some_and(|r| r.start < a.end))
                {
                    continue;
                }
                let (a, b) = (line_end_after(text, a.end), line_end_after(text, b.end));
                let expected = format!("{}{}", &text[..a], &text[b..]);
                let p = cpath.clone();
                run.check(
                    "delete",
                    d,
                    &cpath,
                    expected,
                    move |doc| match &mut get_mut(doc.root_mut(), &p).node {
                        Node::Map(m) => drop(m.remove(i)),
                        Node::Seq(s) => drop(s.remove(i)),
                        _ => unreachable!(),
                    },
                );
            }

            // append
            let last = kids[kids.len() - 1];
            if let (Some(r), Some(cr)) = (range(last), c.meta.span.range())
                && c.meta.anchor.is_none()
                && c.meta.tag.is_none()
                && !ends_with_keep(last)
            {
                let col = cr.start - line_start(text, cr.start);
                let line = match c.node {
                    Node::Map(_) => format!("{}added: value{nl}", " ".repeat(col)),
                    _ => format!("{}- added{nl}", " ".repeat(col)),
                };
                let at = line_end_after(text, r.end);
                let expected = if at == text.len() && !text.ends_with('\n') {
                    format!("{text}{nl}{line}")
                } else {
                    format!("{}{line}{}", &text[..at], &text[at..])
                };
                let p = cpath.clone();
                run.check(
                    "append",
                    d,
                    &cpath,
                    expected,
                    move |doc| match &mut get_mut(doc.root_mut(), &p).node {
                        Node::Map(m) => m.push((
                            MapKey::from("added"),
                            Elem::string("value").with_style(Style::Plain),
                        )),
                        Node::Seq(s) => s.push(Elem::string("added").with_style(Style::Plain)),
                        _ => unreachable!(),
                    },
                );
            }

            // rename
            if let Node::Map(m) = &c.node {
                for (i, (k, v)) in m.iter().enumerate() {
                    let (MapKey::Str(k), Some(r)) = (k, range(v)) else {
                        continue;
                    };
                    let ls = line_start(text, r.start);
                    let Some(colon) = text[ls..r.start].rfind(':').map(|c| ls + c) else {
                        continue;
                    };
                    let key_end = ls + text[ls..colon].trim_end_matches([' ', '\t']).len();
                    if k.is_empty()
                        || key_end < k.len()
                        || text.get(key_end - k.len()..key_end) != Some(k.as_str())
                    {
                        continue;
                    }
                    if !text[colon + 1..r.start]
                        .trim_matches([' ', '\t'])
                        .is_empty()
                    {
                        continue;
                    }
                    let expected =
                        format!("{}renamed{}", &text[..key_end - k.len()], &text[key_end..]);
                    let p = cpath.clone();
                    run.check("rename", d, &cpath, expected, move |doc| {
                        if let Node::Map(m) = &mut get_mut(doc.root_mut(), &p).node {
                            m[i].0 = MapKey::from("renamed");
                        }
                    });
                }
            }
        }
    }
    (run.edits, run.failures)
}

fn inputs() -> Vec<(String, String)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let mut out = Vec::new();
    for name in ["k8s.yaml", "workflow.yaml", "compose.yaml"] {
        let text = std::fs::read_to_string(dir.join("editing").join(name)).unwrap();
        out.push((format!("{name} (CRLF)"), text.replace('\n', "\r\n")));
        out.push((
            format!("{name} (no final line break)"),
            text.trim_end_matches('\n').to_string(),
        ));
        out.push((name.to_string(), text));
    }
    let suite = dir.join("yaml-test-suite-data");
    if let Ok(entries) = std::fs::read_dir(&suite) {
        let mut cases: Vec<_> = entries.flatten().map(|e| e.path()).collect();
        cases.sort();
        for case in cases {
            // `name/` and `tags/` index the cases again by symlinks.
            if case.ends_with("name") || case.ends_with("tags") {
                continue;
            }
            let mut dirs = vec![case.clone()];
            if let Ok(sub) = std::fs::read_dir(&case) {
                let mut sub: Vec<_> = sub
                    .flatten()
                    .map(|e| e.path())
                    .filter(|p| p.is_dir())
                    .collect();
                sub.sort();
                dirs.extend(sub);
            }
            for d in dirs {
                let (input, error) = (d.join("in.yaml"), d.join("error"));
                if input.exists()
                    && !error.exists()
                    && let Ok(text) = std::fs::read_to_string(&input)
                {
                    let id = d.strip_prefix(&suite).unwrap().display().to_string();
                    out.push((format!("suite {id}"), text));
                }
            }
        }
    }
    out
}

#[test]
fn edits_change_only_the_edited_entry() {
    let mut total = 0;
    let mut failures = Vec::new();
    let inputs = inputs();
    for (name, text) in &inputs {
        let (n, f) = edit_all(name, text);
        total += n;
        failures.extend(f);
    }
    println!(
        "editing: {total} edits on {} inputs, {} failures",
        inputs.len(),
        failures.len()
    );
    assert!(
        failures.is_empty(),
        "{} of {total} edits changed more than the edited entry:\n{}",
        failures.len(),
        failures
            .iter()
            .take(25)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn edit_file_in_place_keeps_every_document() {
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join("edit_file_in_place.yaml");
    let text = "\
# Dependencies of each service.
deps: [b, a]   # unsorted
other: 1
---
deps:
- z    # last
- y
";
    std::fs::write(&path, text).unwrap();
    let wrote = forgo_lib_yaml::edit_file_in_place(&path, &["deps".into()], |seq| {
        seq.push(Elem::string("c"));
        true
    })
    .unwrap();
    assert!(wrote);
    // Both documents stay; only the two lists change, and the second
    // list's items move as whole lines (with their comments).
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        "\
# Dependencies of each service.
deps: [a, b, c]   # unsorted
other: 1
---
deps:
- c
- y
- z    # last
"
    );
}
