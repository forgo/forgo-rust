// tests/emitter_scalars.rs
use forgo_lib_yaml::Doc;

#[test]
fn numeric_text_is_preserved_verbatim() {
    // Leading zeros should not be normalized by emitter; we keep original text.
    let d = Doc::from_str("n: 0012\n").unwrap();
    // Re-emit and ensure we didn't turn it into "12"
    let out = d.to_string().unwrap();
    assert!(out.contains("n: 0012"), "{out}");
}

#[test]
fn booleans_and_null_emit_canonically() {
    let d = Doc::from_str("t: true\nf: false\nz: null\n").unwrap();
    let out = d.to_string().unwrap();
    assert!(out.contains("t: true"));
    assert!(out.contains("f: false"));
    assert!(out.contains("z: null"));
}
