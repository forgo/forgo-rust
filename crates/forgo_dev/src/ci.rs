//! CI matrix manipulation: normalize to dash-lists, keep sorted & deduped.
//!
//! We accept either of these input forms and always rewrite to the dash-list:
//!   project: ["forgo", "forgo_lib_cli"]
//!   project:
//!     - forgo
//!     - forgo_lib_cli

use crate::util::{find_matching_bracket, read_to_string, write_string};
use std::path::Path;

/// Add a crate name to every matrix.project (sorted, deduped, dash-list).
pub fn update_ci_matrix_add(ci_path: &Path, crate_name: &str) {
    let changed = rewrite_all_project_lists(ci_path, |items| {
        if !items.iter().any(|i| i == crate_name) {
            items.push(crate_name.to_string());
            true
        } else {
            false
        }
    });

    if changed {
        println!("• added to CI matrices: {}", crate_name);
    } else {
        println!("• already present in CI matrices or no matrix found");
    }
}

/// Remove a crate name from every matrix.project (sorted, deduped, dash-list).
pub fn update_ci_matrix_remove(ci_path: &Path, crate_name: &str) {
    let changed = rewrite_all_project_lists(ci_path, |items| {
        let before = items.len();
        items.retain(|i| i != crate_name);
        items.len() != before
    });

    if changed {
        println!("• removed from CI matrices: {}", crate_name);
    } else {
        println!("• not present in CI matrices or no matrix found");
    }
}

/// Parse all `matrix.project` lists, allow `updater` to mutate the items,
/// then **sort, dedup, and re-render** each as a dash list with consistent indentation.
/// Returns true if any list changed on disk.
fn rewrite_all_project_lists<F>(ci_path: &Path, updater: F) -> bool
where
    F: Fn(&mut Vec<String>) -> bool + Copy,
{
    let text = read_to_string(ci_path);
    let mut lines: Vec<String> = text.lines().map(|s| s.to_string()).collect();
    let mut any_changed = false;

    // We scan all lines; when we see "project:" under some strategy.matrix, we normalize it.
    let mut i = 0usize;
    while i < lines.len() {
        if is_project_key(&lines[i]) {
            // Capture indentation and the form (inline vs block)
            let project_indent = leading_spaces(&lines[i]);
            let after_colon = lines[i].splitn(2, ':').nth(1).unwrap_or("").trim();

            // Collect current items from either inline array or following dash block.
            let (mut items, next_index, detected_item_indent) = if after_colon.starts_with('[') {
                // Inline: project: [ ... ]
                let (items, consumed) = parse_inline_yaml_string_array(after_colon, &lines, i);
                let item_indent = project_indent + 2;
                (items, consumed, item_indent)
            } else {
                // Block form: dash list indented > project line
                parse_dash_block(&lines, i + 1, project_indent)
            };

            // Skip if not actually a list; advance.
            if next_index == i + 1 && items.is_empty() && after_colon.is_empty() {
                i += 1;
                continue;
            }

            // Let caller update items; then normalize (sort+dedup).
            let mut changed_here = updater(&mut items);
            let before_norm = items.clone();
            items.sort();
            items.dedup();
            if items != before_norm {
                changed_here = true;
            }

            if changed_here {
                // Re-render as dash list:
                //   project:
                //     - item
                let item_indent_str = " ".repeat(detected_item_indent);
                let mut new_block: Vec<String> = Vec::with_capacity(1 + items.len());
                // Rewrite the "project:" line to have no trailing content, preserving indent
                new_block.push(format!("{}project:", " ".repeat(project_indent)));
                for it in &items {
                    new_block.push(format!("{item_indent_str}- {it}"));
                }

                // Replace original lines: from current "project:" line through the consumed lines
                lines.splice(i..next_index, new_block.into_iter());
                any_changed = true;
            }

            // Advance i to after the (possibly) inserted project block
            i += 1; // past "project:"
            while i < lines.len() && lines[i].trim_start().starts_with("- ") {
                i += 1;
            }
            continue;
        }

        i += 1;
    }

    if any_changed {
        write_string(ci_path, lines.join("\n"));
    }
    any_changed
}

/// True if the line looks like a YAML "project:" key (ignoring leading spaces).
fn is_project_key(line: &str) -> bool {
    let trimmed = line.trim_start();
    // Exact key "project:" (avoid matching "projects:")
    trimmed.starts_with("project:") && trimmed.chars().nth("project:".len()) != Some('s')
}

/// Count leading spaces.
fn leading_spaces(s: &str) -> usize {
    s.chars().take_while(|c| *c == ' ').count()
}

/// Parse an inline YAML array like `[ "a", "b" ]` that appears after `project:`.
/// Returns (items, next_index) where next_index is the line after the current one.
/// If the inline list spans multiple lines, we handle `[...]` across lines using bracket matching.
fn parse_inline_yaml_string_array(
    tail_after_colon: &str,
    lines: &[String],
    idx: usize,
) -> (Vec<String>, usize) {
    // Same-line full `[...]`?
    let mut buf = tail_after_colon.to_string();
    let mut consumed_to = idx + 1;

    if !(tail_after_colon.contains('[') && tail_after_colon.contains(']')) {
        // Accumulate subsequent lines until matching ']'
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

    // Now `buf` should contain something starting with '[' and ending with ']'.
    let mut items = Vec::new();
    if let Some(open_rel) = buf.find('[') {
        if let Some(close_rel) = find_matching_bracket(buf.as_str(), open_rel) {
            let inner = &buf[(open_rel + 1)..close_rel];
            items = inner
                .split(',')
                .map(|t| t.trim())
                .filter(|t| !t.is_empty())
                .map(|t| t.trim_matches('"').to_string())
                .collect();
        }
    }

    (items, consumed_to)
}

/// Parse a dash-list block that starts after the given `project:` line.
/// Returns (items, next_index, item_indent)
fn parse_dash_block(
    lines: &[String],
    mut j: usize,
    project_indent: usize,
) -> (Vec<String>, usize, usize) {
    let mut items = Vec::new();
    let mut item_indent: Option<usize> = None;

    while j < lines.len() {
        let line = &lines[j];
        if line.trim().is_empty() {
            j += 1;
            continue;
        }
        let lead = leading_spaces(line);
        if lead <= project_indent {
            // we've left the project block
            break;
        }
        let trimmed = line.trim_start();
        if trimmed.starts_with("- ") {
            if item_indent.is_none() {
                item_indent = Some(lead);
            }
            let val = trimmed.trim_start_matches("- ").trim().to_string();
            if !val.is_empty() {
                items.push(val);
            }
            j += 1;
        } else {
            // not a dash-list entry => stop block
            break;
        }
    }

    let deduced_item_indent = item_indent.unwrap_or(project_indent + 2);
    (items, j, deduced_item_indent)
}
