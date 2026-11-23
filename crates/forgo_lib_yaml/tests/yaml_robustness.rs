//! Robustness: every input derived from the official suite by truncation or
//! a single-character mutation must produce `Ok` or `Err` (never a panic
//! or a hang), and whenever the scanner accepts it, every byte must be
//! covered by a token, a comment or white space.
//!
//! Skips quietly when the suite data is missing (see yaml_test_suite.rs).

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
    if ok {
        match forgo_lib_yaml::syntax::uncovered_content(input) {
            Ok(gaps) if gaps.is_empty() => {}
            Ok(gaps) => return Some(format!("uncovered content {gaps:?}")),
            Err(e) => return Some(format!("events accepted but scanner failed: {e}")),
        }
    }
    None
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
                        if found.len() > 3 {
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
