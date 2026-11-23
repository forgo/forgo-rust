//! The official yaml-test-suite: case discovery, the `test.event`
//! notation, and a style-independent normal form for event streams.

use crate::data::{core_bool, core_float, core_int, is_core_null};
use std::fs;
use std::path::{Path, PathBuf};

/// One case of the suite.
pub struct Case {
    pub id: String,
    /// The case's `in.yaml`.
    pub path: PathBuf,
    pub yaml: String,
    pub events: Option<String>,
    pub json: Option<String>,
    pub error: bool,
}

/// Where `crates/forgo_lib_yaml/scripts/fetch-yaml-test-suite.sh` puts the
/// pinned data release.
pub fn data_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../forgo_lib_yaml/tests/yaml-test-suite-data")
}

fn sorted_subdirs(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|n| !n.starts_with('.'))
        .collect();
    names.sort();
    names
}

fn load(id: String, dir: &Path) -> Option<Case> {
    let bytes = fs::read(dir.join("in.yaml")).ok()?;
    Some(Case {
        id,
        path: dir.join("in.yaml"),
        yaml: String::from_utf8(bytes).ok()?,
        events: fs::read_to_string(dir.join("test.event")).ok(),
        json: fs::read_to_string(dir.join("in.json")).ok(),
        error: dir.join("error").exists(),
    })
}

/// Every `<ID>/` with an `in.yaml`, or its numbered subdirectories, in
/// order. `name/` and `tags/` are symlink indexes, not cases.
pub fn discover() -> Vec<Case> {
    let root = data_dir();
    let mut cases = Vec::new();
    for id in sorted_subdirs(&root) {
        if id == "name" || id == "tags" {
            continue;
        }
        let dir = root.join(&id);
        if dir.join("in.yaml").exists() {
            cases.extend(load(id, &dir));
            continue;
        }
        for sub in sorted_subdirs(&dir) {
            let sdir = dir.join(&sub);
            if sdir.join("in.yaml").exists() {
                cases.extend(load(format!("{id}/{sub}"), &sdir));
            }
        }
    }
    cases
}

// ---------------------------------------------------------------------------
// Events
// ---------------------------------------------------------------------------

/// How a node's anchor is reported. Some libraries only report numeric
/// ids, which is enough for the normal form but not for `test.event`.
#[derive(Clone, Debug, PartialEq)]
pub enum Anchor {
    None,
    Name(String),
    Id(usize),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Style {
    Plain,
    Single,
    Double,
    Literal,
    Folded,
}

/// A parse event in a library-neutral form. `None` marks information the
/// library's event API doesn't report.
#[derive(Clone, Debug, PartialEq)]
pub enum Ev {
    StreamStart,
    StreamEnd,
    DocStart(Option<bool>),
    DocEnd(Option<bool>),
    SeqStart {
        anchor: Anchor,
        tag: Option<String>,
        flow: Option<bool>,
    },
    SeqEnd,
    MapStart {
        anchor: Anchor,
        tag: Option<String>,
        flow: Option<bool>,
    },
    MapEnd,
    Scalar {
        anchor: Anchor,
        tag: Option<String>,
        style: Style,
        value: String,
    },
    Alias(Anchor),
}

fn escape(out: &mut String, s: &str) {
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\0' => out.push_str("\\0"),
            '\u{8}' => out.push_str("\\b"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
}

fn unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut it = s.chars();
    while let Some(c) = it.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match it.next() {
            Some('\\') => out.push('\\'),
            Some('0') => out.push('\0'),
            Some('b') => out.push('\u{8}'),
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('t') => out.push('\t'),
            Some(o) => {
                out.push('\\');
                out.push(o);
            }
            None => out.push('\\'),
        }
    }
    out
}

/// The `test.event` line for `ev`, or `None` when the library didn't
/// report something the notation needs.
pub fn exact_line(ev: &Ev) -> Option<String> {
    fn props(out: &mut String, anchor: &Anchor, tag: &Option<String>) -> Option<()> {
        match anchor {
            Anchor::None => {}
            Anchor::Name(a) => {
                out.push_str(" &");
                out.push_str(a);
            }
            Anchor::Id(_) => return None,
        }
        if let Some(t) = tag {
            out.push_str(" <");
            out.push_str(t);
            out.push('>');
        }
        Some(())
    }
    let mut out = String::new();
    match ev {
        Ev::StreamStart => out.push_str("+STR"),
        Ev::StreamEnd => out.push_str("-STR"),
        Ev::DocStart(e) => out.push_str(if (*e)? { "+DOC ---" } else { "+DOC" }),
        Ev::DocEnd(e) => out.push_str(if (*e)? { "-DOC ..." } else { "-DOC" }),
        Ev::SeqStart { anchor, tag, flow } => {
            out.push_str(if (*flow)? { "+SEQ []" } else { "+SEQ" });
            props(&mut out, anchor, tag)?;
        }
        Ev::SeqEnd => out.push_str("-SEQ"),
        Ev::MapStart { anchor, tag, flow } => {
            out.push_str(if (*flow)? { "+MAP {}" } else { "+MAP" });
            props(&mut out, anchor, tag)?;
        }
        Ev::MapEnd => out.push_str("-MAP"),
        Ev::Scalar {
            anchor,
            tag,
            style,
            value,
        } => {
            out.push_str("=VAL");
            props(&mut out, anchor, tag)?;
            out.push(' ');
            out.push(match style {
                Style::Plain => ':',
                Style::Single => '\'',
                Style::Double => '"',
                Style::Literal => '|',
                Style::Folded => '>',
            });
            escape(&mut out, value);
        }
        Ev::Alias(a) => match a {
            Anchor::Name(n) => {
                out.push_str("=ALI *");
                out.push_str(n);
            }
            _ => return None,
        },
    }
    Some(out)
}

/// Read a `test.event` file back into events.
pub fn parse_test_event(text: &str) -> Result<Vec<Ev>, String> {
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim_start();
        if line.is_empty() {
            continue;
        }
        let (head, rest) = line.split_at(4.min(line.len()));
        let ev = match head {
            "+STR" => Ev::StreamStart,
            "-STR" => Ev::StreamEnd,
            "+DOC" => Ev::DocStart(Some(rest.trim() == "---")),
            "-DOC" => Ev::DocEnd(Some(rest.trim() == "...")),
            "-SEQ" => Ev::SeqEnd,
            "-MAP" => Ev::MapEnd,
            "=ALI" => Ev::Alias(Anchor::Name(
                rest.trim()
                    .strip_prefix('*')
                    .ok_or("alias without *")?
                    .to_string(),
            )),
            "+SEQ" | "+MAP" => {
                let mut flow = false;
                let mut anchor = Anchor::None;
                let mut tag = None;
                for part in rest.split_whitespace() {
                    if part == "[]" || part == "{}" {
                        flow = true;
                    } else if let Some(a) = part.strip_prefix('&') {
                        anchor = Anchor::Name(a.to_string());
                    } else if let Some(t) = part.strip_prefix('<').and_then(|t| t.strip_suffix('>'))
                    {
                        tag = Some(t.to_string());
                    } else {
                        return Err(format!("bad collection event: {line}"));
                    }
                }
                if head == "+SEQ" {
                    Ev::SeqStart {
                        anchor,
                        tag,
                        flow: Some(flow),
                    }
                } else {
                    Ev::MapStart {
                        anchor,
                        tag,
                        flow: Some(flow),
                    }
                }
            }
            "=VAL" => {
                let mut rest = rest.strip_prefix(' ').ok_or("bad scalar event")?;
                let mut anchor = Anchor::None;
                let mut tag = None;
                if let Some(r) = rest.strip_prefix('&') {
                    let end = r.find(' ').ok_or("bad anchor")?;
                    anchor = Anchor::Name(r[..end].to_string());
                    rest = &r[end + 1..];
                }
                if let Some(r) = rest.strip_prefix('<') {
                    let end = r.find("> ").ok_or("bad tag")?;
                    tag = Some(r[..end].to_string());
                    rest = &r[end + 2..];
                }
                let mut chars = rest.chars();
                let style = match chars.next() {
                    Some(':') => Style::Plain,
                    Some('\'') => Style::Single,
                    Some('"') => Style::Double,
                    Some('|') => Style::Literal,
                    Some('>') => Style::Folded,
                    _ => return Err(format!("bad scalar style: {line}")),
                };
                Ev::Scalar {
                    anchor,
                    tag,
                    style,
                    value: unescape(chars.as_str()),
                }
            }
            _ => return Err(format!("unknown event: {line}")),
        };
        out.push(ev);
    }
    Ok(out)
}

const CORE: &str = "tag:yaml.org,2002:";

/// The type and canonical value of a scalar under the YAML 1.2 core schema.
fn scalar_kind(value: &str, style: Style, tag: Option<&str>) -> (String, String) {
    let kind = match tag {
        Some("!") => "str",
        Some(t) => match t.strip_prefix(CORE) {
            Some(k @ ("str" | "null" | "bool" | "int" | "float")) => k,
            _ => return (t.to_string(), value.to_string()),
        },
        None if style != Style::Plain => "str",
        None if is_core_null(value) => "null",
        None if core_bool(value).is_some() => "bool",
        None if core_int(value).is_some() => "int",
        None if core_float(value).is_some() => "float",
        None => "str",
    };
    let canon = match kind {
        "null" => "~".to_string(),
        "bool" => core_bool(value).map_or_else(|| value.to_string(), |b| b.to_string()),
        "int" => core_int(value).map_or_else(|| value.to_string(), |i| i.to_string()),
        "float" => match core_float(value).or_else(|| core_int(value).map(|i| i as f64)) {
            Some(f) if f.is_nan() => "nan".into(),
            Some(f) => format!("{f:?}"),
            None => value.to_string(),
        },
        _ => value.to_string(),
    };
    (kind.to_string(), canon)
}

/// Style-independent event lines: node structure, anchors numbered by
/// definition order, resolved tags, and each scalar's core-schema type and
/// canonical value. Scalar styles, flow/block, document markers and the
/// spelling of nulls, booleans and numbers are dropped.
pub fn normalize(events: &[Ev]) -> Vec<String> {
    let mut defined: Vec<Anchor> = Vec::new();
    fn define(defined: &mut Vec<Anchor>, a: &Anchor) -> String {
        if *a == Anchor::None {
            return String::new();
        }
        defined.push(a.clone());
        format!(" &{}", defined.len())
    }
    let mut out = Vec::with_capacity(events.len());
    for ev in events {
        out.push(match ev {
            Ev::StreamStart => "+STR".into(),
            Ev::StreamEnd => "-STR".into(),
            Ev::DocStart(_) => "+DOC".into(),
            Ev::DocEnd(_) => "-DOC".into(),
            Ev::SeqEnd => "-SEQ".into(),
            Ev::MapEnd => "-MAP".into(),
            Ev::SeqStart { anchor, tag, .. } | Ev::MapStart { anchor, tag, .. } => {
                let seq = matches!(ev, Ev::SeqStart { .. });
                let default = if seq {
                    "tag:yaml.org,2002:seq"
                } else {
                    "tag:yaml.org,2002:map"
                };
                let tag = match tag.as_deref() {
                    None | Some("!") => String::new(),
                    Some(t) if t == default => String::new(),
                    Some(t) => format!(" <{t}>"),
                };
                let a = define(&mut defined, anchor);
                format!("{}{a}{tag}", if seq { "+SEQ" } else { "+MAP" })
            }
            Ev::Scalar {
                anchor,
                tag,
                style,
                value,
            } => {
                let (kind, canon) = scalar_kind(value, *style, tag.as_deref());
                let a = define(&mut defined, anchor);
                format!("=VAL{a} <{kind}> {canon:?}")
            }
            Ev::Alias(target) => {
                let n = defined.iter().rposition(|d| d == target);
                match n {
                    Some(i) => format!("=ALI *{}", i + 1),
                    None => format!("=ALI *?{target:?}"),
                }
            }
        });
    }
    out
}

/// The first line where two event listings differ.
pub fn first_difference(expected: &[String], got: &[String]) -> String {
    for i in 0..expected.len().max(got.len()) {
        let (e, g) = (expected.get(i), got.get(i));
        if e != g {
            return format!(
                "line {}: expected `{}`, got `{}`",
                i + 1,
                e.map_or("<end>", |s| s.as_str()),
                g.map_or("<end>", |s| s.as_str())
            );
        }
    }
    "identical".into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_round_trips() {
        let text = "+STR\n+DOC ---\n+MAP {} &m <tag:x>\n=VAL &a <tag:yaml.org,2002:str> :a b\\n\n=ALI *a\n-MAP\n-DOC ...\n-STR\n";
        let evs = parse_test_event(text).unwrap();
        let back: Vec<String> = evs.iter().map(|e| exact_line(e).unwrap()).collect();
        assert_eq!(back.join("\n") + "\n", text);
    }

    #[test]
    fn normal_form_ignores_presentation() {
        let a = parse_test_event("+STR\n+DOC ---\n+SEQ []\n=VAL &x '1\n=VAL :0x1\n=VAL :\n=ALI *x\n-SEQ\n-DOC ...\n-STR\n").unwrap();
        let b = vec![
            Ev::StreamStart,
            Ev::DocStart(None),
            Ev::SeqStart {
                anchor: Anchor::None,
                tag: None,
                flow: None,
            },
            Ev::Scalar {
                anchor: Anchor::Id(7),
                tag: None,
                style: Style::Double,
                value: "1".into(),
            },
            Ev::Scalar {
                anchor: Anchor::None,
                tag: None,
                style: Style::Plain,
                value: "1".into(),
            },
            Ev::Scalar {
                anchor: Anchor::None,
                tag: None,
                style: Style::Plain,
                value: "~".into(),
            },
            Ev::Alias(Anchor::Id(7)),
            Ev::SeqEnd,
            Ev::DocEnd(None),
            Ev::StreamEnd,
        ];
        assert_eq!(normalize(&a), normalize(&b));
        let c = parse_test_event("+STR\n+DOC\n=VAL :1\n-DOC\n-STR\n").unwrap();
        let d = parse_test_event("+STR\n+DOC\n=VAL '1\n-DOC\n-STR\n").unwrap();
        assert_ne!(normalize(&c), normalize(&d), "int and string differ");
    }
}
