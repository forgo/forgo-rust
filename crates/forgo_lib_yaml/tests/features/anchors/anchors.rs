//! Anchor and Alias Tests
//!
//! **Related YAML 1.2.2 Spec Sections:**
//! - §3.2.2.2: Anchors and Aliases in serialization tree
//! - §6.9.2: Node Anchors (`&anchor` and `*alias`)
//! - §7.1: Alias Nodes in flow style
//! - §10.1.1.1-3: Merge key `<<` (Failsafe schema extension)
//!
//! **Purpose:**
//! These tests validate anchor definition, alias resolution, and merge key
//! behavior. The spec tests in `spec/ch_6_structural.rs` cover basic anchor
//! syntax; these tests validate the full anchor/alias mechanism including
//! merge keys and round-trip preservation.

use forgo_lib_yaml::{Doc, Node, Scalar};

fn doc(s: &str) -> Doc {
    Doc::from_str(s).expect("parse failed")
}

#[test]
fn anchor_and_merge_alias_structure_and_roundtrip() {
    // No inline maps or multi-merge array here; we stick to the supported subset.
    let input = r#"
defaults: &def
  a: 1
  b: 2

use_def:
  <<: *def
"#;

    let d1 = doc(input);

    // --- Structure checks on the parsed AST ---
    let Node::Map(root) = d1.root().node() else {
        panic!("root not a map");
    };

    // defaults: &def { a: 1, b: 2 }
    let (_, defaults) = root
        .iter()
        .find(|(k, _)| k == "defaults")
        .expect("defaults missing");
    assert_eq!(
        defaults.meta.anchor.as_deref(),
        Some("def"),
        "anchor not attached to 'defaults' value"
    );
    let Node::Map(def_map) = defaults.node() else {
        panic!("defaults not a map");
    };
    assert!(
        def_map.iter().any(|(k, v)| {
            k == "a" && matches!(v.node(), Node::Scalar(Scalar::Num { text }) if text == "1")
        }),
        "defaults.a != 1"
    );
    assert!(
        def_map.iter().any(|(k, v)| {
            k == "b" && matches!(v.node(), Node::Scalar(Scalar::Num { text }) if text == "2")
        }),
        "defaults.b != 2"
    );

    // use_def:
    //   <<: *def
    let (_, use_def) = root
        .iter()
        .find(|(k, _)| k == "use_def")
        .expect("use_def missing");
    let Node::Map(merge_map) = use_def.node() else {
        panic!("use_def not a map");
    };
    let (mk, mval) = merge_map
        .iter()
        .find(|(k, _)| k == "<<")
        .expect("merge key '<<' missing");
    assert_eq!(mk, "<<");
    assert!(
        matches!(mval.node(), Node::Alias(name) if name == "def"),
        "merge value not alias *def"
    );

    // --- Emission checks ---
    let out = d1.to_string().expect("emit");
    // Anchor appears
    assert!(out.contains("&def"), "emitter lost anchor &def");
    // Merge is in inline form
    assert!(
        out.contains("<<: *def"),
        "emitter did not inline merge alias as `<<: *def`"
    );

    // --- Roundtrip equality ---
    let d2 = doc(&out);
    assert_eq!(d1, d2, "roundtrip changed AST\n--- out ---\n{out}");
}

#[test]
fn alias_as_regular_value_is_inlined() {
    // Alias used as a normal map value (not a merge key) should be emitted inline: `ref: *def`.
    let input = r#"
defaults: &def
  a: 1

ref: *def
"#;

    let d = doc(input);

    // Verify structure: ref -> Alias("def")
    let Node::Map(root) = d.root().node() else {
        panic!("root not a map");
    };
    let (_, r) = root.iter().find(|(k, _)| k == "ref").expect("ref missing");
    assert!(
        matches!(r.node(), Node::Alias(n) if n == "def"),
        "ref is not alias *def"
    );

    // Emission check: inline alias form
    let out = d.to_string().unwrap();
    assert!(
        out.contains("ref: *def"),
        "alias value not emitted inline:\n{out}"
    );
}

#[test]
fn test_26dv_alias_as_implicit_key() {
    // Test case 26DV: Aliases should be valid as implicit keys
    // This was previously failing with "flow collections cannot be used as implicit keys"
    let input = r#"top1:
  key1: &alias1 scalar1
top2:
  *alias1 : scalar2
"#;

    // The main goal is that this parses successfully
    let d = doc(input);

    // Verify basic structure
    let Node::Map(root) = d.root().node() else {
        panic!("root not a map");
    };

    // Check top1 contains the anchor
    let (_, top1) = root.iter().find(|(k, _)| k == "top1").expect("top1 missing");
    let Node::Map(top1_map) = top1.node() else {
        panic!("top1 not a map");
    };
    let (_, key1_val) = top1_map.iter().find(|(k, _)| k == "key1").expect("key1 missing");
    assert_eq!(key1_val.meta.anchor.as_deref(), Some("alias1"), "anchor not attached");

    // Check top2 exists - it should parse successfully with alias as key
    let (_k, _v) = root.iter().find(|(k, _)| k == "top2").expect("top2 missing");
    // The fact that it parsed is the main test - aliases as keys are now allowed
}
