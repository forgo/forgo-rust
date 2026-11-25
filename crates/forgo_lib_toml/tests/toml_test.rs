//! The official toml-test suite (<https://github.com/toml-lang/toml-test>),
//! at the commit pinned in `scripts/fetch-toml-test.sh`, for TOML 1.1.0
//! and 1.0.0.
//!
//! For each version, the cases its file list names:
//!
//! - **valid**: the document parses, and its values equal the expected
//!   tagged JSON under toml-test's comparison rules (`json.go`): strings
//!   and integers exactly, floats numerically (any NaN equals any NaN),
//!   offset date-times as instants, local ones field by field. The
//!   expected date-times are read by this file's own RFC 3339 reader, not
//!   by the crate.
//! - **invalid**: the document is rejected.
//! - **encoder**: the expected JSON, built as an [`Item`] and encoded,
//!   parses (as TOML 1.0, so the output works with either version) back to
//!   the same values.
//!
//! Without the suite the tests are skipped, unless
//! `TOML_TEST_SUITE_REQUIRED=1` (as in CI), when they fail.

use forgo_lib_toml::{
    Date, Datetime, Document, Item, Offset, ParseOptions, Time, TomlVersion, Value,
};
use forgo_lib_yaml::{Elem, Node, Scalar};
use std::path::{Path, PathBuf};

fn data_dir() -> Option<PathBuf> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/toml-test-data/tests");
    if dir.join("files-toml-1.1.0").is_file() {
        return Some(dir);
    }
    assert!(
        std::env::var("TOML_TEST_SUITE_REQUIRED").as_deref() != Ok("1"),
        "{} is missing; run scripts/fetch-toml-test.sh",
        dir.display()
    );
    eprintln!("skipped: no toml-test data (scripts/fetch-toml-test.sh)");
    None
}

fn cases(dir: &Path, version: &str) -> (Vec<String>, Vec<String>) {
    let list = std::fs::read_to_string(dir.join(format!("files-toml-{version}"))).unwrap();
    let tomls = list.lines().filter(|l| l.ends_with(".toml"));
    let (valid, invalid): (Vec<&str>, Vec<&str>) = tomls.partition(|l| l.starts_with("valid/"));
    (
        valid.into_iter().map(String::from).collect(),
        invalid.into_iter().map(String::from).collect(),
    )
}

fn options(version: &str) -> ParseOptions {
    ParseOptions::default().with_version(if version == "1.0.0" {
        TomlVersion::V1_0
    } else {
        TomlVersion::V1_1
    })
}

// ---------------------------------------------------------------------------
// Comparing with the expected tagged JSON
// ---------------------------------------------------------------------------

fn map_get<'e>(e: &'e Elem, key: &str) -> Option<&'e Elem> {
    e.node()
        .as_map()?
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v)
}

fn is_tagged_value(e: &Elem) -> bool {
    match e.node().as_map() {
        Some(m) => {
            m.len() == 2
                && map_get(e, "type").is_some_and(|t| t.node().as_str().is_some())
                && map_get(e, "value").is_some()
        }
        None => false,
    }
}

/// Days since 0000-03-01 for a proleptic Gregorian date (for instants).
fn days(y: i64, m: i64, d: i64) -> i64 {
    let (y, m) = if m <= 2 { (y - 1, m + 9) } else { (y, m - 3) };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * m + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe
}

/// An offset date-time as (seconds since an epoch, nanoseconds).
fn instant(dt: &Datetime) -> (i64, u32) {
    let d = dt.date.expect("date");
    let t = dt.time.expect("time");
    let offset = match dt.offset.expect("offset") {
        Offset::Z => 0,
        Offset::Minutes(m) => i64::from(m),
    };
    let secs = days(i64::from(d.year), i64::from(d.month), i64::from(d.day)) * 86_400
        + i64::from(t.hour) * 3_600
        + i64::from(t.minute) * 60
        + i64::from(t.second)
        - offset * 60;
    (secs, t.nanosecond)
}

/// The expected JSON's date-times, read without the crate's own parser:
/// the canonical RFC 3339 forms toml-test writes (`1979-05-27`,
/// `07:32:00.5`, `1979-05-27T07:32:00`, `1979-05-27T07:32:00-07:00`).
fn parse_datetime(s: &str) -> Option<Datetime> {
    fn num<T: std::str::FromStr>(s: &str, at: std::ops::Range<usize>) -> Option<T> {
        let digits = s.get(at)?;
        digits
            .bytes()
            .all(|c| c.is_ascii_digit())
            .then(|| digits.parse().ok())?
    }
    let b = s.as_bytes();
    let mut rest = s;
    let mut date = None;
    if b.get(4) == Some(&b'-') {
        date = Some(Date {
            year: num(s, 0..4)?,
            month: num(s, 5..7)?,
            day: num(s, 8..10)?,
        });
        rest = match s.get(10..)? {
            "" => {
                return Some(Datetime {
                    date,
                    time: None,
                    offset: None,
                });
            }
            r => r.strip_prefix('T')?,
        };
    }
    let (hour, minute, second) = (num(rest, 0..2)?, num(rest, 3..5)?, num(rest, 6..8)?);
    if rest.get(2..3)? != ":" || rest.get(5..6)? != ":" {
        return None;
    }
    rest = &rest[8..];
    let mut nanosecond = 0u32;
    if let Some(frac) = rest.strip_prefix('.') {
        let n = frac.bytes().take_while(u8::is_ascii_digit).count();
        // Nanoseconds: pad, or drop digits beyond nine.
        for c in frac[..n].bytes().chain(std::iter::repeat(b'0')).take(9) {
            nanosecond = nanosecond * 10 + u32::from(c - b'0');
        }
        rest = &frac[n..];
    }
    let offset = match rest {
        "" => None,
        "Z" => Some(Offset::Z),
        _ => {
            let sign: i16 = match rest.as_bytes()[0] {
                b'+' => 1,
                b'-' => -1,
                _ => return None,
            };
            let (h, m): (i16, i16) = (num(rest, 1..3)?, num(rest, 4..6)?);
            (rest.len() == 6 && &rest[3..4] == ":")
                .then_some(Some(Offset::Minutes(sign * (h * 60 + m))))?
        }
    };
    Some(Datetime {
        date,
        time: Some(Time {
            hour,
            minute,
            second,
            nanosecond,
        }),
        offset,
    })
}

/// Compare our value with the expected one; `Err` says where they differ.
fn compare(path: &str, ours: Value<'_>, expected: &Elem) -> Result<(), String> {
    if is_tagged_value(expected) {
        let ty = map_get(expected, "type")
            .and_then(|t| t.node().as_str())
            .unwrap();
        if ty == "array" {
            // A tagged empty array (older toml-test form).
            return compare(path, ours, map_get(expected, "value").unwrap());
        }
        let want = map_get(expected, "value")
            .and_then(|v| v.node().as_str())
            .ok_or("bad expected value")?;
        let fail = || Err(format!("{path}: expected {ty} {want:?}, found {ours:?}"));
        return match (ty, ours) {
            ("string", Value::String(s)) if s == want => Ok(()),
            ("integer", Value::Integer(i)) if want.parse::<i64>() == Ok(i) => Ok(()),
            ("float", Value::Float(f)) => {
                let w = want.to_ascii_lowercase();
                if w.ends_with("nan") {
                    if f.is_nan() { Ok(()) } else { fail() }
                } else {
                    match w.parse::<f64>() {
                        Ok(x) if x == f => Ok(()),
                        _ => fail(),
                    }
                }
            }
            ("bool", Value::Boolean(b))
                if want.eq_ignore_ascii_case(if b { "true" } else { "false" }) =>
            {
                Ok(())
            }
            ("datetime" | "datetime-local" | "date-local" | "time-local", Value::Datetime(d)) => {
                let Some(w) = parse_datetime(want) else {
                    return Err(format!("{path}: can't read the expected datetime {want:?}"));
                };
                if d.kind_name() != ty || w.kind_name() != ty {
                    return fail();
                }
                let same = if ty == "datetime" {
                    instant(&d) == instant(&w)
                } else {
                    d.date == w.date && d.time == w.time
                };
                if same { Ok(()) } else { fail() }
            }
            _ => fail(),
        };
    }
    match (expected.node(), ours) {
        (Node::Map(entries), Value::Table(t)) => {
            if entries.len() != t.len() {
                let mut ours: Vec<&str> = t.keys().collect();
                ours.sort();
                return Err(format!(
                    "{path}: {} keys expected, found {} ({ours:?})",
                    entries.len(),
                    t.len()
                ));
            }
            for (k, v) in entries {
                let k = k.as_str().unwrap_or_default();
                let sub = format!("{path}.{k}");
                let Some(ours) = t.get(k) else {
                    return Err(format!("{sub}: missing"));
                };
                compare(&sub, ours, v)?;
            }
            Ok(())
        }
        (Node::Seq(items), Value::Array(a)) => {
            if items.len() != a.len() {
                return Err(format!(
                    "{path}: {} items expected, found {}",
                    items.len(),
                    a.len()
                ));
            }
            for (i, (e, v)) in items.iter().zip(a.iter()).enumerate() {
                compare(&format!("{path}[{i}]"), v, e)?;
            }
            Ok(())
        }
        (Node::Scalar(Scalar::Null), _) => Err(format!("{path}: null in the expected JSON")),
        _ => Err(format!(
            "{path}: expected {:?}, found {ours:?}",
            expected.node()
        )),
    }
}

// ---------------------------------------------------------------------------
// The checks
// ---------------------------------------------------------------------------

fn run_valid(dir: &Path, version: &str) -> (usize, Vec<String>) {
    let (valid, _) = cases(dir, version);
    let opts = options(version);
    let mut failures = Vec::new();
    for case in &valid {
        let bytes = std::fs::read(dir.join(case)).unwrap();
        let json_path = dir.join(case.replace(".toml", ".json"));
        let json = std::fs::read_to_string(&json_path).unwrap();
        let expected =
            forgo_lib_yaml::Doc::from_str(&json).unwrap_or_else(|e| panic!("{case}: {e}"));
        match Document::from_slice(&bytes, &opts) {
            Err(e) => failures.push(format!("{case}: rejected: {e}")),
            Ok(doc) => {
                if let Err(why) = compare("", Value::Table(doc.root()), expected.root()) {
                    failures.push(format!("{case}: {why}"));
                }
            }
        }
    }
    (valid.len(), failures)
}

fn run_invalid(dir: &Path, version: &str) -> (usize, Vec<String>) {
    let (_, invalid) = cases(dir, version);
    let opts = options(version);
    let mut failures = Vec::new();
    for case in &invalid {
        let bytes = std::fs::read(dir.join(case)).unwrap();
        if let Ok(doc) = Document::from_slice(&bytes, &opts) {
            failures.push(format!("{case}: accepted: {doc:?}"));
        }
    }
    (invalid.len(), failures)
}

/// Tagged JSON to an [`Item`] (toml-test's encoder input).
fn to_item(e: &Elem) -> Result<Item, String> {
    if is_tagged_value(e) {
        let ty = map_get(e, "type").and_then(|t| t.node().as_str()).unwrap();
        if ty == "array" {
            return to_item(map_get(e, "value").unwrap());
        }
        let v = map_get(e, "value")
            .and_then(|v| v.node().as_str())
            .ok_or("bad value")?;
        return Ok(match ty {
            "string" => Item::String(v.to_string()),
            "integer" => Item::Integer(v.parse().map_err(|_| format!("integer {v:?}"))?),
            "float" => {
                let w = v.to_ascii_lowercase();
                Item::Float(match w.as_str() {
                    "nan" | "+nan" => f64::NAN,
                    "-nan" => -f64::NAN,
                    _ => w.parse().map_err(|_| format!("float {v:?}"))?,
                })
            }
            "bool" => Item::Boolean(v == "true"),
            _ => Item::Datetime(parse_datetime(v).ok_or_else(|| format!("datetime {v:?}"))?),
        });
    }
    match e.node() {
        Node::Map(entries) => Ok(Item::Table(
            entries
                .iter()
                .map(|(k, v)| Ok((k.as_str().unwrap_or_default().to_string(), to_item(v)?)))
                .collect::<Result<_, String>>()?,
        )),
        Node::Seq(items) => Ok(Item::Array(
            items.iter().map(to_item).collect::<Result<_, _>>()?,
        )),
        _ => Err("unexpected JSON".into()),
    }
}

fn run_encoder(dir: &Path, version: &str) -> (usize, Vec<String>) {
    let (valid, _) = cases(dir, version);
    let mut failures = Vec::new();
    for case in &valid {
        let json = std::fs::read_to_string(dir.join(case.replace(".toml", ".json"))).unwrap();
        let expected = forgo_lib_yaml::Doc::from_str(&json).unwrap();
        let item = match to_item(expected.root()) {
            Ok(i) => i,
            Err(e) => {
                failures.push(format!("{case}: {e}"));
                continue;
            }
        };
        let text = match item.to_toml_string() {
            Ok(t) => t,
            Err(e) => {
                failures.push(format!("{case}: can't encode: {e}"));
                continue;
            }
        };
        match Document::parse_with(&text, &options("1.0.0")) {
            Err(e) => failures.push(format!(
                "{case}: the encoded text doesn't parse: {e}
{text}"
            )),
            Ok(doc) => {
                if let Err(why) = compare("", Value::Table(doc.root()), expected.root()) {
                    failures.push(format!(
                        "{case}: {why}
{text}"
                    ));
                }
            }
        }
    }
    (valid.len(), failures)
}

fn report(what: &str, (n, failures): (usize, Vec<String>)) {
    println!("{what}: {}/{n} pass", n - failures.len());
    assert!(
        failures.is_empty(),
        "{what}: {} of {n} fail:\n{}",
        failures.len(),
        failures
            .iter()
            .take(40)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn toml_1_1_valid() {
    let Some(dir) = data_dir() else { return };
    report("TOML 1.1.0 valid", run_valid(&dir, "1.1.0"));
}

#[test]
fn toml_1_1_invalid() {
    let Some(dir) = data_dir() else { return };
    report("TOML 1.1.0 invalid", run_invalid(&dir, "1.1.0"));
}

#[test]
fn toml_1_0_valid() {
    let Some(dir) = data_dir() else { return };
    report("TOML 1.0.0 valid", run_valid(&dir, "1.0.0"));
}

#[test]
fn toml_1_0_invalid() {
    let Some(dir) = data_dir() else { return };
    report("TOML 1.0.0 invalid", run_invalid(&dir, "1.0.0"));
}

#[test]
fn toml_1_1_encoder() {
    let Some(dir) = data_dir() else { return };
    report("TOML 1.1.0 encoder", run_encoder(&dir, "1.1.0"));
}

#[test]
fn toml_1_0_encoder() {
    let Some(dir) = data_dir() else { return };
    report("TOML 1.0.0 encoder", run_encoder(&dir, "1.0.0"));
}
