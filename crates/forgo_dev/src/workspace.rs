//! `[workspace].members` in the root `Cargo.toml`.
//!
//! The members array is kept sorted and deduplicated and written one entry
//! per line; the rest of the file is untouched. (This is a small, careful
//! scanner for that one array; forgo_lib_toml will replace it.)

use crate::error::{Result, failed};

/// The workspace manifest, relative to the workspace root.
pub const CARGO_TOML: &str = "Cargo.toml";

/// The manifest with `member` added to `[workspace].members`, or `Ok(None)`
/// if it's already there.
pub fn add_member(text: &str, member: &str) -> Result<Option<String>> {
    edit(text, |items| {
        if items.iter().any(|i| i == member) {
            false
        } else {
            items.push(member.to_string());
            true
        }
    })
}

/// The manifest with `member` removed from `[workspace].members`, or
/// `Ok(None)` if it isn't there.
pub fn remove_member(text: &str, member: &str) -> Result<Option<String>> {
    edit(text, |items| {
        let before = items.len();
        items.retain(|i| i != member);
        items.len() != before
    })
}

/// The members currently listed.
pub fn members(text: &str) -> Result<Vec<String>> {
    let (open, close) = members_array(text)?;
    items_of(&text[open + 1..close])
}

fn edit(text: &str, change: impl FnOnce(&mut Vec<String>) -> bool) -> Result<Option<String>> {
    let (open, close) = members_array(text)?;
    let mut items = items_of(&text[open + 1..close])?;
    if !change(&mut items) {
        return Ok(None);
    }
    if let Some(bad) = items
        .iter()
        .find(|i| i.contains(['"', '\\']) || i.contains(char::is_control))
    {
        return failed(format!(
            "{CARGO_TOML}: can't rewrite `members` entry `{bad}`; edit it by hand"
        ));
    }
    items.sort();
    items.dedup();
    let newline = if text[..open].contains("\r\n") || text[close..].starts_with("]\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let mut out = String::with_capacity(text.len() + 64);
    out.push_str(&text[..open]);
    out.push_str(&render(&items, newline));
    out.push_str(&text[close + 1..]);
    Ok(Some(out))
}

/// Byte offsets of the `[` and `]` of `members = [...]` in `[workspace]`.
fn members_array(text: &str) -> Result<(usize, usize)> {
    let mut in_workspace = false;
    let mut offset = 0;
    for line in text.split_inclusive('\n') {
        let start = offset;
        offset += line.len();
        let t = line.trim_start();
        if t.starts_with('[') {
            in_workspace = table_name(t) == Some("workspace");
            continue;
        }
        // The key must be exactly `members` (not `default-members`).
        let Some(rest) = t.strip_prefix("members") else {
            continue;
        };
        let rest = rest.trim_start();
        if !in_workspace || !rest.starts_with('=') {
            continue;
        }
        let after_eq = rest[1..].trim_start();
        if !after_eq.starts_with('[') {
            return failed(format!("{CARGO_TOML}: `members` isn't an array"));
        }
        let open = start + (line.len() - after_eq.len());
        return match close_of(text, open) {
            Some(close) => Ok((open, close)),
            None => failed(format!("{CARGO_TOML}: `members` array isn't closed")),
        };
    }
    failed(format!("{CARGO_TOML}: no `members` array in `[workspace]`"))
}

/// The name in a `[table]` header line (`None` for `[[array]]` headers):
/// `[ workspace ]  # note` gives `workspace`.
fn table_name(line: &str) -> Option<&str> {
    let inner = line.strip_prefix('[')?;
    if inner.starts_with('[') {
        return None;
    }
    Some(inner[..inner.find(']')?].trim())
}

/// The `]` closing the array opened at `open`, skipping strings and
/// comments.
fn close_of(text: &str, open: usize) -> Option<usize> {
    let b = text.as_bytes();
    let (mut i, mut depth) = (open, 0usize);
    while i < b.len() {
        match b[i] {
            b'[' => depth += 1,
            b']' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            b'"' | b'\'' => {
                let q = b[i];
                i += 1;
                while i < b.len() && b[i] != q && b[i] != b'\n' {
                    i += if b[i] == b'\\' && q == b'"' { 2 } else { 1 };
                }
            }
            b'#' => {
                while i < b.len() && b[i] != b'\n' {
                    i += 1;
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

/// The strings of an array body (`"a", 'b',` with comments).
fn items_of(body: &str) -> Result<Vec<String>> {
    let b = body.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'"' | b'\'' => {
                let q = b[i];
                let start = i + 1;
                i = start;
                while i < b.len() && b[i] != q {
                    i += 1;
                }
                let s = &body[start..i.min(body.len())];
                if q == b'"' && s.contains('\\') {
                    return failed(format!(
                        "{CARGO_TOML}: escapes in `members` entries aren't supported"
                    ));
                }
                out.push(s.to_string());
            }
            b'#' => {
                while i < b.len() && b[i] != b'\n' {
                    i += 1;
                }
            }
            _ => {}
        }
        i += 1;
    }
    Ok(out)
}

/// One entry per line, four-space indent, trailing commas.
fn render(items: &[String], newline: &str) -> String {
    let mut s = String::from("[");
    s.push_str(newline);
    for i in items {
        s.push_str("    \"");
        s.push_str(i);
        s.push_str("\",");
        s.push_str(newline);
    }
    s.push(']');
    s
}
