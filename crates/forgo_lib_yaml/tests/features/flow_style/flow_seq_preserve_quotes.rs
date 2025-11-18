use forgo_lib_yaml::Doc;

#[test]
fn preserves_quotes_in_flow_seq_items() {
    let d = Doc::from_str("l: [\"01\", \"true\", 3]\n").unwrap();
    let out = d.to_string().unwrap();
    // We normalize to dash style, but items that were explicitly quoted should remain quoted
    assert!(out.contains(r#"- "01""#));
    assert!(out.contains(r#"- "true""#));
    assert!(out.contains("\n  - 3\n"));
}
