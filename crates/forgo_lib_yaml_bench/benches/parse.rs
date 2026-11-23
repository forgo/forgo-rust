//! Criterion benchmarks: parsing to events and loading to a tree, per
//! library and input. The `report` binary covers the same ground in one
//! run, plus compliance; these give criterion's statistics.
//!
//!   cargo bench --manifest-path crates/forgo_lib_yaml_bench/Cargo.toml

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use forgo_lib_yaml_bench::libs::CountFn;
use forgo_lib_yaml_bench::{EVENT_LIBS, LOAD_LIBS, small_document, suite_valid, synthetic_inputs};
use std::hint::black_box;

fn group(c: &mut Criterion, name: &str, libs: &[(&str, CountFn)]) {
    for input in synthetic_inputs().into_iter().chain([small_document()]) {
        let mut g = c.benchmark_group(format!("{name}/{}", input.name));
        g.throughput(Throughput::Bytes(input.text.len() as u64));
        for (lib, f) in libs {
            if let Err(e) = f(&input.text) {
                eprintln!("skipping {lib} on {}: {e}", input.name);
                continue;
            }
            g.bench_function(*lib, |b| b.iter(|| f(black_box(&input.text))));
        }
        g.finish();
    }

    // The valid suite files every library accepts, parsed one by one.
    let suite = suite_valid();
    let common: Vec<&str> = suite
        .iter()
        .map(|(_, t)| t.as_str())
        .filter(|t| libs.iter().all(|(_, f)| f(t).is_ok()))
        .collect();
    if common.is_empty() {
        return;
    }
    let bytes: usize = common.iter().map(|t| t.len()).sum();
    let mut g = c.benchmark_group(format!("{name}/suite-{}-files", common.len()));
    g.throughput(Throughput::Bytes(bytes as u64));
    for (lib, f) in libs {
        g.bench_function(*lib, |b| {
            b.iter(|| {
                for t in &common {
                    let _ = f(black_box(t));
                }
            })
        });
    }
    g.finish();
}

fn events(c: &mut Criterion) {
    let libs: Vec<_> = EVENT_LIBS.iter().map(|l| (l.name, l.count)).collect();
    group(c, "events", &libs);
}

fn load(c: &mut Criterion) {
    // rust-yaml is quadratic on two inputs (seconds per run); see the report.
    let libs: Vec<_> = LOAD_LIBS
        .iter()
        .filter(|l| l.name != "rust-yaml")
        .map(|l| (l.name, l.bench))
        .collect();
    group(c, "load", &libs);
}

criterion_group!(benches, events, load);
criterion_main!(benches);
