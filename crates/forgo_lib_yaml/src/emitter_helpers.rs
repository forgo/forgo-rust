// crates/forgo_lib_yaml/src/emitter_helpers.rs
//! How scalars are written: which styles can represent a value in a given
//! context, and the text for each style.
//!
//! The emitter honors a node's recorded `Style` when that style can
//! represent the value in its position, and otherwise falls back:
//!
//! | recorded style        | falls back to   | when                                  |
//! |-----------------------|-----------------|---------------------------------------|
//! | plain / none          | double quotes   | the text isn't plain-safe here        |
//! | single quotes         | double quotes   | non-printable characters, or white space next to a line break, or a multi-line key |
//! | literal / folded      | double quotes   | inside flow or as an implicit key, non-printable characters |
//! | double quotes         | —               | always representable                  |
//!
//! Values without a recorded style are plain when safe; multi-line strings
//! are literal block scalars unless a line ends in white space; anything
//! else is double-quoted.

use crate::ast::{BlockHeader, Chomping, Meta, Scalar, Style};
use crate::schema;
use crate::syntax::chars::{is_flow_indicator, is_printable};

/// Where a scalar is written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Ctx {
    /// Inside a flow collection.
    pub flow: bool,
    /// An implicit mapping key (single line, at most 1024 characters).
    pub key: bool,
    /// Block scalars are allowed (block context, value position).
    pub block_ok: bool,
    /// An empty (zero-length) plain scalar is allowed here.
    pub empty_ok: bool,
    /// The scalar starts at column 0, where `--- ` and `... ` would be
    /// read as document markers.
    pub line_start: bool,
}

/// The chosen representation of a scalar.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Repr {
    /// Plain text, possibly spanning lines (line breaks folded).
    Plain(String),
    /// Nothing at all (an empty null).
    Empty,
    Single,
    Double,
    Block {
        folded: bool,
        header: BlockHeader,
    },
}

/// Characters we never write outside double quotes: they aren't printable,
/// or would be read as a line break or a byte order mark. (NEL, LS and PS
/// are ordinary characters in YAML 1.2; double quotes escape them anyway.)
fn awkward(c: char) -> bool {
    !is_printable(c) || matches!(c, '\r' | '\u{FEFF}')
}

/// Would a plain scalar with this text resolve to something other than a
/// string under the core schema?
fn resolves_non_string(s: &str) -> bool {
    schema::is_null(s)
        || schema::parse_bool(s).is_some()
        || schema::is_int(s)
        || schema::is_float(s)
}

/// Can `line` be one line of a plain scalar? For an implicit key, the
/// text is followed by `: `, so a trailing `:` (or a lone `-`, `?`, `:`)
/// still reads back. At the start of a line, text beginning with a
/// document marker isn't plain.
fn plain_line_ok(line: &str, first: bool, ctx: Ctx) -> bool {
    let (flow, key) = (ctx.flow, ctx.key);
    let b = line.as_bytes();
    if b.is_empty() {
        return false;
    }
    if matches!(b[0], b' ' | b'\t') || matches!(b[b.len() - 1], b' ' | b'\t') {
        return false;
    }
    if line.chars().any(awkward) {
        return false;
    }
    if flow && line.chars().any(is_flow_indicator) {
        return false;
    }
    if line.contains(": ") || line.contains(":\t") || (line.ends_with(':') && !key) {
        return false;
    }
    if line.contains(" #") || line.contains("\t#") {
        return false;
    }
    let c0 = line.chars().next().unwrap();
    if first {
        if matches!(c0, '-' | '?' | ':') {
            // Allowed when followed by a "safe" character (for a key, the
            // `:` of the value indicator follows).
            match line[1..].chars().next() {
                Some(n) if !matches!(n, ' ' | '\t') && !(flow && is_flow_indicator(n)) => {}
                None if key => {}
                _ => return false,
            }
        } else if crate::syntax::chars::is_indicator(c0) {
            return false;
        }
        if ctx.line_start
            && (line.starts_with("---") || line.starts_with("..."))
            && (matches!(b.get(3), Some(b' ' | b'\t')) || (b.get(3).is_none() && !key))
        {
            return false;
        }
    } else if c0 == '#' {
        // Would start a comment after the continuation indent.
        return false;
    }
    true
}

/// Can `s` be written as a plain scalar here (and read back as the same
/// string)?
pub(crate) fn plain_ok(s: &str, ctx: Ctx) -> bool {
    !resolves_non_string(s) && plain_text_ok(s, ctx)
}

/// Can `s` be written as plain text here, ignoring what an untagged plain
/// scalar would resolve to (for nodes whose tag fixes their type)?
pub(crate) fn plain_text_ok(s: &str, ctx: Ctx) -> bool {
    if s.is_empty() {
        return false;
    }
    if s.contains('\n') {
        if ctx.key || s.starts_with('\n') || s.ends_with('\n') {
            return false;
        }
        let mut first = true;
        for seg in s.split('\n') {
            if seg.is_empty() {
                continue; // a run of line breaks
            }
            let seg_ctx = Ctx {
                key: false,
                line_start: ctx.line_start && first,
                ..ctx
            };
            if !plain_line_ok(seg, first, seg_ctx) {
                return false;
            }
            first = false;
        }
        return true;
    }
    if ctx.key && s.chars().count() > 1024 {
        return false;
    }
    plain_line_ok(s, true, ctx)
}

/// Can `s` be written single-quoted here?
pub(crate) fn single_ok(s: &str, ctx: Ctx) -> bool {
    // A BOM is allowed inside quotes (`nb-json`); it was written that way.
    if s.chars()
        .any(|c| c != '\n' && c != '\t' && c != '\u{FEFF}' && awkward(c))
    {
        return false;
    }
    if s.contains('\n') {
        if ctx.key {
            return false;
        }
        // Folding drops white space around line breaks.
        let b = s.as_bytes();
        for (i, &c) in b.iter().enumerate() {
            if c == b'\n' {
                let before = i.checked_sub(1).map(|j| b[j]);
                let after = b.get(i + 1).copied();
                if matches!(before, Some(b' ' | b'\t')) || matches!(after, Some(b' ' | b'\t')) {
                    return false;
                }
            }
        }
    }
    true
}

/// Can `s` be a block scalar at all (content-wise)?
pub(crate) fn block_ok(s: &str) -> bool {
    !s.chars().any(|c| c != '\n' && c != '\t' && awkward(c))
}

fn spelling_ok(spelling: &str, scalar: &Scalar) -> bool {
    match scalar {
        Scalar::Null => schema::is_null(spelling),
        Scalar::Bool(b) => schema::parse_bool(spelling) == Some(*b),
        _ => false,
    }
}

/// Choose how to write `scalar` with metadata `meta` in context `ctx`.
pub(crate) fn scalar_repr(scalar: &Scalar, meta: &Meta, ctx: Ctx) -> Repr {
    let tagged = meta.tag.is_some();
    match scalar {
        Scalar::Str(s) => match meta.style {
            Some(Style::DoubleQuoted) => Repr::Double,
            Some(Style::SingleQuoted) => {
                if single_ok(s, ctx) {
                    Repr::Single
                } else {
                    Repr::Double
                }
            }
            Some(Style::Literal(h)) | Some(Style::Folded(h)) => {
                if ctx.block_ok && block_ok(s) {
                    Repr::Block {
                        folded: matches!(meta.style, Some(Style::Folded(_))),
                        header: h,
                    }
                } else {
                    Repr::Double
                }
            }
            Some(Style::Plain) | Some(Style::Block) | Some(Style::Flow) => {
                // A tag fixes the type, so text like `23` can stay plain.
                if plain_ok(s, ctx) || (tagged && plain_text_ok(s, ctx)) {
                    Repr::Plain(s.clone())
                } else if s.is_empty() && tagged {
                    // A tag on an empty node: `!!str` alone.
                    Repr::Empty
                } else {
                    Repr::Double
                }
            }
            None => {
                if plain_ok(s, ctx) {
                    Repr::Plain(s.clone())
                } else if ctx.block_ok
                    && s.contains('\n')
                    && block_ok(s)
                    && !s.contains(" \n")
                    && !s.contains("\t\n")
                {
                    Repr::Block {
                        folded: false,
                        header: BlockHeader::default(),
                    }
                } else {
                    Repr::Double
                }
            }
        },
        other => {
            // Non-string scalars are plain unless a tag keeps their type
            // inside quotes.
            if tagged && meta.is_quoted() {
                return if meta.style == Some(Style::SingleQuoted) {
                    Repr::Single
                } else {
                    Repr::Double
                };
            }
            if let Some(sp) = &meta.spelling
                && spelling_ok(sp, other)
            {
                if sp.is_empty() {
                    // Properties can stand alone on an empty node anywhere.
                    if ctx.empty_ok || tagged || meta.anchor.is_some() {
                        return Repr::Empty;
                    }
                } else {
                    return Repr::Plain(sp.clone());
                }
            }
            Repr::Plain(match other {
                Scalar::Null => "null".into(),
                Scalar::Bool(true) => "true".into(),
                Scalar::Bool(false) => "false".into(),
                Scalar::Num { text } => text.clone(),
                Scalar::Str(_) => unreachable!(),
            })
        }
    }
}

/// The text a scalar's value has for representations that need it.
pub(crate) fn scalar_text(scalar: &Scalar) -> String {
    match scalar {
        Scalar::Str(s) => s.clone(),
        Scalar::Num { text } => text.clone(),
        Scalar::Bool(b) => b.to_string(),
        Scalar::Null => String::new(),
    }
}

/// Write text that may contain line breaks as folded lines: a run of `k`
/// breaks becomes `k + 1` breaks (an empty line per break), continuation
/// lines start at `indent`.
pub(crate) fn write_folded_lines(s: &str, indent: usize, out: &mut String) {
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\n' {
            let mut k = 1;
            while chars.peek() == Some(&'\n') {
                chars.next();
                k += 1;
            }
            for _ in 0..=k {
                out.push('\n');
            }
            emit_indent(indent, out);
        } else {
            out.push(c);
        }
    }
}

/// Write `s` single-quoted (line breaks folded, continuation at `indent`).
pub(crate) fn emit_single(s: &str, indent: usize, out: &mut String) {
    out.push('\'');
    write_folded_lines(&s.replace('\'', "''"), indent, out);
    out.push('\'');
}

/// Write a block scalar header and body. `parent_indent` is the column
/// the indentation indicator counts from; the body goes two columns
/// further, or at the recorded indentation indicator. The recorded chomping
/// indicator is kept when it yields the same value; an indentation
/// indicator is written when recorded or needed.
pub(crate) fn emit_block(
    s: &str,
    folded: bool,
    header: BlockHeader,
    parent_indent: usize,
    trailing_comment: Option<&str>,
    out: &mut String,
) {
    let m = header.indent.map_or(2, usize::from);
    let content_indent = parent_indent + m;
    let body = s.trim_end_matches('\n');
    let trailing = s.len() - body.len();
    let computed = if s.is_empty() {
        Chomping::Strip
    } else if body.is_empty() {
        Chomping::Keep
    } else {
        match trailing {
            0 => Chomping::Strip,
            1 => Chomping::Clip,
            _ => Chomping::Keep,
        }
    };
    let chomp = match header.chomping {
        Chomping::Keep => Chomping::Keep,
        Chomping::Strip if trailing == 0 => Chomping::Strip,
        Chomping::Clip if s.is_empty() || (trailing == 1 && !body.is_empty()) => Chomping::Clip,
        _ => computed,
    };
    let needs_indicator = s
        .split('\n')
        .find(|l| !l.is_empty())
        .is_some_and(|l| l.starts_with(' '));
    out.push(if folded { '>' } else { '|' });
    if needs_indicator || header.indent.is_some() {
        out.push_str(&m.to_string());
    }
    match chomp {
        Chomping::Strip => out.push('-'),
        Chomping::Keep => out.push('+'),
        Chomping::Clip => {}
    }
    if let Some(tc) = trailing_comment {
        out.push_str(" # ");
        out.push_str(tc);
    }
    out.push('\n');
    if folded {
        write_folded_body(s, content_indent, out);
    } else {
        for piece in s.split_inclusive('\n') {
            if piece != "\n" {
                emit_indent(content_indent, out);
                out.push_str(piece);
            } else {
                out.push('\n');
            }
        }
        if !s.ends_with('\n') && !s.is_empty() {
            out.push('\n');
        }
    }
}

/// Folded body: a line break between two non-indented lines must be
/// written as an empty line, because a single break folds into a space.
fn write_folded_body(s: &str, indent: usize, out: &mut String) {
    let chars: Vec<char> = s.chars().collect();
    let mut breaks = true;
    let mut leading_spaces = true;
    for (i, &c) in chars.iter().enumerate() {
        if c == '\n' {
            if !breaks && !leading_spaces {
                let mut k = i;
                while k < chars.len() && chars[k] == '\n' {
                    k += 1;
                }
                if k < chars.len() && chars[k] != ' ' && chars[k] != '\t' {
                    out.push('\n');
                }
            }
            out.push('\n');
            breaks = true;
        } else {
            if breaks {
                emit_indent(indent, out);
                leading_spaces = c == ' ' || c == '\t';
            }
            out.push(c);
            breaks = false;
        }
    }
    if !s.ends_with('\n') && !s.is_empty() {
        out.push('\n');
    }
}

/// Write a node's tag and anchor, each followed by a space.
pub(crate) fn write_props(meta: &Meta, out: &mut String) {
    if let Some(t) = &meta.tag {
        out.push_str(t);
        out.push(' ');
    }
    if let Some(a) = &meta.anchor {
        out.push('&');
        out.push_str(a);
        out.push(' ');
    }
}

/// Emit a double-quoted string, escaping every character that isn't
/// printable (plus `"`, `\`, line breaks, tabs, the BOM and the Unicode
/// line/paragraph separators).
pub(crate) fn emit_quoted(s: &str, out: &mut String) {
    use std::fmt::Write as _;
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            '\0' => out.push_str("\\0"),
            '\u{7}' => out.push_str("\\a"),
            '\u{8}' => out.push_str("\\b"),
            '\u{B}' => out.push_str("\\v"),
            '\u{C}' => out.push_str("\\f"),
            '\u{1B}' => out.push_str("\\e"),
            '\u{85}' => out.push_str("\\N"),
            '\u{2028}' => out.push_str("\\L"),
            '\u{2029}' => out.push_str("\\P"),
            '\u{FEFF}' => out.push_str("\\uFEFF"),
            c if !is_printable(c) => {
                let v = c as u32;
                let _ = if v <= 0xFF {
                    write!(out, "\\x{v:02X}")
                } else {
                    write!(out, "\\u{v:04X}")
                };
            }
            other => out.push(other),
        }
    }
    out.push('"');
}

/// Emit indentation (spaces).
pub(crate) fn emit_indent(n: usize, out: &mut String) {
    for _ in 0..n {
        out.push(' ');
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BLOCK: Ctx = Ctx {
        flow: false,
        key: false,
        block_ok: true,
        empty_ok: true,
        line_start: true,
    };
    const FLOW: Ctx = Ctx {
        flow: true,
        key: false,
        block_ok: false,
        empty_ok: false,
        line_start: false,
    };

    #[test]
    fn plain_safety() {
        for ok in [
            "a",
            "value with spaces",
            "k:v",
            "http://x",
            "a#b",
            "-a",
            "?a",
            ":a",
            "a\nb",
            "x[y]",
        ] {
            assert!(plain_ok(ok, BLOCK), "{ok:?} should be plain-safe");
        }
        for bad in [
            "", " a", "a ", "a: b", "a #b", "-", "- a", "#a", "&a", "*a", "!a", "|", ">", "'a",
            "\"a", "%a", "@a", "`a", "[x", "{x", "---", "... x", "true", "Null", "1", "0x1F",
            ".inf", "1e3", "a:", "a\n#b", "\na", "a\n", "a\u{7}",
        ] {
            assert!(!plain_ok(bad, BLOCK), "{bad:?} must not be plain");
        }
        assert!(!plain_ok("[x", FLOW));
        assert!(!plain_ok("a,b", FLOW));
        assert!(plain_ok("a b", FLOW));
    }

    #[test]
    fn single_quote_safety() {
        assert!(single_ok("it's", BLOCK));
        assert!(single_ok("a\nb", BLOCK));
        assert!(!single_ok("a \nb", BLOCK));
        assert!(!single_ok("a\n b", BLOCK));
        assert!(!single_ok("a\u{0}", BLOCK));
    }
}
