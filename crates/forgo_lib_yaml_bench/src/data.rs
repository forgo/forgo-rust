//! A library-neutral data model, so values loaded by different libraries
//! can be compared with each other and with the suite's `in.json`.

use std::fmt;

/// A loaded YAML value. Libraries convert their own trees into this.
#[derive(Clone, Debug)]
pub enum Data {
    Null,
    Bool(bool),
    Int(i128),
    Float(f64),
    Str(String),
    Seq(Vec<Data>),
    /// Entries in source order; compared as an unordered multiset.
    Map(Vec<(Data, Data)>),
    /// A node with a tag the library kept (`!local`, `!!set`, ...).
    Tagged(String, Box<Data>),
}

impl Data {
    /// Strict equality for round trips: `1` and `1.0` differ, NaN equals
    /// NaN, mappings are unordered.
    pub fn same(&self, other: &Data) -> bool {
        use Data::*;
        match (self, other) {
            (Null, Null) => true,
            (Bool(a), Bool(b)) => a == b,
            (Int(a), Int(b)) => a == b,
            (Float(a), Float(b)) => a == b || (a.is_nan() && b.is_nan()),
            (Str(a), Str(b)) => a == b,
            (Seq(a), Seq(b)) => a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.same(y)),
            (Map(a), Map(b)) => same_entries(a, b, Data::same),
            (Tagged(t, a), Tagged(u, b)) => t == u && a.same(b),
            _ => false,
        }
    }

    /// Whether any mapping in the value holds two equal keys.
    pub fn has_duplicate_keys(&self) -> bool {
        match self {
            Data::Seq(items) => items.iter().any(Data::has_duplicate_keys),
            Data::Map(entries) => {
                entries
                    .iter()
                    .enumerate()
                    .any(|(i, (k, _))| entries[i + 1..].iter().any(|(k2, _)| k.same(k2)))
                    || entries
                        .iter()
                        .any(|(k, v)| k.has_duplicate_keys() || v.has_duplicate_keys())
            }
            Data::Tagged(_, inner) => inner.has_duplicate_keys(),
            _ => false,
        }
    }

    /// The JSON view of the value, as the suite's `in.json` encodes it:
    /// tags dropped, keys as strings, the last duplicate key wins.
    pub fn to_json(&self) -> Result<Data, String> {
        Ok(match self {
            Data::Tagged(_, inner) => inner.to_json()?,
            Data::Seq(items) => {
                Data::Seq(items.iter().map(Data::to_json).collect::<Result<_, _>>()?)
            }
            Data::Map(entries) => {
                let mut out: Vec<(Data, Data)> = Vec::with_capacity(entries.len());
                for (k, v) in entries {
                    let key = Data::Str(key_text(k)?);
                    let val = v.to_json()?;
                    match out.iter_mut().find(|(kk, _)| kk.same(&key)) {
                        Some(slot) => slot.1 = val,
                        None => out.push((key, val)),
                    }
                }
                Data::Map(out)
            }
            other => other.clone(),
        })
    }
}

fn same_entries(a: &[(Data, Data)], b: &[(Data, Data)], eq: fn(&Data, &Data) -> bool) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut used = vec![false; b.len()];
    a.iter().all(|(ka, va)| {
        let hit = b
            .iter()
            .enumerate()
            .find(|(i, (kb, vb))| !used[*i] && eq(ka, kb) && eq(va, vb));
        match hit {
            Some((i, _)) => {
                used[i] = true;
                true
            }
            None => false,
        }
    })
}

/// JSON equality: numbers compare numerically (`1` equals `1.0`).
pub fn json_eq(a: &Data, b: &Data) -> bool {
    use Data::*;
    match (a, b) {
        (Int(x), Float(y)) | (Float(y), Int(x)) => (*x as f64) == *y,
        (Seq(x), Seq(y)) => x.len() == y.len() && x.iter().zip(y).all(|(p, q)| json_eq(p, q)),
        (Map(x), Map(y)) => same_entries(x, y, json_eq),
        _ => a.same(b),
    }
}

/// How `in.json` spells a non-string key.
fn key_text(k: &Data) -> Result<String, String> {
    Ok(match k {
        Data::Str(s) => s.clone(),
        Data::Null => String::new(),
        Data::Bool(b) => b.to_string(),
        Data::Int(i) => i.to_string(),
        Data::Float(x) => x.to_string(),
        Data::Tagged(_, inner) => key_text(inner)?,
        Data::Seq(_) | Data::Map(_) => return Err("collection used as a mapping key".into()),
    })
}

impl fmt::Display for Data {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Data::Null => f.write_str("null"),
            Data::Bool(b) => write!(f, "{b}"),
            Data::Int(i) => write!(f, "{i}"),
            Data::Float(x) => write!(f, "{x:?}"),
            Data::Str(s) => write!(f, "{s:?}"),
            Data::Seq(items) => {
                f.write_str("[")?;
                for (i, it) in items.iter().enumerate() {
                    if i > 0 {
                        f.write_str(",")?;
                    }
                    write!(f, "{it}")?;
                }
                f.write_str("]")
            }
            Data::Map(entries) => {
                f.write_str("{")?;
                for (i, (k, v)) in entries.iter().enumerate() {
                    if i > 0 {
                        f.write_str(",")?;
                    }
                    write!(f, "{k}:{v}")?;
                }
                f.write_str("}")
            }
            Data::Tagged(t, inner) => write!(f, "{t} {inner}"),
        }
    }
}

// ---------------------------------------------------------------------------
// YAML 1.2 core schema, for libraries that hand back unresolved text
// ---------------------------------------------------------------------------

pub fn is_core_null(s: &str) -> bool {
    matches!(s, "" | "~" | "null" | "Null" | "NULL")
}

pub fn core_bool(s: &str) -> Option<bool> {
    match s {
        "true" | "True" | "TRUE" => Some(true),
        "false" | "False" | "FALSE" => Some(false),
        _ => None,
    }
}

pub fn core_int(s: &str) -> Option<i128> {
    if let Some(h) = s.strip_prefix("0x") {
        return (!h.is_empty())
            .then(|| i128::from_str_radix(h, 16).ok())
            .flatten();
    }
    if let Some(o) = s.strip_prefix("0o") {
        return (!o.is_empty())
            .then(|| i128::from_str_radix(o, 8).ok())
            .flatten();
    }
    let body = s.strip_prefix(['+', '-']).unwrap_or(s);
    if !body.is_empty() && body.bytes().all(|b| b.is_ascii_digit()) {
        s.parse().ok()
    } else {
        None
    }
}

pub fn core_float(s: &str) -> Option<f64> {
    let body = s.strip_prefix(['+', '-']).unwrap_or(s);
    match body {
        ".inf" | ".Inf" | ".INF" => Some(if s.starts_with('-') {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        }),
        ".nan" | ".NaN" | ".NAN" if s == body => Some(f64::NAN),
        _ => {
            let digits = |t: &str| !t.is_empty() && t.bytes().all(|b| b.is_ascii_digit());
            let (m, e) = match body.find(['e', 'E']) {
                Some(i) => (&body[..i], Some(&body[i + 1..])),
                None => (body, None),
            };
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

/// Resolve an untagged plain scalar with the core schema.
pub fn resolve_plain(s: &str) -> Data {
    if is_core_null(s) {
        Data::Null
    } else if let Some(b) = core_bool(s) {
        Data::Bool(b)
    } else if let Some(i) = core_int(s) {
        Data::Int(i)
    } else if let Some(f) = core_float(s) {
        Data::Float(f)
    } else {
        Data::Str(s.to_string())
    }
}

/// Parse a number the way a library printed it (`1.5`, `-.inf`, `.nan`).
pub fn float_text(s: &str) -> Data {
    match core_float(s).or_else(|| s.parse::<f64>().ok()) {
        Some(f) => Data::Float(f),
        None => Data::Str(s.to_string()),
    }
}

// ---------------------------------------------------------------------------
// JSON reader for `in.json`: concatenated values, empty file = no documents
// ---------------------------------------------------------------------------

pub fn parse_json_many(text: &str) -> Result<Vec<Data>, String> {
    let mut p = Json {
        s: text.as_bytes(),
        i: 0,
    };
    let mut out = Vec::new();
    loop {
        p.ws();
        if p.i >= p.s.len() {
            return Ok(out);
        }
        out.push(p.value()?);
    }
}

struct Json<'a> {
    s: &'a [u8],
    i: usize,
}

impl Json<'_> {
    fn err<T>(&self, msg: &str) -> Result<T, String> {
        Err(format!("json: {msg} at byte {}", self.i))
    }

    fn ws(&mut self) {
        while self.i < self.s.len() && matches!(self.s[self.i], b' ' | b'\t' | b'\n' | b'\r') {
            self.i += 1;
        }
    }

    fn lit(&mut self, word: &str, v: Data) -> Result<Data, String> {
        if self.s[self.i..].starts_with(word.as_bytes()) {
            self.i += word.len();
            Ok(v)
        } else {
            self.err("bad literal")
        }
    }

    fn value(&mut self) -> Result<Data, String> {
        self.ws();
        match self.s.get(self.i) {
            None => self.err("unexpected end"),
            Some(b'n') => self.lit("null", Data::Null),
            Some(b't') => self.lit("true", Data::Bool(true)),
            Some(b'f') => self.lit("false", Data::Bool(false)),
            Some(b'"') => Ok(Data::Str(self.string()?)),
            Some(b'[') => {
                self.i += 1;
                let mut items = Vec::new();
                self.ws();
                if self.s.get(self.i) == Some(&b']') {
                    self.i += 1;
                    return Ok(Data::Seq(items));
                }
                loop {
                    items.push(self.value()?);
                    self.ws();
                    match self.s.get(self.i) {
                        Some(b',') => self.i += 1,
                        Some(b']') => {
                            self.i += 1;
                            return Ok(Data::Seq(items));
                        }
                        _ => return self.err("expected , or ]"),
                    }
                }
            }
            Some(b'{') => {
                self.i += 1;
                let mut entries = Vec::new();
                self.ws();
                if self.s.get(self.i) == Some(&b'}') {
                    self.i += 1;
                    return Ok(Data::Map(entries));
                }
                loop {
                    self.ws();
                    let k = self.string()?;
                    self.ws();
                    if self.s.get(self.i) != Some(&b':') {
                        return self.err("expected :");
                    }
                    self.i += 1;
                    let v = self.value()?;
                    entries.push((Data::Str(k), v));
                    self.ws();
                    match self.s.get(self.i) {
                        Some(b',') => self.i += 1,
                        Some(b'}') => {
                            self.i += 1;
                            return Ok(Data::Map(entries));
                        }
                        _ => return self.err("expected , or }"),
                    }
                }
            }
            Some(_) => self.number(),
        }
    }

    fn number(&mut self) -> Result<Data, String> {
        let start = self.i;
        while self.i < self.s.len()
            && matches!(
                self.s[self.i],
                b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9'
            )
        {
            self.i += 1;
        }
        let t = std::str::from_utf8(&self.s[start..self.i]).unwrap_or("");
        if let Ok(i) = t.parse::<i128>() {
            return Ok(Data::Int(i));
        }
        t.parse::<f64>()
            .map(Data::Float)
            .or_else(|_| self.err("bad number"))
    }

    fn hex4(&mut self) -> Result<u32, String> {
        let h = self
            .s
            .get(self.i..self.i + 4)
            .ok_or("json: short \\u escape")?;
        self.i += 4;
        u32::from_str_radix(std::str::from_utf8(h).map_err(|e| e.to_string())?, 16)
            .map_err(|e| e.to_string())
    }

    fn string(&mut self) -> Result<String, String> {
        if self.s.get(self.i) != Some(&b'"') {
            return self.err("expected string");
        }
        self.i += 1;
        let mut out = Vec::new();
        loop {
            let Some(&c) = self.s.get(self.i) else {
                return self.err("unterminated string");
            };
            self.i += 1;
            match c {
                b'"' => return String::from_utf8(out).map_err(|e| e.to_string()),
                b'\\' => {
                    let Some(&e) = self.s.get(self.i) else {
                        return self.err("bad escape");
                    };
                    self.i += 1;
                    let ch = match e {
                        b'"' => '"',
                        b'\\' => '\\',
                        b'/' => '/',
                        b'b' => '\u{8}',
                        b'f' => '\u{c}',
                        b'n' => '\n',
                        b'r' => '\r',
                        b't' => '\t',
                        b'u' => {
                            let hi = self.hex4()?;
                            let cp = if (0xD800..0xDC00).contains(&hi)
                                && self.s[self.i..].starts_with(b"\\u")
                            {
                                self.i += 2;
                                let lo = self.hex4()?;
                                0x10000 + ((hi - 0xD800) << 10) + (lo.wrapping_sub(0xDC00) & 0x3FF)
                            } else {
                                hi
                            };
                            char::from_u32(cp).unwrap_or('\u{FFFD}')
                        }
                        _ => return self.err("bad escape"),
                    };
                    let mut buf = [0; 4];
                    out.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
                }
                c => out.push(c),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_reader_and_equality() {
        let v = parse_json_many("{\"a\": [1, 2.5, \"\\u00e9\\ud83d\\ude00\"]}\n[]\nnull").unwrap();
        assert_eq!(v.len(), 3);
        let expected = Data::Map(vec![(
            Data::Str("a".into()),
            Data::Seq(vec![
                Data::Float(1.0),
                Data::Float(2.5),
                Data::Str("é😀".into()),
            ]),
        )]);
        assert!(json_eq(&v[0], &expected));
        assert!(
            !v[0].same(&expected),
            "strict equality keeps int and float apart"
        );
        assert!(parse_json_many("").unwrap().is_empty());
    }

    #[test]
    fn json_view_drops_tags_and_stringifies_keys() {
        let d = Data::Map(vec![
            (
                Data::Int(1),
                Data::Tagged("!x".into(), Box::new(Data::Str("a".into()))),
            ),
            (Data::Null, Data::Bool(true)),
            (Data::Int(1), Data::Str("b".into())),
        ]);
        let j = d.to_json().unwrap();
        let expected = Data::Map(vec![
            (Data::Str("1".into()), Data::Str("b".into())),
            (Data::Str(String::new()), Data::Bool(true)),
        ]);
        assert!(json_eq(&j, &expected));
    }

    #[test]
    fn core_schema() {
        assert!(matches!(resolve_plain("0x1F"), Data::Int(31)));
        assert!(matches!(resolve_plain("~"), Data::Null));
        assert!(matches!(resolve_plain("1."), Data::Float(f) if f == 1.0));
        assert!(matches!(resolve_plain("1_000"), Data::Str(_)));
    }
}
