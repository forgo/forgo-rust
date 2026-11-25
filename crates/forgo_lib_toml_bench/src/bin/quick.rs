//! A quick parse-throughput check: each crate parses each input (after
//! checking that it accepts it), best of five runs.
//!
//! `cargo run --release --bin quick`
use std::hint::black_box;
use std::time::Instant;

/// MB/s for `f` over `input`, or `None` when `f` rejects it.
fn mbps(input: &str, mut f: impl FnMut(&str) -> bool) -> Option<f64> {
    if !f(input) {
        return None;
    }
    f(input);
    let mut best = f64::MAX;
    for _ in 0..5 {
        let t = Instant::now();
        let mut n = 0;
        while t.elapsed().as_millis() < 300 {
            black_box(f(black_box(input)));
            n += 1;
        }
        best = best.min(t.elapsed().as_secs_f64() / n as f64);
    }
    Some(input.len() as f64 / best / 1e6)
}

type Parser = (&'static str, fn(&str) -> bool);

const PARSERS: &[Parser] = &[
    ("forgo_lib_toml", |s| {
        forgo_lib_toml::Document::parse(s).is_ok()
    }),
    ("toml-spanner", |s| {
        let arena = toml_spanner::Arena::new();
        toml_spanner::parse(s, &arena).is_ok()
    }),
    ("toml (Table)", |s| s.parse::<toml::Table>().is_ok()),
    ("toml (DeTable)", |s| toml::de::DeTable::parse(s).is_ok()),
    ("toml_edit (DocumentMut)", |s| {
        s.parse::<toml_edit::DocumentMut>().is_ok()
    }),
    ("toml-span", |s| toml_span::parse(s).is_ok()),
    ("basic-toml", |s| {
        basic_toml::from_str::<serde_json::Value>(s).is_ok()
    }),
    ("boml", |s| boml::parse(s).is_ok()),
    ("taplo (syntax tree)", |s| {
        taplo::parser::parse(s).errors.is_empty()
    }),
];

fn main() {
    let inputs = [
        ("lockfile (1 MB)", forgo_lib_toml_bench::lockfile(4500)),
        ("mixed (1 MB)", forgo_lib_toml_bench::mixed(1024)),
        (
            "manifest (1 KB)",
            forgo_lib_toml_bench::MANIFEST.to_string(),
        ),
    ];
    for (what, input) in &inputs {
        println!("{what}: {} bytes", input.len());
        for (name, f) in PARSERS {
            match mbps(input, f) {
                Some(v) => println!("  {name:28} {v:8.1} MB/s"),
                None => println!("  {name:28}   rejects the input"),
            }
        }
    }
}
