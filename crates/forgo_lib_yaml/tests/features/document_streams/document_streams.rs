//! Document Stream Tests
//!
//! **Related YAML 1.2.2 Spec Sections:**
//! - §9: Document Stream Productions
//! - §9.1: Documents (bare, explicit, with directives)
//! - §9.1.1: Document Prefix (BOM)
//! - §9.1.2: Document Markers (`---` and `...`)
//! - §9.2: Streams (multiple documents)
//! - §6.8: Directives (`%YAML` and `%TAG`)
//!
//! **Purpose:**
//! These tests validate multi-document stream handling, document markers,
//! and directive preservation. The spec tests in `spec/ch_9_documents.rs`
//! cover basic document syntax; these tests validate implementation-specific
//! behavior like directive round-tripping and marker preservation.

use forgo_lib_yaml::Doc;

#[test]
fn parse_yaml_and_tag_directives_then_docstart() {
    let s = "\
%YAML 1.2
%TAG !e! tag:example.com,2000:app/
---
key: val
";
    let d = Doc::from_str(s).expect("parse");
    assert_eq!(d.yaml_version(), Some((1, 2)));
    assert_eq!(d.tag_handles().len(), 1);
    assert_eq!(d.tag_handles()[0].0, "!e!");
    assert_eq!(d.tag_handles()[0].1, "tag:example.com,2000:app/");
    assert!(d.explicit_start());
    assert!(!d.explicit_end());

    let out = Doc::to_stream_string(&[d]).expect("emit");
    assert!(out.contains("%YAML 1.2"));
    assert!(out.contains("%TAG !e! tag:example.com,2000:app/"));
    assert!(out.contains("---"));
    assert!(out.contains("key: val"));
}

#[test]
fn parse_tag_directives_no_yaml_version() {
    let s = "%TAG !foo! tag:example.org,2001:foo/\n---\n!foo!bar baz\n";
    let d = Doc::from_str(s).expect("parse");
    assert_eq!(d.yaml_version(), None);
    assert_eq!(d.tag_handles().len(), 1);
    assert!(d.explicit_start());
    // Emission sanity
    let out = Doc::to_stream_string(&[d]).expect("emit");
    assert!(out.contains("%TAG !foo! tag:example.org,2001:foo/"));
}

#[test]
fn single_doc_without_markers_behaves_like_before() {
    let s = "a: b\n";
    let d = Doc::from_str(s).expect("parse");
    assert_eq!(d.yaml_version(), None);
    assert!(!d.explicit_start());
    let out = d.to_string().expect("emit");
    assert_eq!(out, "a: b\n"); // back-compat: no --- injected
}

#[test]
fn parse_multi_document_stream_with_start_and_end() {
    let s = "\
%YAML 1.2
--- # first
a: 1
...
--- # second
b: 2
";
    let docs = Doc::from_stream(s).expect("parse stream");
    assert_eq!(docs.len(), 2);

    assert_eq!(docs[0].yaml_version(), Some((1, 2)));
    assert!(docs[0].explicit_start());
    assert!(docs[0].explicit_end());

    assert_eq!(docs[1].yaml_version(), None);
    assert!(docs[1].explicit_start());
    assert!(!docs[1].explicit_end());

    let out = Doc::to_stream_string(&docs).expect("emit stream");
    // crude but stable checks
    assert!(out.contains("%YAML 1.2"));
    assert!(out.contains("---"));
    assert!(out.contains("a: 1"));
    assert!(out.contains("b: 2"));
}

#[test]
fn directives_with_blank_lines_between() {
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
