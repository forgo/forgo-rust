//! Convert composed documents to JSON for comparison with `in.json`.
//!
//! The conversion reports what the library produced and never repairs it:
//! scalar variants map directly (tags are the composer's job), aliases
//! resolve to the most recent preceding anchor, and `Scalar::Num` text is
//! read with YAML 1.2 core-schema number rules.

use super::json::Json;
use forgo_lib_yaml::{Doc, Elem, MapKey, Node, Scalar};
use std::collections::HashMap;

pub fn docs_to_json(docs: &[Doc]) -> Result<Vec<Json>, String> {
    docs.iter()
        .map(|d| elem(d.root(), &mut HashMap::new()))
        .collect()
}

fn elem(e: &Elem, anchors: &mut HashMap<String, Json>) -> Result<Json, String> {
    let v = match &e.node {
        Node::Scalar(s) => scalar(s)?,
        Node::Seq(items) => Json::Arr(
            items
                .iter()
                .map(|i| elem(i, anchors))
                .collect::<Result<_, _>>()?,
        ),
        Node::Map(entries) => {
            let mut out: Vec<(String, Json)> = Vec::with_capacity(entries.len());
            for (k, v) in entries {
                let key = match k {
                    MapKey::Str(s) => s.clone(),
                    MapKey::Complex(ke) => key_text(&elem(ke, anchors)?)?,
                };
                let val = elem(v, anchors)?;
                // A JSON object can't hold duplicates; the last value wins.
                match out.iter_mut().find(|(kk, _)| *kk == key) {
                    Some(slot) => slot.1 = val,
                    None => out.push((key, val)),
                }
            }
            Json::Obj(out)
        }
        Node::Alias(name) => anchors
            .get(name)
            .cloned()
            .ok_or_else(|| format!("undefined alias *{name}"))?,
    };
    if let Some(a) = &e.meta.anchor {
        anchors.insert(a.clone(), v.clone());
    }
    Ok(v)
}

fn key_text(k: &Json) -> Result<String, String> {
    Ok(match k {
        Json::Str(s) => s.clone(),
        Json::Null => String::new(),
        Json::Bool(b) => b.to_string(),
        Json::Int(i) => i.to_string(),
        Json::Float(x) => x.to_string(),
        Json::Arr(_) | Json::Obj(_) => return Err("collection used as a mapping key".into()),
    })
}

fn scalar(s: &Scalar) -> Result<Json, String> {
    Ok(match s {
        Scalar::Null => Json::Null,
        Scalar::Bool(b) => Json::Bool(*b),
        Scalar::Str(s) => Json::Str(s.clone()),
        Scalar::Num { text } => number(text)?,
    })
}

/// YAML 1.2 core schema: `[-+]?[0-9]+`, `0o[0-7]+`, `0x[0-9a-fA-F]+`, and
/// `[-+]?(\.[0-9]+|[0-9]+(\.[0-9]*)?)([eE][-+]?[0-9]+)?`. Infinity and NaN
/// have no JSON form.
fn number(text: &str) -> Result<Json, String> {
    let bad = || format!("`{text}` is not a core-schema number");
    let radix = |digits: &str, r: u32| {
        if digits.is_empty() {
            return Err(bad());
        }
        i128::from_str_radix(digits, r)
            .map(Json::Int)
            .map_err(|_| bad())
    };
    if let Some(h) = text.strip_prefix("0x") {
        return radix(h, 16);
    }
    if let Some(o) = text.strip_prefix("0o") {
        return radix(o, 8);
    }
    let b = text.as_bytes();
    let mut i = usize::from(matches!(b.first(), Some(b'+' | b'-')));
    let int_start = i;
    while i < b.len() && b[i].is_ascii_digit() {
        i += 1;
    }
    let int_digits = i - int_start;
    if i == b.len() && int_digits > 0 {
        return text.parse::<i128>().map(Json::Int).map_err(|_| bad());
    }
    let mut frac_digits = 0;
    if i < b.len() && b[i] == b'.' {
        i += 1;
        let s = i;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
        }
        frac_digits = i - s;
    }
    if int_digits == 0 && frac_digits == 0 {
        return Err(bad());
    }
    if i < b.len() && matches!(b[i], b'e' | b'E') {
        i += 1;
        if i < b.len() && matches!(b[i], b'+' | b'-') {
            i += 1;
        }
        let s = i;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
        }
        if i == s {
            return Err(bad());
        }
    }
    if i != b.len() {
        return Err(bad());
    }
    text.parse::<f64>().map(Json::Float).map_err(|_| bad())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_schema_numbers() {
        assert_eq!(number("12").unwrap(), Json::Int(12));
        assert_eq!(number("-12").unwrap(), Json::Int(-12));
        assert_eq!(number("0x1F").unwrap(), Json::Int(31));
        assert_eq!(number("0o17").unwrap(), Json::Int(15));
        assert_eq!(number("1.5e3").unwrap(), Json::Float(1500.0));
        assert_eq!(number(".5").unwrap(), Json::Float(0.5));
        assert_eq!(number("1.").unwrap(), Json::Float(1.0));
        for bad in [".inf", ".nan", "1_000", "0b11", "1e", "+", "", "0x"] {
            assert!(number(bad).is_err(), "{bad}");
        }
    }
}
