use forgo_lib_yaml::Doc;

#[test]
fn ignores_utf8_bom() {
    let s = "\u{FEFF}key: val\n";
    let d = Doc::from_str(s).unwrap();
    let out = d.to_string().unwrap();
    assert!(out.starts_with("key: val"));
}

#[test]
fn rejects_leading_tab_indent() {
    let s = "k:\n\t- x\n"; // tab before dash
    let d = Doc::from_str(s);
    assert!(d.is_err(), "expected parse error on tab indentation");
}

#[test]
fn handles_crlf_as_newline() {
    let s = "a: 1\r\nb: 2\r\n";
    let d = Doc::from_str(s).unwrap();
    let out = d.to_string().unwrap();
    // CRLF should be normalized to LF in output
    assert!(out.contains("a: 1\n"), "output: {:?}", out);
    assert!(out.contains("\nb: 2\n"), "output: {:?}", out);
}

#[test]
fn handles_bare_cr_as_newline() {
    let s = "x: 1\ry: 2\r";
    let d = Doc::from_str(s).unwrap();
    let out = d.to_string().unwrap();
    // Bare CR should be normalized to LF in output
    assert!(out.contains("x: 1\n"), "output: {:?}", out);
    assert!(out.contains("\ny: 2\n"), "output: {:?}", out);
}
