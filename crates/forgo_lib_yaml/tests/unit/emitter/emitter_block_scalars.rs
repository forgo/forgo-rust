// tests/emitter_block_scalars.rs
use forgo_lib_yaml::Doc;

fn doc(s: &str) -> Doc {
    Doc::from_str(s).expect("parse")
}

#[test]
fn multiline_string_defaults_to_literal_block() {
    let d = doc("msg: |\n  line1\n  line2\n");
    let out = d.to_string().unwrap();
    // stays a block scalar (literal '|')
    assert!(out.contains("msg: |"), "{out}");
    assert!(out.contains("  line1"));
    assert!(out.contains("  line2"));
}

#[test]
fn folded_block_with_chomp_is_preserved() {
    // parser records Folded(Some('-')) and prefer_block
    let d = doc("note: >-\n  a\n  b\n");
    let out = d.to_string().unwrap();
    // emitter preserves '>-'
    assert!(out.contains("note: >-"), "{out}");
}

#[test]
fn block_scalar_trailing_comment_is_on_header_line() {
    let d = doc("msg: | # keep\n  hi\n");
    let out = d.to_string().unwrap();
    assert!(out.contains("msg: | # keep"), "{out}");
}
