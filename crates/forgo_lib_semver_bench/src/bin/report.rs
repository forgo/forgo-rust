//! The comparison report, as GitHub-flavored markdown on stdout.
//!
//!   cargo run --release --manifest-path crates/forgo_lib_semver_bench/Cargo.toml --bin report
//!
//! `--quick` runs fewer timing rounds. The npm comparison needs Node.js
//! and node-semver, fetched by
//! crates/forgo_lib_semver/scripts/fetch-node-semver.sh.

use forgo_lib_semver_bench::{
    NPM_LIBS, PARSERS, Rng, bnf, cargo_req, grammar_version, mutate, npm_range, probe_version,
    typical_version,
};
use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering::Relaxed};
use std::time::Instant;

// ---------------------------------------------------------------------------
// Allocation counting (only while measuring allocations)
// ---------------------------------------------------------------------------

struct Counting;

static COUNTING: AtomicBool = AtomicBool::new(false);
static ALLOCS: AtomicUsize = AtomicUsize::new(0);

// SAFETY: delegates to the system allocator and only updates a counter.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        if COUNTING.load(Relaxed) {
            ALLOCS.fetch_add(1, Relaxed);
        }
        unsafe { System.alloc(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        unsafe { System.dealloc(p, l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, new: usize) -> *mut u8 {
        if COUNTING.load(Relaxed) {
            ALLOCS.fetch_add(1, Relaxed);
        }
        unsafe { System.realloc(p, l, new) }
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

/// Allocations per call of `f` (averaged over `n` calls).
fn allocations(n: usize, mut f: impl FnMut()) -> f64 {
    ALLOCS.store(0, Relaxed);
    COUNTING.store(true, Relaxed);
    for _ in 0..n {
        f();
    }
    COUNTING.store(false, Relaxed);
    ALLOCS.load(Relaxed) as f64 / n as f64
}

// ---------------------------------------------------------------------------
// Timing
// ---------------------------------------------------------------------------

struct Timer {
    rounds: usize,
}

impl Timer {
    /// Median nanoseconds per item, `f` processing `items` items per call.
    fn per_item(&self, items: usize, mut f: impl FnMut()) -> f64 {
        // Warm up, and size a sample to about 20 ms.
        let start = Instant::now();
        let mut calls = 0;
        while start.elapsed().as_millis() < 20 || calls == 0 {
            f();
            calls += 1;
        }
        let mut samples: Vec<f64> = (0..self.rounds)
            .map(|_| {
                let t = Instant::now();
                for _ in 0..calls {
                    f();
                }
                t.elapsed().as_nanos() as f64 / (calls * items) as f64
            })
            .collect();
        samples.sort_by(f64::total_cmp);
        samples[samples.len() / 2]
    }
}

fn ns(x: f64) -> String {
    if x >= 100.0 {
        format!("{x:.0}")
    } else {
        format!("{x:.1}")
    }
}

fn pct(n: usize, of: usize) -> String {
    if of == 0 {
        return "n/a".into();
    }
    let p = 100.0 * n as f64 / of as f64;
    if n == of {
        "100%".into()
    } else if p >= 99.95 {
        "99.9%".into()
    } else {
        format!("{p:.1}%")
    }
}

// ---------------------------------------------------------------------------
// Compliance: version parsing
// ---------------------------------------------------------------------------

const VALID: &[&str] = &[
    "0.0.4",
    "1.2.3",
    "10.20.30",
    "1.1.2-prerelease+meta",
    "1.1.2+meta",
    "1.1.2+meta-valid",
    "1.0.0-alpha",
    "1.0.0-beta",
    "1.0.0-alpha.beta",
    "1.0.0-alpha.beta.1",
    "1.0.0-alpha.1",
    "1.0.0-alpha0.valid",
    "1.0.0-alpha.0valid",
    "1.0.0-alpha-a.b-c-somethinglong+build.1-aef.1-its-okay",
    "1.0.0-rc.1+build.1",
    "2.0.0-rc.1+build.123",
    "1.2.3-beta",
    "10.2.3-DEV-SNAPSHOT",
    "1.2.3-SNAPSHOT-123",
    "1.0.0",
    "2.0.0",
    "1.1.7",
    "2.0.0+build.1848",
    "2.0.1-alpha.1227",
    "1.0.0-alpha+beta",
    "1.2.3----RC-SNAPSHOT.12.9.1--.12+788",
    "1.2.3----R-S.12.9.1--.12+meta",
    "1.2.3----RC-SNAPSHOT.12.9.1--.12",
    "1.0.0+0.build.1-rc.10000aaa-kk-0.1",
    "99999999999999999999999.999999999999999999.99999999999999999",
    "1.0.0-0A.is.legal",
];

const INVALID: &[&str] = &[
    "1",
    "1.2",
    "1.2.3-0123",
    "1.2.3-0123.0123",
    "1.1.2+.123",
    "+invalid",
    "-invalid",
    "-invalid+invalid",
    "-invalid.01",
    "alpha",
    "alpha.beta",
    "alpha.beta.1",
    "alpha.1",
    "alpha+beta",
    "alpha_beta",
    "alpha.",
    "alpha..",
    "beta",
    "1.0.0-alpha_beta",
    "-alpha.",
    "1.0.0-alpha..",
    "1.0.0-alpha..1",
    "1.0.0-alpha...1",
    "1.0.0-alpha....1",
    "1.0.0-alpha.....1",
    "1.0.0-alpha......1",
    "1.0.0-alpha.......1",
    "01.1.1",
    "1.01.1",
    "1.1.01",
    "1.2",
    "1.2.3.DEV",
    "1.2-SNAPSHOT",
    "1.2.31.2.3----RC-SNAPSHOT.12.09.1--..12+788",
    "1.2-RC-SNAPSHOT",
    "-1.0.3-gamma+b7718",
    "+justmeta",
    "9.8.7+meta+meta",
    "9.8.7-whatever+meta+meta",
    "99999999999999999999999.999999999999999999.99999999999999999----RC-SNAPSHOT.12.09.1--------------------------------..12",
];

fn parsing_compliance() -> String {
    let mut r = Rng(0x5e3f_be4c_0001);
    let mut generated: Vec<(String, bool)> = Vec::new();
    for _ in 0..20_000 {
        let v = grammar_version(&mut r);
        let m = mutate(&mut r, &v);
        let (bv, bm) = (bnf::valid(&v), bnf::valid(&m));
        generated.push((v, bv));
        generated.push((m, bm));
    }
    let mut out = String::from(
        "| Crate | Version | Valid (31) | Invalid (40) | Generated (40,000) | Above `u64` | Parses |\n\
         |---|---|---:|---:|---:|:---:|---|\n",
    );
    for p in PARSERS {
        let valid = VALID.iter().filter(|s| (p.accepts)(s)).count();
        let invalid = INVALID.iter().filter(|s| !(p.accepts)(s)).count();
        let agree = generated
            .iter()
            .filter(|(s, ok)| (p.accepts)(s) == *ok)
            .count();
        let big = if (p.accepts)("18446744073709551616.0.0") {
            "yes"
        } else {
            "no"
        };
        out.push_str(&format!(
            "| {} | {} | {valid} | {invalid} | {} | {big} | {} |\n",
            p.name,
            p.version,
            pct(agree, generated.len()),
            p.note
        ));
    }
    out
}

// ---------------------------------------------------------------------------
// Compliance: npm ranges against node-semver
// ---------------------------------------------------------------------------

fn node_semver_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../forgo_lib_semver/tests/node-semver-data/node-semver")
}

struct NodeResult {
    valid: Option<String>,
    bits: String,
}

/// node-semver's answers, or why there are none.
fn node_eval(ranges: &[String], versions: &[String]) -> Result<Vec<NodeResult>, String> {
    let dir = node_semver_dir();
    if !dir.join("index.js").is_file() {
        return Err("node-semver isn't fetched (scripts/fetch-node-semver.sh)".into());
    }
    let tmp = std::env::temp_dir().join(format!("semver-bench-{}.json", std::process::id()));
    let quote = |s: &String| {
        let mut q = String::from("\"");
        for c in s.chars() {
            match c {
                '"' => q.push_str("\\\""),
                '\\' => q.push_str("\\\\"),
                c if (c as u32) < 0x20 => q.push_str(&format!("\\u{:04x}", c as u32)),
                c => q.push(c),
            }
        }
        q.push('"');
        q
    };
    let list = |xs: &[String]| xs.iter().map(quote).collect::<Vec<_>>().join(",");
    std::fs::write(
        &tmp,
        format!(
            "{{\"ranges\":[{}],\"versions\":[{}]}}",
            list(ranges),
            list(versions)
        ),
    )
    .map_err(|e| e.to_string())?;
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("other/node_eval.js");
    let out = Command::new("node")
        .arg(script)
        .arg(&dir)
        .arg(&tmp)
        .output()
        .map_err(|e| format!("can't run node: {e}"))?;
    let _ = std::fs::remove_file(&tmp);
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).into_owned());
    }
    let json = String::from_utf8(out.stdout).map_err(|e| e.to_string())?;
    let doc = forgo_lib_yaml::Doc::from_str(&json).map_err(|e| e.to_string())?;
    let items = doc.root().node().as_seq().ok_or("not a list")?;
    Ok(items
        .iter()
        .map(|item| {
            let map = item.node().as_map().expect("object");
            let get = |k: &str| map.iter().find(|(key, _)| key == k).map(|(_, v)| v);
            NodeResult {
                valid: get("valid")
                    .and_then(|v| v.node().as_str())
                    .map(String::from),
                bits: get("bits")
                    .and_then(|v| v.node().as_str())
                    .unwrap_or_default()
                    .to_string(),
            }
        })
        .collect())
}

fn npm_compliance() -> String {
    let mut r = Rng(0x5e3f_be4c_0002);
    let ranges: Vec<String> = (0..4000).map(|_| npm_range(&mut r)).collect();
    let versions: Vec<String> = (0..24).map(|_| probe_version(&mut r)).collect();
    let node = match node_eval(&ranges, &versions) {
        Ok(n) => n,
        Err(why) => return format!("Not run: {why}\n"),
    };
    let mut out = format!(
        "{} random ranges (odd spacing, `v` and `=` prefixes, x-ranges, hyphen ranges, `||`) \
         and {} versions each, compared with node-semver 7.8.6 itself:\n\n\
         | Crate | Same valid/invalid verdict | Same `satisfies` results |\n\
         |---|---:|---:|\n",
        ranges.len(),
        versions.len()
    );
    for lib in NPM_LIBS {
        let (mut same_verdict, mut same_sat, mut sat_total) = (0, 0, 0);
        for (range, n) in ranges.iter().zip(&node) {
            let ours = std::panic::catch_unwind(|| (lib.eval)(range, &versions))
                .ok()
                .flatten();
            if ours.is_some() == n.valid.is_some() {
                same_verdict += 1;
            }
            if let (Some(results), Some(_)) = (&ours, &n.valid) {
                for (res, bit) in results.iter().zip(n.bits.bytes()) {
                    sat_total += 1;
                    if res.unwrap_or(false) == (bit == b'1') {
                        same_sat += 1;
                    }
                }
            }
        }
        out.push_str(&format!(
            "| {} | {} | {} |\n",
            lib.name,
            pct(same_verdict, ranges.len()),
            pct(same_sat, sat_total)
        ));
    }
    out
}

// ---------------------------------------------------------------------------
// Compliance: Cargo requirements against the semver crate
// ---------------------------------------------------------------------------

fn cargo_compliance() -> String {
    let mut r = Rng(0x5e3f_be4c_0003);
    let reqs: Vec<String> = (0..20_000).map(|_| cargo_req(&mut r)).collect();
    let versions: Vec<String> = (0..24).map(|_| probe_version(&mut r)).collect();
    let (ours_v, theirs_v): (Vec<_>, Vec<_>) = versions
        .iter()
        .map(|v| {
            (
                forgo_lib_semver::Version::parse(v).unwrap(),
                semver::Version::parse(v).unwrap(),
            )
        })
        .unzip();
    let (mut verdict, mut text, mut both, mut matches, mut checks) = (0, 0, 0, 0, 0);
    for req in &reqs {
        let ours = forgo_lib_semver::Req::cargo(req).ok();
        let theirs = semver::VersionReq::parse(req).ok();
        if ours.is_some() == theirs.is_some() {
            verdict += 1;
        }
        if let (Some(a), Some(b)) = (&ours, &theirs) {
            both += 1;
            if a.to_string() == b.to_string() {
                text += 1;
            }
            for (x, y) in ours_v.iter().zip(&theirs_v) {
                checks += 1;
                if a.matches(x) == b.matches(y) {
                    matches += 1;
                }
            }
        }
    }
    format!(
        "{} random requirements (all operators, partial versions, wildcards, \
         pre-releases, spacing) and {} versions each, compared with the `semver` crate \
         (Cargo's parser):\n\n\
         | Check | Same as `semver` 1.0.28 |\n\
         |---|---:|\n\
         | valid/invalid verdict | {} |\n\
         | canonical text | {} |\n\
         | `matches` results | {} |\n",
        reqs.len(),
        versions.len(),
        pct(verdict, reqs.len()),
        pct(text, both),
        pct(matches, checks)
    )
}

// ---------------------------------------------------------------------------
// Performance
// ---------------------------------------------------------------------------

fn performance(timer: &Timer) -> String {
    let mut r = Rng(0x5e3f_be4c_0004);
    let typical: Vec<String> = (0..10_000).map(|_| typical_version(&mut r)).collect();
    let n = typical.len();

    let mut out = String::from(
        "### Parsing versions\n\n\
         10,000 versions as published (mostly `M.m.p`; 20% pre-releases, 5% build metadata).\n\n\
         | Crate | ns per version | Allocations per version | `size_of::<Version>()` (bytes) |\n\
         |---|---:|---:|---:|\n",
    );
    macro_rules! parse_row {
        ($name:expr, $ty:ty, $parse:expr) => {{
            let f = $parse;
            let t = timer.per_item(n, || {
                for s in &typical {
                    black_box(f(black_box(s.as_str())));
                }
            });
            let a = allocations(n, {
                let mut i = 0;
                let f = &f;
                let typical = &typical;
                move || {
                    black_box(f(&typical[i % typical.len()]));
                    i += 1;
                }
            });
            out.push_str(&format!(
                "| {} | {} | {:.2} | {} |\n",
                $name,
                ns(t),
                a,
                std::mem::size_of::<$ty>()
            ));
        }};
    }
    parse_row!("forgo_lib_semver", forgo_lib_semver::Version, |s: &str| {
        forgo_lib_semver::Version::parse(s).ok()
    });
    parse_row!("semver", semver::Version, |s: &str| semver::Version::parse(
        s
    )
    .ok());
    parse_row!("js-semver", js_semver::Version, |s: &str| {
        js_semver::Version::parse(s).ok()
    });
    parse_row!("node-semver", node_semver::Version, |s: &str| {
        node_semver::Version::parse(s).ok()
    });
    parse_row!("nodejs-semver", nodejs_semver::Version, |s: &str| {
        nodejs_semver::Version::parse(s).ok()
    });
    parse_row!("semver_rs", semver_rs::Version, |s: &str| {
        semver_rs::Version::new(s).parse().ok()
    });
    parse_row!("deno_semver", deno_semver::Version, |s: &str| {
        deno_semver::Version::parse_standard(s).ok()
    });
    parse_row!(
        "semver-parser",
        semver_parser::version::Version,
        |s: &str| semver_parser::version::parse(s).ok()
    );
    parse_row!("versions", versions::SemVer, |s: &str| {
        versions::SemVer::new(s)
    });
    parse_row!("lenient_semver", semver::Version, |s: &str| {
        lenient_semver::parse(s).ok()
    });

    // Sorting.
    out.push_str(
        "\n### Sorting versions\n\n\
         100,000 parsed versions as published, sorted (`sort`, by each crate's `Ord`).\n\n\
         | Crate | ms per sort |\n\
         |---|---:|\n",
    );
    let big: Vec<String> = (0..100_000).map(|_| typical_version(&mut r)).collect();
    macro_rules! sort_row {
        ($name:expr, $parse:expr) => {{
            let parsed: Vec<_> = big.iter().filter_map(|s| $parse(s.as_str())).collect();
            let t = timer.per_item(1, || {
                let mut v = parsed.clone();
                v.sort();
                black_box(v);
            });
            out.push_str(&format!("| {} | {:.2} |\n", $name, t / 1e6));
        }};
    }
    sort_row!("forgo_lib_semver", |s: &str| {
        forgo_lib_semver::Version::parse(s).ok()
    });
    sort_row!("semver", |s: &str| semver::Version::parse(s).ok());
    sort_row!("js-semver", |s: &str| js_semver::Version::parse(s).ok());
    sort_row!("nodejs-semver", |s: &str| nodejs_semver::Version::parse(s)
        .ok());
    sort_row!("node-semver", |s: &str| node_semver::Version::parse(s).ok());

    // Cargo requirements.
    let cargo_reqs = [
        "1",
        "1.2",
        "1.2.3",
        "^1.2.3",
        "~1.2",
        "=1.2.3",
        ">=1.2.3, <2.0.0",
        "0.4",
        "*",
        "1.2.*",
        ">=0.10, <0.12",
        "^0.3.1-alpha.2",
    ];
    let probe: Vec<String> = (0..64).map(|_| typical_version(&mut r)).collect();
    out.push_str(&format!(
        "\n### Cargo requirements\n\n\
         {} requirements as written in `Cargo.toml` (`1.2`, `^1.2.3`, `>=1.2.3, <2.0.0`, …); \
         matching: each against {} versions.\n\n\
         | Crate | ns per parse | ns per match |\n\
         |---|---:|---:|\n",
        cargo_reqs.len(),
        probe.len()
    ));
    {
        let parsed: Vec<_> = cargo_reqs
            .iter()
            .map(|s| forgo_lib_semver::Req::cargo(s).unwrap())
            .collect();
        let vs: Vec<_> = probe
            .iter()
            .map(|s| forgo_lib_semver::Version::parse(s).unwrap())
            .collect();
        let p = timer.per_item(cargo_reqs.len(), || {
            for s in &cargo_reqs {
                black_box(forgo_lib_semver::Req::cargo(black_box(s)).ok());
            }
        });
        let m = timer.per_item(parsed.len() * vs.len(), || {
            for req in &parsed {
                for v in &vs {
                    black_box(req.matches(black_box(v)));
                }
            }
        });
        out.push_str(&format!("| forgo_lib_semver | {} | {} |\n", ns(p), ns(m)));
    }
    {
        let parsed: Vec<_> = cargo_reqs
            .iter()
            .map(|s| semver::VersionReq::parse(s).unwrap())
            .collect();
        let vs: Vec<_> = probe
            .iter()
            .map(|s| semver::Version::parse(s).unwrap())
            .collect();
        let p = timer.per_item(cargo_reqs.len(), || {
            for s in &cargo_reqs {
                black_box(semver::VersionReq::parse(black_box(s)).ok());
            }
        });
        let m = timer.per_item(parsed.len() * vs.len(), || {
            for req in &parsed {
                for v in &vs {
                    black_box(req.matches(black_box(v)));
                }
            }
        });
        out.push_str(&format!("| semver | {} | {} |\n", ns(p), ns(m)));
    }

    // npm ranges.
    let npm_ranges = [
        "^1.2.3",
        "~1.2.3",
        ">=1.2.3 <2.0.0",
        "1.x",
        "1.2.3 - 2.3.4",
        "^1.0.0 || ^2.0.0",
        "*",
        ">=0.10.0",
        "~0.4",
        "^0.3.1-alpha.2",
    ];
    out.push_str(&format!(
        "\n### npm ranges\n\n\
         {} ranges as written in `package.json` (`^1.2.3`, `1.x`, `1.2.3 - 2.3.4`, `^1 || ^2`, …); \
         matching: each against {} versions.\n\n\
         | Crate | ns per parse | ns per match |\n\
         |---|---:|---:|\n",
        npm_ranges.len(),
        probe.len()
    ));
    macro_rules! npm_row {
        ($name:expr, $parse_range:expr, $parse_version:expr, $test:expr) => {{
            let parsed: Vec<_> = npm_ranges
                .iter()
                .map(|s| $parse_range(s).unwrap())
                .collect();
            let vs: Vec<_> = probe
                .iter()
                .map(|s| $parse_version(s.as_str()).unwrap())
                .collect();
            let p = timer.per_item(npm_ranges.len(), || {
                for s in &npm_ranges {
                    black_box($parse_range(black_box(s)));
                }
            });
            let m = timer.per_item(parsed.len() * vs.len(), || {
                for range in &parsed {
                    for v in &vs {
                        black_box($test(range, black_box(v)));
                    }
                }
            });
            out.push_str(&format!("| {} | {} | {} |\n", $name, ns(p), ns(m)));
        }};
    }
    npm_row!(
        "forgo_lib_semver",
        |s: &str| forgo_lib_semver::Req::npm(s).ok(),
        |s: &str| forgo_lib_semver::Version::parse(s).ok(),
        |r: &forgo_lib_semver::Req, v| r.matches(v)
    );
    npm_row!(
        "js-semver",
        |s: &str| js_semver::Range::parse(s).ok(),
        |s: &str| js_semver::Version::parse(s).ok(),
        |r: &js_semver::Range, v| r.satisfies(v)
    );
    npm_row!(
        "node-semver",
        |s: &str| node_semver::Range::parse(s).ok(),
        |s: &str| node_semver::Version::parse(s).ok(),
        |r: &node_semver::Range, v| r.satisfies(v)
    );
    npm_row!(
        "nodejs-semver",
        |s: &str| nodejs_semver::Range::parse(s).ok(),
        |s: &str| nodejs_semver::Version::parse(s).ok(),
        |r: &nodejs_semver::Range, v| r.satisfies(v)
    );
    npm_row!(
        "semver_rs",
        |s: &str| semver_rs::Range::new(s).parse().ok(),
        |s: &str| semver_rs::Version::new(s).parse().ok(),
        |r: &semver_rs::Range, v| r.test(v)
    );
    npm_row!(
        "deno_semver",
        |s: &str| deno_semver::VersionReq::parse_from_npm(s).ok(),
        |s: &str| deno_semver::Version::parse_from_npm(s).ok(),
        |r: &deno_semver::VersionReq, v| r.matches(v)
    );
    out
}

fn rustc() -> String {
    Command::new("rustc")
        .arg("--version")
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

fn main() {
    let quick = std::env::args().any(|a| a == "--quick");
    let timer = Timer {
        rounds: if quick { 5 } else { 21 },
    };
    // Silence panics from crates under test (counted as failures).
    std::panic::set_hook(Box::new(|_| {}));
    println!("# forgo_lib_semver: how it compares\n");
    println!(
        "Generated by `cargo run --release --manifest-path crates/forgo_lib_semver_bench/Cargo.toml --bin report` \
         ({}, {}).\n",
        rustc(),
        std::env::consts::ARCH
    );
    println!("## Parsing versions: compliance\n");
    println!(
        "Against semver.org's lists of valid and invalid versions, and the SemVer 2.0.0 \
         BNF on 20,000 generated versions and 20,000 mutations of them (numbers up to \
         30 digits). Crates that read npm's grammar accept `v1.2.3` and similar by design; \
         the columns show how far each is from the specification.\n"
    );
    println!("{}", parsing_compliance());
    println!("## npm ranges: compliance\n");
    println!("{}", npm_compliance());
    println!("## Cargo requirements: compliance\n");
    println!("{}", cargo_compliance());
    println!("## Performance\n");
    println!(
        "Medians of {} rounds; lower is better. Allocations are counted with a counting \
         global allocator.\n",
        timer.rounds
    );
    println!("{}", performance(&timer));
}
