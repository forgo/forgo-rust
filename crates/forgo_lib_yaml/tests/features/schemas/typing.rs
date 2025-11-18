use forgo_lib_yaml::Doc;

#[test]
fn bare_bools_and_numbers_and_null_are_typed() {
    let d = Doc::from_str("a: true\nb: -42\nc: 3.14\nd: null\n").unwrap();
    let out = d.to_string().unwrap();
    assert!(out.contains("a: true"));
    assert!(out.contains("b: -42"));
    assert!(out.contains("c: 3.14"));
    assert!(out.contains("d: null"));
}

#[test]
fn quoted_scalars_remain_strings() {
    let d = Doc::from_str(
        r#"a: "true"
b: "3.14"
c: "null"
"#,
    )
    .unwrap();
    let out = d.to_string().unwrap();
    assert!(out.contains(r#"a: "true""#));
    assert!(out.contains(r#"b: "3.14""#));
    assert!(out.contains(r#"c: "null""#));
}
