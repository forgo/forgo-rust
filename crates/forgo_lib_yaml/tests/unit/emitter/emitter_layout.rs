// tests/emitter_layout.rs
use forgo_lib_yaml::Doc;

fn doc(s: &str) -> Doc {
    Doc::from_str(s).expect("parse")
}

#[test]
fn seq_as_map_value_is_indented_two_spaces_from_key() {
    let d = doc("k:\n  - 1\n  - 2\n");
    let out = d.to_string().unwrap();
    // Key line ends with colon+newline; list items start at key-indent + 2
    assert!(out.contains("k:\n  - 1\n  - 2\n"), "{out}");
}

#[test]
fn nested_map_as_map_value_is_indented_two_spaces_from_key() {
    let d = doc("root:\n  child: 1\n");
    let out = d.to_string().unwrap();
    assert!(out.contains("root:\n  child: 1\n"), "{out}");
}
