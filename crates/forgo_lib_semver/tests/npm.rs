//! The `npm` module against node-semver itself.
//!
//! `scripts/fetch-node-semver.sh` downloads node-semver at a pinned commit
//! and records, with Node, what it does on the inputs of its own test
//! suite (`scripts/capture-node-semver.js`): about 5,900 calls to its
//! functions, and a cross product of every range and version they use
//! under all four option combinations (`validRange`, `minVersion`, and
//! `satisfies` for each version). This replays all of it.
//!
//! The one deliberate difference: node-semver rejects numbers above
//! 2^53 - 1 and versions over 256 characters, and this crate doesn't.
//! Calls whose inputs exceed those limits are counted, not compared.
//!
//! Without the data the test is skipped, unless `SEMVER_NODE_REQUIRED=1`
//! (as in CI), when it fails.

use forgo_lib_semver::Version;
use forgo_lib_semver::npm::{self, CoerceOptions, IdentifierBase, NpmOptions};
use forgo_lib_yaml::{Doc, Elem, Node, Scalar};
use std::cmp::Ordering;
use std::path::Path;

/// A JSON value.
#[derive(Clone, Debug, PartialEq)]
enum J {
    Null,
    Bool(bool),
    Num(String),
    Str(String),
    Arr(Vec<J>),
    Obj(Vec<(String, J)>),
}

impl J {
    fn of(e: &Elem) -> J {
        match e.node() {
            Node::Scalar(Scalar::Null) => J::Null,
            Node::Scalar(Scalar::Bool(b)) => J::Bool(*b),
            Node::Scalar(Scalar::Num { text }) => J::Num(text.clone()),
            Node::Scalar(Scalar::Str(s)) => J::Str(s.clone()),
            Node::Seq(items) => J::Arr(items.iter().map(J::of).collect()),
            Node::Map(entries) => J::Obj(
                entries
                    .iter()
                    .map(|(k, v)| (k.as_str().unwrap_or_default().to_string(), J::of(v)))
                    .collect(),
            ),
            Node::Alias(_) => panic!("no aliases in JSON"),
        }
    }

    fn get(&self, key: &str) -> Option<&J> {
        match self {
            J::Obj(entries) => entries.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    fn str(&self) -> Option<&str> {
        match self {
            J::Str(s) => Some(s),
            _ => None,
        }
    }

    /// A version argument as node-semver would read it: a string, or a
    /// `SemVer` object (by its version and build).
    fn version_arg(&self) -> Option<String> {
        match self {
            J::Str(s) => Some(s.clone()),
            J::Obj(_) => {
                let v = self.get("$semver")?.str()?;
                let b = self.get("build").and_then(J::str).unwrap_or("");
                Some(if b.is_empty() {
                    v.to_string()
                } else {
                    format!("{v}+{b}")
                })
            }
            _ => None,
        }
    }

    /// A range argument: a string, or a `Range` or `Comparator` object.
    fn range_arg(&self) -> Option<String> {
        match self {
            J::Str(s) => Some(s.clone()),
            J::Obj(_) => self
                .get("raw")
                .and_then(J::str)
                .or_else(|| self.get("$comparator").and_then(J::str))
                .map(String::from),
            _ => None,
        }
    }

    fn throws(&self) -> bool {
        self.get("$throws").is_some()
    }

    fn is_undefined(&self) -> bool {
        self.get("$").and_then(J::str) == Some("undefined")
    }
}

/// node-semver's options argument (`true` means loose).
fn options(j: Option<&J>) -> Option<NpmOptions> {
    let truthy = |j: Option<&J>| match j {
        None | Some(J::Null) | Some(J::Bool(false)) => false,
        Some(J::Num(n)) => n != "0",
        Some(J::Str(s)) => !s.is_empty(),
        Some(J::Obj(_)) => !j.is_some_and(J::is_undefined),
        Some(_) => true,
    };
    match j {
        None | Some(J::Null) | Some(J::Bool(false)) => Some(NpmOptions::default()),
        Some(j) if j.is_undefined() => Some(NpmOptions::default()),
        Some(J::Bool(true)) => Some(NpmOptions::LOOSE),
        Some(o @ J::Obj(_)) => Some(NpmOptions {
            loose: truthy(o.get("loose")),
            include_prerelease: truthy(o.get("includePrerelease")),
        }),
        Some(_) => None,
    }
}

const MAX_SAFE_INTEGER: &str = "9007199254740991";

/// Whether node-semver's size limits apply: a string over 256 characters,
/// or a number at or above 2^53 - 1.
fn beyond_node_limits(j: &J) -> bool {
    match j {
        J::Str(s) => {
            if s.chars().count() > 256 {
                return true;
            }
            s.split(|c: char| !c.is_ascii_digit()).any(|run| {
                let run = run.trim_start_matches('0');
                run.len() > MAX_SAFE_INTEGER.len()
                    || (run.len() == MAX_SAFE_INTEGER.len() && run >= MAX_SAFE_INTEGER)
            })
        }
        J::Num(n) => beyond_node_limits(&J::Str(n.clone())),
        J::Arr(items) => items.iter().any(beyond_node_limits),
        J::Obj(entries) => entries.iter().any(|(_, v)| beyond_node_limits(v)),
        _ => false,
    }
}

/// Our answer, as node-semver's would be serialized.
fn semver_json(v: &Version) -> J {
    let mut core = format!("{}.{}.{}", v.major, v.minor, v.patch);
    if !v.pre.is_empty() {
        core.push('-');
        core.push_str(v.pre.as_str());
    }
    J::Obj(vec![
        ("$semver".into(), J::Str(core)),
        ("build".into(), J::Str(v.build.as_str().into())),
    ])
}

fn ord_json(o: Ordering) -> J {
    J::Num(
        match o {
            Ordering::Less => "-1",
            Ordering::Equal => "0",
            Ordering::Greater => "1",
        }
        .into(),
    )
}

fn throws<T>(r: Result<T, forgo_lib_semver::Error>, ok: impl FnOnce(T) -> J) -> J {
    match r {
        Ok(v) => ok(v),
        Err(_) => J::Obj(vec![("$throws".into(), J::Str(String::new()))]),
    }
}

fn opt_str(s: Option<String>) -> J {
    s.map_or(J::Null, J::Str)
}

fn same(ours: &J, theirs: &J) -> bool {
    if theirs.throws() {
        return ours.throws();
    }
    match (ours, theirs) {
        (J::Num(a), J::Num(b)) => a.parse::<f64>().ok() == b.parse::<f64>().ok(),
        (J::Arr(a), J::Arr(b)) => a.len() == b.len() && a.iter().zip(b).all(|(x, y)| same(x, y)),
        _ => ours == theirs,
    }
}

/// What this crate returns for a recorded call; `None` if the call can't
/// be expressed in Rust (arguments that aren't strings or options).
fn ours(f: &str, a: &[J]) -> Option<J> {
    let arg = |i: usize| a.get(i);
    let ver = |i: usize| arg(i).and_then(J::version_arg);
    let range = |i: usize| arg(i).and_then(J::range_arg);
    let opts = |i: usize| options(arg(i));
    let cmp_with = |o: NpmOptions| -> Option<J> {
        let (x, y) = (ver(0)?, ver(1)?);
        Some(throws(npm::compare(&x, &y, o), ord_json))
    };
    Some(match f {
        "functions/valid" => opt_str(npm::valid(&ver(0)?, opts(1)?)),
        "functions/clean" => opt_str(npm::clean(&ver(0)?, opts(1)?)),
        "functions/parse" => {
            let v = ver(0)?;
            let throw_errors = matches!(arg(2), Some(J::Bool(true)));
            match npm::parse(&v, opts(1)?) {
                Some(p) => semver_json(&p),
                None if throw_errors => throws::<()>(Err(dummy_error()), |_| J::Null),
                None => J::Null,
            }
        }
        "functions/coerce" => {
            let text = match arg(0)? {
                J::Num(n) => n.clone(),
                other => other.version_arg()?,
            };
            let o = match arg(1) {
                None | Some(J::Null) => CoerceOptions::default(),
                Some(j) if j.is_undefined() => CoerceOptions::default(),
                Some(o @ J::Obj(_)) => CoerceOptions {
                    loose: matches!(o.get("loose"), Some(J::Bool(true))),
                    include_prerelease: matches!(o.get("includePrerelease"), Some(J::Bool(true))),
                    rtl: matches!(o.get("rtl"), Some(J::Bool(true))),
                },
                _ => return None,
            };
            npm::coerce(&text, o).map_or(J::Null, |v| semver_json(&v))
        }
        "functions/inc" => {
            // inc(version, release, options, identifier, identifierBase),
            // where a string `options` is really the identifier.
            let v = ver(0)?;
            let release = arg(1)?.str()?.to_string();
            let (o, id, base) = match arg(2) {
                Some(J::Str(s)) => (NpmOptions::default(), Some(s.clone()), arg(3)),
                _ => (opts(2)?, arg(3).and_then(J::str).map(String::from), arg(4)),
            };
            let base = match base {
                None | Some(J::Null) => IdentifierBase::Zero,
                Some(J::Bool(false)) => IdentifierBase::None,
                Some(J::Str(s)) if s.parse::<f64>().is_ok_and(|n| n != 0.0) => IdentifierBase::One,
                Some(J::Num(n)) if n.parse::<f64>().is_ok_and(|n| n != 0.0) => IdentifierBase::One,
                Some(J::Bool(true)) => IdentifierBase::One,
                Some(j) if j.is_undefined() => IdentifierBase::Zero,
                Some(_) => IdentifierBase::Zero,
            };
            opt_str(npm::inc(&v, &release, o, id.as_deref(), base))
        }
        "functions/diff" => throws(npm::diff(&ver(0)?, &ver(1)?), |d| {
            d.map_or(J::Null, |r| J::Str(r.as_str().into()))
        }),
        "functions/compare" => cmp_with(opts(2)?)?,
        "functions/compare-loose" => cmp_with(NpmOptions::LOOSE)?,
        "functions/rcompare" => {
            let (x, y) = (ver(0)?, ver(1)?);
            throws(npm::compare(&y, &x, opts(2)?), ord_json)
        }
        "functions/compare-build" => {
            let (x, y) = (ver(0)?, ver(1)?);
            throws(npm::compare_build(&x, &y, opts(2)?), ord_json)
        }
        "functions/eq" | "functions/neq" | "functions/gt" | "functions/gte" | "functions/lt"
        | "functions/lte" => {
            let (x, y) = (ver(0)?, ver(1)?);
            let name = f.trim_start_matches("functions/");
            throws(npm::compare(&x, &y, opts(2)?), |o| {
                J::Bool(op_holds(name, o))
            })
        }
        "functions/cmp" => {
            let (x, op, y) = (ver(0)?, arg(1)?.str()?.to_string(), ver(2)?);
            let name = match op.as_str() {
                "===" | "!==" => {
                    // Compares strings as given, and SemVer objects by
                    // their version (without build metadata).
                    let text = |j: &J| match j {
                        J::Obj(_) => j.get("$semver").and_then(J::str).map(String::from),
                        other => other.str().map(String::from),
                    };
                    let eq = text(arg(0)?) == text(arg(2)?);
                    return Some(J::Bool(if op == "===" { eq } else { !eq }));
                }
                "" | "=" | "==" => "eq",
                "!=" => "neq",
                ">" => "gt",
                ">=" => "gte",
                "<" => "lt",
                "<=" => "lte",
                _ => return Some(throws::<()>(Err(dummy_error()), |_| J::Null)),
            };
            throws(npm::compare(&x, &y, opts(3)?), |o| {
                J::Bool(op_holds(name, o))
            })
        }
        "functions/major" | "functions/minor" | "functions/patch" => {
            let v = ver(0)?;
            let o = opts(1)?;
            match npm::parse(&v, o) {
                None => throws::<()>(Err(dummy_error()), |_| J::Null),
                Some(p) => J::Num(
                    match f {
                        "functions/major" => p.major,
                        "functions/minor" => p.minor,
                        _ => p.patch,
                    }
                    .to_string(),
                ),
            }
        }
        "functions/prerelease" => match npm::parse(&ver(0)?, opts(1)?) {
            Some(p) if !p.pre.is_empty() => J::Arr(
                p.pre
                    .identifiers()
                    .map(|i| {
                        if i.is_numeric() {
                            J::Num(i.as_str().into())
                        } else {
                            J::Str(i.as_str().into())
                        }
                    })
                    .collect(),
            ),
            _ => J::Null,
        },
        "functions/sort" | "functions/rsort" => {
            let J::Arr(items) = arg(0)? else { return None };
            let list: Vec<String> = items.iter().map(J::version_arg).collect::<Option<_>>()?;
            let o = opts(1)?;
            let mut sorted = list.clone();
            let mut bad = false;
            sorted.sort_by(|x, y| {
                npm::compare_build(x, y, o).unwrap_or_else(|_| {
                    bad = true;
                    Ordering::Equal
                })
            });
            if f == "functions/rsort" {
                sorted = list.clone();
                sorted.sort_by(|x, y| {
                    npm::compare_build(y, x, o).unwrap_or_else(|_| {
                        bad = true;
                        Ordering::Equal
                    })
                });
            }
            if bad {
                return None;
            }
            J::Arr(sorted.into_iter().map(J::Str).collect())
        }
        "functions/satisfies" => {
            let (Some(v), Some(r)) = (ver(0), range(1)) else {
                return None;
            };
            J::Bool(npm::satisfies(&v, &r, opts(2)?))
        }
        "functions/truncate" => opt_str(npm::truncate(&ver(0)?, arg(1)?.str()?, opts(2)?)),
        "ranges/valid" => opt_str(npm::valid_range(&range(0)?, opts(1)?)),
        "ranges/to-comparators" => throws(npm::to_comparators(&range(0)?, opts(1)?), |sets| {
            J::Arr(
                sets.into_iter()
                    .map(|s| J::Arr(s.into_iter().map(J::Str).collect()))
                    .collect(),
            )
        }),
        "ranges/min-version" => throws(npm::min_version(&range(0)?, opts(1)?), |v| {
            v.map_or(J::Null, |v| semver_json(&v))
        }),
        "ranges/max-satisfying" | "ranges/min-satisfying" => {
            let J::Arr(items) = arg(0)? else { return None };
            let list: Vec<String> = items.iter().map(J::version_arg).collect::<Option<_>>()?;
            let r = range(1)?;
            let o = opts(2)?;
            let found = if f == "ranges/max-satisfying" {
                npm::max_satisfying(&list, &r, o)
            } else {
                npm::min_satisfying(&list, &r, o)
            };
            opt_str(found.map(String::from))
        }
        "ranges/gtr" => throws(npm::gtr(&ver(0)?, &range(1)?, opts(2)?), J::Bool),
        "ranges/ltr" => throws(npm::ltr(&ver(0)?, &range(1)?, opts(2)?), J::Bool),
        "ranges/outside" => {
            let (v, r) = (ver(0)?, range(1)?);
            let o = opts(3)?;
            match arg(2)?.str()? {
                ">" => throws(npm::gtr(&v, &r, o), J::Bool),
                "<" => throws(npm::ltr(&v, &r, o), J::Bool),
                _ => throws::<()>(Err(dummy_error()), |_| J::Null),
            }
        }
        "ranges/intersects" => throws(npm::intersects(&range(0)?, &range(1)?, opts(2)?), J::Bool),
        "ranges/subset" => throws(npm::subset(&range(0)?, &range(1)?, opts(2)?), J::Bool),
        "ranges/simplify" => {
            let J::Arr(items) = arg(0)? else { return None };
            let list: Vec<String> = items.iter().map(J::version_arg).collect::<Option<_>>()?;
            throws(npm::simplify_range(&list, &range(1)?, opts(2)?), J::Str)
        }
        _ => return None,
    })
}

fn op_holds(name: &str, o: Ordering) -> bool {
    match name {
        "eq" => o == Ordering::Equal,
        "neq" => o != Ordering::Equal,
        "gt" => o == Ordering::Greater,
        "gte" => o != Ordering::Less,
        "lt" => o == Ordering::Less,
        _ => o != Ordering::Greater,
    }
}

fn dummy_error() -> forgo_lib_semver::Error {
    Version::parse("").unwrap_err()
}

/// node-semver's result in the recording: ranges come back as their text.
fn theirs(f: &str, r: &J) -> J {
    match (f, r) {
        ("ranges/simplify", J::Obj(_)) if r.get("raw").is_some() => {
            r.get("raw").cloned().unwrap_or(J::Null)
        }
        _ => r.clone(),
    }
}

fn data() -> Option<String> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/node-semver-data/calls.json");
    match std::fs::read_to_string(&path) {
        Ok(text) => Some(text),
        Err(_) => {
            assert!(
                std::env::var("SEMVER_NODE_REQUIRED").as_deref() != Ok("1"),
                "{} is missing; run scripts/fetch-node-semver.sh",
                path.display()
            );
            eprintln!("skipped: no node-semver data (scripts/fetch-node-semver.sh)");
            None
        }
    }
}

#[test]
fn npm_matches_node_semver() {
    let Some(text) = data() else { return };
    let doc = Doc::from_str(&text).expect("calls.json is JSON");
    let root = J::of(doc.root());

    // 1. Every recorded call.
    let J::Arr(calls) = root.get("calls").expect("calls") else {
        panic!("calls is an array")
    };
    let (mut compared, mut beyond, mut skipped) = (0, 0, 0);
    let mut failures = Vec::new();
    for call in calls {
        let f = call.get("f").and_then(J::str).expect("f");
        let J::Arr(args) = call.get("a").expect("a") else {
            panic!("a is an array")
        };
        let expected = theirs(f, call.get("r").expect("r"));
        let Some(got) = ours(f, args) else {
            skipped += 1;
            continue;
        };
        if args.iter().any(beyond_node_limits) || beyond_node_limits(&expected) {
            beyond += 1;
            continue;
        }
        compared += 1;
        if !same(&got, &expected) {
            failures.push(format!(
                "{f}{args:?}\n    node-semver: {expected:?}\n    ours:        {got:?}"
            ));
        }
    }

    // 2. The cross product.
    let matrix = root.get("matrix").expect("matrix");
    let J::Arr(versions) = matrix.get("versions").expect("versions") else {
        panic!("versions")
    };
    let versions: Vec<&str> = versions.iter().map(|v| v.str().expect("string")).collect();
    let J::Arr(rows) = matrix.get("rows").expect("rows") else {
        panic!("rows")
    };
    let all = [
        NpmOptions::default(),
        NpmOptions::LOOSE,
        NpmOptions::INCLUDE_PRERELEASE,
        NpmOptions {
            loose: true,
            include_prerelease: true,
        },
    ];
    let (mut row_checks, mut row_beyond) = (0usize, 0usize);
    for row in rows {
        let J::Arr(cells) = row else { panic!("row") };
        let range = cells[0].str().expect("range");
        let o = all[cells[1].clone().num_index()];
        if beyond_node_limits(&cells[0]) {
            row_beyond += 1;
            continue;
        }
        let parsed = opt_str(npm::valid_range(range, o));
        if !same(&parsed, &cells[2]) {
            failures.push(format!(
                "validRange({range:?}, {o:?})\n    node-semver: {:?}\n    ours:        {parsed:?}",
                cells[2]
            ));
        }
        let min = match npm::min_version(range, o) {
            Ok(Some(v)) => J::Str(v.to_string()),
            _ => J::Null,
        };
        if !same(&min, &cells[3]) {
            failures.push(format!(
                "minVersion({range:?}, {o:?})\n    node-semver: {:?}\n    ours:        {min:?}",
                cells[3]
            ));
        }
        let bits = cells[4].str().expect("bits").as_bytes();
        // satisfies(v, range, o), with the range parsed once.
        let parsed = forgo_lib_semver::Req::npm_with(range, o).ok();
        for (v, &bit) in versions.iter().zip(bits) {
            let got = match (&parsed, npm::parse(v, o)) {
                (Some(r), Some(v)) => r.matches(&v),
                _ => false,
            };
            if got != (bit == b'1') {
                failures.push(format!(
                    "satisfies({v:?}, {range:?}, {o:?}): node-semver {}, ours {got}",
                    bit == b'1'
                ));
            }
        }
        row_checks += 2 + bits.len();
    }

    println!(
        "node-semver: {compared} calls compared, {beyond} beyond node-semver's limits, {skipped} not expressible in Rust; \
         {row_checks} cross-product checks ({} rows, {row_beyond} beyond limits); {} failures",
        rows.len(),
        failures.len()
    );
    assert!(
        failures.is_empty(),
        "{} differences from node-semver:\n{}",
        failures.len(),
        failures
            .iter()
            .take(40)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

trait NumIndex {
    fn num_index(self) -> usize;
}

impl NumIndex for J {
    fn num_index(self) -> usize {
        match self {
            J::Num(n) => n.parse().expect("index"),
            _ => panic!("index"),
        }
    }
}
