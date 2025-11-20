//! AST Integration Tests
//!
//! **Related YAML 1.2.2 Spec Sections:**
//! - §3.2: Information Models (representation graph, serialization tree, presentation stream)
//! - §6: Structural Productions (overall YAML structure)
//! - §7-8: Flow and Block Styles (concrete syntax)
//!
//! **Purpose:**
//! These integration tests validate complete document handling through the full
//! parse → AST manipulation → emit cycle. They test end-to-end workflows combining
//! lexer, parser, AST, and emitter components. Unlike unit tests that test components
//! in isolation, these validate that the system works correctly as a whole.

use forgo_lib_yaml::{Doc, Node, Scalar, needs_quotes};

#[test]
fn glued_colon_isnt_split_in_seq_item() {
    let d = Doc::from_str("- k:v\n- x\n").unwrap();
    let out = d.to_string().unwrap();
    // should remain a single scalar "k:v"
    let want = "- k:v";
    assert!(out.lines().any(|l| l == want));
}

#[test]
fn anchor_and_comment_on_block_header() {
    let s = "a: &anc | # hdr\n  line\n";
    let d = Doc::from_str(s).unwrap();

    // --- dump parsed structure for key "a"
    {
        let root = d.root();
        eprintln!(
            "PARSED root meta: anchor={:?} prefer_block={} block_style={:?} trailing={:?} leading={:?}",
            root.meta.anchor,
            root.meta.prefer_block,
            root.meta.block_style,
            root.meta.trailing_comment,
            root.meta.leading_comments
        );
        match &root.node {
            Node::Map(entries) => {
                for (k, v) in entries {
                    eprintln!(
                        "ENTRY key={:?} meta: anchor={:?} prefer_block={} block_style={:?} trailing={:?} leading={:?}",
                        k,
                        v.meta.anchor,
                        v.meta.prefer_block,
                        v.meta.block_style,
                        v.meta.trailing_comment,
                        v.meta.leading_comments
                    );
                    match &v.node {
                        Node::Scalar(Scalar::Str(body)) => {
                            eprintln!(
                                "  SCALAR body (len={}): {:?}",
                                body.len(),
                                body.replace('\n', "\\n")
                            );
                        }
                        other => eprintln!("  VALUE node kind: {:?}", other),
                    }
                }
            }
            other => eprintln!("ROOT node kind: {:?}", other),
        }
    }

    let out = d.to_string().unwrap();
    // assert!(out.starts_with("a: &anc |"));
    // assert!(out.contains("# hdr"));
    // assert!(out.contains("\n  line\n"));
    eprintln!("EMITTED (debug):\n{out:?}\n");
    // Split assertions so we know exactly which piece failed.
    assert!(out.starts_with("a: &anc |"), "header missing: {:?}", out);
    assert!(
        out.contains("# hdr"),
        "header trailing comment missing: {:?}",
        out
    );
    assert!(
        out.contains("\n  line\n"),
        "block body line missing or wrongly indented/newlined: {:?}",
        out
    );
}

#[test]
fn unicode_doesnt_break_needs_quotes() {
    let d = Doc::from_str("k: café\n").unwrap();
    let out = d.to_string().unwrap();
    // café should not need quotes
    assert!(out.contains("k: café"));
}

#[test]
fn colon_without_space_is_plain() {
    let d = Doc::from_str("- k:v\n").unwrap();
    let out = d.to_string().unwrap();
    let want = "- k:v";
    assert!(out.lines().any(|l| l == want));
}

#[test]
fn colon_with_space_requires_quotes() {
    let d = Doc::from_str("- \"k: v\"\n").unwrap();
    let out = d.to_string().unwrap();
    // Emission may choose single or double quotes; just assert it's quoted.
    let line = out
        .lines()
        .find(|l| l.trim_start().starts_with("- "))
        .unwrap();
    assert!(line.contains("\"k: v\"") || line.contains("'k: v'"));
}

#[test]
fn unicode_plain_is_ok() {
    let d = Doc::from_str("k: café\n").unwrap();
    let out = d.to_string().unwrap();
    assert!(out.contains("k: café"));
}

#[test]
fn leading_space_forces_quotes() {
    let d = Doc::from_str("k: \" leading\"\n").unwrap();
    let out = d.to_string().unwrap();
    assert!(out.contains("k: \" leading\"") || out.contains("k: ' leading'"));
}

#[test]
fn inline_comment_hazard_forces_quotes() {
    let d = Doc::from_str("k: \"x # not a comment\"\n").unwrap();
    let out = d.to_string().unwrap();
    assert!(out.contains("\"x # not a comment\"") || out.contains("'x # not a comment'"));
}

#[test]
fn needs_quotes_colon_rules() {
    assert!(!needs_quotes("k:v")); // glued colon ok
    assert!(needs_quotes("k: v")); // colon + space must quote
    assert!(needs_quotes("key:")); // trailing colon must quote
}

#[test]
fn debug_emit_kv_glued() {
    let d = forgo_lib_yaml::Doc::from_str("- k:v\n- x\n").unwrap();
    let out = d.to_string().unwrap();
    eprintln!("EMITTED:\n{out:?}\n");
    let want = "- k:v";
    assert!(out.lines().any(|l| l == want));
}
