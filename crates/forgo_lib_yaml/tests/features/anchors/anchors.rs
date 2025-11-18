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
