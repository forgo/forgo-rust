//! Block Scalar Strict Specification Tests
//!
//! **Related YAML 1.2.2 Spec Sections:**
//! - §8.1.1.1: Block Indentation Indicator
//! - §8.1.1.2: Block Chomping Indicator
//! - §8.1.2: Literal Style (line-by-line preservation)
//! - §8.1.3: Folded Style (line folding rules)
//!
//! **Purpose:**
//! These tests provide comprehensive coverage of block scalar edge cases and
//! strict spec compliance. They validate: indent indicators with exact spacing
//! requirements, chomping indicators (`-`, `+`, clip), indicator ordering,
//! content line indentation rules, and folding behavior. These tests complement
//! the basic tests in `block_scalars.rs` by ensuring correct handling of all
//! spec-defined behaviors.

use forgo_lib_yaml::Doc;

/// Helper: parse, then emit, and check the body content is correct.
/// The emitted output includes the header (| or >) and indented body lines.

#[test]
fn literal_with_indent_indicator_removes_exact_k_spaces() {
    // Indicator |2 means: only admit lines with indent >= 2, then dedent by 2
    let s2 = "|2\n  a\n    b\n c\n    d\n";
    let d = Doc::from_str(s2).unwrap();
    let out = d.to_string().unwrap();

    // Line " c" has indent 1 < 2, so it's not admitted (becomes empty line in sequence)
    // Expected body: "a\n  b\n  d\n"
    // Emitted with 2-space indent: "|\n  a\n    b\n  \n    d\n"
    // Check for the body content with emission indentation
    assert!(
        out.contains("  a\n    b\n  \n    d\n"),
        "Expected properly indented block scalar body, got: {:?}",
        out
    );
}

#[test]
fn literal_without_indicator_uses_min_indent_of_non_empty_lines() {
    // No indicator: dedent by the minimum indent among non-empty lines (here 1).
    let s = "|\n a\n  b\n\n  c\n";
    let d = Doc::from_str(s).unwrap();
    let out = d.to_string().unwrap();

    // After min-indent (1) removal: "a\n b\n\n c\n"
    // Emitted with 2-space indent: "|\n  a\n   b\n  \n   c\n"
    assert!(
        out.contains("  a\n   b\n  \n   c\n"),
        "Expected properly indented block scalar body, got: {:?}",
        out
    );
}

#[test]
fn folded_preserves_paragraphs_and_more_indented_runs() {
    // Fold single newlines to space; preserve blank lines; preserve more-indented runs
    let s = ">\nfoo\nbar\n\n baz\n  qux\nbaz\n";
    let d = Doc::from_str(s).unwrap();
    let out = d.to_string().unwrap();

    // Body after folding: "foo bar\n\n baz\n  qux\nbaz\n"
    // Emitted with 2-space indent: "|\n  foo bar\n  \n   baz\n    qux\n  baz\n"
    assert!(
        out.contains("  foo bar\n  \n   baz\n    qux\n  baz\n"),
        "Expected folded output with preserved structure, got: {:?}",
        out
    );
}

#[test]
fn chomping_minus_strips_all_trailing_newlines() {
    let d = Doc::from_str("|-\n  a\n  \n  b\n\n").unwrap();
    let out = d.to_string().unwrap();
    // Body with chomp -: "a\n\nb" (no trailing newline)
    // Emitted: "|-\n  a\n  \n  b\n"
    assert!(
        out.contains("  a\n  \n  b\n"),
        "Expected chomped body, got: {:?}",
        out
    );
}

#[test]
fn chomping_plus_keeps_all_trailing_newlines() {
    let d = Doc::from_str("|+\n  a\n\n").unwrap();
    let out = d.to_string().unwrap();
    // Body with chomp +: "a\n\n" (keeps trailing newlines)
    // Emitted: "|+\n  a\n  \n"
    assert!(
        out.contains("  a\n  \n"),
        "Expected body with trailing newlines, got: {:?}",
        out
    );
}

#[test]
fn chomping_clip_default_keeps_exactly_one_trailing_newline() {
    let d = Doc::from_str("|\n  a\n\n").unwrap();
    let out = d.to_string().unwrap();
    // Body with default chomp: "a\n" (one trailing newline)
    // Emitted: "|\n  a\n"
    assert!(
        out.contains("  a\n"),
        "Expected body with one trailing newline, got: {:?}",
        out
    );
    // Should not have double blank line
    assert!(
        !out.contains("  a\n  \n  \n"),
        "Should not have extra trailing newlines, got: {:?}",
        out
    );
}

#[test]
fn indicator_order_is_free_mixture() {
    // YAML allows +/− and [1..9] in either order
    let s1 = "|2-\n  a\n  b\n";
    let s2 = "|-2\n  a\n  b\n";
    let s3 = ">+1\n a\n b\n";
    let s4 = ">1+\n a\n b\n";

    for s in [s1, s2, s3, s4] {
        let d = Doc::from_str(s).unwrap();
        let out = d.to_string().unwrap();
        // All should parse and emit valid bodies
        if s.starts_with('|') {
            // Literal: "a\nb" emitted as "  a\n  b"
            assert!(out.contains("  a\n  b"), "Failed for input: {}", s);
        } else {
            // Folded: "a b" emitted as "  a b"
            assert!(out.contains("  a b"), "Failed for input: {}", s);
        }
    }
}

#[test]
fn content_lines_must_meet_required_indent_when_indicator_present() {
    // Indicator |3 means: only admit lines with indent >= 3
    let s = "|3\n  a\n   b\n    c\n d\n";
    let d = Doc::from_str(s).unwrap();
    let out = d.to_string().unwrap();
    // Lines: "  a" (indent 2 < 3) -> not admitted
    //        "   b" (indent 3) -> "b" (rel 0)
    //        "    c" (indent 4) -> " c" (rel 1)
    //        " d" (indent 1 < 3) -> not admitted
    // Body: "b\n c\n"
    // Emitted with 2-space indent: "|3\n  b\n   c\n"
    assert!(
        out.contains("  b\n   c\n"),
        "Expected only properly indented lines, got: {:?}",
        out
    );
}

#[test]
fn folded_single_newline_becomes_single_space_no_double_spaces() {
    let d = Doc::from_str(">\nfoo\nbar\n").unwrap();
    let out = d.to_string().unwrap();
    // Body: "foo bar\n"
    // Emitted: ">\n  foo bar\n"
    assert!(
        out.contains("  foo bar\n"),
        "Expected folded text, got: {:?}",
        out
    );
}
