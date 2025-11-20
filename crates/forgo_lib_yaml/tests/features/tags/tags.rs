//! Tag Tests
//!
//! **Related YAML 1.2.2 Spec Sections:**
//! - §3.2.1.2: Tags in representation graph
//! - §6.8.2: TAG Directives (`%TAG`)
//! - §6.8.2.1: Tag Handles (`!`, `!!`, `!e!`)
//! - §6.8.2.2: Tag Prefixes (verbatim tags like `!<...>`)
//! - §6.9.1: Node Tags (explicit tag syntax)
//!
//! **Purpose:**
//! These tests validate tag directive parsing, tag handle resolution,
//! and tag preservation during round-trip. The spec tests in
//! `spec/ch_6_structural.rs` cover basic tag syntax; these tests
//! validate complete tag system behavior including directives.

use forgo_lib_yaml::Doc;

#[test]
fn simple_short_and_verbatim_tags_parse_and_emit() {
    let s = "%YAML 1.2\n%TAG !e! tag:example.com,2000:app/\n---\n- !e!thing 1\n- !!str foo\n- !<tag:example.com,2000:app/other> bar\n";
    let docs = Doc::from_stream(s).unwrap();
    let out = Doc::to_stream_string(&docs).unwrap();
    assert!(out.contains("!e!thing"));
    assert!(out.contains("!!str"));
    assert!(out.contains("!<tag:example.com,2000:app/other>"));
}
