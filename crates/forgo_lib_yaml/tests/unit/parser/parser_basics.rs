//! Core Parser Functionality Tests
//!
//! **Related YAML 1.2.2 Spec Sections:**
//! - §6: Basic Structures
//! - §7: Flow Styles
//! - §8: Block Styles
//! - §9: Document Stream Productions
//!
//! **Purpose:**
//! These are unit tests for core parser functionality. They test implementation-specific
//! behavior and edge cases not explicitly covered by the spec. Unlike spec
//! tests which validate compliance, these tests validate internal correctness
//! and error handling.

use forgo_lib_yaml::{Doc, Elem, Error, Node, Scalar};

fn doc(s: &str) -> Doc {
    Doc::from_str(s).expect("parse")
}

#[test]
fn parse_simple_map() {
    let d = doc("a: b\nc: d\n");

    let Node::Map(m) = d.root().node() else {
        panic!("root not map");
    };
    assert_eq!(m.len(), 2);

    assert_eq!(m[0].0, "a");
    assert_eq!(m[1].0, "c");

    // map values are Elem; read through .node().as_str()
    assert_eq!(m[0].1.node().as_str(), Some("b"));
    assert_eq!(m[1].1.node().as_str(), Some("d"));
}

#[test]
fn parse_inline_array_under_key() {
    let d = doc("arr: [a, b, c]\n");

    let Node::Map(m) = d.root().node() else {
        panic!("root not map");
    };
    assert_eq!(m[0].0, "arr");

    let Node::Seq(items) = m[0].1.node() else {
        panic!("arr not seq");
    };
    let vals: Vec<_> = items.iter().map(|e| e.node().as_str().unwrap()).collect();
    assert_eq!(vals, vec!["a", "b", "c"]);
}

#[test]
fn parse_dash_list_block() {
    let d = doc("list:\n  - one\n  - two\n");

    let Node::Map(m) = d.root().node() else {
        panic!("root not map");
    };
    assert_eq!(m[0].0, "list");

    let Node::Seq(items) = m[0].1.node() else {
        panic!("list not seq");
    };
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].node().as_str(), Some("one"));
    assert_eq!(items[1].node().as_str(), Some("two"));
}

#[test]
fn parse_scalar_fallback_in_seq() {
    // A "k:v" on a dash line is parsed as a plain scalar string in our subset
    let d = doc("- k:v\n- plain\n");

    let Node::Seq(items) = d.root().node() else {
        panic!("root not seq");
    };
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].node().as_str(), Some("k:v"));
    assert_eq!(items[1].node().as_str(), Some("plain"));
}

#[test]
fn block_scalar_literal_pipe() {
    let d = doc("msg: |\n  line 1\n  line 2\n");

    let Node::Map(m) = d.root().node() else {
        panic!("root not map");
    };
    assert_eq!(m[0].0, "msg");

    match m[0].1.node() {
        Node::Scalar(Scalar::Str(s)) => {
            // literal keeps newlines; emitter will use '|' on output
            assert!(s.contains("line 1"));
            assert!(s.contains("line 2"));
        }
        other => panic!("expected Scalar::Str, got {other:?}"),
    }
}

#[test]
fn typed_scalars_in_map() {
    let d = doc("t: true\nn: -42\nz: null\n");

    let Node::Map(m) = d.root().node() else {
        panic!("root not map");
    };
    // Find by key for clarity
    let find = |k: &str| -> &Elem { &m.iter().find(|(kk, _)| kk == k).unwrap().1 };

    match find("t").node() {
        Node::Scalar(Scalar::Bool(true)) => {}
        x => panic!("expected bool(true), got {x:?}"),
    }
    match find("n").node() {
        Node::Scalar(Scalar::Num { text }) if text == "-42" => {}
        x => panic!("expected num -42, got {x:?}"),
    }
    match find("z").node() {
        Node::Scalar(Scalar::Null) => {}
        x => panic!("expected null, got {x:?}"),
    }
}

fn assert_parse_err(input: &str, needle: &str) {
    match Doc::from_str(input) {
        Ok(_) => panic!("expected parse error containing {needle:?}, but parsing succeeded"),
        Err(Error::Parse(msg)) => {
            assert!(
                msg.contains(needle),
                "expected parse error to contain {needle:?}, got: {msg:?}"
            );
        }
        Err(e) => panic!("expected Error::Parse, got: {e:?}"),
    }
}

#[test]
fn accept_yaml_version_1_1() {
    // Per YAML spec Example 6.14, we should accept YAML 1.1/1.3 and attempt to parse
    let s = "%YAML 1.1\n---\na: 1\n";
    let result = Doc::from_str(s);
    assert!(result.is_ok(), "Should accept YAML 1.1 and parse with 1.2 rules");
}

#[test]
fn accept_yaml_version_1_3() {
    // Per YAML spec Example 6.14, we should accept YAML 1.3 and attempt to parse
    let s = "%YAML 1.3\n---\na: 1\n";
    let result = Doc::from_str(s);
    assert!(result.is_ok(), "Should accept YAML 1.3 and parse with 1.2 rules");
}

#[test]
fn error_on_directive_after_docstart() {
    // A directive line appearing *after* '---' must be rejected.
    let s = "\
---  # start
%TAG !e! tag:example.com,2000:app/
a: 1
";
    // Your suggested message: "directives (%YAML/%TAG) must appear before document start '---'"
    assert_parse_err(s, "directives");
}

#[test]
fn tag_directive_before_docstart_is_ok() {
    let s = "\
%TAG !foo! tag:example.org,2001:foo/
---
!foo!bar baz
";
    let d = Doc::from_str(s).expect("parse");
    assert!(d.explicit_start());
    assert_eq!(d.yaml_version(), None);
    assert_eq!(d.tag_handles().len(), 1);
    assert_eq!(d.tag_handles()[0].0, "!foo!");
    assert_eq!(d.tag_handles()[0].1, "tag:example.org,2001:foo/");
}

#[test]
fn top_level_doc_end_ends_document_and_next_doc_parses() {
    // At top level, '...' ends the first doc; next '---' begins the second.
    let s = "\
a:
  b: 1
...
---  # next doc
c: 2
";
    let docs = Doc::from_stream(s).expect("parse stream");
    assert_eq!(docs.len(), 2);

    assert!(!docs[0].explicit_start()); // first doc began implicitly
    assert!(docs[0].explicit_end());

    assert!(docs[1].explicit_start());
    assert!(!docs[1].explicit_end());
}

#[test]
fn nested_doc_marker_inside_block_is_error() {
    // A '---' *inside* a block (non-zero indent) must error.
    let s = "\
a:
  b: 1
  ---
  c: 2
";

    // DEBUG: show the tokenization so we can see what the lexer is doing.
    // Expect to see ... Indent(2)  then  DocStart  then Newline ...
    #[cfg(test)]
    {
        let toks = dump_tokens(s);
        println!("--- TOKEN DUMP ---\n{toks}");
    }

    // If parsing unexpectedly succeeds, print what the parser built/emitted.
    match forgo_lib_yaml::Doc::from_str(s) {
        Ok(doc) => {
            let emitted = doc.to_string().unwrap_or_else(|_| "<emit error>".into());
            panic!(
                "expected parse error containing \"document marker not allowed inside block\", \
                 but parsing succeeded.\n\nEmitted:\n{emitted}\n"
            );
        }
        Err(forgo_lib_yaml::Error::Parse(msg)) => {
            assert!(
                msg.contains("document marker not allowed inside block"),
                "expected parse error to contain \"document marker not allowed inside block\", got: {msg:?}"
            );
        }
        Err(e) => panic!("expected Error::Parse, got: {e:?}"),
    }
}

#[test]
fn stray_doc_end_between_docs_is_ignored() {
    // Extra '...' between docs is allowed and should be ignored.
    let s = "\
a: 1
...
...
---  # second
b: 2
";
    let docs = Doc::from_stream(s).expect("parse stream");
    assert_eq!(docs.len(), 2);
    assert!(!docs[0].explicit_start());
    assert!(docs[0].explicit_end());
    assert!(docs[1].explicit_start());
}

#[test]
fn inline_array_with_explicit_sign_numbers_is_parsed_and_roundtrips_signs() {
    // Ensures + / - signs are glued correctly in inline context and kept as numbers.
    let s = "[+5, -3, 0]\n";
    let d = Doc::from_str(s).expect("parse");
    let out = d.to_string().expect("emit");
    // Be tolerant about whitespace, but signs should still be present.
    assert!(out.contains("+5"), "emitted text missing +5: {out:?}");
    assert!(out.contains("-3"), "emitted text missing -3: {out:?}");
}

#[test]
fn directives_separated_by_blank_lines_are_accepted() {
    // %YAML followed by blank lines, then %TAG, then --- is valid.
    let s = "\
%YAML 1.2

%TAG !e! tag:example.com,2000:app/
---
key: val
";
    let d = Doc::from_str(s).expect("parse");
    assert_eq!(d.yaml_version(), Some((1, 2)));
    assert_eq!(d.tag_handles().len(), 1);
    assert!(d.explicit_start());
}

fn dump_tokens(input: &str) -> String {
    use forgo_lib_yaml::{Lexer, Tok}; // <— from crate root, not ::lexer
    let mut lx = Lexer::new(input);
    let mut s = String::new();
    let mut i = 0usize;
    loop {
        let t = lx.next_token();
        s.push_str(&format!("{:02}: {:?}\n", i, t));
        if matches!(t, Tok::Eof) {
            break;
        }
        i += 1;
    }
    s
}
