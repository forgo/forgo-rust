//! Official yaml-test-suite harness.
//!
//! Data: the release pinned by `scripts/fetch-yaml-test-suite.sh`, read from
//! `tests/yaml-test-suite-data/`. Every case is checked for:
//!
//! - `error`:  accept/reject agrees with the presence of an `error` file;
//! - `events`: the event stream equals `test.event` (valid cases);
//! - `json`:   the composed documents equal `in.json` (valid cases that have one);
//! - `coverage`: every input byte outside tokens and comments is white space
//!   (valid cases), so the scanner can't silently drop content;
//! - `source`: writing the parsed, untouched documents gives back the input
//!   byte for byte (valid cases);
//! - `roundtrip`: parse → reformat → emit → parse keeps the content: the
//!   normalized event streams (types, values, tags, anchors; no styles) are
//!   equal;
//! - `presentation`: parse → reformat → emit → parse gives an equal `Doc`
//!   (comments, styles and spellings included; source positions ignored).
//!
//! `roundtrip` and `presentation` test the emitter: `Doc::reformat` drops
//! the source text, so everything is rendered.
//!
//! Checks listed in [`GATED`] fail this test when a failure is missing from
//! `yaml_test_suite/known_failures.txt`, or when a listed failure passes, so
//! the list can only shrink. Other checks are report-only. Each case runs in
//! its own thread under `catch_unwind` with a timeout. The full report is
//! written to `target/tmp/yaml-test-suite-report.md`.
//!
//! Missing data is a hard failure when `CI` or `YAML_TEST_SUITE_REQUIRED` is
//! set, and a loud skip otherwise.

#[path = "yaml_test_suite/doc_json.rs"]
mod doc_json;
#[path = "yaml_test_suite/json.rs"]
mod json;
#[path = "yaml_test_suite/normalize.rs"]
mod normalize;

use json::Json;
use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

const DATA_TAG: &str = "data-2022-01-17";
const TIMEOUT: Duration = Duration::from_secs(10);
const WORKER: &str = "yaml-suite-case";

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum Check {
    Error,
    Events,
    Json,
    Coverage,
    Source,
    RoundTrip,
    Presentation,
}

impl Check {
    const ALL: [Check; 7] = [
        Check::Error,
        Check::Events,
        Check::Json,
        Check::Coverage,
        Check::Source,
        Check::RoundTrip,
        Check::Presentation,
    ];

    fn name(self) -> &'static str {
        match self {
            Check::Error => "error",
            Check::Events => "events",
            Check::Json => "json",
            Check::Coverage => "coverage",
            Check::Source => "source",
            Check::RoundTrip => "roundtrip",
            Check::Presentation => "presentation",
        }
    }

    fn parse(s: &str) -> Option<Check> {
        Check::ALL.into_iter().find(|c| c.name() == s)
    }
}

/// Checks that gate this test.
const GATED: &[Check] = &[
    Check::Error,
    Check::Events,
    Check::Json,
    Check::Coverage,
    Check::Source,
    Check::RoundTrip,
    Check::Presentation,
];

/// What is under test, for the report header.
const SUBJECT: &str = "`forgo_lib_yaml::events` for error/events/coverage; `Doc::from_stream` for json/source/roundtrip/presentation";

// ---------------------------------------------------------------------------
// Cases
// ---------------------------------------------------------------------------

struct Case {
    id: String,
    name: String,
    yaml: Vec<u8>,
    events: Option<String>,
    json: Option<String>,
    error: bool,
}

fn data_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/yaml-test-suite-data")
}

fn sorted_subdirs(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("read {}: {e}", dir.display()))
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|n| !n.starts_with('.'))
        .collect();
    names.sort();
    names
}

fn read_opt(path: PathBuf) -> Option<String> {
    fs::read_to_string(path).ok()
}

fn load_case(id: String, dir: &Path) -> Case {
    Case {
        name: read_opt(dir.join("==="))
            .unwrap_or_default()
            .trim()
            .to_string(),
        yaml: fs::read(dir.join("in.yaml")).expect("in.yaml"),
        events: read_opt(dir.join("test.event")),
        json: read_opt(dir.join("in.json")),
        error: dir.join("error").exists(),
        id,
    }
}

/// Every `<ID>/` with an `in.yaml`, or its numbered subdirectories.
/// `name/` and `tags/` are symlink indexes, not cases.
fn discover(root: &Path) -> Vec<Case> {
    let mut cases = Vec::new();
    for id in sorted_subdirs(root) {
        if id == "name" || id == "tags" {
            continue;
        }
        let dir = root.join(&id);
        if dir.join("in.yaml").exists() {
            cases.push(load_case(id, &dir));
            continue;
        }
        for sub in sorted_subdirs(&dir) {
            let sdir = dir.join(&sub);
            if sdir.join("in.yaml").exists() {
                cases.push(load_case(format!("{id}/{sub}"), &sdir));
            }
        }
    }
    cases
}

// ---------------------------------------------------------------------------
// Subject under test
// ---------------------------------------------------------------------------

/// Partial events produced before an error, plus the error message.
type EventsError = (Vec<String>, String);

/// What the implementation produced for one input. `None` means the
/// capability isn't implemented yet.
struct Run {
    accepted: Result<(), String>,
    events: Option<Result<Vec<String>, EventsError>>,
    json: Option<Result<Vec<Json>, String>>,
    coverage: Option<Result<(), String>>,
    source: Option<Result<(), String>>,
    roundtrip: Option<Result<(), String>>,
    presentation: Option<Result<(), String>>,
}

fn run_subject(yaml: Vec<u8>) -> Run {
    use forgo_lib_yaml::syntax::{input, test_event::to_test_event, uncovered_content};

    let text = match input::decode(&yaml) {
        Ok(t) => t.into_owned(),
        Err(e) => {
            let msg = e.to_string();
            return Run {
                accepted: Err(msg.clone()),
                events: Some(Err((Vec::new(), msg.clone()))),
                json: Some(Err(msg)),
                coverage: None,
                source: None,
                roundtrip: None,
                presentation: None,
            };
        }
    };

    let mut lines = Vec::new();
    let mut error = None;
    for r in forgo_lib_yaml::events(&text) {
        match r {
            Ok((ev, _)) => lines.push(to_test_event(&ev)),
            Err(e) => {
                error = Some(e.to_string());
                break;
            }
        }
    }
    let (accepted, events, coverage) = match error {
        None => {
            let coverage = match uncovered_content(&text) {
                Ok(gaps) if gaps.is_empty() => Ok(()),
                Ok(gaps) => Err(format!(
                    "uncovered content: {}",
                    gaps.iter()
                        .take(3)
                        .map(|g| format!("{:?} {:?}", g, &text[g.clone()]))
                        .collect::<Vec<_>>()
                        .join(", ")
                )),
                Err(e) => Err(format!("scanner failed: {e}")),
            };
            (Ok(()), Ok(lines), Some(coverage))
        }
        Some(e) => (Err(e.clone()), Err((lines, e)), None),
    };

    let docs = forgo_lib_yaml::Doc::from_stream(&text);
    let source = match (&docs, &accepted) {
        (Ok(docs), Ok(())) => Some(check_source(&text, docs)),
        _ => None,
    };
    let reformatted = docs.as_ref().ok().map(|docs| {
        let mut docs = docs.clone();
        for d in &mut docs {
            d.reformat();
        }
        docs
    });
    let roundtrip = match (&reformatted, &accepted) {
        (Some(docs), Ok(())) => Some(check_roundtrip(&text, docs)),
        _ => None,
    };
    let presentation = match (&reformatted, &accepted) {
        (Some(docs), Ok(())) => Some(check_presentation(docs)),
        _ => None,
    };
    let json = match docs {
        // A stream without documents gives a placeholder for its comments.
        Ok(mut docs) => {
            docs.retain(|d| !d.is_placeholder());
            doc_json::docs_to_json(&docs)
        }
        Err(e) => Err(e.to_string()),
    };

    Run {
        accepted,
        events: Some(events),
        json: Some(json),
        coverage,
        source,
        roundtrip,
        presentation,
    }
}

/// `Debug` output without source spans, which differ between parses.
fn debug_without_spans(docs: &[forgo_lib_yaml::Doc]) -> Vec<String> {
    let text = format!("{docs:#?}");
    let mut out = Vec::new();
    let mut depth = 0usize;
    for line in text.lines() {
        if depth > 0 {
            depth += line.matches('(').count();
            depth = depth.saturating_sub(line.matches(')').count());
            continue;
        }
        if line.trim_start().starts_with("span: SourceSpan(") {
            depth = line
                .matches('(')
                .count()
                .saturating_sub(line.matches(')').count());
            continue;
        }
        out.push(line.to_string());
    }
    out
}

/// Emit `docs` and parse again: the documents must be equal, including
/// comments and styles.
fn check_presentation(docs: &[forgo_lib_yaml::Doc]) -> Result<(), String> {
    let emitted =
        forgo_lib_yaml::Doc::to_stream_string(docs).map_err(|e| format!("emit failed: {e}"))?;
    let again = forgo_lib_yaml::Doc::from_stream(&emitted)
        .map_err(|e| format!("emitted output doesn't parse ({e}): {emitted:?}"))?;
    // A placeholder without comments (no documents, no comments) carries
    // nothing, and writes nothing.
    let bare = |d: &forgo_lib_yaml::Doc| d.is_placeholder() && d.footer_comments().is_empty();
    let docs: Vec<_> = docs.iter().filter(|d| !bare(d)).cloned().collect();
    let again: Vec<_> = again.into_iter().filter(|d| !bare(d)).collect();
    if again == docs {
        return Ok(());
    }
    let docs = &docs[..];
    let a = debug_without_spans(docs);
    let b = debug_without_spans(&again);
    let refs: Vec<&str> = a.iter().map(String::as_str).collect();
    Err(format!(
        "{}; emitted {emitted:?}",
        first_difference(&refs, &b)
    ))
}

/// Untouched documents write back as their source text.
fn check_source(text: &str, docs: &[forgo_lib_yaml::Doc]) -> Result<(), String> {
    let written =
        forgo_lib_yaml::Doc::to_stream_string(docs).map_err(|e| format!("emit failed: {e}"))?;
    if written == text {
        return Ok(());
    }
    let a: Vec<&str> = text.split_inclusive('\n').collect();
    let b: Vec<String> = written.split_inclusive('\n').map(String::from).collect();
    Err(format!("{}; wrote {written:?}", first_difference(&a, &b)))
}

/// Emit `docs` and parse the output again; the content must be unchanged.
fn check_roundtrip(text: &str, docs: &[forgo_lib_yaml::Doc]) -> Result<(), String> {
    let emitted =
        forgo_lib_yaml::Doc::to_stream_string(docs).map_err(|e| format!("emit failed: {e}"))?;
    let original = normalize::normalized_events(text).map_err(|e| format!("input: {e}"))?;
    let again = normalize::normalized_events(&emitted)
        .map_err(|e| format!("emitted output doesn't parse ({e}): {emitted:?}"))?;
    if original == again {
        return Ok(());
    }
    let refs: Vec<&str> = original.iter().map(String::as_str).collect();
    Err(format!(
        "{}; emitted {emitted:?}",
        first_difference(&refs, &again)
    ))
}

enum Outcome<T> {
    Done(T),
    Panicked(String),
    TimedOut,
}

fn isolated<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> Outcome<T> {
    let (tx, rx) = mpsc::channel();
    std::thread::Builder::new()
        .name(WORKER.into())
        .stack_size(64 << 20)
        .spawn(move || {
            let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
            let _ = tx.send(r);
        })
        .expect("spawn worker thread");
    match rx.recv_timeout(TIMEOUT) {
        Ok(Ok(v)) => Outcome::Done(v),
        Ok(Err(payload)) => Outcome::Panicked(
            payload
                .downcast_ref::<&str>()
                .map(|s| s.to_string())
                .or_else(|| payload.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "non-string panic payload".into()),
        ),
        Err(_) => Outcome::TimedOut,
    }
}

// ---------------------------------------------------------------------------
// Evaluation
// ---------------------------------------------------------------------------

#[derive(Default)]
struct Tally {
    pass: Vec<String>,
    /// (case id, detail)
    fail: Vec<(String, String)>,
    /// Applicable, but the capability isn't implemented yet.
    not_run: usize,
}

fn first_difference(expected: &[&str], got: &[String]) -> String {
    let n = expected.len().max(got.len());
    for i in 0..n {
        let e = expected.get(i).copied();
        let g = got.get(i).map(String::as_str);
        if e != g {
            return format!(
                "line {}: expected `{}`, got `{}`",
                i + 1,
                e.unwrap_or("<end>"),
                g.unwrap_or("<end>")
            );
        }
    }
    "identical".into()
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let t: String = s.chars().take(max).collect();
        format!("{t}…")
    }
}

struct Results {
    tallies: [Tally; 7],
    /// Error cases where `test.event` is a prefix of our events before the error.
    error_pos_agree: usize,
    error_pos_checked: usize,
}

fn evaluate(cases: &[Case]) -> Results {
    let mut r = Results {
        tallies: Default::default(),
        error_pos_agree: 0,
        error_pos_checked: 0,
    };
    for case in cases {
        let outcome = isolated({
            let yaml = case.yaml.clone();
            move || run_subject(yaml)
        });
        let run = match outcome {
            Outcome::Done(run) => run,
            Outcome::Panicked(msg) => {
                fail_all(&mut r, case, &format!("panic: {msg}"));
                continue;
            }
            Outcome::TimedOut => {
                fail_all(&mut r, case, &format!("timed out after {TIMEOUT:?}"));
                continue;
            }
        };

        // error
        let t = &mut r.tallies[Check::Error as usize];
        match (&run.accepted, case.error) {
            (Err(_), true) | (Ok(()), false) => t.pass.push(case.id.clone()),
            (Ok(()), true) => t
                .fail
                .push((case.id.clone(), "accepted invalid input".into())),
            (Err(e), false) => t.fail.push((case.id.clone(), format!("rejected: {e}"))),
        }

        // events
        let expected_events: Option<Vec<&str>> =
            case.events.as_deref().map(|s| s.lines().collect());
        if case.error {
            if let (Some(exp), Some(Err((partial, _)))) = (&expected_events, &run.events) {
                r.error_pos_checked += 1;
                if exp.len() <= partial.len() && exp.iter().zip(partial).all(|(e, g)| *e == g) {
                    r.error_pos_agree += 1;
                }
            }
        } else if let Some(exp) = &expected_events {
            let t = &mut r.tallies[Check::Events as usize];
            match &run.events {
                None => t.not_run += 1,
                Some(Ok(got))
                    if exp.len() == got.len() && exp.iter().zip(got).all(|(e, g)| *e == g) =>
                {
                    t.pass.push(case.id.clone())
                }
                Some(Ok(got)) => t.fail.push((case.id.clone(), first_difference(exp, got))),
                Some(Err((_, e))) => t.fail.push((case.id.clone(), format!("error: {e}"))),
            }
        }

        // coverage
        if !case.error {
            let t = &mut r.tallies[Check::Coverage as usize];
            match &run.coverage {
                None => t.not_run += 1,
                Some(Ok(())) => t.pass.push(case.id.clone()),
                Some(Err(e)) => t.fail.push((case.id.clone(), e.clone())),
            }
        }

        // source
        if !case.error {
            let t = &mut r.tallies[Check::Source as usize];
            match &run.source {
                None => t.not_run += 1,
                Some(Ok(())) => t.pass.push(case.id.clone()),
                Some(Err(e)) => t.fail.push((case.id.clone(), e.clone())),
            }
        }

        // roundtrip
        if !case.error {
            let t = &mut r.tallies[Check::RoundTrip as usize];
            match &run.roundtrip {
                None => t.not_run += 1,
                Some(Ok(())) => t.pass.push(case.id.clone()),
                Some(Err(e)) => t.fail.push((case.id.clone(), e.clone())),
            }
        }

        // presentation
        if !case.error {
            let t = &mut r.tallies[Check::Presentation as usize];
            match &run.presentation {
                None => t.not_run += 1,
                Some(Ok(())) => t.pass.push(case.id.clone()),
                Some(Err(e)) => t.fail.push((case.id.clone(), e.clone())),
            }
        }

        // json
        if let (false, Some(text)) = (case.error, &case.json) {
            let expected = json::parse_many(text)
                .unwrap_or_else(|e| panic!("{}: unreadable in.json: {e}", case.id));
            let t = &mut r.tallies[Check::Json as usize];
            match &run.json {
                None => t.not_run += 1,
                Some(Ok(got)) if *got == expected => t.pass.push(case.id.clone()),
                Some(Ok(got)) => {
                    let show = |v: &[Json]| {
                        v.iter()
                            .map(|j| j.to_string())
                            .collect::<Vec<_>>()
                            .join(" ")
                    };
                    t.fail.push((
                        case.id.clone(),
                        format!(
                            "expected {}, got {}",
                            truncate(&show(&expected), 120),
                            truncate(&show(got), 120)
                        ),
                    ))
                }
                Some(Err(e)) => t.fail.push((case.id.clone(), e.clone())),
            }
        }
    }
    r
}

/// A panic or timeout fails every check that applies to the case.
fn fail_all(r: &mut Results, case: &Case, detail: &str) {
    r.tallies[Check::Error as usize]
        .fail
        .push((case.id.clone(), detail.to_string()));
    if !case.error && case.events.is_some() {
        r.tallies[Check::Events as usize]
            .fail
            .push((case.id.clone(), detail.to_string()));
    }
    if !case.error && case.json.is_some() {
        r.tallies[Check::Json as usize]
            .fail
            .push((case.id.clone(), detail.to_string()));
    }
    if !case.error {
        r.tallies[Check::Coverage as usize]
            .fail
            .push((case.id.clone(), detail.to_string()));
        r.tallies[Check::Source as usize]
            .fail
            .push((case.id.clone(), detail.to_string()));
        r.tallies[Check::RoundTrip as usize]
            .fail
            .push((case.id.clone(), detail.to_string()));
        r.tallies[Check::Presentation as usize]
            .fail
            .push((case.id.clone(), detail.to_string()));
    }
}

// ---------------------------------------------------------------------------
// Gate and report
// ---------------------------------------------------------------------------

fn known_failures() -> BTreeSet<(String, Check)> {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/yaml_test_suite/known_failures.txt");
    let text = fs::read_to_string(&path).unwrap_or_default();
    let mut set = BTreeSet::new();
    for (n, line) in text.lines().enumerate() {
        let line = line.split('#').next().unwrap().trim();
        if line.is_empty() {
            continue;
        }
        let (id, check) = line
            .rsplit_once(':')
            .unwrap_or_else(|| panic!("known_failures.txt:{}: expected `ID:check`", n + 1));
        let check = Check::parse(check)
            .unwrap_or_else(|| panic!("known_failures.txt:{}: unknown check `{check}`", n + 1));
        set.insert((id.to_string(), check));
    }
    set
}

fn gate(results: &Results, cases: &[Case]) -> Vec<String> {
    let known = known_failures();
    let ids: BTreeSet<&str> = cases.iter().map(|c| c.id.as_str()).collect();
    let mut problems = Vec::new();
    for (id, check) in &known {
        if !ids.contains(id.as_str()) {
            problems.push(format!(
                "known_failures.txt lists unknown case `{id}:{}`",
                check.name()
            ));
        }
    }
    for &check in GATED {
        let t = &results.tallies[check as usize];
        for (id, detail) in &t.fail {
            if !known.contains(&(id.clone(), check)) {
                problems.push(format!(
                    "new failure {id}:{} — {}",
                    check.name(),
                    truncate(detail, 160)
                ));
            }
        }
        for id in &t.pass {
            if known.contains(&(id.clone(), check)) {
                problems.push(format!(
                    "{id}:{} now passes — remove it from known_failures.txt",
                    check.name()
                ));
            }
        }
    }
    problems
}

fn md_escape(s: &str) -> String {
    s.replace('|', "\\|").replace('\n', "↵").replace('\t', "→")
}

fn report(results: &Results, cases: &[Case]) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "# yaml-test-suite report\n");
    let _ = writeln!(out, "- Data: `{DATA_TAG}`, {} cases", cases.len());
    let _ = writeln!(out, "- Subject: {SUBJECT}");
    let gated: Vec<&str> = GATED.iter().map(|c| c.name()).collect();
    let _ = writeln!(
        out,
        "- Gated checks: {}\n",
        if gated.is_empty() {
            "none (report-only)".to_string()
        } else {
            gated.join(", ")
        }
    );
    let _ = writeln!(out, "| check | pass | fail | not run |\n|---|---|---|---|");
    for check in Check::ALL {
        let t = &results.tallies[check as usize];
        let _ = writeln!(
            out,
            "| {} | {} | {} | {} |",
            check.name(),
            t.pass.len(),
            t.fail.len(),
            t.not_run
        );
    }
    if results.error_pos_checked > 0 {
        let _ = writeln!(
            out,
            "\nError position (informational): {}/{} error cases emit the expected events before failing.",
            results.error_pos_agree, results.error_pos_checked
        );
    }
    let names: std::collections::HashMap<&str, &str> = cases
        .iter()
        .map(|c| (c.id.as_str(), c.name.as_str()))
        .collect();
    for check in Check::ALL {
        let t = &results.tallies[check as usize];
        if t.fail.is_empty() {
            continue;
        }
        let _ = writeln!(out, "\n## {} failures ({})\n", check.name(), t.fail.len());
        let _ = writeln!(out, "| case | name | detail |\n|---|---|---|");
        for (id, detail) in &t.fail {
            let _ = writeln!(
                out,
                "| {id} | {} | {} |",
                md_escape(names.get(id.as_str()).copied().unwrap_or("")),
                md_escape(&truncate(detail, 200))
            );
        }
    }
    out
}

#[test]
fn yaml_test_suite() {
    let root = data_dir();
    if !root.join(".pinned-commit").exists() {
        let msg = format!(
            "yaml-test-suite data missing at {}. Run scripts/fetch-yaml-test-suite.sh.",
            root.display()
        );
        if std::env::var_os("CI").is_some()
            || std::env::var_os("YAML_TEST_SUITE_REQUIRED").is_some()
        {
            panic!("{msg}");
        }
        eprintln!("\n!!!!! SKIPPING yaml-test-suite: {msg}\n");
        return;
    }

    // Keep panics inside case workers quiet; the report records them.
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if std::thread::current().name() != Some(WORKER) {
            default_hook(info);
        }
    }));

    let cases = discover(&root);
    assert_eq!(cases.len(), 402, "unexpected case count for {DATA_TAG}");

    let results = evaluate(&cases);
    let text = report(&results, &cases);
    let report_path = Path::new(env!("CARGO_TARGET_TMPDIR")).join("yaml-test-suite-report.md");
    fs::write(&report_path, &text).expect("write report");

    println!(
        "\nyaml-test-suite ({DATA_TAG}, {} cases) — {SUBJECT}",
        cases.len()
    );
    for check in Check::ALL {
        let t = &results.tallies[check as usize];
        let applicable = t.pass.len() + t.fail.len() + t.not_run;
        let mode = if GATED.contains(&check) {
            "gated"
        } else {
            "report-only"
        };
        if t.not_run == applicable {
            println!(
                "  {:<7} not implemented yet ({applicable} applicable)",
                check.name()
            );
        } else {
            println!(
                "  {:<7} {:>3}/{:<3} pass  ({mode})",
                check.name(),
                t.pass.len(),
                t.pass.len() + t.fail.len()
            );
        }
    }
    println!("  report: {}", report_path.display());

    let problems = gate(&results, &cases);
    if !problems.is_empty() {
        panic!("yaml-test-suite gate failed:\n  {}", problems.join("\n  "));
    }
}
