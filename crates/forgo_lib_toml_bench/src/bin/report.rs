//! The comparison report, as GitHub-flavored markdown on stdout.
//!
//!   scripts/fetch-inputs.sh   # the real-world files, once
//!   cargo run --release --bin report > RESULTS.md
//!
//! Needs the toml-test data fetched by
//! crates/forgo_lib_toml/scripts/fetch-toml-test.sh. `--quick` runs fewer
//! timing rounds.

use forgo_lib_toml::{Document, ParseOptions, Seg};
use forgo_lib_toml_bench::{CONTENDERS, readings, same};
use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering::Relaxed};
use std::time::Instant;

// ---------------------------------------------------------------------------
// Counting allocations and peak heap (only while measuring)
// ---------------------------------------------------------------------------

struct Counting;

static COUNTING: AtomicBool = AtomicBool::new(false);
static ALLOCS: AtomicUsize = AtomicUsize::new(0);
static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

fn grew(by: usize) {
    let now = LIVE.fetch_add(by, Relaxed) + by;
    PEAK.fetch_max(now, Relaxed);
}

// SAFETY: delegates to the system allocator and only updates counters.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        if COUNTING.load(Relaxed) {
            ALLOCS.fetch_add(1, Relaxed);
            grew(l.size());
        }
        unsafe { System.alloc(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        if COUNTING.load(Relaxed) {
            LIVE.fetch_sub(l.size().min(LIVE.load(Relaxed)), Relaxed);
        }
        unsafe { System.dealloc(p, l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, new: usize) -> *mut u8 {
        if COUNTING.load(Relaxed) {
            ALLOCS.fetch_add(1, Relaxed);
            if new > l.size() {
                grew(new - l.size());
            } else {
                LIVE.fetch_sub((l.size() - new).min(LIVE.load(Relaxed)), Relaxed);
            }
        }
        unsafe { System.realloc(p, l, new) }
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

/// Allocations by one call of `f`, and the most heap held at once.
fn memory<T>(f: impl FnOnce() -> T) -> (usize, usize) {
    ALLOCS.store(0, Relaxed);
    LIVE.store(0, Relaxed);
    PEAK.store(0, Relaxed);
    COUNTING.store(true, Relaxed);
    let kept = f();
    COUNTING.store(false, Relaxed);
    let r = (ALLOCS.load(Relaxed), PEAK.load(Relaxed));
    drop(black_box(kept));
    r
}

// ---------------------------------------------------------------------------
// Timing
// ---------------------------------------------------------------------------

struct Timer {
    rounds: usize,
}

impl Timer {
    /// Median seconds per call of `f`.
    fn per_call(&self, mut f: impl FnMut()) -> f64 {
        let start = Instant::now();
        let mut calls = 0u32;
        while start.elapsed().as_millis() < 30 || calls == 0 {
            f();
            calls += 1;
        }
        let mut samples: Vec<f64> = (0..self.rounds)
            .map(|_| {
                let t = Instant::now();
                for _ in 0..calls {
                    f();
                }
                t.elapsed().as_secs_f64() / f64::from(calls)
            })
            .collect();
        samples.sort_by(f64::total_cmp);
        samples[samples.len() / 2]
    }
}

fn fixed(x: f64) -> String {
    if x >= 100.0 {
        let n = x.round() as u64;
        let s = n.to_string();
        // Thousands separators.
        let mut out = String::new();
        for (i, c) in s.chars().enumerate() {
            if i > 0 && (s.len() - i).is_multiple_of(3) {
                out.push(',');
            }
            out.push(c);
        }
        out
    } else if x >= 10.0 {
        format!("{x:.1}")
    } else {
        format!("{x:.2}")
    }
}

/// A count, with thousands separators.
fn int(n: usize) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

fn pct(n: usize, of: usize) -> String {
    if n == of {
        "100%".into()
    } else {
        format!("{:.1}%", 100.0 * n as f64 / of as f64)
    }
}

fn bytes(n: usize) -> String {
    if n >= 1 << 20 {
        format!("{:.1} MiB", n as f64 / f64::from(1 << 20))
    } else if n >= 1 << 10 {
        format!("{:.0} KiB", n as f64 / 1024.0)
    } else {
        format!("{n} B")
    }
}

// ---------------------------------------------------------------------------
// Inputs
// ---------------------------------------------------------------------------

fn crate_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn real(name: &str) -> String {
    let path = crate_dir().join("inputs").join(name);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{}: {e} (run scripts/fetch-inputs.sh)", path.display()))
}

struct Input {
    name: &'static str,
    about: &'static str,
    text: String,
}

fn inputs() -> Vec<Input> {
    vec![
        Input {
            name: "zed Cargo.lock",
            about: "zed's lock file (real)",
            text: real("zed-Cargo.lock"),
        },
        Input {
            name: "zed Cargo.toml",
            about: "zed's workspace manifest (real)",
            text: real("zed-Cargo.toml"),
        },
        Input {
            name: "ruff pyproject.toml",
            about: "ruff's Python project file (real)",
            text: real("ruff-pyproject.toml"),
        },
        Input {
            name: "Cargo.lock 1 MB",
            about: "4,500 packages (generated)",
            text: forgo_lib_toml_bench::lockfile(4500),
        },
        Input {
            name: "mixed 1 MB",
            about: "every kind of value (generated)",
            text: forgo_lib_toml_bench::mixed(1024),
        },
        Input {
            name: "mixed 5 MiB",
            about: "the same, larger (generated)",
            text: forgo_lib_toml_bench::mixed(5 * 1024),
        },
        Input {
            name: "Cargo.toml 1 KB",
            about: "a small manifest",
            text: forgo_lib_toml_bench::MANIFEST.to_string(),
        },
    ]
}

fn toml_test_dir() -> PathBuf {
    crate_dir().join("../forgo_lib_toml/tests/toml-test-data/tests")
}

/// The valid and invalid cases of a toml-test list.
fn toml_test(list: &str) -> (Vec<String>, Vec<String>) {
    let dir = toml_test_dir();
    let names = std::fs::read_to_string(dir.join(list)).unwrap_or_else(|e| {
        panic!("{list}: {e} (run crates/forgo_lib_toml/scripts/fetch-toml-test.sh)")
    });
    let read = |n: &str| String::from_utf8_lossy(&std::fs::read(dir.join(n)).unwrap()).into_owned();
    let (mut valid, mut invalid) = (Vec::new(), Vec::new());
    for n in names.lines().filter(|l| l.ends_with(".toml")) {
        // Invalid UTF-8 is a rejection for every crate here: they take &str.
        if std::str::from_utf8(&std::fs::read(dir.join(n)).unwrap()).is_err() {
            continue;
        }
        if n.starts_with("valid/") {
            valid.push(read(n));
        } else {
            invalid.push(read(n));
        }
    }
    (valid, invalid)
}

// ---------------------------------------------------------------------------
// The report
// ---------------------------------------------------------------------------

fn main() {
    let quick = std::env::args().any(|a| a == "--quick");
    let timer = Timer {
        rounds: if quick { 5 } else { 15 },
    };
    let rustc = std::process::Command::new("rustc")
        .arg("--version")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();
    println!("# forgo_lib_toml: how it compares\n");
    println!(
        "Generated by `cargo run --release --bin report` in `crates/forgo_lib_toml_bench` ({rustc}, {}).\n",
        std::env::consts::ARCH
    );
    compliance();
    official_runner();
    agreement();
    speed(&timer);
    memory_use();
    encoding(&timer);
    editing(&timer);
}

fn compliance() {
    println!("## Compliance: toml-test\n");
    println!(
        "Every case of [toml-test](https://github.com/toml-lang/toml-test) at commit `96fd088`, for each TOML version: is each valid document accepted, and each invalid one rejected? (Cases that aren't UTF-8 are left out: these crates take `&str`.) Only acceptance is compared here; forgo_lib_toml's values are also checked against every expected result in its own tests.\n"
    );
    let lists = [("files-toml-1.1.0", "1.1.0"), ("files-toml-1.0.0", "1.0.0")];
    let cases: Vec<_> = lists.iter().map(|(l, _)| toml_test(l)).collect();
    println!(
        "| Crate | Version | 1.1 valid ({}) | 1.1 invalid ({}) | 1.0 valid ({}) | 1.0 invalid ({}) |",
        cases[0].0.len(),
        cases[0].1.len(),
        cases[1].0.len(),
        cases[1].1.len()
    );
    println!("|---|---|---:|---:|---:|---:|");
    for c in CONTENDERS {
        let mut row = format!("| {} | {} |", c.name, c.version);
        for (valid, invalid) in &cases {
            let ok = valid.iter().filter(|s| (c.accepts)(s)).count();
            let rejected = invalid.iter().filter(|s| !(c.accepts)(s)).count();
            row.push_str(&format!(
                " {} | {} |",
                pct(ok, valid.len()),
                pct(rejected, invalid.len())
            ));
        }
        println!("{row}");
        if c.name == "forgo_lib_toml" {
            // Asked for TOML 1.0.
            let v1_0 = ParseOptions::default().with_version(forgo_lib_toml::TomlVersion::V1_0);
            let accepts = |s: &String| Document::parse_with(s, &v1_0).is_ok();
            let (valid, invalid) = &cases[1];
            let ok = valid.iter().filter(|s| accepts(s)).count();
            let rejected = invalid.iter().filter(|s| !accepts(s)).count();
            println!(
                "| forgo_lib_toml, asked for 1.0 | {} | | | {} | {} |",
                c.version,
                pct(ok, valid.len()),
                pct(rejected, invalid.len())
            );
        }
    }
    println!(
        "\nThe 1.0 lists include documents TOML 1.0 forbids and 1.1 allows (newlines in inline tables, `\\e`, times without seconds), so a 1.1 parser rejects only 98% of them. forgo_lib_toml also parses TOML 1.0 when asked (`TomlVersion::V1_0`); the other crates have one grammar each.\n"
    );
}

/// toml-test's own runner (Go), if scripts/official-toml-test.sh has
/// built it, on forgo_lib_toml's decoder and encoder.
fn official_runner() {
    println!("## Compliance: toml-test's own runner\n");
    let runner =
        crate_dir().join("target/toml-test-96fd088448ec0f4d9f061c730ed7854399c46faf/toml-test");
    let bin = crate_dir().join("target/release");
    if !runner.exists() || !bin.join("toml-test-decoder").exists() {
        println!("Not run: `scripts/official-toml-test.sh` builds the runner (it needs Go).\n");
        return;
    }
    println!(
        "toml-test's runner, built from the same commit, on forgo_lib_toml's decoder and encoder (`src/bin/toml-test-*.rs`). The runner reads the encoder's TOML with BurntSushi/toml, the Go reference implementation, so the encoder check doesn't depend on forgo_lib_toml's parser.\n"
    );
    println!("| TOML | Valid | Invalid | Encoder |");
    println!("|---|---:|---:|---:|");
    for (version, flag) in [("1.1", ""), ("1.0", " --toml-1.0")] {
        let out = std::process::Command::new(&runner)
            .arg("test")
            .arg(format!("-toml={version}"))
            .arg(format!(
                "-decoder={}{flag}",
                bin.join("toml-test-decoder").display()
            ))
            .arg(format!(
                "-encoder={}",
                bin.join("toml-test-encoder").display()
            ))
            .output()
            .expect("runner");
        let text = String::from_utf8_lossy(&out.stdout);
        let count = |what: &str| {
            text.lines()
                .find(|l| l.trim_start().starts_with(what))
                .and_then(|l| {
                    let nums: Vec<usize> = l
                        .split(|c: char| !c.is_ascii_digit())
                        .filter_map(|s| s.parse().ok())
                        .collect();
                    match nums[..] {
                        [passed, failed, ..] => Some(format!("{passed} / {}", passed + failed)),
                        _ => None,
                    }
                })
                .unwrap_or_else(|| "?".into())
        };
        println!(
            "| {version} | {} | {} | {} |",
            count("valid tests"),
            count("invalid tests"),
            count("encoder tests")
        );
    }
    println!();
}

fn agreement() {
    let inputs = forgo_lib_toml_bench::mutations(
        &forgo_lib_toml_bench::toml_test_valid().expect("toml-test data"),
    );
    println!("## Agreement on mutated documents\n");
    println!(
        "{} documents made from the valid toml-test cases: truncated, or with one of 17 characters inserted at a dozen places each. Each is read by forgo_lib_toml and by the three other crates that parse TOML 1.1 into values, and their answers compared: does each accept the document, and when both do, are the values equal?\n",
        inputs.len()
    );
    let names = ["toml", "toml_edit", "toml-spanner"];
    let mut same_verdict = [0usize; 3];
    let mut same_values = [0usize; 3];
    let mut both = [0usize; 3];
    let mut examples: [Vec<(String, bool, bool)>; 3] = Default::default();
    for s in &inputs {
        let r = readings(s);
        let ours = &r[0].1;
        for (k, (_, theirs)) in r[1..].iter().enumerate() {
            if ours.is_some() == theirs.is_some() {
                same_verdict[k] += 1;
            } else if examples[k].len() < 400 {
                examples[k].push((s.clone(), ours.is_some(), theirs.is_some()));
            }
            if let (Some(a), Some(b)) = (ours, theirs) {
                both[k] += 1;
                if same(a, b) {
                    same_values[k] += 1;
                }
            }
        }
    }
    println!(
        "| Compared with | Same verdict | Same values (both accept) | Only forgo_lib_toml accepts | Only it accepts | …of those, newlines inside an inline table's pair |"
    );
    println!("|---|---:|---:|---:|---:|---:|");
    let mut unexplained = 0;
    for k in 0..3 {
        let only_ours = examples[k].iter().filter(|e| e.1).count();
        let theirs: Vec<&String> = examples[k].iter().filter(|e| e.2).map(|e| &e.0).collect();
        // Do they read the same as forgo_lib_toml once the pairs are on
        // one line?
        let explained = theirs
            .iter()
            .filter(|s| {
                readings(s)[k + 1]
                    .1
                    .as_ref()
                    .is_some_and(|t| forgo_lib_toml_bench::only_split_pairs(s, t))
            })
            .count();
        unexplained += only_ours + theirs.len() - explained;
        println!(
            "| {} | {} | {} of {} | {} | {} | {} |",
            names[k],
            pct(same_verdict[k], inputs.len()),
            pct(same_values[k], both[k]),
            both[k],
            int(only_ours),
            int(theirs.len()),
            int(explained)
        );
    }
    println!(
        "\nWhere they differ, the other crates accept a line break inside a key/value pair of an inline table: a key at the end of one line and its `= value` on the next, for example. TOML 1.1 allows newlines *between* an inline table's pairs, but not within one: its ABNF has `keyval-sep = ws %x3D ws`, and `ws` is spaces and tabs. forgo_lib_toml rejects them.{}\n",
        if unexplained == 0 {
            " Every difference found is of this kind.".to_string()
        } else {
            format!(" **{unexplained} other differences need a look.**")
        }
    );
    // The disagreements, by what each crate says about them, for the notes.
    if std::env::var("SHOW_DISAGREEMENTS").is_ok() {
        for k in 0..3 {
            for (s, ours, _) in examples[k].iter().take(40) {
                eprintln!(
                    "--- {} (forgo_lib_toml {}):\n{s}",
                    names[k],
                    if *ours { "accepts" } else { "rejects" }
                );
            }
        }
    }
}

fn speed(timer: &Timer) {
    let inputs = inputs();
    println!("## Speed: parse and read every value\n");
    println!(
        "MB/s, higher is better: parse the text, then walk every key and value the crate produced (so no crate can defer work until values are read). Median of {} rounds. A dash: the crate rejects the input.\n",
        timer.rounds
    );
    print!("| Crate |");
    for i in &inputs {
        print!(" {} |", i.name);
    }
    println!();
    print!("|---|");
    for _ in &inputs {
        print!("---:|");
    }
    println!();
    for c in CONTENDERS {
        print!("| {} |", c.name);
        for i in &inputs {
            if (c.load)(&i.text).is_none() {
                print!(" — |");
                continue;
            }
            let secs = timer.per_call(|| {
                black_box((c.load)(black_box(&i.text)));
            });
            print!(" {} |", fixed(i.text.len() as f64 / secs / 1e6));
        }
        println!();
    }
    println!("\nThe inputs:\n");
    for i in &inputs {
        println!("- **{}** ({}): {}", i.name, bytes(i.text.len()), i.about);
    }
    println!(
        "\nThe real files are fetched at pinned commits by `scripts/fetch-inputs.sh` (zed `f38d614`, ruff `29e26e5`). toml-span and basic-toml don't read date-times, so they reject the mixed inputs. toml-span, boml and taplo build their own trees, which this measures after parsing without walking (taplo: syntax tree, then its DOM, validated).\n"
    );
    // Latency on the small manifest.
    let manifest = forgo_lib_toml_bench::MANIFEST;
    println!("**Latency** on the 1 KB manifest, µs per parse and walk:\n");
    println!("| Crate | µs |");
    println!("|---|---:|");
    for c in CONTENDERS {
        let secs = timer.per_call(|| {
            black_box((c.load)(black_box(manifest)));
        });
        println!("| {} | {} |", c.name, fixed(secs * 1e6));
    }
    println!();
}

fn memory_use() {
    // Results may borrow their input: give it the program's lifetime.
    let lock: &'static str = Box::leak(real("zed-Cargo.lock").into_boxed_str());
    let manifest: &'static str = forgo_lib_toml_bench::MANIFEST;
    println!("## Memory\n");
    println!(
        "Allocations, and the most heap held at once, to parse a document and keep the result (counted with a counting global allocator):\n"
    );
    println!(
        "| Crate | zed Cargo.lock: allocations | peak heap | 1 KB manifest: allocations | peak heap |"
    );
    println!("|---|---:|---:|---:|---:|");
    macro_rules! row {
        ($name:expr, $parse:expr) => {{
            let (a1, p1) = memory(|| $parse(lock));
            let (a2, p2) = memory(|| $parse(manifest));
            println!(
                "| {} | {} | {} | {} | {} |",
                $name,
                int(a1),
                bytes(p1),
                int(a2),
                bytes(p2)
            );
        }};
    }
    row!("forgo_lib_toml", |s: &'static str| Document::parse(s)
        .unwrap());
    // toml-spanner's arena lives as long as the result.
    let spanner = |s: &'static str| {
        let arena = Box::leak(Box::new(toml_spanner::Arena::new()));
        toml_spanner::parse(s, arena).unwrap()
    };
    row!("toml-spanner", spanner);
    row!("toml (DeTable)", |s: &'static str| {
        toml::de::DeTable::parse(s).unwrap()
    });
    row!("toml (Table)", |s: &'static str| s
        .parse::<toml::Table>()
        .unwrap());
    row!("toml_edit", |s: &'static str| s
        .parse::<toml_edit::DocumentMut>()
        .unwrap());
    row!("basic-toml", |s: &'static str| basic_toml::from_str::<
        serde_json::Value,
    >(s)
    .unwrap());
    row!("toml-span", |s: &'static str| toml_span::parse(s).unwrap());
    row!("boml", |s: &'static str| boml::parse(s).unwrap());
    row!("taplo", |s: &'static str| taplo::parser::parse(s)
        .into_dom());
    println!(
        "\nforgo_lib_toml keeps a copy of the text in the document (the 500 KiB of the lock file are part of its peak); toml-spanner, toml's `DeTable` and boml borrow the caller's text instead. toml-spanner's arena is counted with its tree.\n"
    );
}

fn encoding(timer: &Timer) {
    let lock = real("zed-Cargo.lock");
    let manifest = real("zed-Cargo.toml");
    println!("## Writing TOML\n");
    println!(
        "Turning each crate's own value tree back into TOML text, µs per document, lower is better. The tree is built beforehand; only writing is timed. (`toml_edit` writes its document back as it was parsed, so its time is copying the layout, not choosing one.)\n"
    );
    println!("| Crate | zed Cargo.lock | zed Cargo.toml |");
    println!("|---|---:|---:|");
    let cell = |f: &mut dyn FnMut()| fixed(timer.per_call(f) * 1e6);
    let (lock_item, man_item) = (
        Document::parse(&lock).unwrap().to_item(),
        Document::parse(&manifest).unwrap().to_item(),
    );
    println!(
        "| forgo_lib_toml (`Item`) | {} | {} |",
        cell(&mut || {
            black_box(lock_item.to_toml_string().unwrap());
        }),
        cell(&mut || {
            black_box(man_item.to_toml_string().unwrap());
        })
    );
    let (lt, mt) = (
        lock.parse::<toml::Table>().unwrap(),
        manifest.parse::<toml::Table>().unwrap(),
    );
    println!(
        "| toml (`Table`) | {} | {} |",
        cell(&mut || {
            black_box(toml::to_string(&lt).unwrap());
        }),
        cell(&mut || {
            black_box(toml::to_string(&mt).unwrap());
        })
    );
    let (la, ma) = (toml_spanner::Arena::new(), toml_spanner::Arena::new());
    let (ld, md) = (
        toml_spanner::parse(&lock, &la).unwrap(),
        toml_spanner::parse(&manifest, &ma).unwrap(),
    );
    println!(
        "| toml-spanner (`Table`) | {} | {} |",
        cell(&mut || {
            black_box(toml_spanner::to_string(ld.table()).unwrap());
        }),
        cell(&mut || {
            black_box(toml_spanner::to_string(md.table()).unwrap());
        })
    );
    let (lj, mj) = (
        basic_toml::from_str::<serde_json::Value>(&lock).unwrap(),
        basic_toml::from_str::<serde_json::Value>(&manifest).unwrap(),
    );
    let basic = |v: &serde_json::Value| match basic_toml::to_string(v) {
        Ok(_) => cell(&mut || {
            black_box(basic_toml::to_string(v).ok());
        }),
        Err(e) => format!("fails ({e})"),
    };
    let (bl, bm) = (basic(&lj), basic(&mj));
    println!("| basic-toml (serde) | {bl} | {bm} |");
    let (le, me) = (
        lock.parse::<toml_edit::DocumentMut>().unwrap(),
        manifest.parse::<toml_edit::DocumentMut>().unwrap(),
    );
    println!(
        "| toml_edit (`DocumentMut`, as parsed) | {} | {} |",
        cell(&mut || {
            black_box(le.to_string());
        }),
        cell(&mut || {
            black_box(me.to_string());
        })
    );
    println!();
}

fn editing(timer: &Timer) {
    let manifest = forgo_lib_toml_bench::MANIFEST;
    let lock = real("zed-Cargo.lock");
    println!("## Editing\n");
    println!(
        "Parse, edit, and write the text, keeping everything else as it was. µs per edit, lower is better. All crates produce the same text (checked).\n"
    );
    println!(
        "- **Manifest**: set `package.version`, add `dependencies.anyhow`, in the 1 KB manifest."
    );
    println!("- **Lock file**: set the version of the 100th package of zed's Cargo.lock.\n");
    let lock_index = 100;
    let forgo_manifest = |s: &str, opts: &ParseOptions| {
        let mut d = Document::parse_with(s, opts).unwrap();
        d.set(["package", "version"], "0.5.0").unwrap();
        d.set(["dependencies", "anyhow"], "1").unwrap();
        d.to_string()
    };
    let forgo_lock = |s: &str, opts: &ParseOptions| {
        let mut d = Document::parse_with(s, opts).unwrap();
        d.set(
            [
                Seg::Key("package"),
                Seg::Index(lock_index),
                Seg::Key("version"),
            ],
            "9.9.9",
        )
        .unwrap();
        d.to_string()
    };
    let edit_manifest = |s: &str| {
        let mut d = s.parse::<toml_edit::DocumentMut>().unwrap();
        d["package"]["version"] = toml_edit::value("0.5.0");
        d["dependencies"]["anyhow"] = toml_edit::value("1");
        d.to_string()
    };
    let edit_lock = |s: &str| {
        let mut d = s.parse::<toml_edit::DocumentMut>().unwrap();
        d["package"][lock_index]["version"] = toml_edit::value("9.9.9");
        d.to_string()
    };
    let spanner_manifest = |s: &str| {
        let arena = toml_spanner::Arena::new();
        let doc = toml_spanner::parse(s, &arena).unwrap();
        let mut table = doc.table().clone_in(&arena);
        let package = table.get_mut("package").unwrap().as_table_mut().unwrap();
        *package.get_mut("version").unwrap() = toml_spanner::Item::string("0.5.0");
        let deps = table
            .get_mut("dependencies")
            .unwrap()
            .as_table_mut()
            .unwrap();
        let key = toml_spanner::Key {
            name: "anyhow",
            span: toml_spanner::Span::default(),
        };
        deps.insert(key, toml_spanner::Item::string("1"), &arena);
        toml_spanner::Formatting::preserved_from(&doc)
            .format(&table)
            .unwrap()
    };
    let spanner_lock = |s: &str| {
        let arena = toml_spanner::Arena::new();
        let doc = toml_spanner::parse(s, &arena).unwrap();
        let mut table = doc.table().clone_in(&arena);
        let packages = table.get_mut("package").unwrap().as_array_mut().unwrap();
        let p = packages
            .get_mut(lock_index)
            .unwrap()
            .as_table_mut()
            .unwrap();
        *p.get_mut("version").unwrap() = toml_spanner::Item::string("9.9.9");
        toml_spanner::Formatting::preserved_from(&doc)
            .format(&table)
            .unwrap()
    };
    let plain = ParseOptions::default();
    let for_editing = ParseOptions::default().for_editing();
    let want_manifest = forgo_manifest(manifest, &plain);
    let want_lock = forgo_lock(&lock, &plain);
    let check = |name: &str, got: String, want: &str| {
        if got != want {
            println!("\n> **{name}** writes different text than forgo_lib_toml here.\n");
        }
    };
    check(
        "toml_edit (manifest)",
        edit_manifest(manifest),
        &want_manifest,
    );
    check("toml_edit (lock file)", edit_lock(&lock), &want_lock);
    check(
        "toml-spanner (manifest)",
        spanner_manifest(manifest),
        &want_manifest,
    );
    check("toml-spanner (lock file)", spanner_lock(&lock), &want_lock);
    println!("| Crate | Manifest (µs) | Lock file (µs) |");
    println!("|---|---:|---:|");
    let us = |f: &mut dyn FnMut()| timer.per_call(f) * 1e6;
    let rows: Vec<(&str, f64, f64)> = vec![
        (
            "forgo_lib_toml",
            us(&mut || {
                black_box(forgo_manifest(black_box(manifest), &plain));
            }),
            us(&mut || {
                black_box(forgo_lock(black_box(&lock), &plain));
            }),
        ),
        (
            "forgo_lib_toml (parsed for editing)",
            us(&mut || {
                black_box(forgo_manifest(black_box(manifest), &for_editing));
            }),
            us(&mut || {
                black_box(forgo_lock(black_box(&lock), &for_editing));
            }),
        ),
        (
            "toml-spanner (`Formatting::preserved_from`)",
            us(&mut || {
                black_box(spanner_manifest(black_box(manifest)));
            }),
            us(&mut || {
                black_box(spanner_lock(black_box(&lock)));
            }),
        ),
        (
            "toml_edit",
            us(&mut || {
                black_box(edit_manifest(black_box(manifest)));
            }),
            us(&mut || {
                black_box(edit_lock(black_box(&lock)));
            }),
        ),
    ];
    let base = rows.last().map(|r| (r.1, r.2)).expect("rows");
    for (name, m, l) in &rows {
        if *name == "toml_edit" {
            println!("| {name} | {} | {} |", fixed(*m), fixed(*l));
        } else {
            println!(
                "| {name} | {} ({:.1}× faster than toml_edit) | {} ({:.1}×) |",
                fixed(*m),
                base.0 / m,
                fixed(*l),
                base.1 / l
            );
        }
    }
    println!();
}
