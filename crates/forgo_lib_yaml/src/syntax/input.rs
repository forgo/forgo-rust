//! Input decoding (YAML 1.2.2 §5.2).
//!
//! A YAML stream may be UTF-8, UTF-16 or UTF-32, detected from a byte order
//! mark or from the position of the null bytes around the first ASCII
//! character. A byte order mark is kept in the decoded text; the scanner
//! accepts it at the start of the stream and before each document.

use crate::ast::Error;
use std::borrow::Cow;

/// The character encoding of a YAML byte stream.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Encoding {
    /// UTF-8.
    Utf8,
    /// UTF-16, little-endian.
    Utf16Le,
    /// UTF-16, big-endian.
    Utf16Be,
    /// UTF-32, little-endian.
    Utf32Le,
    /// UTF-32, big-endian.
    Utf32Be,
}

/// Detect the encoding using the table in spec §5.2.
pub fn detect_encoding(bytes: &[u8]) -> Encoding {
    match bytes {
        [0, 0, 0xFE, 0xFF, ..] | [0, 0, 0, _, ..] => Encoding::Utf32Be,
        [0xFF, 0xFE, 0, 0, ..] | [_, 0, 0, 0, ..] => Encoding::Utf32Le,
        [0xFE, 0xFF, ..] | [0, _, ..] => Encoding::Utf16Be,
        [0xFF, 0xFE, ..] | [_, 0, ..] => Encoding::Utf16Le,
        _ => Encoding::Utf8,
    }
}

fn position_error(decoded: &str, msg: String) -> Error {
    let line = decoded.matches('\n').count() + 1;
    let col = decoded.rsplit('\n').next().map_or(0, |l| l.chars().count()) + 1;
    Error::Syntax { line, col, msg }
}

/// Decode a YAML byte stream to text. UTF-8 input is borrowed.
pub fn decode(bytes: &[u8]) -> Result<Cow<'_, str>, Error> {
    match detect_encoding(bytes) {
        Encoding::Utf8 => match std::str::from_utf8(bytes) {
            Ok(s) => Ok(Cow::Borrowed(s)),
            Err(e) => {
                let ok = std::str::from_utf8(&bytes[..e.valid_up_to()]).unwrap_or_default();
                Err(position_error(
                    ok,
                    format!("invalid UTF-8 at byte {}", e.valid_up_to()),
                ))
            }
        },
        enc @ (Encoding::Utf16Le | Encoding::Utf16Be) => {
            if !bytes.len().is_multiple_of(2) {
                return Err(Error::Syntax {
                    line: 1,
                    col: 1,
                    msg: "UTF-16 input has an odd number of bytes".into(),
                });
            }
            let units = bytes.as_chunks::<2>().0.iter().map(|&p| match enc {
                Encoding::Utf16Le => u16::from_le_bytes(p),
                _ => u16::from_be_bytes(p),
            });
            let mut out = String::with_capacity(bytes.len() / 2);
            for (i, r) in char::decode_utf16(units).enumerate() {
                match r {
                    Ok(c) => out.push(c),
                    Err(_) => {
                        return Err(position_error(
                            &out,
                            format!("invalid UTF-16 at byte {}", i * 2),
                        ));
                    }
                }
            }
            Ok(Cow::Owned(out))
        }
        enc @ (Encoding::Utf32Le | Encoding::Utf32Be) => {
            if !bytes.len().is_multiple_of(4) {
                return Err(Error::Syntax {
                    line: 1,
                    col: 1,
                    msg: "UTF-32 input length is not a multiple of 4".into(),
                });
            }
            let mut out = String::with_capacity(bytes.len() / 4);
            for (i, &q) in bytes.as_chunks::<4>().0.iter().enumerate() {
                let v = match enc {
                    Encoding::Utf32Le => u32::from_le_bytes(q),
                    _ => u32::from_be_bytes(q),
                };
                match char::from_u32(v) {
                    Some(c) => out.push(c),
                    None => {
                        return Err(position_error(
                            &out,
                            format!("invalid UTF-32 code point at byte {}", i * 4),
                        ));
                    }
                }
            }
            Ok(Cow::Owned(out))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utf16(s: &str, le: bool, bom: bool) -> Vec<u8> {
        let mut v = Vec::new();
        let units = bom.then_some(0xFEFFu16).into_iter().chain(s.encode_utf16());
        for u in units {
            v.extend(if le { u.to_le_bytes() } else { u.to_be_bytes() });
        }
        v
    }

    fn utf32(s: &str, le: bool, bom: bool) -> Vec<u8> {
        let mut v = Vec::new();
        let chars = bom.then_some('\u{FEFF}').into_iter().chain(s.chars());
        for c in chars {
            let u = c as u32;
            v.extend(if le { u.to_le_bytes() } else { u.to_be_bytes() });
        }
        v
    }

    #[test]
    fn detects_and_decodes_every_encoding() {
        let text = "a: é\n";
        for bom in [false, true] {
            let expect = if bom {
                format!("\u{FEFF}{text}")
            } else {
                text.to_string()
            };
            assert_eq!(decode(&utf16(text, true, bom)).unwrap(), expect);
            assert_eq!(decode(&utf16(text, false, bom)).unwrap(), expect);
            assert_eq!(decode(&utf32(text, true, bom)).unwrap(), expect);
            assert_eq!(decode(&utf32(text, false, bom)).unwrap(), expect);
        }
        assert_eq!(decode(text.as_bytes()).unwrap(), text);
        assert_eq!(decode(b"\xEF\xBB\xBFa").unwrap(), "\u{FEFF}a");
        assert!(matches!(decode(text.as_bytes()).unwrap(), Cow::Borrowed(_)));
    }

    #[test]
    fn rejects_invalid_input() {
        assert!(decode(b"a\n\xFF").is_err());
        assert!(decode(&[0, b'a', 0]).is_err());
    }
}
