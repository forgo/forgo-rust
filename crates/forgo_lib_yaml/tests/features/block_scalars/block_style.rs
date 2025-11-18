use forgo_lib_yaml::Doc;

#[test]
fn preserves_literal_and_chomp() {
    // trailing + should be preserved on re-emit
    let s = "k: |+\n  a\n  b\n";
    let d = Doc::from_str(s).unwrap();
    let out = d.to_string().unwrap();
    assert!(out.starts_with("k: |+"));
    assert!(out.contains("\n  a\n"));
    assert!(out.contains("\n  b\n"));
}

#[test]
fn preserves_folded_header() {
    let s = "k: >-\n  a\n  b\n";
    let d = Doc::from_str(s).unwrap();
    let out = d.to_string().unwrap();
    assert!(out.starts_with("k: >-"), "got:\n{out}");
}
