//! Style-independent event streams, for checking that emitting a document
//! and parsing it again preserves its content.
//!
//! Scalar and collection styles and document markers are dropped. Each
//! scalar is reduced to its effective type (explicit tag, or the YAML 1.2
//! core schema for untagged plain scalars, or string) and a canonical
//! value, so `1` → `"1"` (int → string) is a difference while `'a'` →
//! `"a"` is not. This deliberately re-implements resolution instead of
//! using the library's, as an independent cross-check.

use forgo_lib_yaml::{Event, ScalarStyle, Tag};

const CORE: &str = "tag:yaml.org,2002:";

fn is_null(s: &str) -> bool {
    matches!(s, "" | "~" | "null" | "Null" | "NULL")
}

fn bool_value(s: &str) -> Option<bool> {
    match s {
        "true" | "True" | "TRUE" => Some(true),
        "false" | "False" | "FALSE" => Some(false),
        _ => None,
    }
}

fn int_value(s: &str) -> Option<i128> {
    let dec = |t: &str| !t.is_empty() && t.bytes().all(|b| b.is_ascii_digit());
    if let Some(h) = s.strip_prefix("0x") {
        return i128::from_str_radix(h, 16).ok().filter(|_| !h.is_empty());
    }
    if let Some(o) = s.strip_prefix("0o") {
        return i128::from_str_radix(o, 8).ok().filter(|_| !o.is_empty());
    }
    let body = s.strip_prefix(['+', '-']).unwrap_or(s);
    if dec(body) { s.parse().ok() } else { None }
}

fn float_value(s: &str) -> Option<f64> {
    let body = s.strip_prefix(['+', '-']).unwrap_or(s);
    match body {
        ".inf" | ".Inf" | ".INF" => Some(if s.starts_with('-') {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        }),
        ".nan" | ".NaN" | ".NAN" if s == body => Some(f64::NAN),
        _ => {
            let (m, e) = match body.find(['e', 'E']) {
                Some(i) => (&body[..i], Some(&body[i + 1..])),
                None => (body, None),
            };
            let digits = |t: &str| !t.is_empty() && t.bytes().all(|b| b.is_ascii_digit());
            let m_ok = match m.split_once('.') {
                Some((a, b)) => {
                    (a.is_empty() && digits(b)) || (digits(a) && (b.is_empty() || digits(b)))
                }
                None => digits(m),
            };
            let e_ok = e.is_none_or(|e| digits(e.strip_prefix(['+', '-']).unwrap_or(e)));
            if m_ok && e_ok { s.parse().ok() } else { None }
        }
    }
}

fn canonical_float(f: f64) -> String {
    if f.is_nan() {
        "nan".into()
    } else {
        format!("{f:?}")
    }
}

/// The effective type and canonical value of a scalar.
fn scalar(value: &str, style: ScalarStyle, tag: Option<&str>) -> (String, String) {
    let kind = match tag {
        Some("!") => "str",
        Some(t) => match t.strip_prefix(CORE) {
            Some(k @ ("str" | "null" | "bool" | "int" | "float")) => k,
            _ => return (t.to_string(), value.to_string()),
        },
        None if style != ScalarStyle::Plain => "str",
        None if is_null(value) => "null",
        None if bool_value(value).is_some() => "bool",
        None if int_value(value).is_some() => "int",
        None if float_value(value).is_some() => "float",
        None => "str",
    };
    let canon = match kind {
        "null" => "~".to_string(),
        "bool" => bool_value(value).map_or_else(|| value.to_string(), |b| b.to_string()),
        "int" => int_value(value).map_or_else(|| value.to_string(), |i| i.to_string()),
        "float" => float_value(value)
            .or_else(|| int_value(value).map(|i| i as f64))
            .map_or_else(|| value.to_string(), canonical_float),
        _ => value.to_string(),
    };
    (kind.to_string(), canon)
}

fn anchor(a: &Option<std::borrow::Cow<'_, str>>) -> String {
    a.as_ref().map(|a| format!(" &{a}")).unwrap_or_default()
}

fn collection_tag(tag: &Option<Tag<'_>>, kind: &str) -> String {
    match tag.as_ref().map(|t| t.resolved.as_ref()) {
        None | Some("!") => String::new(),
        Some(t) if t.strip_prefix(CORE) == Some(kind) => String::new(),
        Some(t) => format!(" <{t}>"),
    }
}

/// The normalized event lines of `text`.
pub fn normalized_events(text: &str) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    for r in forgo_lib_yaml::events(text) {
        let (ev, _) = r.map_err(|e| e.to_string())?;
        out.push(match ev {
            Event::StreamStart => "+STR".into(),
            Event::StreamEnd => "-STR".into(),
            Event::DocumentStart { .. } => "+DOC".into(),
            Event::DocumentEnd { .. } => "-DOC".into(),
            Event::MappingStart { anchor: a, tag, .. } => {
                format!("+MAP{}{}", anchor(&a), collection_tag(&tag, "map"))
            }
            Event::SequenceStart { anchor: a, tag, .. } => {
                format!("+SEQ{}{}", anchor(&a), collection_tag(&tag, "seq"))
            }
            Event::MappingEnd => "-MAP".into(),
            Event::SequenceEnd => "-SEQ".into(),
            Event::Alias { name } => format!("=ALI *{name}"),
            Event::Scalar {
                anchor: a,
                tag,
                value,
                style,
                ..
            } => {
                let (t, v) = scalar(&value, style, tag.as_ref().map(|t| t.resolved.as_ref()));
                format!("=VAL{} <{t}> {v:?}", anchor(&a))
            }
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn style_is_ignored_but_type_is_not() {
        let a = normalized_events("- 'x'\n- 1\n- ~\n- [a]\n").unwrap();
        let b = normalized_events("- \"x\"\n- 0x1\n- null\n-\n  - a\n").unwrap();
        assert_eq!(a, b);
        let c = normalized_events("- \"1\"\n").unwrap();
        let d = normalized_events("- 1\n").unwrap();
        assert_ne!(c, d);
    }
}
