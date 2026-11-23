//! The yaml-rust family: yaml-rust (the original), yaml-rust2, saphyr
//! (parser and loader) and granit-parser (serde-saphyr's parser).

use crate::data::{Data, float_text};
use crate::libs::Emitter;
use crate::suite::{Anchor, Ev, Style};

const CORE: &str = "tag:yaml.org,2002:";

fn id(n: usize) -> Anchor {
    if n == 0 { Anchor::None } else { Anchor::Id(n) }
}

// ---------------------------------------------------------------------------
// yaml-rust 0.4
// ---------------------------------------------------------------------------

fn yr_tag(t: Option<yaml_rust::scanner::TokenType>) -> Option<String> {
    match t {
        Some(yaml_rust::scanner::TokenType::Tag(handle, suffix)) => Some(match handle.as_str() {
            "!!" => format!("{CORE}{suffix}"),
            _ => format!("{handle}{suffix}"),
        }),
        _ => None,
    }
}

fn yr_style(s: yaml_rust::scanner::TScalarStyle) -> Style {
    use yaml_rust::scanner::TScalarStyle as T;
    match s {
        T::SingleQuoted => Style::Single,
        T::DoubleQuoted => Style::Double,
        T::Literal => Style::Literal,
        T::Foled => Style::Folded,
        _ => Style::Plain,
    }
}

pub fn yaml_rust_events(s: &str) -> Result<Vec<Ev>, String> {
    use yaml_rust::parser::Event as E;
    let mut p = yaml_rust::parser::Parser::new(s.chars());
    let mut out = Vec::new();
    loop {
        let (ev, _) = p.next().map_err(|e| e.to_string())?;
        let done = ev == E::StreamEnd;
        out.push(match ev {
            E::Nothing => continue,
            E::StreamStart => Ev::StreamStart,
            E::StreamEnd => Ev::StreamEnd,
            E::DocumentStart => Ev::DocStart(None),
            E::DocumentEnd => Ev::DocEnd(None),
            E::Alias(n) => Ev::Alias(id(n)),
            E::Scalar(v, style, n, tag) => Ev::Scalar {
                anchor: id(n),
                tag: yr_tag(tag),
                style: yr_style(style),
                value: v,
            },
            E::SequenceStart(n) => Ev::SeqStart {
                anchor: id(n),
                tag: None,
                flow: None,
            },
            E::SequenceEnd => Ev::SeqEnd,
            E::MappingStart(n) => Ev::MapStart {
                anchor: id(n),
                tag: None,
                flow: None,
            },
            E::MappingEnd => Ev::MapEnd,
        });
        if done {
            return Ok(out);
        }
    }
}

pub fn yaml_rust_count(s: &str) -> Result<usize, String> {
    let mut p = yaml_rust::parser::Parser::new(s.chars());
    let mut n = 0;
    loop {
        let (ev, _) = p.next().map_err(|e| e.to_string())?;
        n += 1;
        if ev == yaml_rust::parser::Event::StreamEnd {
            return Ok(n);
        }
    }
}

fn yr_data(y: &yaml_rust::Yaml) -> Result<Data, String> {
    use yaml_rust::Yaml as Y;
    Ok(match y {
        Y::Real(t) => float_text(t),
        Y::Integer(i) => Data::Int(*i as i128),
        Y::String(s) => Data::Str(s.clone()),
        Y::Boolean(b) => Data::Bool(*b),
        Y::Array(a) => Data::Seq(a.iter().map(yr_data).collect::<Result<_, _>>()?),
        Y::Hash(h) => Data::Map(
            h.iter()
                .map(|(k, v)| Ok((yr_data(k)?, yr_data(v)?)))
                .collect::<Result<_, String>>()?,
        ),
        Y::Null => Data::Null,
        Y::Alias(_) => return Err("unresolved alias".into()),
        Y::BadValue => Data::Str(crate::libs::BAD_VALUE.into()),
    })
}

pub fn yaml_rust_load(s: &str) -> Result<Vec<Data>, String> {
    let docs = yaml_rust::YamlLoader::load_from_str(s).map_err(|e| e.to_string())?;
    docs.iter().map(yr_data).collect()
}

pub fn yaml_rust_bench(s: &str) -> Result<usize, String> {
    yaml_rust::YamlLoader::load_from_str(s)
        .map(|d| d.len())
        .map_err(|e| e.to_string())
}

fn yaml_rust_dump(docs: &[yaml_rust::Yaml]) -> Result<String, String> {
    crate::libs::join_docs(docs.iter().map(|d| {
        let mut one = String::new();
        yaml_rust::YamlEmitter::new(&mut one)
            .dump(d)
            .map_err(|e| format!("{e:?}"))?;
        Ok(one)
    }))
}

pub fn yaml_rust_emit(s: &str) -> Result<String, String> {
    let docs = yaml_rust::YamlLoader::load_from_str(s).map_err(|e| e.to_string())?;
    yaml_rust_dump(&docs)
}

pub fn yaml_rust_emitter(s: &str) -> Result<Emitter, String> {
    let docs = yaml_rust::YamlLoader::load_from_str(s).map_err(|e| e.to_string())?;
    Ok(Box::new(move || yaml_rust_dump(&docs).map(|t| t.len())))
}

// ---------------------------------------------------------------------------
// yaml-rust2 0.13
// ---------------------------------------------------------------------------

fn yr2_style(s: yaml_rust2::scanner::TScalarStyle) -> Style {
    use yaml_rust2::scanner::TScalarStyle as T;
    match s {
        T::SingleQuoted => Style::Single,
        T::DoubleQuoted => Style::Double,
        T::Literal => Style::Literal,
        T::Folded => Style::Folded,
        _ => Style::Plain,
    }
}

fn yr2_tag(t: Option<yaml_rust2::parser::Tag>) -> Option<String> {
    t.map(|t| format!("{}{}", t.handle, t.suffix))
}

pub fn yaml_rust2_events(s: &str) -> Result<Vec<Ev>, String> {
    use yaml_rust2::parser::Event as E;
    let mut p = yaml_rust2::parser::Parser::new_from_str(s);
    let mut out = Vec::new();
    loop {
        let (ev, _) = p.next_token().map_err(|e| e.to_string())?;
        let done = ev == E::StreamEnd;
        out.push(match ev {
            E::Nothing => continue,
            E::StreamStart => Ev::StreamStart,
            E::StreamEnd => Ev::StreamEnd,
            E::DocumentStart => Ev::DocStart(None),
            E::DocumentEnd => Ev::DocEnd(None),
            E::Alias(n) => Ev::Alias(id(n)),
            E::Scalar(v, style, n, tag) => Ev::Scalar {
                anchor: id(n),
                tag: yr2_tag(tag),
                style: yr2_style(style),
                value: v,
            },
            E::SequenceStart(n, tag) => Ev::SeqStart {
                anchor: id(n),
                tag: yr2_tag(tag),
                flow: None,
            },
            E::SequenceEnd => Ev::SeqEnd,
            E::MappingStart(n, tag) => Ev::MapStart {
                anchor: id(n),
                tag: yr2_tag(tag),
                flow: None,
            },
            E::MappingEnd => Ev::MapEnd,
        });
        if done {
            return Ok(out);
        }
    }
}

pub fn yaml_rust2_count(s: &str) -> Result<usize, String> {
    let mut p = yaml_rust2::parser::Parser::new_from_str(s);
    let mut n = 0;
    loop {
        let (ev, _) = p.next_token().map_err(|e| e.to_string())?;
        n += 1;
        if ev == yaml_rust2::parser::Event::StreamEnd {
            return Ok(n);
        }
    }
}

fn yr2_data(y: &yaml_rust2::Yaml) -> Result<Data, String> {
    use yaml_rust2::Yaml as Y;
    Ok(match y {
        Y::Real(t) => float_text(t),
        Y::Integer(i) => Data::Int(*i as i128),
        Y::String(s) => Data::Str(s.clone()),
        Y::Boolean(b) => Data::Bool(*b),
        Y::Array(a) => Data::Seq(a.iter().map(yr2_data).collect::<Result<_, _>>()?),
        Y::Hash(h) => Data::Map(
            h.iter()
                .map(|(k, v)| Ok((yr2_data(k)?, yr2_data(v)?)))
                .collect::<Result<_, String>>()?,
        ),
        Y::Null => Data::Null,
        Y::Alias(_) => return Err("unresolved alias".into()),
        Y::BadValue => Data::Str(crate::libs::BAD_VALUE.into()),
    })
}

pub fn yaml_rust2_load(s: &str) -> Result<Vec<Data>, String> {
    let docs = yaml_rust2::YamlLoader::load_from_str(s).map_err(|e| e.to_string())?;
    docs.iter().map(yr2_data).collect()
}

pub fn yaml_rust2_bench(s: &str) -> Result<usize, String> {
    yaml_rust2::YamlLoader::load_from_str(s)
        .map(|d| d.len())
        .map_err(|e| e.to_string())
}

fn yaml_rust2_dump(docs: &[yaml_rust2::Yaml]) -> Result<String, String> {
    crate::libs::join_docs(docs.iter().map(|d| {
        let mut one = String::new();
        yaml_rust2::YamlEmitter::new(&mut one)
            .dump(d)
            .map_err(|e| format!("{e:?}"))?;
        Ok(one)
    }))
}

pub fn yaml_rust2_emit(s: &str) -> Result<String, String> {
    let docs = yaml_rust2::YamlLoader::load_from_str(s).map_err(|e| e.to_string())?;
    yaml_rust2_dump(&docs)
}

pub fn yaml_rust2_emitter(s: &str) -> Result<Emitter, String> {
    let docs = yaml_rust2::YamlLoader::load_from_str(s).map_err(|e| e.to_string())?;
    Ok(Box::new(move || yaml_rust2_dump(&docs).map(|t| t.len())))
}

// ---------------------------------------------------------------------------
// saphyr 0.4 (saphyr-parser events, saphyr loader)
// ---------------------------------------------------------------------------

fn sp_style(s: saphyr_parser::ScalarStyle) -> Style {
    use saphyr_parser::ScalarStyle as S;
    match s {
        S::SingleQuoted => Style::Single,
        S::DoubleQuoted => Style::Double,
        S::Literal => Style::Literal,
        S::Folded => Style::Folded,
        _ => Style::Plain,
    }
}

fn sp_tag(t: Option<std::borrow::Cow<'_, saphyr_parser::Tag>>) -> Option<String> {
    t.map(|t| format!("{}{}", t.handle, t.suffix))
}

pub fn saphyr_events(s: &str) -> Result<Vec<Ev>, String> {
    use saphyr_parser::Event as E;
    let mut out = Vec::new();
    for r in saphyr_parser::Parser::new_from_str(s) {
        let (ev, _) = r.map_err(|e| e.to_string())?;
        out.push(match ev {
            E::Nothing => continue,
            E::StreamStart => Ev::StreamStart,
            E::StreamEnd => Ev::StreamEnd,
            E::DocumentStart(explicit) => Ev::DocStart(Some(explicit)),
            E::DocumentEnd => Ev::DocEnd(None),
            E::Alias(n) => Ev::Alias(id(n)),
            E::Scalar(v, style, n, tag) => Ev::Scalar {
                anchor: id(n),
                tag: sp_tag(tag),
                style: sp_style(style),
                value: v.into_owned(),
            },
            E::SequenceStart(n, tag) => Ev::SeqStart {
                anchor: id(n),
                tag: sp_tag(tag),
                flow: None,
            },
            E::SequenceEnd => Ev::SeqEnd,
            E::MappingStart(n, tag) => Ev::MapStart {
                anchor: id(n),
                tag: sp_tag(tag),
                flow: None,
            },
            E::MappingEnd => Ev::MapEnd,
        });
    }
    Ok(out)
}

pub fn saphyr_count(s: &str) -> Result<usize, String> {
    let mut n = 0;
    for r in saphyr_parser::Parser::new_from_str(s) {
        r.map_err(|e| e.to_string())?;
        n += 1;
    }
    Ok(n)
}

fn sy_data(y: &saphyr::Yaml<'_>) -> Result<Data, String> {
    use saphyr::{Scalar as S, Yaml as Y};
    Ok(match y {
        Y::Value(S::Null) => Data::Null,
        Y::Value(S::Boolean(b)) => Data::Bool(*b),
        Y::Value(S::Integer(i)) => Data::Int(*i as i128),
        Y::Value(S::FloatingPoint(f)) => Data::Float(f.into_inner()),
        Y::Value(S::String(s)) => Data::Str(s.to_string()),
        Y::Representation(text, style, tag) => {
            let plain = matches!(style, saphyr::ScalarStyle::Plain);
            let v = if plain && tag.is_none() {
                crate::data::resolve_plain(text)
            } else {
                Data::Str(text.to_string())
            };
            match tag {
                Some(t) => Data::Tagged(format!("{}{}", t.handle, t.suffix), Box::new(v)),
                None => v,
            }
        }
        Y::Sequence(a) => Data::Seq(a.iter().map(sy_data).collect::<Result<_, _>>()?),
        Y::Mapping(h) => Data::Map(
            h.iter()
                .map(|(k, v)| Ok((sy_data(k)?, sy_data(v)?)))
                .collect::<Result<_, String>>()?,
        ),
        Y::Tagged(t, inner) => Data::Tagged(
            format!("{}{}", t.handle, t.suffix),
            Box::new(sy_data(inner)?),
        ),
        Y::Alias(_) => return Err("unresolved alias".into()),
        Y::BadValue => Data::Str(crate::libs::BAD_VALUE.into()),
    })
}

fn saphyr_docs(s: &str) -> Result<Vec<saphyr::Yaml<'_>>, String> {
    use saphyr::LoadableYamlNode as _;
    saphyr::Yaml::load_from_str(s).map_err(|e| e.to_string())
}

pub fn saphyr_load(s: &str) -> Result<Vec<Data>, String> {
    saphyr_docs(s)?.iter().map(sy_data).collect()
}

pub fn saphyr_bench(s: &str) -> Result<usize, String> {
    saphyr_docs(s).map(|d| d.len())
}

fn saphyr_dump(docs: &[saphyr::Yaml<'_>]) -> Result<String, String> {
    crate::libs::join_docs(docs.iter().map(|d| {
        let mut one = String::new();
        saphyr::YamlEmitter::new(&mut one)
            .dump(d)
            .map_err(|e| format!("{e:?}"))?;
        Ok(one)
    }))
}

pub fn saphyr_emit(s: &str) -> Result<String, String> {
    saphyr_dump(&saphyr_docs(s)?)
}

/// saphyr's emitter takes the borrowing `Yaml<'input>`, so the input is
/// leaked to give the loaded tree a `'static` source (benchmark only).
pub fn saphyr_emitter(s: &str) -> Result<Emitter, String> {
    let text: &'static str = Box::leak(s.to_string().into_boxed_str());
    let docs = saphyr_docs(text)?;
    Ok(Box::new(move || saphyr_dump(&docs).map(|t| t.len())))
}

// ---------------------------------------------------------------------------
// granit-parser 2.2 (the parser under serde-saphyr)
// ---------------------------------------------------------------------------

fn gr_style(s: granit_parser::ScalarStyle) -> Style {
    use granit_parser::ScalarStyle as S;
    match s {
        S::SingleQuoted => Style::Single,
        S::DoubleQuoted => Style::Double,
        S::Literal => Style::Literal,
        S::Folded => Style::Folded,
        _ => Style::Plain,
    }
}

fn gr_tag(t: Option<std::borrow::Cow<'_, granit_parser::Tag>>) -> Option<String> {
    t.map(|t| format!("{}{}", t.handle(), t.suffix()))
}

fn granit_options() -> granit_parser::Options {
    let mut o = granit_parser::Options::default();
    o.emit_anchor_events = true;
    o
}

/// Anchor names come from `Anchor` events, which follow their node; the
/// explicit `...` flag isn't an event field, so it is read from the
/// `DocumentEnd` event's span.
pub fn granit_events(s: &str) -> Result<Vec<Ev>, String> {
    use granit_parser::{Event as E, StructureStyle};
    let mut out: Vec<Ev> = Vec::new();
    let mut names: std::collections::HashMap<usize, String> = Default::default();
    for r in granit_parser::Parser::new_from_str_with_options(s, granit_options()) {
        let (ev, span) = r.map_err(|e| e.to_string())?;
        out.push(match ev {
            E::StreamStart => Ev::StreamStart,
            E::StreamEnd => Ev::StreamEnd,
            E::DocumentStart(explicit, _) => Ev::DocStart(Some(explicit)),
            E::DocumentEnd => {
                let text = match (span.start.byte_offset(), span.end.byte_offset()) {
                    (Some(a), Some(b)) => s.get(a..b).unwrap_or(""),
                    _ => "",
                };
                Ev::DocEnd(Some(text == "..."))
            }
            E::Anchor(n, name) => {
                names.insert(n, name.to_string());
                if let Some(
                    Ev::Scalar { anchor, .. }
                    | Ev::SeqStart { anchor, .. }
                    | Ev::MapStart { anchor, .. },
                ) = out.iter_mut().rev().find(|e| {
                    matches!(e, Ev::Scalar { anchor: Anchor::Id(m), .. }
                        | Ev::SeqStart { anchor: Anchor::Id(m), .. }
                        | Ev::MapStart { anchor: Anchor::Id(m), .. } if *m == n)
                }) {
                    *anchor = Anchor::Name(name.into_owned());
                }
                continue;
            }
            E::Alias(n) => Ev::Alias(
                names
                    .get(&n)
                    .map_or(Anchor::Id(n), |a| Anchor::Name(a.clone())),
            ),
            E::UnresolvedAlias(name) => return Err(format!("unresolved alias *{name}")),
            E::Scalar(v, style, n, tag) => Ev::Scalar {
                anchor: id(n),
                tag: gr_tag(tag),
                style: gr_style(style),
                value: v.into_owned(),
            },
            E::SequenceStart(st, n, tag) => Ev::SeqStart {
                anchor: id(n),
                tag: gr_tag(tag),
                flow: Some(st == StructureStyle::Flow),
            },
            E::SequenceEnd => Ev::SeqEnd,
            E::MappingStart(st, n, tag) => Ev::MapStart {
                anchor: id(n),
                tag: gr_tag(tag),
                flow: Some(st == StructureStyle::Flow),
            },
            E::MappingEnd => Ev::MapEnd,
            #[allow(unreachable_patterns)]
            _ => continue,
        });
    }
    Ok(out)
}

pub fn granit_count(s: &str) -> Result<usize, String> {
    let mut n = 0;
    for r in granit_parser::Parser::new_from_str(s) {
        r.map_err(|e| e.to_string())?;
        n += 1;
    }
    Ok(n)
}
