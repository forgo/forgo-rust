//! libyaml: the C reference parser, via its c2rust transpilation
//! (`unsafe-libyaml`, used by serde_yaml and serde_yaml_ng) and the safe
//! Rust port (`libyaml-safer`).

use crate::suite::{Anchor, Ev, Style};

/// Run libyaml over `s`, handing each event to `f`.
fn libyaml_parse(s: &str, mut f: impl FnMut(&unsafe_libyaml::yaml_event_t)) -> Result<(), String> {
    use std::mem::MaybeUninit;
    #[allow(clippy::unsafe_removed_from_name)] // a short alias for the C names
    use unsafe_libyaml as y;
    // SAFETY: the documented libyaml parser lifecycle; `s` outlives the
    // parser and every event is deleted after use.
    unsafe {
        let mut parser = MaybeUninit::<y::yaml_parser_t>::uninit();
        if !y::yaml_parser_initialize(parser.as_mut_ptr()).ok {
            return Err("libyaml init failed".into());
        }
        let mut parser = parser.assume_init();
        y::yaml_parser_set_input_string(&mut parser, s.as_ptr(), s.len() as _);
        let result = loop {
            let mut event = MaybeUninit::<y::yaml_event_t>::uninit();
            if !y::yaml_parser_parse(&mut parser, event.as_mut_ptr()).ok {
                let msg = if parser.problem.is_null() {
                    "libyaml error".to_string()
                } else {
                    std::ffi::CStr::from_ptr(parser.problem as *const std::ffi::c_char)
                        .to_string_lossy()
                        .into_owned()
                };
                break Err(msg);
            }
            let mut event = event.assume_init();
            let done = event.type_ == y::YAML_STREAM_END_EVENT;
            f(&event);
            y::yaml_event_delete(&mut event);
            if done {
                break Ok(());
            }
        };
        y::yaml_parser_delete(&mut parser);
        result
    }
}

/// # Safety
/// `p` is null or a NUL-terminated string owned by a live event.
unsafe fn cstr(p: *const u8) -> Option<String> {
    if p.is_null() {
        None
    } else {
        // SAFETY: guaranteed by the caller.
        Some(
            unsafe { std::ffi::CStr::from_ptr(p as *const std::ffi::c_char) }
                .to_string_lossy()
                .into_owned(),
        )
    }
}

pub fn libyaml_events(s: &str) -> Result<Vec<Ev>, String> {
    #[allow(clippy::unsafe_removed_from_name)] // a short alias for the C names
    use unsafe_libyaml as y;
    let mut out = Vec::new();
    libyaml_parse(s, |e| {
        // SAFETY: each union field is read only for its event type, and
        // the pointers stay valid until the event is deleted.
        let ev = unsafe {
            match e.type_ {
                y::YAML_STREAM_START_EVENT => Ev::StreamStart,
                y::YAML_STREAM_END_EVENT => Ev::StreamEnd,
                y::YAML_DOCUMENT_START_EVENT => Ev::DocStart(Some(!e.data.document_start.implicit)),
                y::YAML_DOCUMENT_END_EVENT => Ev::DocEnd(Some(!e.data.document_end.implicit)),
                y::YAML_ALIAS_EVENT => {
                    Ev::Alias(Anchor::Name(cstr(e.data.alias.anchor).unwrap_or_default()))
                }
                y::YAML_SCALAR_EVENT => {
                    let d = &e.data.scalar;
                    let bytes = std::slice::from_raw_parts(d.value, d.length as usize);
                    Ev::Scalar {
                        anchor: cstr(d.anchor).map_or(Anchor::None, Anchor::Name),
                        tag: cstr(d.tag),
                        style: match d.style {
                            y::YAML_SINGLE_QUOTED_SCALAR_STYLE => Style::Single,
                            y::YAML_DOUBLE_QUOTED_SCALAR_STYLE => Style::Double,
                            y::YAML_LITERAL_SCALAR_STYLE => Style::Literal,
                            y::YAML_FOLDED_SCALAR_STYLE => Style::Folded,
                            _ => Style::Plain,
                        },
                        value: String::from_utf8_lossy(bytes).into_owned(),
                    }
                }
                y::YAML_SEQUENCE_START_EVENT => {
                    let d = &e.data.sequence_start;
                    Ev::SeqStart {
                        anchor: cstr(d.anchor).map_or(Anchor::None, Anchor::Name),
                        tag: cstr(d.tag),
                        flow: Some(d.style == y::YAML_FLOW_SEQUENCE_STYLE),
                    }
                }
                y::YAML_SEQUENCE_END_EVENT => Ev::SeqEnd,
                y::YAML_MAPPING_START_EVENT => {
                    let d = &e.data.mapping_start;
                    Ev::MapStart {
                        anchor: cstr(d.anchor).map_or(Anchor::None, Anchor::Name),
                        tag: cstr(d.tag),
                        flow: Some(d.style == y::YAML_FLOW_MAPPING_STYLE),
                    }
                }
                y::YAML_MAPPING_END_EVENT => Ev::MapEnd,
                _ => return,
            }
        };
        out.push(ev);
    })?;
    Ok(out)
}

pub fn libyaml_count(s: &str) -> Result<usize, String> {
    let mut n = 0;
    libyaml_parse(s, |_| n += 1)?;
    Ok(n)
}

fn safer_parse(s: &str, mut f: impl FnMut(libyaml_safer::Event)) -> Result<(), String> {
    let mut bytes = s.as_bytes();
    let mut parser = libyaml_safer::Parser::new();
    parser.set_input_string(&mut bytes);
    loop {
        let ev = parser.parse().map_err(|e| e.to_string())?;
        let done = matches!(ev.data, libyaml_safer::EventData::StreamEnd);
        f(ev);
        if done {
            return Ok(());
        }
    }
}

pub fn safer_events(s: &str) -> Result<Vec<Ev>, String> {
    use libyaml_safer::{EventData as D, MappingStyle, ScalarStyle as S, SequenceStyle};
    let mut out = Vec::new();
    safer_parse(s, |e| {
        out.push(match e.data {
            D::StreamStart { .. } => Ev::StreamStart,
            D::StreamEnd => Ev::StreamEnd,
            D::DocumentStart { implicit, .. } => Ev::DocStart(Some(!implicit)),
            D::DocumentEnd { implicit } => Ev::DocEnd(Some(!implicit)),
            D::Alias { anchor } => Ev::Alias(Anchor::Name(anchor)),
            D::Scalar {
                anchor,
                tag,
                value,
                style,
                ..
            } => Ev::Scalar {
                anchor: anchor.map_or(Anchor::None, Anchor::Name),
                tag,
                style: match style {
                    S::SingleQuoted => Style::Single,
                    S::DoubleQuoted => Style::Double,
                    S::Literal => Style::Literal,
                    S::Folded => Style::Folded,
                    _ => Style::Plain,
                },
                value,
            },
            D::SequenceStart {
                anchor, tag, style, ..
            } => Ev::SeqStart {
                anchor: anchor.map_or(Anchor::None, Anchor::Name),
                tag,
                flow: Some(style == SequenceStyle::Flow),
            },
            D::SequenceEnd => Ev::SeqEnd,
            D::MappingStart {
                anchor, tag, style, ..
            } => Ev::MapStart {
                anchor: anchor.map_or(Anchor::None, Anchor::Name),
                tag,
                flow: Some(style == MappingStyle::Flow),
            },
            D::MappingEnd => Ev::MapEnd,
        });
    })?;
    Ok(out)
}

pub fn safer_count(s: &str) -> Result<usize, String> {
    let mut n = 0;
    safer_parse(s, |_| n += 1)?;
    Ok(n)
}
