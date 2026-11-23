//! YAML 1.2 core schema (spec §10.3): how a scalar's tag and text resolve
//! to a typed [`Scalar`].
//!
//! - Untagged plain scalars resolve by content: null, bool, int, float, or
//!   string. Quoted and block scalars are strings.
//! - The non-specific tag `!` forces a string.
//! - `!!str`, `!!null`, `!!bool`, `!!int` and `!!float` force their type
//!   when the text is a valid form of it; otherwise the text stays a string
//!   (with the tag kept on the node).
//! - Any other tag is application-specific: the value stays a string.

use crate::ast::Scalar;
use crate::syntax::ScalarStyle;

const CORE: &str = "tag:yaml.org,2002:";

pub(crate) fn is_null(s: &str) -> bool {
    matches!(s, "" | "~" | "null" | "Null" | "NULL")
}

pub(crate) fn parse_bool(s: &str) -> Option<bool> {
    match s {
        "true" | "True" | "TRUE" => Some(true),
        "false" | "False" | "FALSE" => Some(false),
        _ => None,
    }
}

fn digits(s: &str, radix: u32) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_digit(radix))
}

/// `[-+]?[0-9]+ | 0o[0-7]+ | 0x[0-9a-fA-F]+`
pub(crate) fn is_int(s: &str) -> bool {
    if let Some(h) = s.strip_prefix("0x") {
        return digits(h, 16);
    }
    if let Some(o) = s.strip_prefix("0o") {
        return digits(o, 8);
    }
    digits(s.strip_prefix(['-', '+']).unwrap_or(s), 10)
}

/// `[-+]? ( \. [0-9]+ | [0-9]+ ( \. [0-9]* )? ) ( [eE] [-+]? [0-9]+ )?`,
/// `[-+]? \. (inf|Inf|INF)` and `\. (nan|NaN|NAN)`.
pub(crate) fn is_float(s: &str) -> bool {
    if matches!(s, ".nan" | ".NaN" | ".NAN") {
        return true;
    }
    let body = s.strip_prefix(['-', '+']).unwrap_or(s);
    if matches!(body, ".inf" | ".Inf" | ".INF") {
        return true;
    }
    let (mantissa, exp) = match body.find(['e', 'E']) {
        Some(i) => (&body[..i], Some(&body[i + 1..])),
        None => (body, None),
    };
    let ok_mantissa = match mantissa.split_once('.') {
        Some((int, frac)) => {
            (int.is_empty() && digits(frac, 10))
                || (digits(int, 10) && (frac.is_empty() || digits(frac, 10)))
        }
        None => digits(mantissa, 10),
    };
    let ok_exp = exp.is_none_or(|e| digits(e.strip_prefix(['-', '+']).unwrap_or(e), 10));
    ok_mantissa && ok_exp
}

/// Parse core-schema integer text.
pub(crate) fn int_value(s: &str) -> Option<i128> {
    if let Some(h) = s.strip_prefix("0x") {
        return i128::from_str_radix(h, 16).ok();
    }
    if let Some(o) = s.strip_prefix("0o") {
        return i128::from_str_radix(o, 8).ok();
    }
    s.parse().ok()
}

/// Parse core-schema float (or integer) text.
pub(crate) fn float_value(s: &str) -> Option<f64> {
    let body = s.strip_prefix(['-', '+']).unwrap_or(s);
    let neg = s.starts_with('-');
    match body {
        ".inf" | ".Inf" | ".INF" => Some(if neg {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        }),
        ".nan" | ".NaN" | ".NAN" if s == body => Some(f64::NAN),
        _ if is_int(s) => int_value(s).map(|i| i as f64),
        _ if is_float(s) => s.parse().ok(),
        _ => None,
    }
}

/// Resolve a scalar's text using its style and resolved tag.
pub(crate) fn resolve(text: String, style: ScalarStyle, tag: Option<&str>) -> Scalar {
    let core = tag.and_then(|t| t.strip_prefix(CORE));
    match (tag, core) {
        (None, _) if style == ScalarStyle::Plain => resolve_plain(text),
        (None, _) | (Some("!"), _) => Scalar::Str(text),
        (_, Some("str")) => Scalar::Str(text),
        (_, Some("null")) if is_null(&text) => Scalar::Null,
        (_, Some("bool")) => match parse_bool(&text) {
            Some(b) => Scalar::Bool(b),
            None => Scalar::Str(text),
        },
        (_, Some("int")) if is_int(&text) => Scalar::Num { text },
        (_, Some("float")) if is_float(&text) || is_int(&text) => Scalar::Num { text },
        _ => Scalar::Str(text),
    }
}

fn resolve_plain(text: String) -> Scalar {
    if is_null(&text) {
        Scalar::Null
    } else if let Some(b) = parse_bool(&text) {
        Scalar::Bool(b)
    } else if is_int(&text) || is_float(&text) {
        Scalar::Num { text }
    } else {
        Scalar::Str(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain(s: &str) -> Scalar {
        resolve(s.to_string(), ScalarStyle::Plain, None)
    }

    #[test]
    fn core_schema_plain_resolution() {
        for s in ["", "~", "null", "Null", "NULL"] {
            assert_eq!(plain(s), Scalar::Null, "{s:?}");
        }
        assert_eq!(plain("True"), Scalar::Bool(true));
        assert_eq!(plain("FALSE"), Scalar::Bool(false));
        for s in [
            "0", "-12", "+7", "0o17", "0x1F", "1.5", ".5", "1.", "1e3", "-1.2E-3", ".inf", "-.Inf",
            ".NaN",
        ] {
            assert!(matches!(plain(s), Scalar::Num { .. }), "{s:?}");
        }
        for s in [
            "nULL", "yes", "on", "1_000", "0b11", "0x", "1e", "+.nan", "1.2.3", ".",
        ] {
            assert!(matches!(plain(s), Scalar::Str(_)), "{s:?}");
        }
    }

    #[test]
    fn tags_and_styles_override() {
        let q = resolve("12".into(), ScalarStyle::DoubleQuoted, None);
        assert_eq!(q, Scalar::Str("12".into()));
        let ns = resolve("12".into(), ScalarStyle::Plain, Some("!"));
        assert_eq!(ns, Scalar::Str("12".into()));
        let int = resolve(
            "12".into(),
            ScalarStyle::DoubleQuoted,
            Some("tag:yaml.org,2002:int"),
        );
        assert!(matches!(int, Scalar::Num { .. }));
        let custom = resolve(
            "12".into(),
            ScalarStyle::Plain,
            Some("tag:example.com,2000:app/int"),
        );
        assert_eq!(custom, Scalar::Str("12".into()));
        let bad = resolve(
            "x".into(),
            ScalarStyle::Plain,
            Some("tag:yaml.org,2002:int"),
        );
        assert_eq!(bad, Scalar::Str("x".into()));
    }

    #[test]
    fn numeric_values() {
        assert_eq!(int_value("0x1F"), Some(31));
        assert_eq!(int_value("0o17"), Some(15));
        assert_eq!(float_value("-.inf"), Some(f64::NEG_INFINITY));
        assert!(float_value(".nan").unwrap().is_nan());
        assert_eq!(float_value("1e3"), Some(1000.0));
    }
}
