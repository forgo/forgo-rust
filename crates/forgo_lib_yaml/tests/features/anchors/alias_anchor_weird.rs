use forgo_lib_yaml::{Doc, Node};

fn doc(s: &str) -> Doc {
    Doc::from_str(s).expect("parse failed")
}

#[test]
fn anchor_on_alias_item_roundtrip() {
    let input = "- &x *x\n";

    // Parse
    let d1 = doc(input);

    // Structure: root is a seq with one Elem; that Elem has anchor "x" and Node::Alias("x")
    let Node::Seq(items) = d1.root().node() else {
        panic!("root not a seq");
    };
    assert_eq!(items.len(), 1, "expected single sequence item");
    let it = &items[0];
    assert_eq!(
        it.meta.anchor.as_deref(),
        Some("x"),
        "anchor missing on alias item"
    );
    match it.node() {
        Node::Alias(name) => assert_eq!(name, "x", "alias name wrong"),
        other => panic!("expected alias node, got {:?}", other),
    }

    // Emit and check exact inline form
    let out = d1.to_string().expect("emit");
    assert!(
        out.contains("- &x *x"),
        "did not emit inline alias with anchor:\n{out}"
    );

    // Roundtrip equality
    let d2 = doc(&out);
    assert_eq!(d1, d2, "roundtrip changed AST\n--- out ---\n{out}");
}
