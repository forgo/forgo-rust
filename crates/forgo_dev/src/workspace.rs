//! Workspace `[workspace].members` manipulation and normalization.

use crate::util::{find_matching_bracket, read_to_string, write_string};
use std::path::Path;

/// Add `rel_path` to the members list (alphabetically, normalized, one-per-line).
pub fn add_workspace_member_sorted(workspace_toml: &Path, rel_path: &str) {
    let mut s = read_to_string(workspace_toml);

    if let Some((open, close)) = find_workspace_members_array(&s) {
        let inner = &s[(open + 1)..close];
        let mut items = extract_quoted_strings(inner);

        if !items.iter().any(|i| i == rel_path) {
            items.push(rel_path.to_string());
        }

        items.sort();
        items.dedup();

        let block = render_members_block(&items);
        let mut new_s = s.clone();
        new_s.replace_range(open..=close, &block);
        write_string(workspace_toml, new_s);
        println!("• normalized workspace members (alphabetical): {rel_path}");
    } else {
        // No [workspace] members list present; create one with our path.
        let block = render_members_block(&vec![rel_path.to_string()]);
        let addition = format!("\n[workspace]\nresolver = \"3\"\nmembers = {block}\n");
        s.push_str(&addition);
        write_string(workspace_toml, s);
        println!("• created workspace members and added: {rel_path}");
    }
}

/// Remove `rel_path` from the members list (keeps normalized formatting).
pub fn remove_workspace_member(workspace_toml: &Path, rel_path: &str) {
    let s = read_to_string(workspace_toml);
    if let Some((open, close)) = find_workspace_members_array(&s) {
        let inner = &s[(open + 1)..close];
        let mut items = extract_quoted_strings(inner);

        let before_len = items.len();
        items.retain(|i| i != rel_path);

        if items.len() == before_len {
            println!("• workspace members did not reference: {rel_path}");
            return;
        }

        items.sort();
        items.dedup();

        let block = render_members_block(&items);
        let mut new_s = s.clone();
        new_s.replace_range(open..=close, &block);
        write_string(workspace_toml, new_s);
        println!("• removed from workspace members: {rel_path}");
    } else {
        println!("• workspace members list not found; nothing to remove");
    }
}

/// Find the '[' and matching ']' of the members array **inside the [workspace] table**.
pub fn find_workspace_members_array(s: &str) -> Option<(usize, usize)> {
    // Find the [workspace] header
    let mut scan_start = 0usize;
    while let Some(pos) = s[scan_start..].find("[workspace]") {
        let ws_start = scan_start + pos;

        // Determine the bounds of the [workspace] section
        let after_header = s[ws_start..]
            .find('\n')
            .map(|x| ws_start + x + 1)
            .unwrap_or(s.len());
        // Next header = a line that starts with '[' after after_header
        let mut section_end = s.len();
        let mut i = after_header;
        while i < s.len() {
            if let Some(nl) = s[i..].find('\n') {
                let line_start = i;
                let next_i = i + nl + 1;
                if s[line_start..].starts_with('[') {
                    section_end = line_start;
                    break;
                }
                i = next_i;
            } else {
                break;
            }
        }

        let section = &s[after_header..section_end];

        // Inside this section, find "members = ["
        if let Some(members_pos) = section.find("members") {
            let abs = after_header + members_pos;
            // scan forward: optional spaces, '=', optional spaces, then '['
            let mut i = abs + "members".len();
            while i < s.len() && s.as_bytes()[i].is_ascii_whitespace() {
                i += 1;
            }
            if i >= s.len() || s.as_bytes()[i] != b'=' {
                scan_start = section_end;
                continue;
            }
            i += 1;
            while i < s.len() && s.as_bytes()[i].is_ascii_whitespace() {
                i += 1;
            }
            if i >= s.len() || s.as_bytes()[i] != b'[' {
                scan_start = section_end;
                continue;
            }
            let open = i;
            if let Some(close) = find_matching_bracket(s, open) {
                return Some((open, close));
            } else {
                return None;
            }
        }

        // No members in this [workspace] — keep scanning
        scan_start = section_end;
    }
    None
}

/// Extract TOML-quoted strings from an array body.
pub fn extract_quoted_strings(inner: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut in_quote = false;
    let mut buf = String::new();

    for ch in inner.chars() {
        match ch {
            '"' if !in_quote => {
                in_quote = true;
                buf.clear();
            }
            '"' if in_quote => {
                in_quote = false;
                out.push(buf.clone());
            }
            _ if in_quote => buf.push(ch),
            _ => {}
        }
    }
    out
}

/// Render members array as a normalized multi-line block with trailing commas.
pub fn render_members_block(items: &[String]) -> String {
    if items.is_empty() {
        return "[\n]".to_string();
    }
    let lines = items
        .iter()
        .map(|i| format!("    \"{}\",", i))
        .collect::<Vec<_>>()
        .join("\n");
    format!("[\n{lines}\n]")
}
