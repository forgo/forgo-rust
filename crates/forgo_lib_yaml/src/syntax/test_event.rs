//! The yaml-test-suite event notation (`test.event`), as printed by
//! libyaml's `run-parser-test-suite`.

use super::event::{Event, ScalarStyle, Tag};
use std::fmt::Write as _;

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

fn props(out: &mut String, anchor: &Option<std::borrow::Cow<'_, str>>, tag: &Option<Tag<'_>>) {
    if let Some(a) = anchor {
        let _ = write!(out, " &{a}");
    }
    if let Some(t) = tag {
        let _ = write!(out, " <{}>", t.resolved);
    }
}

/// One event as a `test.event` line, e.g. `=VAL &a <tag:yaml.org,2002:str> :x`.
pub fn to_test_event(ev: &Event<'_>) -> String {
    let mut out = String::new();
    match ev {
        Event::StreamStart => out.push_str("+STR"),
        Event::StreamEnd => out.push_str("-STR"),
        Event::DocumentStart { explicit, .. } => {
            out.push_str(if *explicit { "+DOC ---" } else { "+DOC" })
        }
        Event::DocumentEnd { explicit } => {
            out.push_str(if *explicit { "-DOC ..." } else { "-DOC" })
        }
        Event::MappingStart { anchor, tag, flow } => {
            out.push_str("+MAP");
            if *flow {
                out.push_str(" {}");
            }
            props(&mut out, anchor, tag);
        }
        Event::MappingEnd => out.push_str("-MAP"),
        Event::SequenceStart { anchor, tag, flow } => {
            out.push_str("+SEQ");
            if *flow {
                out.push_str(" []");
            }
            props(&mut out, anchor, tag);
        }
        Event::SequenceEnd => out.push_str("-SEQ"),
        Event::Scalar {
            anchor,
            tag,
            value,
            style,
        } => {
            out.push_str("=VAL");
            props(&mut out, anchor, tag);
            out.push(' ');
            out.push(match style {
                ScalarStyle::Plain => ':',
                ScalarStyle::SingleQuoted => '\'',
                ScalarStyle::DoubleQuoted => '"',
                ScalarStyle::Literal => '|',
                ScalarStyle::Folded => '>',
            });
            escape(&mut out, value);
        }
        Event::Alias { name } => {
            let _ = write!(out, "=ALI *{name}");
        }
    }
    out
}
