//! Robustness: every input derived from the official suite by truncation or
//! a single-character mutation must produce `Ok` or `Err` (never a panic
//! or a hang), and the event parser and the composer must agree on it.
//! Whenever it parses:
//! - every byte is covered by a token, a comment or white space;
//! - writing the untouched documents gives the input back, byte for byte;
//! - after an edit (a scalar gets a new value) the output reads back as
//!   the edited documents;
//! - rendering them (`Doc::reformat`) gives YAML that parses back to equal
//!   documents: same content, comments, styles and spellings.
//!
//! Skips quietly when the suite data is missing (see yaml_test_suite.rs).

#[path = "yaml_test_suite/normalize.rs"]
mod normalize;

use std::path::Path;
use std::sync::mpsc;
use std::time::Duration;

const MUTATIONS: &[&str] = &[
    ":", "-", "?", "[", "]", "{", "}", ",", "#", "'", "\"", "|", ">", "&", "*", "!", "%", "@",
    "\\", " ", "\t", "\n", "\r", "\u{FEFF}", "é", "",
];

fn inputs() -> Vec<(String, String)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/yaml-test-suite-data");
    let mut out = Vec::new();
    let Ok(dirs) = std::fs::read_dir(&root) else {
        return out;
    };
    let mut paths: Vec<_> = dirs.filter_map(Result::ok).map(|e| e.path()).collect();
    paths.sort();
    for p in paths {
        let name = p.file_name().unwrap().to_string_lossy().to_string();
        if name == "name" || name == "tags" || name.starts_with('.') || !p.is_dir() {
            continue;
        }
        let mut files = vec![p.join("in.yaml")];
        if let Ok(subs) = std::fs::read_dir(&p) {
            for s in subs.filter_map(Result::ok) {
                files.push(s.path().join("in.yaml"));
            }
        }
        for f in files {
            if let Ok(text) = std::fs::read_to_string(&f) {
                out.push((f.display().to_string(), text));
            }
        }
    }
    out
}

/// Parse to completion; on success also check coverage. Returns a problem
/// description, if any.
fn check(input: &str) -> Option<String> {
    let mut ok = true;
    for r in forgo_lib_yaml::events(input) {
        if r.is_err() {
            ok = false;
            break;
        }
    }
    if !ok {
        // The composer must agree that the input is invalid.
        return forgo_lib_yaml::Doc::from_stream(input)
            .ok()
            .map(|_| "events rejected the input but the composer accepted it".to_string());
    }
    match forgo_lib_yaml::syntax::uncovered_content(input) {
        Ok(gaps) if gaps.is_empty() => {}
        Ok(gaps) => return Some(format!("uncovered content {gaps:?}")),
        Err(e) => return Some(format!("events accepted but scanner failed: {e}")),
    }
    let docs = match forgo_lib_yaml::Doc::from_stream(input) {
        Ok(d) => d,
        // An alias to an undefined anchor is a composition error only.
        Err(e) if e.to_string().contains("alias") => return None,
        Err(e) => return Some(format!("events accepted but composing failed: {e}")),
    };
    // Untouched documents write back as their source, byte for byte.
    match forgo_lib_yaml::Doc::to_stream_string(&docs) {
        Ok(s) if s == input => {}
        Ok(s) => return Some(format!("source not kept: wrote {s:?}")),
        Err(e) => return Some(format!("emit failed: {e}")),
    }
    // An edit: give the last plain or quoted scalar a new value. The
    // output (spliced into the source) must read back as the edited
    // documents.
    let mut edited = docs.clone();
    if let Some(d) = edited
        .iter_mut()
        .rev()
        .find(|d| has_quotable_scalar(d.root()))
    {
        edit_last_scalar(d.root_mut());
        match forgo_lib_yaml::Doc::to_stream_string(&edited) {
            Ok(out) => match forgo_lib_yaml::Doc::from_stream(&out) {
                Ok(back) if back == edited => {}
                Ok(_) => return Some(format!("edit reads back differently: wrote {out:?}")),
                Err(e) => return Some(format!("edited output doesn't parse ({e}): {out:?}")),
            },
            Err(e) => return Some(format!("emit after edit failed: {e}")),
        }
    }
    // The emitter itself: rewrite everything.
    let mut docs = docs;
    for d in &mut docs {
        d.reformat();
    }
    let emitted = match forgo_lib_yaml::Doc::to_stream_string(&docs) {
        Ok(s) => s,
        Err(e) => return Some(format!("emit failed: {e}")),
    };
    let again = match forgo_lib_yaml::Doc::from_stream(&emitted) {
        Ok(d) => d,
        Err(e) => return Some(format!("emitted output doesn't parse ({e}): {emitted:?}")),
    };
    if normalize::normalized_events(input) != normalize::normalized_events(&emitted) {
        return Some(format!("round trip changed content; emitted {emitted:?}"));
    }
    // A placeholder without comments (whitespace-only input) carries nothing.
    let bare = |d: &forgo_lib_yaml::Doc| d.is_placeholder() && d.footer_comments().is_empty();
    let (docs, again): (Vec<_>, Vec<_>) = (
        docs.into_iter().filter(|d| !bare(d)).collect(),
        again.into_iter().filter(|d| !bare(d)).collect(),
    );
    if again != docs {
        let strip = |d: &[forgo_lib_yaml::Doc]| -> Vec<String> {
            let text = format!("{d:#?}");
            let mut out = Vec::new();
            let mut depth = 0usize;
            for line in text.lines() {
                if depth > 0 {
                    depth += line.matches('(').count();
                    depth = depth.saturating_sub(line.matches(')').count());
                    continue;
                }
                if line.trim_start().starts_with("origin: Origin(") {
                    continue;
                }
                if line.trim_start().starts_with("span: SourceSpan(") {
                    depth = line
                        .matches('(')
                        .count()
                        .saturating_sub(line.matches(')').count());
                    continue;
                }
                out.push(line.trim().to_string());
            }
            out
        };
        let (a, b) = (strip(&docs), strip(&again));
        let i = a
            .iter()
            .zip(&b)
            .position(|(x, y)| x != y)
            .unwrap_or(a.len().min(b.len()));
        let ctx = |v: &[String]| {
            v.get(i.saturating_sub(2)..(i + 1).min(v.len()))
                .map(|s| s.join(" "))
                .unwrap_or_default()
        };
        return Some(format!(
            "re-parsed documents differ: [{}] vs [{}]; emitted {emitted:?}",
            ctx(&a),
            ctx(&b)
        ));
    }
    None
}

/// A plain or quoted scalar that can take the value `edited` in its style.
fn quotable(e: &forgo_lib_yaml::Elem) -> bool {
    use forgo_lib_yaml::{Node, Style};
    matches!(e.node, Node::Scalar(_))
        && matches!(
            e.meta.style,
            Some(Style::Plain | Style::SingleQuoted | Style::DoubleQuoted)
        )
        && e.meta.tag.is_none()
}

fn has_quotable_scalar(e: &forgo_lib_yaml::Elem) -> bool {
    use forgo_lib_yaml::Node;
    quotable(e)
        || match &e.node {
            Node::Map(m) => m.iter().any(|(_, v)| has_quotable_scalar(v)),
            Node::Seq(s) => s.iter().any(has_quotable_scalar),
            _ => false,
        }
}

/// Set the last quotable scalar (in document order, values only) to `edited`.
fn edit_last_scalar(e: &mut forgo_lib_yaml::Elem) -> bool {
    use forgo_lib_yaml::{Node, Scalar};
    let done = match &mut e.node {
        Node::Map(m) => m.iter_mut().rev().any(|(_, v)| edit_last_scalar(v)),
        Node::Seq(s) => s.iter_mut().rev().any(edit_last_scalar),
        _ => false,
    };
    if done || !quotable(e) {
        return done;
    }
    e.node = Node::Scalar(Scalar::Str("edited".into()));
    e.meta.spelling = None;
    true
}

fn variants(text: &str) -> Vec<String> {
    let bounds: Vec<usize> = text
        .char_indices()
        .map(|(i, _)| i)
        .chain(std::iter::once(text.len()))
        .collect();
    let mut out = Vec::new();
    for &b in &bounds {
        out.push(text[..b].to_string());
    }
    for w in bounds.windows(2) {
        let (a, b) = (w[0], w[1]);
        for m in MUTATIONS {
            let mut s = String::with_capacity(text.len() + 4);
            s.push_str(&text[..a]);
            s.push_str(m);
            s.push_str(&text[b..]);
            out.push(s);
            // insertion
            let mut s = String::with_capacity(text.len() + 4);
            s.push_str(&text[..a]);
            s.push_str(m);
            s.push_str(&text[a..]);
            out.push(s);
        }
    }
    out
}

#[test]
fn mutated_suite_inputs_never_panic() {
    let inputs = inputs();
    if inputs.is_empty() {
        eprintln!("SKIPPING robustness test: yaml-test-suite data missing");
        return;
    }
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if std::thread::current().name() != Some("robustness") {
            default_hook(info);
        }
    }));

    // Spread files over worker threads; each reports per file.
    let workers = std::thread::available_parallelism()
        .map_or(4, |n| n.get())
        .min(8);
    let mut chunks: Vec<Vec<(String, String)>> = vec![Vec::new(); workers];
    let expected = inputs.len();
    for (i, item) in inputs.into_iter().enumerate() {
        chunks[i % workers].push(item);
    }
    let (tx, rx) = mpsc::channel::<(String, usize, Vec<String>)>();
    for chunk in chunks {
        let tx = tx.clone();
        std::thread::Builder::new()
            .name("robustness".into())
            .spawn(move || {
                for (name, text) in chunk {
                    let vars = variants(&text);
                    let count = vars.len();
                    let mut found = Vec::new();
                    for v in vars {
                        let v2 = v.clone();
                        match std::panic::catch_unwind(move || check(&v2)) {
                            Ok(None) => {}
                            Ok(Some(p)) => found.push(format!("{p} on {v:?}")),
                            Err(_) => found.push(format!("panic on {v:?}")),
                        }
                        if found.len() > 50 {
                            break;
                        }
                    }
                    let _ = tx.send((name, count, found));
                }
            })
            .unwrap();
    }
    drop(tx);
    let mut problems = Vec::new();
    let mut total = 0usize;
    let mut done = 0usize;
    while done < expected {
        match rx.recv_timeout(Duration::from_secs(120)) {
            Ok((name, count, found)) => {
                done += 1;
                total += count;
                problems.extend(found.into_iter().map(|p| format!("{name}: {p}")));
            }
            Err(_) => {
                problems.push(format!(
                    "{} input(s) did not finish within 120s (possible infinite loop)",
                    expected - done
                ));
                break;
            }
        }
    }
    println!("robustness: {total} variants checked");
    assert!(
        problems.is_empty(),
        "{} problems:\n{}",
        problems.len(),
        problems.join("\n")
    );
}

#[test]
fn deep_nesting_is_an_error_not_a_crash() {
    // The event parser handles any depth; composing stops at the limit.
    let deep = format!("{}{}", "[".repeat(100_000), "]".repeat(100_000));
    assert_eq!(forgo_lib_yaml::events(&deep).count(), 200_004);
    let err = forgo_lib_yaml::Doc::from_str(&deep)
        .unwrap_err()
        .to_string();
    assert!(err.contains("nested more than 512"), "{err}");
    let block: String = (0..2000)
        .map(|i| format!("{}a:\n", "  ".repeat(i)))
        .collect();
    assert!(forgo_lib_yaml::Doc::from_str(&block).is_err());

    // At the limit, parse, emit, clone, compare and drop all work.
    let ok = format!("{}{}", "[".repeat(512), "]".repeat(512));
    let d = forgo_lib_yaml::Doc::from_str(&ok).unwrap();
    let out = d.to_string().unwrap();
    assert_eq!(forgo_lib_yaml::Doc::from_str(&out).unwrap(), d.clone());
    let opts = forgo_lib_yaml::ParseOptions { max_depth: 10 };
    assert!(forgo_lib_yaml::Doc::from_str_with(&ok, &opts).is_err());
}
