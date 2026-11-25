//! Loops one parser over one input, for a sampling profiler.
//! `prof <lockfile|mixed|manifest> <seconds>`
use std::hint::black_box;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

struct Counting;
static ALLOCS: AtomicUsize = AtomicUsize::new(0);
unsafe impl std::alloc::GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: std::alloc::Layout) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        unsafe { std::alloc::System.alloc(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: std::alloc::Layout) {
        unsafe { std::alloc::System.dealloc(p, l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: std::alloc::Layout, n: usize) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        unsafe { std::alloc::System.realloc(p, l, n) }
    }
}
#[global_allocator]
static A: Counting = Counting;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let input = match args.get(1).map(String::as_str) {
        Some("lockfile") => forgo_lib_toml_bench::lockfile(4500),
        Some("mixed") => forgo_lib_toml_bench::mixed(1024),
        _ => forgo_lib_toml_bench::MANIFEST.to_string(),
    };
    let secs: u64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);
    if args.get(1).map(String::as_str) == Some("walk") {
        let lock = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/inputs/zed-Cargo.lock"
        ))
        .unwrap();
        let load = forgo_lib_toml_bench::CONTENDERS[0].load;
        let t = Instant::now();
        let mut n = 0u64;
        while t.elapsed() < Duration::from_secs(secs) {
            black_box(load(black_box(&lock)));
            n += 1;
        }
        println!(
            "{:.1} MB/s",
            lock.len() as f64 * n as f64 / t.elapsed().as_secs_f64() / 1e6
        );
        return;
    }
    if args.get(1).map(String::as_str) == Some("edit") {
        let edit = |s: &str| {
            let mut d = forgo_lib_toml::Document::parse(s).unwrap();
            d.set(["package", "version"], "0.5.0").unwrap();
            d.set(["dependencies", "anyhow"], "1").unwrap();
            d.to_string()
        };
        let before = ALLOCS.load(Ordering::Relaxed);
        black_box(edit(&input));
        println!(
            "allocations per edit: {}",
            ALLOCS.load(Ordering::Relaxed) - before
        );
        let t = Instant::now();
        let mut n = 0u64;
        while t.elapsed() < Duration::from_secs(secs) {
            black_box(edit(black_box(&input)));
            n += 1;
        }
        if n > 0 {
            println!(
                "{:.2} µs per edit",
                t.elapsed().as_secs_f64() / n as f64 * 1e6
            );
        }
        return;
    }
    let before = ALLOCS.load(Ordering::Relaxed);
    black_box(forgo_lib_toml::Document::parse(&input).unwrap());
    println!(
        "allocations per parse: {}",
        ALLOCS.load(Ordering::Relaxed) - before
    );
    let before = ALLOCS.load(Ordering::Relaxed);
    let arena = toml_spanner::Arena::new();
    black_box(toml_spanner::parse(&input, &arena).unwrap());
    drop(arena);
    println!(
        "toml-spanner allocations per parse: {}",
        ALLOCS.load(Ordering::Relaxed) - before
    );
    let t = Instant::now();
    let mut n = 0u64;
    while t.elapsed() < Duration::from_secs(secs) {
        for _ in 0..100 {
            black_box(forgo_lib_toml::Document::parse(black_box(&input)).unwrap());
        }
        n += 100;
    }
    if n > 0 {
        println!(
            "{:.1} MB/s",
            (input.len() as f64 * n as f64) / t.elapsed().as_secs_f64() / 1e6
        );
    }
}
