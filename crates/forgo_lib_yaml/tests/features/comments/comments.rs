//! Comment Preservation Tests
//!
//! **Related YAML 1.2.2 Spec Sections:**
//! - §3.2.3.3: Comments in presentation stream
//! - §6.6: Comment syntax and placement
//! - Throughout §7 (Flow Styles): Comments in flow collections
//! - Throughout §8 (Block Styles): Comments in block collections
//!
//! **Purpose:**
//! These tests validate that comments are preserved during parse/emit cycles
//! across all YAML constructs. While the spec defines comment syntax (§6.6),
//! comment *preservation* is an implementation quality goal beyond basic
//! spec compliance. The spec tests in `spec/ch_6_structural.rs` validate
//! basic comment syntax; these tests validate preservation behavior.

use forgo_lib_yaml::Doc;

fn show(tag: &str, s: &str) {
    println!("--- {} (visible) ---", tag);
    let vis: String = s.chars().map(|c| if c == ' ' { '·' } else { c }).collect();
    println!("{}", vis);
    println!("\n--- BYTES ---");
    println!(
        "{}",
        s.as_bytes()
            .iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<_>>()
            .join(" ")
    );
    println!();
}

#[test]
fn preserves_leading_and_trailing_comments() {
    let input = r#"
# top banner
root: # trailing on root line ignored (root is a map)
  # before key
  key: value  # after value
  list:
    # before item
    - a  # after item a
    - b
"#;
    let d = Doc::from_str(input).unwrap();
    let out = d.to_string().unwrap();
    show("EMITTED YAML", &out);

    // Leading comment survives
    assert!(out.starts_with("# top banner"));

    // Inline trailing comment after value preserved on same line as the value
    assert!(out.contains(r#"key: "value" # after value"#));

    // Comment before item appears above it; trailing on item a preserved
    assert!(out.contains("# before item"));
    assert!(out.contains("- a # after item a"));
}

#[test]
fn comment_block_before_scalar_top_level() {
    let input = r#"
# hello
"abc"
"#;
    let d = Doc::from_str(input).unwrap();
    let s = d.to_string().unwrap();
    assert!(s.contains("# hello"));
    assert!(s.contains(r#""abc""#));
}

#[test]
fn quoted_scalar_top_level_with_inline_comment() {
    let input = r#"
# banner
"abc" # note
"#;
    let d = Doc::from_str(input).unwrap();
    let out = d.to_string().unwrap();
    assert!(out.starts_with("# banner"));
    assert!(out.contains(r#""abc" # note"#));
}

#[test]
fn unquoted_scalar_top_level_stays_unquoted() {
    let input = r#"
abc
"#;
    let d = Doc::from_str(input).unwrap();
    let out = d.to_string().unwrap();
    // Should NOT force quotes when not originally quoted
    assert!(out.trim() == "abc");
}

#[test]
fn quoted_scalar_as_seq_item_preserves_quotes() {
    let input = r#"
- "abc"
- def
"#;
    let d = Doc::from_str(input).unwrap();
    let out = d.to_string().unwrap();
    assert!(out.contains(r#"- "abc""#));
    assert!(out.contains("- def"));
}

#[test]
fn leading_and_trailing_comment_force_quotes_for_map_value() {
    // To test that a value with both leading and trailing comments gets quoted,
    // we need a structure where the leading comment is actually on the VALUE,
    // not on the map itself. This requires nesting.
    let d = Doc::from_str("root:\n  # before value\n  key: value # after value\n").unwrap();
    let out = d.to_string().unwrap();
    assert!(out.contains(r#"key: "value" # after value"#), "{out}");
}
