//! Minimal zero-dependency JSON reader for the suite's `in.json` files.
//!
//! `in.json` holds one JSON value per YAML document, concatenated and
//! separated by whitespace; an empty file means an empty stream.

use std::fmt;

#[derive(Debug, Clone)]
pub enum Json {
    Null,
    Bool(bool),
    Int(i128),
    Float(f64),
    Str(String),
    Arr(Vec<Json>),
    /// Insertion-ordered; compared as an unordered map.
    Obj(Vec<(String, Json)>),
}

impl PartialEq for Json {
    fn eq(&self, other: &Self) -> bool {
        use Json::*;
        match (self, other) {
            (Null, Null) => true,
            (Bool(a), Bool(b)) => a == b,
            (Int(a), Int(b)) => a == b,
            (Float(a), Float(b)) => a == b,
            (Int(a), Float(b)) | (Float(b), Int(a)) => (*a as f64) == *b,
            (Str(a), Str(b)) => a == b,
            (Arr(a), Arr(b)) => a == b,
            (Obj(a), Obj(b)) => {
                a.len() == b.len()
                    && a.iter()
                        .all(|(k, v)| b.iter().any(|(k2, v2)| k == k2 && v == v2))
            }
            _ => false,
        }
    }
}

impl fmt::Display for Json {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Json::Null => f.write_str("null"),
            Json::Bool(b) => write!(f, "{b}"),
            Json::Int(i) => write!(f, "{i}"),
            Json::Float(x) => write!(f, "{x:?}"),
            Json::Str(s) => write_str(f, s),
            Json::Arr(items) => {
                f.write_str("[")?;
                for (i, it) in items.iter().enumerate() {
                    if i > 0 {
                        f.write_str(",")?;
                    }
                    write!(f, "{it}")?;
                }
                f.write_str("]")
            }
            Json::Obj(entries) => {
                f.write_str("{")?;
                for (i, (k, v)) in entries.iter().enumerate() {
                    if i > 0 {
                        f.write_str(",")?;
                    }
                    write_str(f, k)?;
                    write!(f, ":{v}")?;
                }
                f.write_str("}")
            }
        }
    }
}

fn write_str(f: &mut fmt::Formatter<'_>, s: &str) -> fmt::Result {
    f.write_str("\"")?;
    for c in s.chars() {
        match c {
            '"' => f.write_str("\\\"")?,
            '\\' => f.write_str("\\\\")?,
            '\n' => f.write_str("\\n")?,
            '\t' => f.write_str("\\t")?,
            '\r' => f.write_str("\\r")?,
            c if (c as u32) < 0x20 => write!(f, "\\u{:04x}", c as u32)?,
            c => write!(f, "{c}")?,
        }
    }
    f.write_str("\"")
}

/// Parse every whitespace-separated JSON value in `text`.
pub fn parse_many(text: &str) -> Result<Vec<Json>, String> {
    let mut p = Reader {
        s: text.as_bytes(),
        i: 0,
    };
    let mut out = Vec::new();
    loop {
        p.skip_ws();
        if p.i >= p.s.len() {
            return Ok(out);
        }
        out.push(p.value()?);
    }
}

struct Reader<'a> {
    s: &'a [u8],
    i: usize,
}

impl Reader<'_> {
    fn err<T>(&self, msg: &str) -> Result<T, String> {
        Err(format!("JSON error at byte {}: {msg}", self.i))
    }

    fn skip_ws(&mut self) {
        while self.i < self.s.len() && matches!(self.s[self.i], b' ' | b'\t' | b'\n' | b'\r') {
            self.i += 1;
        }
    }

    fn eat(&mut self, lit: &str) -> Result<(), String> {
        if self.s[self.i..].starts_with(lit.as_bytes()) {
            self.i += lit.len();
            Ok(())
        } else {
            self.err(&format!("expected `{lit}`"))
        }
    }

    fn value(&mut self) -> Result<Json, String> {
        self.skip_ws();
        match self.s.get(self.i) {
            None => self.err("unexpected end of input"),
            Some(b'n') => self.eat("null").map(|_| Json::Null),
            Some(b't') => self.eat("true").map(|_| Json::Bool(true)),
            Some(b'f') => self.eat("false").map(|_| Json::Bool(false)),
            Some(b'"') => self.string().map(Json::Str),
            Some(b'[') => {
                self.i += 1;
                let mut items = Vec::new();
                self.skip_ws();
                if self.s.get(self.i) == Some(&b']') {
                    self.i += 1;
                    return Ok(Json::Arr(items));
                }
                loop {
                    items.push(self.value()?);
                    self.skip_ws();
                    match self.s.get(self.i) {
                        Some(b',') => self.i += 1,
                        Some(b']') => {
                            self.i += 1;
                            return Ok(Json::Arr(items));
                        }
                        _ => return self.err("expected `,` or `]`"),
                    }
                }
            }
            Some(b'{') => {
                self.i += 1;
                let mut entries = Vec::new();
                self.skip_ws();
                if self.s.get(self.i) == Some(&b'}') {
                    self.i += 1;
                    return Ok(Json::Obj(entries));
                }
                loop {
                    self.skip_ws();
                    if self.s.get(self.i) != Some(&b'"') {
                        return self.err("expected object key");
                    }
                    let k = self.string()?;
                    self.skip_ws();
                    self.eat(":")?;
                    let v = self.value()?;
                    entries.push((k, v));
                    self.skip_ws();
                    match self.s.get(self.i) {
                        Some(b',') => self.i += 1,
                        Some(b'}') => {
                            self.i += 1;
                            return Ok(Json::Obj(entries));
                        }
                        _ => return self.err("expected `,` or `}`"),
                    }
                }
            }
            Some(b'-' | b'0'..=b'9') => self.number(),
            Some(_) => self.err("unexpected character"),
        }
    }

    fn number(&mut self) -> Result<Json, String> {
        let start = self.i;
        let digits = |r: &mut Self| {
            let s = r.i;
            while r.i < r.s.len() && r.s[r.i].is_ascii_digit() {
                r.i += 1;
            }
            r.i > s
        };
        if self.s.get(self.i) == Some(&b'-') {
            self.i += 1;
        }
        if !digits(self) {
            return self.err("expected digits");
        }
        let mut float = false;
        if self.s.get(self.i) == Some(&b'.') {
            self.i += 1;
            float = true;
            if !digits(self) {
                return self.err("expected fraction digits");
            }
        }
        if matches!(self.s.get(self.i), Some(b'e' | b'E')) {
            self.i += 1;
            float = true;
            if matches!(self.s.get(self.i), Some(b'+' | b'-')) {
                self.i += 1;
            }
            if !digits(self) {
                return self.err("expected exponent digits");
            }
        }
        let text = std::str::from_utf8(&self.s[start..self.i]).unwrap();
        if !float && let Ok(i) = text.parse::<i128>() {
            return Ok(Json::Int(i));
        }
        text.parse::<f64>()
            .map(Json::Float)
            .map_err(|e| format!("bad number {text}: {e}"))
    }

    fn hex4(&mut self) -> Result<u32, String> {
        let Some(h) = self.s.get(self.i..self.i + 4) else {
            return self.err("truncated \\u escape");
        };
        let h = std::str::from_utf8(h).map_err(|e| e.to_string())?;
        let v = u32::from_str_radix(h, 16).map_err(|e| format!("bad \\u escape {h}: {e}"))?;
        self.i += 4;
        Ok(v)
    }

    fn string(&mut self) -> Result<String, String> {
        self.i += 1; // opening quote
        let mut out = String::new();
        loop {
            let start = self.i;
            while self.i < self.s.len() && !matches!(self.s[self.i], b'"' | b'\\') {
                self.i += 1;
            }
            out.push_str(std::str::from_utf8(&self.s[start..self.i]).map_err(|e| e.to_string())?);
            match self.s.get(self.i) {
                None => return self.err("unterminated string"),
                Some(b'"') => {
                    self.i += 1;
                    return Ok(out);
                }
                _ => {
                    self.i += 1; // backslash
                    let Some(&c) = self.s.get(self.i) else {
                        return self.err("truncated escape");
                    };
                    self.i += 1;
                    match c {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'b' => out.push('\u{8}'),
                        b'f' => out.push('\u{c}'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'u' => {
                            let hi = self.hex4()?;
                            let cp = if (0xD800..0xDC00).contains(&hi) {
                                self.eat("\\u")?;
                                let lo = self.hex4()?;
                                if !(0xDC00..0xE000).contains(&lo) {
                                    return self.err("unpaired surrogate");
                                }
                                0x10000 + ((hi - 0xD800) << 10) + (lo - 0xDC00)
                            } else {
                                hi
                            };
                            match char::from_u32(cp) {
                                Some(ch) => out.push(ch),
                                None => return self.err("invalid code point"),
                            }
                        }
                        _ => return self.err("unknown escape"),
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_concatenated_values() {
        let v = parse_many("{\"a\": [1, 2.5, \"x\\u263A\"]}\n null \"\\ud83d\\ude00\"").unwrap();
        assert_eq!(v.len(), 3);
        assert_eq!(
            v[0],
            Json::Obj(vec![(
                "a".into(),
                Json::Arr(vec![
                    Json::Int(1),
                    Json::Float(2.5),
                    Json::Str("x\u{263A}".into())
                ])
            )])
        );
        assert_eq!(v[1], Json::Null);
        assert_eq!(v[2], Json::Str("\u{1F600}".into()));
        assert!(parse_many("").unwrap().is_empty());
    }

    #[test]
    fn numbers_compare_numerically_and_objects_unordered() {
        assert_eq!(Json::Int(12), Json::Float(12.0));
        let a = parse_many("{\"x\":1,\"y\":2}").unwrap();
        let b = parse_many("{\"y\":2.0,\"x\":1}").unwrap();
        assert_eq!(a, b);
    }
}
