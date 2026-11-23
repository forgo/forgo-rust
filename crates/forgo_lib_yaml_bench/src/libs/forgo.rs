//! forgo_lib_yaml: events, `Doc` loading and emission.

use crate::data::{Data, float_text};
use crate::libs::Emitter;
use crate::suite::{Anchor, Ev, Style};
use forgo_lib_yaml::{Doc, Elem, Event, MapKey, Node, Scalar, ScalarStyle};
use std::collections::HashMap;

fn anchor(a: Option<std::borrow::Cow<'_, str>>) -> Anchor {
    a.map_or(Anchor::None, |a| Anchor::Name(a.into_owned()))
}

pub fn events(s: &str) -> Result<Vec<Ev>, String> {
    let mut out = Vec::new();
    for r in forgo_lib_yaml::events(s) {
        let (ev, _) = r.map_err(|e| e.to_string())?;
        out.push(match ev {
            Event::StreamStart => Ev::StreamStart,
            Event::StreamEnd => Ev::StreamEnd,
            Event::DocumentStart { explicit, .. } => Ev::DocStart(Some(explicit)),
            Event::DocumentEnd { explicit } => Ev::DocEnd(Some(explicit)),
            Event::SequenceStart {
                anchor: a,
                tag,
                flow,
            } => Ev::SeqStart {
                anchor: anchor(a),
                tag: tag.map(|t| t.resolved.into_owned()),
                flow: Some(flow),
            },
            Event::SequenceEnd => Ev::SeqEnd,
            Event::MappingStart {
                anchor: a,
                tag,
                flow,
            } => Ev::MapStart {
                anchor: anchor(a),
                tag: tag.map(|t| t.resolved.into_owned()),
                flow: Some(flow),
            },
            Event::MappingEnd => Ev::MapEnd,
            Event::Scalar {
                anchor: a,
                tag,
                value,
                style,
                ..
            } => Ev::Scalar {
                anchor: anchor(a),
                tag: tag.map(|t| t.resolved.into_owned()),
                style: match style {
                    ScalarStyle::Plain => Style::Plain,
                    ScalarStyle::SingleQuoted => Style::Single,
                    ScalarStyle::DoubleQuoted => Style::Double,
                    ScalarStyle::Literal => Style::Literal,
                    ScalarStyle::Folded => Style::Folded,
                },
                value: value.into_owned(),
            },
            Event::Alias { name } => Ev::Alias(Anchor::Name(name.into_owned())),
        });
    }
    Ok(out)
}

pub fn count_events(s: &str) -> Result<usize, String> {
    let mut n = 0;
    for r in forgo_lib_yaml::events(s) {
        r.map_err(|e| e.to_string())?;
        n += 1;
    }
    Ok(n)
}

pub fn load_bench(s: &str) -> Result<usize, String> {
    Doc::from_stream(s)
        .map(|d| d.len())
        .map_err(|e| e.to_string())
}

/// Core-schema tags are applied by the composer; other tags are kept.
fn is_core_tag(t: &str) -> bool {
    let t = t.strip_prefix("!!").or_else(|| {
        t.strip_prefix("!<tag:yaml.org,2002:")
            .and_then(|r| r.strip_suffix('>'))
    });
    matches!(
        t,
        Some("str" | "int" | "float" | "bool" | "null" | "seq" | "map")
    )
}

fn elem(e: &Elem, anchors: &mut HashMap<String, Data>) -> Result<Data, String> {
    let v = match &e.node {
        Node::Scalar(Scalar::Null) => Data::Null,
        Node::Scalar(Scalar::Bool(b)) => Data::Bool(*b),
        Node::Scalar(Scalar::Str(s)) => Data::Str(s.clone()),
        Node::Scalar(Scalar::Num { text }) => match crate::data::core_int(text) {
            Some(i) => Data::Int(i),
            None => float_text(text),
        },
        Node::Seq(items) => Data::Seq(
            items
                .iter()
                .map(|i| elem(i, anchors))
                .collect::<Result<_, _>>()?,
        ),
        Node::Map(entries) => {
            let mut out = Vec::with_capacity(entries.len());
            for (k, v) in entries {
                let key = match k {
                    MapKey::Str(s) => Data::Str(s.clone()),
                    MapKey::Complex(ke) => elem(ke, anchors)?,
                };
                out.push((key, elem(v, anchors)?));
            }
            Data::Map(out)
        }
        Node::Alias(name) => anchors
            .get(name)
            .cloned()
            .ok_or_else(|| format!("undefined alias *{name}"))?,
    };
    let v = match e.meta.tag.as_deref() {
        Some(t) if t != "!" && !is_core_tag(t) => Data::Tagged(t.to_string(), Box::new(v)),
        _ => v,
    };
    if let Some(a) = &e.meta.anchor {
        anchors.insert(a.clone(), v.clone());
    }
    Ok(v)
}

/// A stream without documents loads as one placeholder `Doc` (it keeps the
/// stream's comments); it holds no data.
pub fn load(s: &str) -> Result<Vec<Data>, String> {
    let docs = Doc::from_stream(s).map_err(|e| e.to_string())?;
    docs.iter()
        .filter(|d| !d.is_placeholder())
        .map(|d| elem(d.root(), &mut HashMap::new()))
        .collect()
}

/// Parsed documents keep their source and write it back where the tree is
/// unchanged; `reformat` makes the emitter render the whole tree, as the
/// other libraries' emitters do.
fn rendered(s: &str) -> Result<Vec<Doc>, String> {
    let mut docs = Doc::from_stream(s).map_err(|e| e.to_string())?;
    docs.iter_mut().for_each(Doc::reformat);
    Ok(docs)
}

pub fn emit(s: &str) -> Result<String, String> {
    Doc::to_stream_string(&rendered(s)?).map_err(|e| e.to_string())
}

pub fn emitter(s: &str) -> Result<Emitter, String> {
    let docs = rendered(s)?;
    Ok(Box::new(move || {
        Doc::to_stream_string(&docs)
            .map(|t| t.len())
            .map_err(|e| e.to_string())
    }))
}

/// Writing untouched parsed documents back (their source text).
pub fn writer(s: &str) -> Result<Emitter, String> {
    let docs = Doc::from_stream(s).map_err(|e| e.to_string())?;
    Ok(Box::new(move || {
        Doc::to_stream_string(&docs)
            .map(|t| t.len())
            .map_err(|e| e.to_string())
    }))
}
