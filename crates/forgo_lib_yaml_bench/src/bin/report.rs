//! The comparison report, as GitHub-flavored markdown on stdout.
//!
//!   cargo run --release --manifest-path crates/forgo_lib_yaml_bench/Cargo.toml --bin report
//!
//! Options: `--quick` (fewer timing runs), `--compliance` or `--performance`
//! (one half only), `--verbose` (failing cases on stderr).
//!
//! Compliance runs every library in worker subprocesses, one case at a
//! time, so a crash (stack overflow, abort) or hang fails that case only.
//! Performance runs in this process, one library at a time.

use forgo_lib_yaml_bench::data::{Data, json_eq, parse_json_many};
use forgo_lib_yaml_bench::suite::{
    self, Case, exact_line, first_difference, normalize, parse_test_event,
};
use forgo_lib_yaml_bench::{
    EVENT_LIBS, LOAD_LIBS, RUSTC, small_document, synthetic_inputs, version,
};
use std::alloc::{GlobalAlloc, Layout, System};
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering::Relaxed};
use std::sync::mpsc;
use std::time::{Duration, Instant};

// ---------------------------------------------------------------------------
// Allocation counting (off while timing)
// ---------------------------------------------------------------------------

struct Counting;

static COUNTING: AtomicBool = AtomicBool::new(false);
static ALLOCS: AtomicUsize = AtomicUsize::new(0);
static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

fn grow(n: usize) {
    let live = LIVE.fetch_add(n, Relaxed) + n;
    PEAK.fetch_max(live, Relaxed);
}

// SAFETY: delegates to the system allocator and only updates counters.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        if COUNTING.load(Relaxed) {
            ALLOCS.fetch_add(1, Relaxed);
            grow(l.size());
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
                grow(new - l.size());
            } else {
                LIVE.fetch_sub((l.size() - new).min(LIVE.load(Relaxed)), Relaxed);
            }
        }
        unsafe { System.realloc(p, l, new) }
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

// ---------------------------------------------------------------------------
// Options and environment
// ---------------------------------------------------------------------------

struct Opts {
    quick: bool,
    compliance: bool,
    performance: bool,
    verbose: bool,
}

fn opts() -> Opts {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let has = |f: &str| args.iter().any(|a| a == f);
    let only_c = has("--compliance");
    let only_p = has("--performance");
    Opts {
        quick: has("--quick"),
        compliance: only_c || !only_p,
        performance: only_p || !only_c,
        verbose: has("--verbose"),
    }
}

fn cmd(program: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(program).args(args).output().ok()?;
    let s = String::from_utf8(out.stdout).ok()?.trim().to_string();
    (!s.is_empty()).then_some(s)
}

fn machine() -> String {
    let cpu = cmd("sysctl", &["-n", "machdep.cpu.brand_string"])
        .or_else(|| {
            std::fs::read_to_string("/proc/cpuinfo").ok().and_then(|t| {
                t.lines()
                    .find(|l| l.starts_with("model name"))
                    .and_then(|l| l.split(':').nth(1))
                    .map(|s| s.trim().to_string())
            })
        })
        .unwrap_or_else(|| "unknown CPU".into());
    let os = match cmd("sw_vers", &["-productVersion"]) {
        Some(v) => format!("macOS {v}"),
        None => cmd("uname", &["-sr"]).unwrap_or_else(|| std::env::consts::OS.into()),
    };
    format!("{cpu}, {os}")
}

/// Today's date as YYYY-MM-DD (local time when `date` is available).
fn today() -> String {
    cmd("date", &["+%Y-%m-%d"]).unwrap_or_else(today_utc)
}

fn today_utc() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs()) as i64;
    let z = secs.div_euclid(86_400) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

// ---------------------------------------------------------------------------
// Isolation
// ---------------------------------------------------------------------------

enum Outcome<T> {
    Done(T),
    Panicked(String),
    TimedOut,
}

fn isolated<T: Send + 'static>(
    timeout: Duration,
    f: impl FnOnce() -> T + Send + 'static,
) -> Outcome<T> {
    let (tx, rx) = mpsc::channel();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || {
            let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
            let _ = tx.send(r);
        })
        .expect("spawn worker thread");
    match rx.recv_timeout(timeout) {
        Ok(Ok(v)) => Outcome::Done(v),
        Ok(Err(p)) => Outcome::Panicked(
            p.downcast_ref::<&str>()
                .map(|s| s.to_string())
                .or_else(|| p.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "panic".into()),
        ),
        Err(_) => Outcome::TimedOut,
    }
}

// ---------------------------------------------------------------------------
// Compliance: workers
// ---------------------------------------------------------------------------

const CASE_TIMEOUT: Duration = Duration::from_secs(5);

/// `1`, `0`, or `-` (not applicable).
fn flag(v: Option<bool>) -> &'static str {
    match v {
        Some(true) => "1",
        Some(false) => "0",
        None => "-",
    }
}

/// What one case produced, as one line: `case <i> <status> <accepted> <a> <b> | detail`.
/// For event parsers `a`/`b` are exact/data events; for loaders JSON/round trip.
fn event_case(lib: usize, case: &Case) -> (Option<bool>, Option<bool>, Option<bool>, String) {
    let l = &EVENT_LIBS[lib];
    let got = (l.events)(&case.yaml);
    let accepted = got.is_ok();
    if case.error {
        let detail = if accepted {
            "accepted invalid input".to_string()
        } else {
            String::new()
        };
        return (Some(accepted), None, None, detail);
    }
    let Some(expected_text) = &case.events else {
        return (Some(accepted), None, None, String::new());
    };
    let expected = parse_test_event(expected_text).expect("readable test.event");
    let got = match got {
        Ok(g) => g,
        Err(e) => {
            return (
                Some(false),
                l.exact.then_some(false),
                Some(false),
                format!("rejected: {e}"),
            );
        }
    };
    let mut detail = String::new();
    let exact = l.exact.then(|| {
        let want: Vec<String> = expected.iter().filter_map(exact_line).collect();
        let have: Vec<String> = got
            .iter()
            .map(|e| exact_line(e).unwrap_or_else(|| format!("{e:?}")))
            .collect();
        let ok = want == have;
        if !ok {
            detail = format!("exact: {}", first_difference(&want, &have));
        }
        ok
    });
    let (want, have) = (normalize(&expected), normalize(&got));
    let data = want == have;
    if !data {
        detail = format!("data: {}", first_difference(&want, &have));
    }
    (Some(true), exact, Some(data), detail)
}

fn load_case(lib: usize, case: &Case) -> (Option<bool>, Option<bool>, Option<bool>, String) {
    let l = &LOAD_LIBS[lib];
    let got = (l.load)(&case.yaml);
    if case.error {
        let detail = if got.is_ok() {
            "accepted invalid input".to_string()
        } else {
            String::new()
        };
        return (Some(got.is_ok()), None, None, detail);
    }
    let docs = match got {
        Ok(d) => d,
        Err(e) => {
            return (
                Some(false),
                case.json.as_ref().map(|_| false),
                l.emit.map(|_| false),
                format!("rejected: {e}"),
            );
        }
    };
    let mut detail = String::new();
    let json = case.json.as_ref().map(|text| {
        let expected = parse_json_many(text).expect("readable in.json");
        let got: Result<Vec<Data>, String> = docs.iter().map(Data::to_json).collect();
        match got {
            Ok(g)
                if g.len() == expected.len()
                    && g.iter().zip(&expected).all(|(a, b)| json_eq(a, b)) =>
            {
                true
            }
            Ok(g) => {
                let show = |v: &[Data]| {
                    v.iter()
                        .map(|d| d.to_string())
                        .collect::<Vec<_>>()
                        .join(" ")
                };
                detail = format!("json: expected {}, got {}", show(&expected), show(&g));
                false
            }
            Err(e) => {
                detail = format!("json: {e}");
                false
            }
        }
    });
    let roundtrip = l.emit.map(|emit| match emit(&case.yaml) {
        Ok(text) => match (l.load)(&text) {
            Ok(again)
                if again.len() == docs.len() && again.iter().zip(&docs).all(|(a, b)| a.same(b)) =>
            {
                true
            }
            Ok(again) => {
                let show = |v: &[Data]| {
                    v.iter()
                        .map(|d| d.to_string())
                        .collect::<Vec<_>>()
                        .join(" ")
                };
                detail = format!(
                    "roundtrip: {} became {} via {text:?}",
                    show(&docs),
                    show(&again)
                );
                false
            }
            Err(e) => {
                detail = format!("roundtrip: output doesn't load ({e}): {text:?}");
                false
            }
        },
        Err(e) => {
            detail = format!("roundtrip: emit failed: {e}");
            false
        }
    });
    (Some(true), json, roundtrip, detail)
}

fn worker(kind: &str, lib: usize, start: usize) {
    std::panic::set_hook(Box::new(|_| {}));
    let cases = suite::discover();
    let stdout = std::io::stdout();
    for (i, case) in cases.into_iter().enumerate().skip(start) {
        {
            let mut out = stdout.lock();
            let _ = writeln!(out, "start {i}");
            let _ = out.flush();
        }
        let id = case.id.clone();
        let kind_owned = kind.to_string();
        let outcome = isolated(CASE_TIMEOUT, move || {
            if kind_owned == "events" {
                event_case(lib, &case)
            } else {
                load_case(lib, &case)
            }
        });
        let line = match outcome {
            Outcome::Done((acc, a, b, detail)) => {
                format!(
                    "case {i} ok {} {} {} | {detail}",
                    flag(acc),
                    flag(a),
                    flag(b)
                )
            }
            Outcome::Panicked(msg) => {
                format!("case {i} panic - - - | panic: {}", msg.replace('\n', " "))
            }
            Outcome::TimedOut => {
                format!("case {i} timeout - - - | timed out after {CASE_TIMEOUT:?}")
            }
        };
        let timed_out = line.contains(" timeout ");
        {
            let mut out = stdout.lock();
            let _ = writeln!(out, "{line}");
            let _ = out.flush();
        }
        let _ = id;
        if timed_out {
            // The hung thread can't be stopped; start a fresh process.
            std::process::exit(3);
        }
    }
    std::process::exit(0);
}

// ---------------------------------------------------------------------------
// Compliance: aggregation
// ---------------------------------------------------------------------------

#[derive(Default, Clone)]
struct Score {
    valid_ok: usize,
    invalid_ok: usize,
    /// (passed, applicable)
    a: (usize, usize),
    b: (usize, usize),
    all: usize,
    panics: usize,
    timeouts: usize,
    crashes: usize,
    failures: Vec<(String, String)>,
}

/// Per case: status (`ok`, `panic`, `timeout`, `crash`), accepted, check a,
/// check b (`1`, `0` or `-`), and a detail for failures.
type CaseResult = (String, String, String, String, String);

fn failed(status: &str, detail: &str) -> CaseResult {
    (
        status.into(),
        "-".into(),
        "-".into(),
        "-".into(),
        detail.into(),
    )
}

/// Run a worker process from case `next` on until every case has a
/// result, starting a new process after a crash, or after `idle` without
/// output (a hang). Workers print `start <i>` before each case and
/// `case <i> ...` after it; `parse` reads the rest of that line.
fn collect(
    cases: &[Case],
    idle: Duration,
    spawn: &dyn Fn(usize) -> Command,
    parse: &dyn Fn(&str, &Case) -> Option<CaseResult>,
) -> Vec<Option<CaseResult>> {
    let mut results: Vec<Option<CaseResult>> = vec![None; cases.len()];
    let mut next = 0;
    while next < cases.len() {
        let mut child = spawn(next)
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn worker");
        let stdout = child.stdout.take().unwrap();
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                if tx.send(line).is_err() {
                    break;
                }
            }
        });
        let (mut in_flight, mut hung, mut progressed) = (None, false, false);
        loop {
            match rx.recv_timeout(idle) {
                Ok(line) => {
                    if let Some(i) = line.strip_prefix("start ") {
                        in_flight = i.parse::<usize>().ok();
                        progressed = true;
                    } else if let Some(rest) = line.strip_prefix("case ") {
                        let (i, rest) = rest.split_once(' ').unwrap_or((rest, ""));
                        let i: usize = i.parse().expect("case index");
                        results[i] = Some(
                            parse(rest, &cases[i])
                                .unwrap_or_else(|| failed("crash", "unreadable result")),
                        );
                        in_flight = None;
                        next = i + 1;
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    hung = true;
                    let _ = child.kill();
                    break;
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
        let _ = child.wait();
        match in_flight {
            Some(i) => {
                results[i] = Some(if hung {
                    failed("timeout", "no output in time")
                } else {
                    failed("crash", "process crashed")
                });
                next = i + 1;
            }
            None if !progressed && next < cases.len() => {
                // The worker didn't start a case at all: give up on the rest.
                for r in results.iter_mut().skip(next) {
                    *r = Some(failed("crash", "worker failed to start"));
                }
                next = cases.len();
            }
            None => {}
        }
    }
    results
}

/// The fields of a Rust worker's line: `status accepted a b | detail`.
fn parse_worker_line(rest: &str, _: &Case) -> Option<CaseResult> {
    let (head, detail) = rest.split_once(" | ").unwrap_or((rest, ""));
    let f: Vec<&str> = head.split(' ').collect();
    Some((
        f.first()?.to_string(),
        f.get(1)?.to_string(),
        f.get(2)?.to_string(),
        f.get(3)?.to_string(),
        detail.into(),
    ))
}

/// Tally results. `dup` marks valid cases whose mappings repeat a key:
/// YAML 1.2.2 §3.2.1.1 makes that an error a loader may report, so a
/// loader rejecting them passes. For event parsers, `a` is exact events
/// (when `exact`) and `b` data events; for loaders `a` is JSON and `b`
/// the round trip (when `emits`).
fn score(
    events: bool,
    exact: bool,
    emits: bool,
    cases: &[Case],
    dup: &[bool],
    results: Vec<Option<CaseResult>>,
) -> Score {
    let mut s = Score::default();
    for (ci, (case, r)) in cases.iter().zip(results).enumerate() {
        let (status, acc, a, b, detail) = r.unwrap_or_else(|| failed("crash", "no result"));
        let ok = status == "ok";
        match status.as_str() {
            "panic" => s.panics += 1,
            "timeout" => s.timeouts += 1,
            "crash" => s.crashes += 1,
            _ => {}
        }
        let accepted = acc == "1";
        let excused = !events && dup[ci] && ok && !accepted;
        let error_ok = (ok && accepted != case.error) || excused;
        if error_ok {
            if case.error {
                s.invalid_ok += 1;
            } else {
                s.valid_ok += 1;
            }
        }
        // Applicability doesn't depend on the outcome: a crash fails every
        // check that applies to the case.
        let (a_applies, b_applies) = if events {
            (
                !case.error && case.events.is_some() && exact,
                !case.error && case.events.is_some(),
            )
        } else {
            (!case.error && case.json.is_some(), !case.error && emits)
        };
        if a_applies {
            s.a.1 += 1;
            s.a.0 += usize::from(excused || (ok && a == "1"));
        }
        if b_applies {
            s.b.1 += 1;
            s.b.0 += usize::from(excused || (ok && b == "1"));
        }
        // Overall: every applicable check; for event parsers the data-level
        // events stand in for exact events, which not every API can give.
        let all = excused
            || error_ok
                && (case.error
                    || if events {
                        !b_applies || b == "1"
                    } else {
                        (!a_applies || a == "1") && (!b_applies || b == "1")
                    });
        if all {
            s.all += 1;
            if !detail.is_empty() {
                s.failures.push((case.id.clone(), detail));
            }
        } else {
            let detail = if detail.is_empty() {
                format!("{status} accepted={acc}")
            } else {
                detail
            };
            s.failures.push((case.id.clone(), detail));
        }
    }
    s
}

fn run_library(kind: &'static str, lib: usize, cases: &[Case], dup: &[bool]) -> Score {
    let exe = std::env::current_exe().expect("current exe");
    let spawn = |next: usize| {
        let mut c = Command::new(&exe);
        c.args(["--worker", kind, &lib.to_string(), &next.to_string()]);
        c
    };
    // Workers enforce the per-case limit themselves; this only catches a
    // worker that stops responding altogether.
    let results = collect(cases, Duration::from_secs(60), &spawn, &parse_worker_line);
    if kind == "events" {
        score(true, EVENT_LIBS[lib].exact, false, cases, dup, results)
    } else {
        score(
            false,
            false,
            LOAD_LIBS[lib].emit.is_some(),
            cases,
            dup,
            results,
        )
    }
}

fn run_all(kind: &'static str, n: usize, cases: &[Case], dup: &[bool]) -> Vec<Score> {
    std::thread::scope(|scope| {
        let handles: Vec<_> = (0..n)
            .map(|i| scope.spawn(move || run_library(kind, i, cases, dup)))
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().expect("library run"))
            .collect()
    })
}

// ---------------------------------------------------------------------------
// Compliance: other languages (stretch; installed by other/setup.sh)
// ---------------------------------------------------------------------------

struct OtherLib {
    name: &'static str,
    /// Program and leading arguments; `cases.txt` and the start index follow.
    argv: Vec<String>,
    env: Vec<(String, String)>,
}

fn other_libs() -> Vec<OtherLib> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let dest = root.join("target/other");
    let other = root.join("other");
    let p = |x: std::path::PathBuf| x.to_string_lossy().into_owned();
    let mut libs = Vec::new();
    let python = dest.join("venv/bin/python");
    if python.exists() {
        for (name, lib) in [
            ("PyYAML (Python)", "pyyaml"),
            ("ruamel.yaml (Python)", "ruamel"),
        ] {
            libs.push(OtherLib {
                name,
                argv: vec![
                    p(python.clone()),
                    "-I".into(),
                    p(other.join("py_driver.py")),
                    lib.into(),
                ],
                env: Vec::new(),
            });
        }
    }
    if dest.join("node/node_modules").exists() && cmd("node", &["--version"]).is_some() {
        for (name, lib) in [
            ("yaml (JavaScript)", "yaml"),
            ("js-yaml (JavaScript)", "js-yaml"),
        ] {
            libs.push(OtherLib {
                name,
                argv: vec!["node".into(), p(other.join("js_driver.mjs")), lib.into()],
                env: vec![("NODE_DEPS".into(), p(dest.join("node")))],
            });
        }
    }
    if dest.join("go-driver").exists() {
        libs.push(OtherLib {
            name: "go-yaml (Go)",
            argv: vec![p(dest.join("go-driver"))],
            env: Vec::new(),
        });
    }
    libs
}

fn other_command(l: &OtherLib, cases_file: &std::path::Path, start: usize) -> Command {
    let mut c = Command::new(&l.argv[0]);
    c.args(&l.argv[1..]).arg(cases_file).arg(start.to_string());
    for (k, v) in &l.env {
        c.env(k, v);
    }
    c
}

/// A driver's `{"ok", "docs", "rt", "err"}` line, checked against the case.
fn parse_driver_line(rest: &str, case: &Case) -> Option<CaseResult> {
    let Data::Map(fields) = parse_json_many(rest).ok()?.pop()? else {
        return None;
    };
    let get = |k: &str| {
        fields
            .iter()
            .find(|(kk, _)| matches!(kk, Data::Str(s) if s == k))
            .map(|(_, v)| v)
    };
    let ok = matches!(get("ok"), Some(Data::Bool(true)));
    let mut detail = match get("err") {
        Some(Data::Str(e)) => e.clone(),
        _ => String::new(),
    };
    if !ok {
        let detail = if case.error {
            String::new()
        } else {
            format!("rejected: {detail}")
        };
        return Some(("ok".into(), "0".into(), "-".into(), "-".into(), detail));
    }
    let json = match (&case.json, get("docs")) {
        (Some(text), Some(Data::Seq(docs))) => {
            let expected = parse_json_many(text).ok()?;
            let same = docs.len() == expected.len()
                && docs.iter().zip(&expected).all(|(a, b)| json_eq(a, b));
            if !same && detail.is_empty() {
                let show = |v: &[Data]| {
                    v.iter()
                        .map(|d| d.to_string())
                        .collect::<Vec<_>>()
                        .join(" ")
                };
                detail = format!("json: expected {}, got {}", show(&expected), show(docs));
            }
            if same { "1" } else { "0" }
        }
        _ => "-",
    };
    let rt = match get("rt") {
        Some(Data::Bool(true)) => "1",
        Some(Data::Bool(false)) => "0",
        _ => "-",
    };
    Some(("ok".into(), "1".into(), json.into(), rt.into(), detail))
}

fn other_languages(o: &Opts, cases: &[Case], dup: &[bool]) {
    let libs = other_libs();
    println!("### Other languages\n");
    if libs.is_empty() {
        println!(
            "Not run: install the parsers with `crates/forgo_lib_yaml_bench/other/setup.sh`.\n"
        );
        return;
    }
    let dest = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/other");
    let cases_file = dest.join("cases.txt");
    let list: String = cases
        .iter()
        .map(|c| format!("{}\n", c.path.display()))
        .collect();
    std::fs::write(&cases_file, list).expect("write cases.txt");
    let results: Vec<(String, Score)> = std::thread::scope(|scope| {
        let handles: Vec<_> = libs
            .iter()
            .map(|l| {
                let cases_file = &cases_file;
                scope.spawn(move || {
                    let version = other_command(l, cases_file, cases.len())
                        .output()
                        .ok()
                        .and_then(|out| String::from_utf8(out.stdout).ok())
                        .and_then(|t| {
                            t.lines()
                                .find_map(|x| x.strip_prefix("version ").map(str::to_string))
                        })
                        .unwrap_or_else(|| "?".into());
                    let spawn = |next: usize| other_command(l, cases_file, next);
                    let r = collect(cases, CASE_TIMEOUT * 2, &spawn, &parse_driver_line);
                    (version, score(false, false, true, cases, dup, r))
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().expect("driver run"))
            .collect()
    });
    let valid = cases.iter().filter(|c| !c.error).count();
    let invalid = cases.len() - valid;
    println!("| library | version | valid | invalid | JSON | round trip | overall |");
    println!("|---|---|--:|--:|--:|--:|--:|");
    let scores: Vec<&Score> = results.iter().map(|(_, s)| s).collect();
    let best = |f: &dyn Fn(&Score) -> usize| scores.iter().map(|s| f(s)).max().unwrap_or(0);
    let (bv, bi, ba, bb, ball) = (
        best(&|s| s.valid_ok),
        best(&|s| s.invalid_ok),
        best(&|s| s.a.0),
        best(&|s| s.b.0),
        best(&|s| s.all),
    );
    for (l, (version, s)) in libs.iter().zip(&results) {
        println!(
            "| {} | {version} | {} | {} | {} | {} | {} |",
            l.name,
            bold_if(frac(s.valid_ok, valid), s.valid_ok == bv),
            bold_if(frac(s.invalid_ok, invalid), s.invalid_ok == bi),
            bold_if(frac(s.a.0, s.a.1), s.a.0 == ba),
            bold_if(frac(s.b.0, s.b.1), s.b.0 == bb),
            bold_if(pct(s.all, cases.len()), s.all == ball),
        );
    }
    println!();
    println!(
        "- The same cases and checks as the Rust loaders, through each library's usual API: PyYAML `safe_load_all` and `safe_dump_all`; ruamel.yaml `YAML()` (round-trip mode); yaml `parseAllDocuments` and `stringify`; js-yaml `loadAll` and `dump`; go-yaml (`go.yaml.in/yaml/v3`) `Decoder` and `Encoder` into `any`."
    );
    println!(
        "- Installed by `other/setup.sh` into `target/other/`. Values typed beyond the YAML 1.2 core schema (timestamps, binary, sets, ordered maps, YAML 1.1 booleans) count as JSON mismatches."
    );
    let names: Vec<&str> = libs.iter().map(|l| l.name).collect();
    let owned: Vec<Score> = results.iter().map(|(_, s)| s.clone()).collect();
    if let Some(n) = robustness_note(&names, &owned) {
        println!(
            "- Errors outside the parser's normal error path, hangs (> {}s) and crashes count as failures: {n}.",
            (CASE_TIMEOUT * 2).as_secs()
        );
    }
    println!();
    if o.verbose {
        for (l, (_, s)) in libs.iter().zip(&results) {
            for (id, d) in &s.failures {
                eprintln!("other  {:<20} {id:<8} {d}", l.name);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Markdown helpers
// ---------------------------------------------------------------------------

fn pct(n: usize, d: usize) -> String {
    if d == 0 {
        "—".into()
    } else {
        format!("{:.1}%", 100.0 * n as f64 / d as f64)
    }
}

fn frac(n: usize, d: usize) -> String {
    if d == 0 {
        "—".into()
    } else {
        format!("{n}/{d}")
    }
}

fn bold_if(s: String, best: bool) -> String {
    if best && s != "—" {
        format!("**{s}**")
    } else {
        s
    }
}

fn robustness_note(names: &[&str], scores: &[Score]) -> Option<String> {
    let parts: Vec<String> = names
        .iter()
        .zip(scores)
        .filter(|(_, s)| s.panics + s.timeouts + s.crashes > 0)
        .map(|(n, s)| {
            let mut bits = Vec::new();
            if s.panics > 0 {
                bits.push(format!("{} panics", s.panics));
            }
            if s.timeouts > 0 {
                bits.push(format!("{} hangs", s.timeouts));
            }
            if s.crashes > 0 {
                bits.push(format!("{} crashes", s.crashes));
            }
            format!("{n} ({})", bits.join(", "))
        })
        .collect();
    (!parts.is_empty()).then(|| parts.join("; "))
}

fn compliance(o: &Opts) {
    let cases = suite::discover();
    if cases.is_empty() {
        println!(
            "\n> The yaml-test-suite data is missing: run `crates/forgo_lib_yaml/scripts/fetch-yaml-test-suite.sh`.\n"
        );
        return;
    }
    let valid = cases.iter().filter(|c| !c.error).count();
    let invalid = cases.len() - valid;
    let with_json = cases
        .iter()
        .filter(|c| !c.error && c.json.is_some())
        .count();

    println!("## Compliance\n");
    println!(
        "Official [yaml-test-suite](https://github.com/yaml/yaml-test-suite) release `data-2022-01-17`: {} cases, {valid} valid and {invalid} invalid. Every library runs with its default options.\n",
        cases.len()
    );

    // Valid cases whose mappings repeat a key (compared by value, aliases
    // resolved).
    let dup: Vec<bool> = cases
        .iter()
        .map(|c| {
            !c.error
                && forgo_lib_yaml_bench::libs::forgo::load(&c.yaml)
                    .is_ok_and(|docs| docs.iter().any(Data::has_duplicate_keys))
        })
        .collect();
    let dup_ids: Vec<&str> = cases
        .iter()
        .zip(&dup)
        .filter(|(_, d)| **d)
        .map(|(c, _)| c.id.as_str())
        .collect();
    let ev = run_all("events", EVENT_LIBS.len(), &cases, &dup);
    let ld = run_all("load", LOAD_LIBS.len(), &cases, &dup);

    // Event parsers
    println!("### Event parsers\n");
    println!("| library | valid | invalid | events | data events | overall |");
    println!("|---|--:|--:|--:|--:|--:|");
    let best = |f: &dyn Fn(&Score) -> usize, v: &[Score]| v.iter().map(f).max().unwrap_or(0);
    let (bv, bi, ba, bb, ball) = (
        best(&|s| s.valid_ok, &ev),
        best(&|s| s.invalid_ok, &ev),
        best(&|s| s.a.0, &ev),
        best(&|s| s.b.0, &ev),
        best(&|s| s.all, &ev),
    );
    for (l, s) in EVENT_LIBS.iter().zip(&ev) {
        println!(
            "| {} | {} | {} | {} | {} | {} |",
            l.name,
            bold_if(frac(s.valid_ok, valid), s.valid_ok == bv),
            bold_if(frac(s.invalid_ok, invalid), s.invalid_ok == bi),
            bold_if(frac(s.a.0, s.a.1), s.a.0 == ba),
            bold_if(frac(s.b.0, s.b.1), s.b.0 == bb),
            bold_if(pct(s.all, cases.len()), s.all == ball),
        );
    }
    println!();
    println!(
        "- **valid**: valid inputs parsed without error. **invalid**: invalid inputs rejected."
    );
    println!(
        "- **events**: the event stream equals `test.event` exactly. **—**: the event API doesn't report anchor names, flow vs. block, or explicit `---`/`...`."
    );
    println!(
        "- **data events**: the same nodes, anchors, tags, scalar types and values; styles and document markers are ignored."
    );
    println!(
        "- **overall**: share of the {} cases where every applicable check (valid or invalid, then data events) passes.",
        cases.len()
    );
    println!(
        "- granit-parser runs with `emit_anchor_events` on, and its explicit `...` is read from the event's span."
    );
    let names: Vec<&str> = EVENT_LIBS.iter().map(|l| l.name).collect();
    if let Some(n) = robustness_note(&names, &ev) {
        println!(
            "- Panics, hangs (> {}s) and crashes count as failures: {n}.",
            CASE_TIMEOUT.as_secs()
        );
    }
    println!();

    // Loaders
    println!("### Loaders\n");
    println!("| library | valid | invalid | JSON | round trip | overall |");
    println!("|---|--:|--:|--:|--:|--:|");
    let (bv, bi, ba, bb, ball) = (
        best(&|s| s.valid_ok, &ld),
        best(&|s| s.invalid_ok, &ld),
        best(&|s| s.a.0, &ld),
        best(&|s| s.b.0, &ld),
        best(&|s| s.all, &ld),
    );
    for (l, s) in LOAD_LIBS.iter().zip(&ld) {
        println!(
            "| {} | {} | {} | {} | {} | {} |",
            l.name,
            bold_if(frac(s.valid_ok, valid), s.valid_ok == bv),
            bold_if(frac(s.invalid_ok, invalid), s.invalid_ok == bi),
            bold_if(frac(s.a.0, s.a.1), s.a.0 == ba),
            bold_if(frac(s.b.0, s.b.1), s.b.0 == bb),
            bold_if(pct(s.all, cases.len()), s.all == ball),
        );
    }
    println!();
    println!(
        "- **JSON**: the loaded documents equal `in.json` ({with_json} valid cases have one; tags are ignored, numbers compare by value)."
    );
    println!(
        "- **round trip**: load, write with the library's own emitter, load again: the data is unchanged."
    );
    println!(
        "- **overall**: share of the {} cases where every applicable check passes.",
        cases.len()
    );
    println!(
        "- {} repeat a mapping key, which YAML 1.2.2 §3.2.1.1 makes an error a loader may report: rejecting them counts as a pass.",
        dup_ids
            .iter()
            .map(|i| format!("`{i}`"))
            .collect::<Vec<_>>()
            .join(" and ")
    );
    println!(
        "- serde-saphyr skips null documents in multi-document streams (documented), and reads `y` as `true`; granit-parser and serde-saphyr accept three under-indented inputs unless `strict_indentation` is set."
    );
    println!(
        "- noyalib (CST) is noyalib's lossless, format-preserving tree (`noyalib::cst`); it returns a document for streams that hold none (comments only), which fails the JSON check."
    );
    let names: Vec<&str> = LOAD_LIBS.iter().map(|l| l.name).collect();
    if let Some(n) = robustness_note(&names, &ld) {
        println!(
            "- Panics, hangs (> {}s) and crashes count as failures: {n}.",
            CASE_TIMEOUT.as_secs()
        );
    }
    println!();

    other_languages(o, &cases, &dup);

    if o.verbose {
        for (l, s) in EVENT_LIBS.iter().zip(&ev) {
            for (id, d) in &s.failures {
                eprintln!("events {:<15} {id:<8} {d}", l.name);
            }
        }
        for (l, s) in LOAD_LIBS.iter().zip(&ld) {
            for (id, d) in &s.failures {
                eprintln!("load   {:<15} {id:<8} {d}", l.name);
            }
        }
    } else {
        for (kind, scores, names) in [
            (
                "events",
                &ev,
                EVENT_LIBS.iter().map(|l| l.name).collect::<Vec<_>>(),
            ),
            (
                "load",
                &ld,
                LOAD_LIBS.iter().map(|l| l.name).collect::<Vec<_>>(),
            ),
        ] {
            for (n, s) in names.iter().zip(scores.iter()) {
                if *n == "forgo_lib_yaml" {
                    for (id, d) in &s.failures {
                        eprintln!("forgo_lib_yaml {kind} failure {id}: {d}");
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Performance
// ---------------------------------------------------------------------------

/// Longest a single measurement (one library, one input) may take.
const MEASURE_TIMEOUT: Duration = Duration::from_secs(60);

/// Median seconds per call of `f`: 11 runs after a warm-up (3 with
/// `--quick`), batching calls under ~2 ms. A call over 1 s is timed once.
fn median_secs(quick: bool, f: &dyn Fn() -> Result<usize, String>) -> Result<f64, String> {
    let t = Instant::now();
    std::hint::black_box(f()?);
    let first = t.elapsed().as_secs_f64();
    if first > 1.0 {
        return Ok(first);
    }
    let batch = ((0.002 / first.max(1e-9)).ceil() as usize).max(1);
    let runs = if quick { 3 } else { 11 };
    let mut samples: Vec<f64> = (0..runs)
        .map(|_| {
            let t = Instant::now();
            for _ in 0..batch {
                let _ = std::hint::black_box(f());
            }
            t.elapsed().as_secs_f64() / batch as f64
        })
        .collect();
    samples.sort_by(f64::total_cmp);
    Ok(samples[samples.len() / 2])
}

/// Allocation count and peak live heap (bytes) of one call.
fn memory(f: &dyn Fn() -> Result<usize, String>) -> Result<(usize, usize), String> {
    ALLOCS.store(0, Relaxed);
    LIVE.store(0, Relaxed);
    PEAK.store(0, Relaxed);
    COUNTING.store(true, Relaxed);
    let r = std::hint::black_box(f());
    COUNTING.store(false, Relaxed);
    r?;
    Ok((ALLOCS.load(Relaxed), PEAK.load(Relaxed)))
}

/// One measurement, run in a child process: prints `ok <values>`,
/// `wrong <why>` or `err <why>`.
fn measure_worker(what: &str, lib: usize, input: &str, quick: bool) {
    let inputs = synthetic_inputs();
    let small = small_document();
    let input = inputs
        .into_iter()
        .chain(std::iter::once(small))
        .find(|i| i.name == input)
        .expect("input");
    let text = input.text.as_str();
    let mb = text.len() as f64 / 1e6;
    let checked = |n: usize| -> Result<usize, String> {
        if n == input.docs {
            Ok(n)
        } else {
            Err(format!("wrong: loaded {n} of {} documents", input.docs))
        }
    };
    let out: Result<String, String> = match what {
        "events" => {
            let count = EVENT_LIBS[lib].count;
            median_secs(quick, &|| count(text)).map(|s| format!("{}", mb / s))
        }
        "load" => {
            let bench = LOAD_LIBS[lib].bench;
            median_secs(quick, &|| bench(text).and_then(checked)).map(|s| format!("{}", mb / s))
        }
        "small" => {
            let bench = LOAD_LIBS[lib].bench;
            median_secs(quick, &|| bench(text).and_then(checked)).map(|s| format!("{}", s * 1e6))
        }
        "memory" => {
            let bench = LOAD_LIBS[lib].bench;
            memory(&|| bench(text).and_then(checked))
                .map(|(a, p)| format!("{a} {}", p as f64 / 1e6))
        }
        "writeback" => forgo_lib_yaml_bench::libs::forgo::writer(text).and_then(|e| {
            let bytes = e()?;
            median_secs(quick, &|| e()).map(|s| format!("{}", bytes as f64 / 1e6 / s))
        }),
        "emit" => match LOAD_LIBS[lib].emitter {
            None => Err("no emitter".into()),
            Some(make) => make(text).and_then(|e| {
                let bytes = e()?;
                median_secs(quick, &|| e()).map(|s| format!("{}", bytes as f64 / 1e6 / s))
            }),
        },
        _ => Err("unknown measurement".into()),
    };
    match out {
        Ok(v) => println!("ok {v}"),
        Err(e) if e.starts_with("wrong: ") => println!("wrong {}", &e[7..]),
        Err(e) => println!("err {}", e.replace('\n', " ")),
    }
}

/// A table cell: a number, or why there isn't one.
#[derive(Clone)]
enum Cell {
    Val(Vec<f64>),
    /// Not measured for this library (shown as `—`).
    Na,
    Err(String),
    Wrong(String),
    Timeout,
    Crash,
}

fn measure(what: &str, lib: usize, input: &str, quick: bool) -> Cell {
    let exe = std::env::current_exe().expect("current exe");
    let mut args = vec!["--measure", what, &lib.to_string(), input]
        .into_iter()
        .map(String::from)
        .collect::<Vec<_>>();
    if quick {
        args.push("--quick".into());
    }
    let mut child = Command::new(exe)
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn measurement");
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if start.elapsed() > MEASURE_TIMEOUT => {
                let _ = child.kill();
                let _ = child.wait();
                return Cell::Timeout;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(20)),
            Err(_) => return Cell::Crash,
        }
    }
    let mut out = String::new();
    let _ = std::io::Read::read_to_string(&mut child.stdout.take().unwrap(), &mut out);
    let out = out.trim();
    if let Some(v) = out.strip_prefix("ok ") {
        Cell::Val(v.split(' ').filter_map(|x| x.parse().ok()).collect())
    } else if let Some(w) = out.strip_prefix("wrong ") {
        Cell::Wrong(w.into())
    } else if let Some(e) = out.strip_prefix("err ") {
        Cell::Err(e.into())
    } else {
        Cell::Crash
    }
}

/// Format a column, bolding the best value (max or min).
fn column(
    cells: &[Cell],
    idx: usize,
    higher_better: bool,
    fmt: &dyn Fn(f64) -> String,
) -> Vec<String> {
    let vals: Vec<Option<f64>> = cells
        .iter()
        .map(|c| match c {
            Cell::Val(v) => v.get(idx).copied(),
            _ => None,
        })
        .collect();
    let present = vals.iter().flatten().copied();
    let best = if higher_better {
        present.fold(f64::NEG_INFINITY, f64::max)
    } else {
        present.fold(f64::INFINITY, f64::min)
    };
    cells
        .iter()
        .zip(&vals)
        .map(|(c, v)| match (c, v) {
            (_, Some(x)) => bold_if(fmt(*x), fmt(*x) == fmt(best)),
            (Cell::Na, _) => "—".into(),
            (Cell::Err(_), _) => "error".into(),
            (Cell::Wrong(_), _) => "wrong".into(),
            (Cell::Timeout, _) => format!("> {} s", MEASURE_TIMEOUT.as_secs()),
            _ => "crash".into(),
        })
        .collect()
}

fn rate(x: f64) -> String {
    if x >= 100.0 {
        format!("{x:.0}")
    } else {
        format!("{x:.1}")
    }
}

fn table(header: &[&str], names: &[&str], cols: &[Vec<String>]) {
    println!("| {} |", header.join(" | "));
    println!("|---|{}", "--:|".repeat(header.len() - 1));
    for (i, n) in names.iter().enumerate() {
        let cells: Vec<&str> = cols.iter().map(|c| c[i].as_str()).collect();
        println!("| {n} | {} |", cells.join(" | "));
    }
    println!();
}

/// Notes for cells without a number.
fn cell_notes(rows: &[(&str, &str, &Cell)]) {
    for (lib, what, c) in rows {
        match c {
            Cell::Err(e) => println!(
                "- {lib}, {what}: error: {}",
                e.chars().take(120).collect::<String>()
            ),
            Cell::Wrong(w) => println!("- {lib}, {what}: {w}"),
            Cell::Crash => println!("- {lib}, {what}: the process crashed"),
            _ => {}
        }
    }
}

fn performance(o: &Opts) {
    let inputs = synthetic_inputs();
    let small = small_document();
    println!("## Performance\n");
    println!(
        "The same inputs for every library, each measurement in a fresh process with the system allocator; release build. Each number is the median of {} runs after a warm-up (a single run when one takes over 1 s; cut off after {} s).\n",
        if o.quick { "3 (`--quick`)" } else { "11" },
        MEASURE_TIMEOUT.as_secs()
    );
    println!("| input | size | content |");
    println!("|---|--:|---|");
    for i in inputs.iter().chain(std::iter::once(&small)) {
        let size = if i.text.len() >= 100_000 {
            format!("{:.2} MB", i.text.len() as f64 / 1e6)
        } else {
            format!("{:.1} KB", i.text.len() as f64 / 1e3)
        };
        println!("| {} | {size} | {} |", i.name, i.about);
    }
    println!();
    let header: Vec<&str> = std::iter::once("library")
        .chain(inputs.iter().map(|i| i.name))
        .collect();

    // Events
    let names: Vec<&str> = EVENT_LIBS.iter().map(|l| l.name).collect();
    let mut cols = Vec::new();
    let mut notes = Vec::new();
    for input in &inputs {
        let cells: Vec<Cell> = (0..EVENT_LIBS.len())
            .map(|i| measure("events", i, input.name, o.quick))
            .collect();
        cols.push(column(&cells, 0, true, &rate));
        notes.extend(
            cells
                .into_iter()
                .enumerate()
                .map(|(i, c)| (EVENT_LIBS[i].name, input.name, c)),
        );
        eprintln!("events/{} done", input.name);
    }
    println!("### Parse to events, MB/s (higher is better)\n");
    table(&header, &names, &cols);
    cell_notes(
        &notes
            .iter()
            .map(|(l, w, c)| (*l, *w, c))
            .collect::<Vec<_>>(),
    );

    // Load
    let names: Vec<&str> = LOAD_LIBS.iter().map(|l| l.name).collect();
    let mut cols = Vec::new();
    let mut notes = Vec::new();
    for input in &inputs {
        let cells: Vec<Cell> = (0..LOAD_LIBS.len())
            .map(|i| measure("load", i, input.name, o.quick))
            .collect();
        cols.push(column(&cells, 0, true, &rate));
        notes.extend(
            cells
                .into_iter()
                .enumerate()
                .map(|(i, c)| (LOAD_LIBS[i].name, input.name, c)),
        );
        eprintln!("load/{} done", input.name);
    }
    println!("### Load to the library's tree, MB/s (higher is better)\n");
    table(&header, &names, &cols);
    cell_notes(
        &notes
            .iter()
            .map(|(l, w, c)| (*l, *w, c))
            .collect::<Vec<_>>(),
    );
    println!(
        "- serde-saphyr and noyalib stop at 1,024 and 1,000 documents by default (serde-saphyr also at depth 64); these runs lift those limits."
    );
    println!();

    // Small document, memory, emit
    let small_cells: Vec<Cell> = (0..LOAD_LIBS.len())
        .map(|i| measure("small", i, small.name, o.quick))
        .collect();
    let mem: Vec<Cell> = (0..LOAD_LIBS.len())
        .map(|i| measure("memory", i, inputs[0].name, o.quick))
        .collect();
    // Lossless trees write their source back instead of rendering: their
    // speed goes with forgo_lib_yaml's write-back, not in the emit column.
    let emitted: Vec<Cell> = (0..LOAD_LIBS.len())
        .map(|i| measure("emit", i, inputs[0].name, o.quick))
        .collect();
    let emit: Vec<Cell> = emitted
        .iter()
        .zip(LOAD_LIBS)
        .map(|(c, l)| if l.lossless { Cell::Na } else { c.clone() })
        .collect();
    let mut writeback = vec![(
        "forgo_lib_yaml",
        measure("writeback", 0, inputs[0].name, o.quick),
    )];
    writeback.extend(
        emitted
            .iter()
            .zip(LOAD_LIBS)
            .filter(|(_, l)| l.lossless)
            .map(|(c, l)| (l.name, c.clone())),
    );
    eprintln!("small/memory/emit done");
    println!("### Small documents, memory and emitting\n");
    let cols = vec![
        column(&small_cells, 0, false, &|x| {
            if x >= 100.0 {
                format!("{x:.0}")
            } else {
                format!("{x:.1}")
            }
        }),
        column(&mem, 0, false, &|x| format!("{:.0}k", x / 1e3)),
        column(&mem, 1, false, &|x| format!("{x:.1}")),
        column(&emit, 0, true, &rate),
    ];
    table(
        &[
            "library",
            "1 KB load, µs",
            "allocations",
            "peak heap, MB",
            "emit, MB/s",
        ],
        &names,
        &cols,
    );
    let mut notes = Vec::new();
    for (i, l) in LOAD_LIBS.iter().enumerate() {
        notes.push((l.name, "1 KB load", small_cells[i].clone()));
        notes.push((l.name, "memory", mem[i].clone()));
        notes.push((l.name, "emit", emit[i].clone()));
    }
    cell_notes(
        &notes
            .iter()
            .map(|(l, w, c)| (*l, *w, c))
            .collect::<Vec<_>>(),
    );
    println!("- **1 KB load**: microseconds to load the 1 KB document (lower is better).");
    println!(
        "- **allocations** and **peak heap**: one load of the {:.2} MB k8s input, counted by a wrapping global allocator (lower is better).",
        inputs[0].text.len() as f64 / 1e6
    );
    println!(
        "- **emit**: rendering the loaded k8s tree as YAML with the library's own emitter, in MB of output per second (higher is better). forgo_lib_yaml's documents are reformatted first so that it renders every node; **—**: a lossless tree, which can only write its source back."
    );
    let wb: Vec<String> = writeback
        .iter()
        .filter_map(|(n, c)| match c {
            Cell::Val(v) => Some(format!("{n} {} MB/s", rate(v[0]))),
            _ => None,
        })
        .collect();
    println!(
        "- **Write back** an untouched loaded tree as its source text (not a render, so not in the emit column): {}.",
        wb.join(", ")
    );
    println!(
        "- Trees differ: forgo_lib_yaml's `Doc` also keeps comments, styles and source spans; serde-saphyr has no tree type and loads into a minimal generic serde value."
    );
    println!();
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("--worker") => {
            let lib: usize = args[3].parse().expect("library index");
            let start: usize = args[4].parse().expect("start case");
            worker(&args[2], lib, start);
            return;
        }
        Some("--measure") => {
            let lib: usize = args[3].parse().expect("library index");
            measure_worker(&args[2], lib, &args[4], args.iter().any(|a| a == "--quick"));
            return;
        }
        _ => {}
    }
    let o = opts();
    println!("# YAML libraries for Rust: compliance and performance\n");
    println!(
        "Generated {} by `crates/forgo_lib_yaml_bench` on {}; {}.\n",
        today(),
        machine(),
        RUSTC
    );
    println!("| library | version | parser | events | tree |");
    println!("|---|---|---|:-:|---|");
    let mut seen = Vec::new();
    for l in LOAD_LIBS {
        let ev = EVENT_LIBS
            .iter()
            .any(|e| e.krate == l.krate || (l.krate == "saphyr" && e.krate == "saphyr-parser"));
        println!(
            "| {} | {} | {} | {} | {} |",
            l.name,
            version(l.krate),
            engine(l.krate),
            if ev { "✓" } else { "" },
            l.tree
        );
        seen.push(l.krate);
    }
    for e in EVENT_LIBS {
        if !seen.contains(&e.krate) && e.krate != "saphyr-parser" {
            println!(
                "| {} | {} | {} | ✓ | |",
                e.name,
                version(e.krate),
                engine(e.krate)
            );
        }
    }
    println!();
    if o.compliance {
        compliance(&o);
    }
    if o.performance {
        performance(&o);
    }
}

/// The parser a library is built on.
fn engine(krate: &str) -> &'static str {
    match krate {
        "serde_yaml" | "serde_yaml_ng" | "serde_norway" | "yaml_serde" | "unsafe-libyaml" => {
            "libyaml (C, transpiled)"
        }
        "libyaml-safer" => "libyaml, ported to safe Rust",
        "serde-saphyr" | "granit-parser" => "granit-parser (saphyr fork)",
        "saphyr" | "saphyr-parser" => "saphyr-parser (yaml-rust2 fork)",
        "yaml-rust2" => "own (yaml-rust fork)",
        _ => "own",
    }
}
