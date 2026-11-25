//! `cargo x semver`: versions and requirements on the command line, with
//! forgo_lib_semver.
//!
//! Every command prints text, or JSON with `--json`. Exit codes: 0 when
//! the answer is yes (valid, matches, found), 1 when it is no, 2 for a
//! usage mistake.

use crate::error::{Error, Result};
use forgo_lib_semver::npm::NpmOptions;
use forgo_lib_semver::{Bump, Identifier, Req, Version};
use std::cmp::Ordering;
use std::io::Read;

/// Help for `cargo x semver`.
pub const HELP: &str = "\
cargo x semver: versions and requirements (Semantic Versioning 2.0.0)

  validate <version>...            Is each a valid version?
  explain <version|requirement>    The parts of a version, or what a
                                   requirement means
  compare <a> <b>                  Precedence: a < b, a = b or a > b
  sort [<version>...]              Sorted by precedence (stdin if none)
  bump <release> <version> [<id>]  The next version: major, minor, patch,
                                   premajor, preminor, prepatch,
                                   prerelease or release
  matches <requirement> <version>...  Does each version meet it?
  max <requirement> [<version>...] The highest that meets it (stdin if none)
  min <requirement> [<version>...] The lowest that meets it (stdin if none)

Options:
  --npm       requirements in npm's syntax (default: Cargo's)
  --lenient   versions as npm reads them loosely (v1.2.3, =1.2.3)
  --reverse   sort from highest to lowest
  --json      print JSON

Exit codes: 0 yes, 1 no, 2 usage mistake.";

struct Opts {
    npm: bool,
    lenient: bool,
    reverse: bool,
    json: bool,
    args: Vec<String>,
}

fn usage<T>(msg: impl Into<String>) -> Result<T> {
    Err(Error::Usage(format!("semver: {}", msg.into())))
}

fn opts(args: &[String], allowed: &[&str]) -> Result<Opts> {
    let mut o = Opts {
        npm: false,
        lenient: false,
        reverse: false,
        json: false,
        args: Vec::new(),
    };
    for a in args {
        let flag = match a.as_str() {
            "--npm" => &mut o.npm,
            "--lenient" => &mut o.lenient,
            "--reverse" => &mut o.reverse,
            "--json" => &mut o.json,
            _ if a.starts_with("--") => return usage(format!("unknown option `{a}`")),
            _ => {
                o.args.push(a.clone());
                continue;
            }
        };
        if !allowed.contains(&a.as_str()) {
            return usage(format!("`{a}` doesn't apply here"));
        }
        *flag = true;
    }
    Ok(o)
}

/// Run `cargo x semver <args>`.
pub fn run(args: &[String]) -> Result<String> {
    let Some(cmd) = args.first() else {
        return Ok(format!("{HELP}\n"));
    };
    let rest = &args[1..];
    let versions = ["--lenient", "--json"];
    let reqs = ["--npm", "--lenient", "--json"];
    match cmd.as_str() {
        "help" | "--help" | "-h" => Ok(format!("{HELP}\n")),
        "validate" => validate(opts(rest, &versions)?),
        "explain" => explain(opts(rest, &reqs)?),
        "compare" => compare(opts(rest, &versions)?),
        "sort" => sort(opts(rest, &["--lenient", "--json", "--reverse"])?),
        "bump" => bump(opts(rest, &versions)?),
        "matches" => matches(opts(rest, &reqs)?),
        "max" => extreme(opts(rest, &reqs)?, true),
        "min" => extreme(opts(rest, &reqs)?, false),
        other => usage(format!("unknown command `{other}`")),
    }
}

fn parse_version(o: &Opts, s: &str) -> std::result::Result<Version, String> {
    let r = if o.lenient {
        Version::parse_lenient(s)
    } else {
        Version::parse(s)
    };
    r.map_err(|e| {
        let hint = if !o.lenient && (s.starts_with('v') || s.starts_with('=')) {
            " (--lenient accepts a `v` or `=` prefix)"
        } else {
            ""
        };
        format!("{e}{hint}")
    })
}

fn version(o: &Opts, s: &str) -> Result<Version> {
    parse_version(o, s).map_err(|e| Error::Failed(format!("`{s}` isn't a version: {e}")))
}

fn requirement(o: &Opts, s: &str) -> Result<Req> {
    let r = if o.npm {
        Req::npm_with(
            s,
            NpmOptions {
                loose: o.lenient,
                include_prerelease: false,
            },
        )
    } else {
        Req::cargo(s)
    };
    r.map_err(|e| {
        let syntax = if o.npm { "npm" } else { "Cargo" };
        Error::Failed(format!("`{s}` isn't a {syntax} requirement: {e}"))
    })
}

/// Versions from the arguments, or else from standard input (one per
/// line or separated by spaces).
fn inputs(o: &Opts, from: usize) -> Result<Vec<String>> {
    if o.args.len() > from {
        return Ok(o.args[from..].to_vec());
    }
    let mut text = String::new();
    std::io::stdin()
        .read_to_string(&mut text)
        .map_err(|e| Error::Failed(format!("read standard input: {e}")))?;
    Ok(text.split_whitespace().map(String::from).collect())
}

fn parse_all(o: &Opts, texts: &[String]) -> Result<Vec<Version>> {
    let mut out = Vec::with_capacity(texts.len());
    let mut bad = Vec::new();
    for t in texts {
        match parse_version(o, t) {
            Ok(v) => out.push(v),
            Err(e) => bad.push(format!("  {t}: {e}")),
        }
    }
    if !bad.is_empty() {
        return Err(Error::Failed(format!("not versions:\n{}", bad.join("\n"))));
    }
    Ok(out)
}

/// A JSON string.
fn js(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn verdict(all: bool, report: String) -> Result<String> {
    if all {
        Ok(report)
    } else {
        Err(Error::Unmet(report))
    }
}

fn validate(o: Opts) -> Result<String> {
    if o.args.is_empty() {
        return usage("validate: expected at least one version");
    }
    let mut all = true;
    let mut lines = Vec::new();
    for a in &o.args {
        let r = parse_version(&o, a);
        all &= r.is_ok();
        lines.push(match (&r, o.json) {
            (Ok(v), true) => format!(
                "{{\"input\":{},\"valid\":true,\"version\":{}}}",
                js(a),
                js(&v.to_string())
            ),
            (Err(e), true) => format!(
                "{{\"input\":{},\"valid\":false,\"error\":{}}}",
                js(a),
                js(e)
            ),
            (Ok(_), false) => format!("{a}: valid"),
            (Err(e), false) => format!("{a}: invalid: {e}"),
        });
    }
    let report = if o.json {
        format!("[{}]\n", lines.join(","))
    } else {
        format!("{}\n", lines.join("\n"))
    };
    verdict(all, report)
}

fn identifiers<'a>(ids: impl Iterator<Item = Identifier<'a>>) -> Vec<(bool, &'a str)> {
    ids.map(|i| (i.is_numeric(), i.as_str())).collect()
}

fn explain(o: Opts) -> Result<String> {
    let [text] = o.args.as_slice() else {
        return usage("explain: expected one version or requirement");
    };
    if !o.npm
        && let Ok(v) = parse_version(&o, text)
    {
        return Ok(explain_version(&o, &v));
    }
    let r = requirement(&o, text)?;
    let syntax = if o.npm { "npm" } else { "Cargo" };
    let lowest = r.min_version();
    if o.json {
        return Ok(format!(
            "{{\"requirement\":{},\"syntax\":{},\"canonical\":{},\"lowest\":{}}}\n",
            js(text),
            js(syntax),
            js(&r.to_string()),
            lowest.map_or("null".into(), |v| js(&v.to_string()))
        ));
    }
    Ok(format!(
        "requirement  {text} ({syntax})\ncanonical    {r}\nlowest       {}\n",
        lowest.map_or("none: no version meets it".into(), |v| v.to_string())
    ))
}

fn explain_version(o: &Opts, v: &Version) -> String {
    let pre = identifiers(v.pre.identifiers());
    let build = identifiers(v.build.identifiers());
    if o.json {
        let list = |ids: &[(bool, &str)]| {
            ids.iter()
                .map(|(n, s)| format!("{{\"value\":{},\"numeric\":{n}}}", js(s)))
                .collect::<Vec<_>>()
                .join(",")
        };
        return format!(
            "{{\"version\":{},\"major\":{},\"minor\":{},\"patch\":{},\"pre\":[{}],\"build\":[{}]}}\n",
            js(&v.to_string()),
            v.major,
            v.minor,
            v.patch,
            list(&pre),
            list(&build)
        );
    }
    let describe = |ids: &[(bool, &str)], whole: &str| {
        if ids.is_empty() {
            return "none".to_string();
        }
        let kinds: Vec<String> = ids
            .iter()
            .map(|(n, s)| format!("{s}: {}", if *n { "number" } else { "text" }))
            .collect();
        format!("{whole} ({})", kinds.join(", "))
    };
    let mut out = format!(
        "version      {v}\nmajor        {}\nminor        {}\npatch        {}\npre-release  {}\nbuild        {}\n",
        v.major,
        v.minor,
        v.patch,
        describe(&pre, v.pre.as_str()),
        describe(&build, v.build.as_str()),
    );
    if v.is_prerelease() {
        out.push_str("note         a pre-release: lower precedence than its release\n");
    }
    out
}

fn compare(o: Opts) -> Result<String> {
    let [a, b] = o.args.as_slice() else {
        return usage("compare: expected two versions");
    };
    let (x, y) = (version(&o, a)?, version(&o, b)?);
    let ord = x.cmp_precedence(&y);
    let (word, sign) = match ord {
        Ordering::Less => ("less", "<"),
        Ordering::Equal => ("equal", "="),
        Ordering::Greater => ("greater", ">"),
    };
    let build_differs = ord == Ordering::Equal && x.build != y.build;
    if o.json {
        return Ok(format!(
            "{{\"a\":{},\"b\":{},\"precedence\":{},\"identical\":{}}}\n",
            js(&x.to_string()),
            js(&y.to_string()),
            js(word),
            x == y
        ));
    }
    let note = if build_differs {
        " (same precedence; the build metadata differs)"
    } else {
        ""
    };
    Ok(format!("{x} {sign} {y}{note}\n"))
}

fn sort(o: Opts) -> Result<String> {
    let texts = inputs(&o, 0)?;
    let mut vs = parse_all(&o, &texts)?;
    vs.sort();
    if o.reverse {
        vs.reverse();
    }
    // Groups of equal precedence (they differ only in build metadata).
    let mut groups: Vec<Vec<&Version>> = Vec::new();
    for v in &vs {
        match groups.last_mut() {
            Some(g) if g[0].precedence_eq(v) => g.push(v),
            _ => groups.push(vec![v]),
        }
    }
    if o.json {
        let gs: Vec<String> = groups
            .iter()
            .map(|g| {
                let items: Vec<String> = g.iter().map(|v| js(&v.to_string())).collect();
                format!("[{}]", items.join(","))
            })
            .collect();
        return Ok(format!("[{}]\n", gs.join(",")));
    }
    let mut out = String::new();
    for g in &groups {
        for (i, v) in g.iter().enumerate() {
            out.push_str(&v.to_string());
            if i > 0 {
                out.push_str("  (same precedence as the line above)");
            }
            out.push('\n');
        }
    }
    Ok(out)
}

fn bump(o: Opts) -> Result<String> {
    let (release, text, id) = match o.args.as_slice() {
        [r, v] => (r, v, None),
        [r, v, id] => (r, v, Some(id.as_str())),
        _ => return usage("bump: expected <release> <version> [<pre-release id>]"),
    };
    let b = match release.as_str() {
        "major" => Bump::Major,
        "minor" => Bump::Minor,
        "patch" => Bump::Patch,
        "premajor" => Bump::Premajor(id),
        "preminor" => Bump::Preminor(id),
        "prepatch" => Bump::Prepatch(id),
        "prerelease" => Bump::Prerelease(id),
        "release" => Bump::Release,
        other => return usage(format!("bump: unknown release `{other}`")),
    };
    if id.is_some() && matches!(b, Bump::Major | Bump::Minor | Bump::Patch | Bump::Release) {
        return usage(format!("bump: `{release}` takes no pre-release id"));
    }
    let v = version(&o, text)?;
    let next = v
        .bump(b)
        .map_err(|e| Error::Failed(format!("can't bump {v} ({release}): {e}")))?;
    Ok(if o.json {
        format!("{{\"version\":{}}}\n", js(&next.to_string()))
    } else {
        format!("{next}\n")
    })
}

fn matches(o: Opts) -> Result<String> {
    if o.args.len() < 2 {
        return usage("matches: expected a requirement and at least one version");
    }
    let r = requirement(&o, &o.args[0])?;
    let vs = parse_all(&o, &o.args[1..])?;
    let results: Vec<(String, bool)> = vs.iter().map(|v| (v.to_string(), r.matches(v))).collect();
    let all = results.iter().all(|(_, m)| *m);
    let report = if o.json {
        let items: Vec<String> = results
            .iter()
            .map(|(v, m)| format!("{{\"version\":{},\"matches\":{m}}}", js(v)))
            .collect();
        format!("[{}]\n", items.join(","))
    } else {
        results
            .iter()
            .map(|(v, m)| format!("{v}: {}\n", if *m { "matches" } else { "doesn't match" }))
            .collect()
    };
    verdict(all, report)
}

fn extreme(o: Opts, highest: bool) -> Result<String> {
    if o.args.is_empty() {
        return usage("expected a requirement");
    }
    let r = requirement(&o, &o.args[0])?;
    let texts = inputs(&o, 1)?;
    let vs = parse_all(&o, &texts)?;
    let found = if highest {
        r.max_satisfying(&vs)
    } else {
        r.min_satisfying(&vs)
    };
    if o.json {
        let v = found.map_or("null".into(), |v| js(&v.to_string()));
        return verdict(found.is_some(), format!("{{\"version\":{v}}}\n"));
    }
    match found {
        Some(v) => Ok(format!("{v}\n")),
        None => Err(Error::Unmet(format!("no version meets {r}\n"))),
    }
}
