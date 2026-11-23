//! Markdown report: throughput, allocations and peak heap per parser and
//! input, plus accept/reject agreement with the official yaml-test-suite.
//!
//!   cargo run --release --manifest-path crates/forgo_lib_yaml_bench/Cargo.toml --bin report

use forgo_lib_yaml_bench::{Adapter, EVENT_PARSERS, LOADERS, synthetic_inputs};
use std::alloc::{GlobalAlloc, Layout, System};
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};
use std::time::Instant;

struct Counting;

static ALLOCS: AtomicUsize = AtomicUsize::new(0);
static BYTES: AtomicUsize = AtomicUsize::new(0);
static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

fn track(grow: usize) {
    let live = LIVE.fetch_add(grow, Relaxed) + grow;
    PEAK.fetch_max(live, Relaxed);
}

// SAFETY: delegates to the system allocator and only adds counters.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        ALLOCS.fetch_add(1, Relaxed);
        BYTES.fetch_add(l.size(), Relaxed);
        track(l.size());
        unsafe { System.alloc(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        LIVE.fetch_sub(l.size(), Relaxed);
        unsafe { System.dealloc(p, l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, new: usize) -> *mut u8 {
        ALLOCS.fetch_add(1, Relaxed);
        if new > l.size() {
            BYTES.fetch_add(new - l.size(), Relaxed);
            track(new - l.size());
        } else {
            LIVE.fetch_sub(l.size() - new, Relaxed);
        }
        unsafe { System.realloc(p, l, new) }
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

struct Row {
    parser: &'static str,
    mb_s: f64,
    allocs: usize,
    bytes: usize,
    peak: usize,
}

fn measure(name: &'static str, f: Adapter, text: &str) -> Option<Row> {
    f(text).ok()?;
    ALLOCS.store(0, Relaxed);
    BYTES.store(0, Relaxed);
    let base = LIVE.load(Relaxed);
    PEAK.store(base, Relaxed);
    let _ = f(text);
    let (allocs, bytes, peak) = (
        ALLOCS.load(Relaxed),
        BYTES.load(Relaxed),
        PEAK.load(Relaxed) - base,
    );
    let mut times: Vec<f64> = (0..11)
        .map(|_| {
            let t = Instant::now();
            let _ = std::hint::black_box(f(std::hint::black_box(text)));
            t.elapsed().as_secs_f64()
        })
        .collect();
    times.sort_by(f64::total_cmp);
    let mb_s = text.len() as f64 / times[times.len() / 2] / 1e6;
    Some(Row {
        parser: name,
        mb_s,
        allocs,
        bytes,
        peak,
    })
}

fn table(title: &str, parsers: &[(&'static str, Adapter)]) {
    println!("\n## {title}\n");
    for input in synthetic_inputs() {
        println!(
            "### {} ({:.2} MB)\n",
            input.name,
            input.text.len() as f64 / 1e6
        );
        println!("| parser | MB/s | allocations | allocated MB | peak heap MB |");
        println!("|---|---:|---:|---:|---:|");
        for (name, f) in parsers {
            match measure(name, *f, &input.text) {
                Some(r) => println!(
                    "| {} | {:.1} | {} | {:.2} | {:.2} |",
                    r.parser,
                    r.mb_s,
                    r.allocs,
                    r.bytes as f64 / 1e6,
                    r.peak as f64 / 1e6
                ),
                None => println!("| {name} | rejected | | | |"),
            }
        }
        println!();
    }
}

/// Accept/reject agreement with the suite (not event equality).
fn compliance(title: &str, parsers: &[(&'static str, Adapter)]) {
    let root =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../forgo_lib_yaml/tests/yaml-test-suite-data");
    let mut cases = Vec::new();
    let Ok(entries) = std::fs::read_dir(&root) else {
        return;
    };
    for e in entries.filter_map(Result::ok) {
        let d = e.path();
        let id = d.file_name().unwrap().to_string_lossy().to_string();
        if id == "name" || id == "tags" || id.starts_with('.') || !d.is_dir() {
            continue;
        }
        let dirs: Vec<_> = if d.join("in.yaml").exists() {
            vec![d]
        } else {
            std::fs::read_dir(&d)
                .unwrap()
                .filter_map(Result::ok)
                .map(|e| e.path())
                .collect()
        };
        for dir in dirs {
            if let Ok(text) = std::fs::read_to_string(dir.join("in.yaml")) {
                cases.push((text, dir.join("error").exists()));
            }
        }
    }
    let valid = cases.iter().filter(|c| !c.1).count();
    let invalid = cases.len() - valid;
    println!(
        "\n## yaml-test-suite accept/reject, {title} ({} cases)\n",
        cases.len()
    );
    println!("| parser | valid accepted (of {valid}) | invalid rejected (of {invalid}) | total |");
    println!("|---|---:|---:|---:|");
    for (name, f) in parsers {
        let (mut ok_valid, mut ok_invalid) = (0, 0);
        for (text, is_error) in &cases {
            let accepted = std::panic::catch_unwind(|| f(text).is_ok()).unwrap_or(false);
            match (is_error, accepted) {
                (false, true) => ok_valid += 1,
                (true, false) => ok_invalid += 1,
                _ => {}
            }
        }
        println!(
            "| {name} | {ok_valid} | {ok_invalid} | {:.1}% |",
            100.0 * (ok_valid + ok_invalid) as f64 / cases.len() as f64
        );
    }
}

fn main() {
    println!("# forgo_lib_yaml benchmark report");
    table("Event parsing", EVENT_PARSERS);
    table("Loading documents", LOADERS);
    std::panic::set_hook(Box::new(|_| {}));
    compliance("event parsers", EVENT_PARSERS);
    compliance("loaders", LOADERS);
}
