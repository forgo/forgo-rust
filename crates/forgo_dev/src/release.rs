//! Release workflow options list: normalize to dash-list, keep alphabetical.

use crate::util::{read_to_string, write_string};
use std::path::Path;

/// Add a crate to `options:` list (alphabetical, dash-list).
pub fn update_release_options_add(release_path: &Path, crate_name: &str) {
    let changed = rewrite_release_options_sorted(release_path, |items| {
        if !items.iter().any(|i| i == crate_name) {
            items.push(crate_name.to_string());
            true
        } else {
            false
        }
    });

    if changed {
        println!("• added to Release options: {}", crate_name);
    } else {
        println!("• already present in Release options: {}", crate_name);
    }
}

/// Remove a crate from `options:` list (alphabetical persisted, dash-list).
pub fn update_release_options_remove(release_path: &Path, crate_name: &str) {
    let changed = rewrite_release_options_sorted(release_path, |items| {
        let before = items.len();
        items.retain(|i| i != crate_name);
        items.len() != before
    });

    if changed {
        println!("• removed from Release options: {}", crate_name);
    } else {
        println!("• not present in Release options");
    }
}

/// Parse the `options:` list which may be in **either** inline array or dash-list form,
/// let the updater mutate it, then **sort, dedup, and re-render** the block as a dash-list.
/// Returns true if a write occurred.
fn rewrite_release_options_sorted<F>(release_path: &Path, updater: F) -> bool
where
    F: FnOnce(&mut Vec<String>) -> bool,
{
    let text = read_to_string(release_path);
    let lines: Vec<String> = text.lines().map(|s| s.to_string()).collect();

    // Locate "options:" line
    let mut options_idx: Option<usize> = None;
    for (i, line) in lines.iter().enumerate() {
        if line.trim_start().starts_with("options:") {
            options_idx = Some(i);
            break;
        }
    }
    let Some(opts_i) = options_idx else {
        eprintln!("WARN: 'options:' not found in Release workflow; skipped update.");
        return false;
    };

    // Determine base indent = indent of "options:" line
    let base_indent = leading_spaces(&lines[opts_i]);

    // Check if same-line inline array: `options: [ ... ]`
    let tail = lines[opts_i].splitn(2, ':').nth(1).unwrap_or("").trim();

    let (mut items, after_block_idx, item_indent) = if tail.starts_with('[') {
        // Inline array case
        let (items, consumed_to) = parse_inline_array_after_colon(tail, &lines, opts_i);
        (items, consumed_to, base_indent + 2)
    } else {
        // Dash-list case
        parse_dash_list_after_key(&lines, opts_i + 1, base_indent)
    };

    // Allow caller to modify items
    let mut changed = updater(&mut items);

    // Normalize (sort & dedup)
    let before_norm = items.clone();
    items.sort();
    items.dedup();
    if items != before_norm {
        changed = true;
    }

    if !changed {
        return false;
    }

    // Rebuild the options block as dash-list, preserving indentation
    let indent_spaces = " ".repeat(item_indent);
    let mut new_block: Vec<String> = Vec::with_capacity(2 + items.len());
    new_block.push(format!("{}options:", " ".repeat(base_indent)));
    for name in &items {
        new_block.push(format!("{indent_spaces}- {name}"));
    }

    let mut out: Vec<String> = Vec::new();
    out.extend_from_slice(&lines[..opts_i]);
    out.extend_from_slice(&new_block);
    out.extend_from_slice(&lines[after_block_idx..]);

    write_string(release_path, out.join("\n"));
    true
}

/* ------------------------------ helpers ------------------------------ */

fn leading_spaces(s: &str) -> usize {
    s.chars().take_while(|c| *c == ' ').count()
}

/// Parse inline `[ "...", "..." ]` that appears on the same "key:" line.
fn parse_inline_array_after_colon(
    tail_after_colon: &str,
    lines: &[String],
    idx: usize,
) -> (Vec<String>, usize) {
    // If bracket pair is on the same line, just parse; otherwise, accumulate until ']' matched.
    let mut buf = tail_after_colon.to_string();
    let mut consumed_to = idx + 1;

    if !(tail_after_colon.contains('[') && tail_after_colon.contains(']')) {
        // Accumulate
        let mut nesting = 0i32;
        for ch in tail_after_colon.chars() {
            if ch == '[' {
                nesting += 1;
            } else if ch == ']' {
                nesting -= 1;
            }
        }
        let mut k = idx + 1;
        while k < lines.len() && nesting > 0 {
            buf.push_str(lines[k].as_str());
            for ch in lines[k].chars() {
                if ch == '[' {
                    nesting += 1;
                } else if ch == ']' {
                    nesting -= 1;
                }
            }
            k += 1;
        }
        consumed_to = k;
    }

    (extract_inline_items(&buf), consumed_to)
}

fn extract_inline_items(buf: &str) -> Vec<String> {
    // Find inner `[ ... ]`
    if let Some(open) = buf.find('[') {
        if let Some(close) = crate::util::find_matching_bracket(buf, open) {
            let inner = &buf[(open + 1)..close];
            return inner
                .split(',')
                .map(|t| t.trim())
                .filter(|t| !t.is_empty())
                .map(|t| t.trim_matches('"').to_string())
                .collect();
        }
    }
    Vec::new()
}

/// Parse a dash-list that follows the key line. Returns (items, next_index, item_indent).
fn parse_dash_list_after_key(
    lines: &[String],
    mut j: usize,
    base_indent: usize,
) -> (Vec<String>, usize, usize) {
    let mut items = Vec::new();
    let mut item_indent: Option<usize> = None;

    while j < lines.len() {
        let l = &lines[j];
        if l.trim().is_empty() {
            j += 1;
            continue;
        }
        let lead = leading_spaces(l);
        if lead <= base_indent {
            break;
        }
        let trimmed = l.trim_start();
        if trimmed.starts_with("- ") {
            if item_indent.is_none() {
                item_indent = Some(lead);
            }
            let name = trimmed.trim_start_matches("- ").trim().to_string();
            if !name.is_empty() {
                items.push(name);
            }
            j += 1;
        } else {
            break;
        }
    }

    (items, j, item_indent.unwrap_or(base_indent + 2))
}
