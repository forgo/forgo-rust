//! `cargo x toml`: read and set values in any TOML file, keeping the rest
//! of the file as it was (forgo_lib_toml).

use crate::error::{Error, Result, failed};
use crate::fsx::{self, Plan};
use crate::workspace;
use forgo_lib_toml::{Document, Item, Seg};
use std::path::Path;

/// Help for `cargo x toml`.
pub const HELP: &str = "\
cargo x toml: values in TOML files

  get <file> <key>               Print the value (a string as its text,
                                 anything else as TOML)
  set <file> <key> <value>       Set it to a TOML value: '\"text\"', 42,
                                 true, [1, 2], { a = 1 }
  set --string <file> <key> <text>   Set it to the string <text>

<key> is a TOML key, dotted for nested tables, with [N] for an item of an
array: package.version, dependencies.\"my-crate\".features[0],
package[3].version. `set` makes missing tables, and keeps comments and
layout everywhere else. `get` exits with code 1 when the key isn't set.";

fn usage<T>(msg: impl Into<String>) -> Result<T> {
    Err(Error::Usage(format!("toml: {}", msg.into())))
}

/// A step of a key path, owning its text.
enum Step {
    Key(String),
    Index(usize),
}

/// A key path: TOML's dotted keys (bare, "basic" or 'literal'), each
/// followed by any number of `[N]`.
fn parse_key(text: &str) -> Result<Vec<Step>> {
    let bad = || Error::Usage(format!("toml: `{text}` isn't a key"));
    let b = text.as_bytes();
    let mut i = 0;
    let mut steps = Vec::new();
    loop {
        while matches!(b.get(i), Some(b' ' | b'\t')) {
            i += 1;
        }
        let start = i;
        match b.get(i) {
            Some(b'"' | b'\'') => {
                let q = b[i];
                i += 1;
                while i < b.len() && b[i] != q {
                    i += if q == b'"' && b[i] == b'\\' { 2 } else { 1 };
                }
                if i >= b.len() {
                    return Err(bad());
                }
                i += 1;
                // Let the TOML parser decode the quoted key.
                let doc =
                    Document::parse(&format!("{} = 0", &text[start..i])).map_err(|_| bad())?;
                let key = doc.root().keys().next().map(str::to_string);
                steps.push(Step::Key(key.ok_or_else(bad)?));
            }
            Some(c) if c.is_ascii_alphanumeric() || *c == b'_' || *c == b'-' => {
                while b
                    .get(i)
                    .is_some_and(|c| c.is_ascii_alphanumeric() || *c == b'_' || *c == b'-')
                {
                    i += 1;
                }
                steps.push(Step::Key(text[start..i].to_string()));
            }
            _ => return Err(bad()),
        }
        while b.get(i) == Some(&b'[') {
            let close = text[i..].find(']').map(|j| i + j).ok_or_else(bad)?;
            let n = text[i + 1..close].trim().parse().map_err(|_| bad())?;
            steps.push(Step::Index(n));
            i = close + 1;
        }
        while matches!(b.get(i), Some(b' ' | b'\t')) {
            i += 1;
        }
        match b.get(i) {
            None => return Ok(steps),
            Some(b'.') => i += 1,
            Some(_) => return Err(bad()),
        }
    }
}

fn segs(steps: &[Step]) -> Vec<Seg<'_>> {
    steps
        .iter()
        .map(|s| match s {
            Step::Key(k) => Seg::Key(k),
            Step::Index(i) => Seg::Index(*i),
        })
        .collect()
}

/// Run `cargo x toml <args>`; files are relative to `root`.
pub fn run(root: &Path, args: &[String]) -> Result<String> {
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match args.as_slice() {
        [] | ["help"] => Ok(format!("{HELP}\n")),
        ["get", file, key] => {
            let path = root.join(file);
            let doc = workspace::parse(&fsx::read(&path)?, file)?;
            let steps = parse_key(key)?;
            match doc.get(segs(&steps)) {
                Some(v) => Ok(match v.as_str() {
                    Some(s) => format!("{s}\n"),
                    None => format!("{}\n", v.to_item()),
                }),
                None => Err(Error::Unmet(format!("{file}: `{key}` isn't set\n"))),
            }
        }
        ["set", "--string", file, key, text] => {
            set(root, file, key, Item::String(text.to_string()))
        }
        ["set", file, key, value] => {
            let doc = Document::parse(&format!("x = {value}")).map_err(|_| {
                Error::Usage(format!(
                    "toml: `{value}` isn't a TOML value (quote a string, '\"{value}\"', or use --string)"
                ))
            })?;
            let item = doc.root().get("x").map(|v| v.to_item()).expect("parsed");
            set(root, file, key, item)
        }
        ["get", ..] => usage("get: expected <file> <key>"),
        ["set", ..] => usage("set: expected <file> <key> <value>, or --string <file> <key> <text>"),
        [other, ..] => usage(format!("unknown command `{other}` (expected get or set)")),
    }
}

fn set(root: &Path, file: &str, key: &str, value: Item) -> Result<String> {
    let path = root.join(file);
    let text = fsx::read(&path)?;
    let mut doc = workspace::parse(&text, file)?;
    let steps = parse_key(key)?;
    if steps.is_empty() {
        return failed("toml: no key");
    }
    doc.set(segs(&steps), value)
        .map_err(|e| Error::Failed(format!("{file}: {e}")))?;
    let new = doc.to_string();
    if new == text {
        return Ok(String::new());
    }
    let mut plan = Plan::new(root);
    plan.edit(path, new);
    plan.apply()?;
    Ok(plan.describe())
}
