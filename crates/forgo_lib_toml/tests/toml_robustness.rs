//! Robustness: inputs derived from every toml-test case (valid and
//! invalid) by truncation and one-character edits. None may panic or hang;
//! every input that parses must encode to TOML that parses back to the
//! same values. Deep nesting is an error, not a stack overflow.

use forgo_lib_toml::{Document, ErrorKind, Item, ParseOptions, TomlVersion};
use std::path::Path;

fn corpus() -> Vec<Vec<u8>> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/toml-test-data/tests");
    let Ok(list) = std::fs::read_to_string(dir.join("files-toml-1.1.0")) else {
        assert!(
            std::env::var("TOML_TEST_SUITE_REQUIRED").as_deref() != Ok("1"),
            "toml-test data is missing; run scripts/fetch-toml-test.sh"
        );
        eprintln!("skipped: no toml-test data (scripts/fetch-toml-test.sh)");
        return Vec::new();
    };
    list.lines()
        .filter(|l| l.ends_with(".toml"))
        .filter_map(|l| std::fs::read(dir.join(l)).ok())
        .collect()
}

const EDITS: &[&[u8]] = &[
    b"\"",
    b"'",
    b"=",
    b".",
    b"[",
    b"]",
    b"{",
    b"}",
    b",",
    b"#",
    b"\n",
    b"\r",
    b"\r\n",
    b" ",
    b"\\",
    b"0",
    b"e",
    b"-",
    b"+",
    b":",
    b"T",
    b"Z",
    b"_",
    b"x",
    b"\t",
    b"\x7f",
    b"\xc3\xa9",
];

struct Stats {
    inputs: usize,
    parsed: usize,
    slowest: std::time::Duration,
}

fn check(bytes: &[u8], opts: &ParseOptions, stats: &mut Stats) {
    stats.inputs += 1;
    let start = std::time::Instant::now();
    let parsed = Document::from_slice(bytes, opts);
    stats.slowest = stats.slowest.max(start.elapsed());
    let Ok(doc) = parsed else {
        return;
    };
    stats.parsed += 1;
    let item = doc.to_item();
    let text = item.to_toml_string().expect("a document is a table");
    let back = Document::parse_with(
        &text,
        &ParseOptions::default().with_version(TomlVersion::V1_0),
    )
    .unwrap_or_else(|e| {
        panic!(
            "{}\nencodes to unparsable {text:?}: {e}",
            String::from_utf8_lossy(bytes)
        )
    });
    assert!(
        same(&back.to_item(), &item),
        "{}\nencodes to {text:?}, which reads back differently",
        String::from_utf8_lossy(bytes)
    );
}

/// Equal as TOML values: tables in any order, any NaN equal to any NaN.
fn same(a: &Item, b: &Item) -> bool {
    match (a, b) {
        (Item::Float(x), Item::Float(y)) => (x.is_nan() && y.is_nan()) || x == y,
        (Item::Array(x), Item::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(p, q)| same(p, q))
        }
        (Item::Table(x), Item::Table(y)) => {
            x.len() == y.len()
                && x.iter()
                    .all(|(k, p)| y.iter().any(|(l, q)| k == l && same(p, q)))
        }
        _ => a == b,
    }
}

#[test]
fn mutated_inputs_never_panic_and_round_trip() {
    let corpus = corpus();
    if corpus.is_empty() {
        return;
    }
    let mut stats = Stats {
        inputs: 0,
        parsed: 0,
        slowest: std::time::Duration::ZERO,
    };
    for opts in [
        ParseOptions::default(),
        ParseOptions::default().with_version(TomlVersion::V1_0),
    ] {
        for case in &corpus {
            // Every truncation.
            for end in 0..case.len() {
                check(&case[..end], &opts, &mut stats);
            }
            // At up to 20 positions: a deletion, and every edit both
            // replacing the byte there and inserted before it.
            let step = (case.len() / 20).max(1);
            for i in (0..case.len()).step_by(step) {
                let mut deleted = case.clone();
                deleted.remove(i);
                check(&deleted, &opts, &mut stats);
                for e in EDITS {
                    let mut replaced = case.clone();
                    replaced.splice(i..i + 1, e.iter().copied());
                    check(&replaced, &opts, &mut stats);
                    let mut inserted = case.clone();
                    inserted.splice(i..i, e.iter().copied());
                    check(&inserted, &opts, &mut stats);
                }
            }
        }
    }
    println!(
        "robustness: {} inputs, {} parsed and round-tripped; slowest parse {:?}",
        stats.inputs, stats.parsed, stats.slowest
    );
    assert!(stats.inputs > 1_000_000, "{}", stats.inputs);
    // Linear time: no input of these sizes takes long.
    assert!(stats.slowest.as_millis() < 250, "{:?}", stats.slowest);
}

#[test]
fn deep_nesting_is_an_error() {
    for (open, close) in [("[", "]"), ("{ a = ", " }")] {
        let deep = format!("x = {}1{}", open.repeat(100_000), close.repeat(100_000));
        let err = Document::parse(&deep).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::Depth, "{open}");
    }
    let ok = format!("x = {}1{}", "[".repeat(128), "]".repeat(128));
    assert!(Document::parse(&ok).is_ok());
    let limited = ParseOptions::default().with_max_depth(8);
    assert!(
        Document::parse_with(
            &format!("x = {}1{}", "[".repeat(9), "]".repeat(9)),
            &limited
        )
        .is_err()
    );
}
