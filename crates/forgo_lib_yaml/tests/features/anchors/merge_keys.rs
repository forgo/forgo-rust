use forgo_lib_yaml::Doc;

#[test]
fn merge_key_roundtrips_as_plain() {
    let s = "base: &b {a: 1}\nobj:\n <<: *b\n a: 2\n";
    let d = Doc::from_str(s).unwrap();
    let out = d.to_string().unwrap();
    assert!(out.contains("<<: *b")); // no semantics, just round-trip
}
