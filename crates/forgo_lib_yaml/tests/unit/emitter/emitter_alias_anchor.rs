// tests/emitter_alias_anchor.rs
use forgo_lib_yaml::Doc;

fn doc(s: &str) -> Doc {
    Doc::from_str(s).expect("parse")
}

#[test]
fn inline_alias_as_map_value_is_emitted_inline() {
    let d = doc("def: &a 1\nref: *a\n");
    let out = d.to_string().unwrap();
    assert!(out.contains("ref: *a"), "{out}");
}

#[test]
fn anchor_on_seq_item_and_alias_roundtrip() {
    let d = doc("- &x 1\n- *x\n");
    let out = d.to_string().unwrap();
    assert!(out.contains("- &x 1"), "{out}");
    assert!(out.contains("- *x"), "{out}");
    // roundtrip stable
    let d2 = Doc::from_str(&out).unwrap();
    assert_eq!(d, d2);
}

#[test]
fn merge_key_is_emitted_as_inline_merge() {
    let d = doc("base: &def\n  a: 1\n  b: 2\n\nobj:\n  <<: *def\n  c: 3\n");
    let out = d.to_string().unwrap();
    assert!(out.contains("<<: *def"), "{out}");
}
