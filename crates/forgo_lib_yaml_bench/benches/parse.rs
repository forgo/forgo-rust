//! Throughput of event parsing and document loading, per parser.
//!
//!   cargo bench --manifest-path crates/forgo_lib_yaml_bench/Cargo.toml

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use forgo_lib_yaml_bench::{
    Adapter, EVENT_PARSERS, Input, LOADERS, suite_inputs, synthetic_inputs,
};
use std::hint::black_box;

fn bench_group(c: &mut Criterion, group: &str, parsers: &[(&str, Adapter)]) {
    for input in synthetic_inputs() {
        let mut g = c.benchmark_group(format!("{group}/{}", input.name));
        g.throughput(Throughput::Bytes(input.text.len() as u64));
        for (name, f) in parsers {
            if let Err(e) = f(&input.text) {
                eprintln!("skipping {name} on {}: {e}", input.name);
                continue;
            }
            g.bench_function(*name, |b| b.iter(|| f(black_box(&input.text))));
        }
        g.finish();
    }

    // Every valid suite file that all parsers accept, parsed one by one.
    let suite = suite_inputs();
    let common: Vec<&Input> = suite
        .iter()
        .filter(|i| parsers.iter().all(|(_, f)| f(&i.text).is_ok()))
        .collect();
    if common.is_empty() {
        return;
    }
    let bytes: usize = common.iter().map(|i| i.text.len()).sum();
    let mut g = c.benchmark_group(format!("{group}/suite-{}-files", common.len()));
    g.throughput(Throughput::Bytes(bytes as u64));
    for (name, f) in parsers {
        g.bench_function(*name, |b| {
            b.iter(|| {
                for i in &common {
                    let _ = f(black_box(&i.text));
                }
            })
        });
    }
    g.finish();
}

fn events(c: &mut Criterion) {
    bench_group(c, "events", EVENT_PARSERS);
}

fn load(c: &mut Criterion) {
    bench_group(c, "load", LOADERS);
}

criterion_group!(benches, events, load);
criterion_main!(benches);
