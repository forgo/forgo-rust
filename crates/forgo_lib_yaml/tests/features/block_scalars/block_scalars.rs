// crates/forgo_lib_yaml/tests/block_scalars.rs
use forgo_lib_yaml::Doc;

#[test]
fn indent_indicator_and_folding() {
    // Test literal block scalar with explicit indent indicator
    // Indicator 2 means content must have 2 spaces of indentation
    let s = "|2\n  a\n  b\n\n  c\n";

    let d = Doc::from_str(s).unwrap();
    let out = d.to_string().unwrap();

    // The parsed body should be "a\nb\n\nc\n" (no leading spaces - stripped by base_indent)
    // The emitted output will have consistent 2-space indentation for block scalar lines
    assert!(
        out.contains("  a\n  b\n  \n  c\n"),
        "Expected substring '  a\\n  b\\n  \\n  c\\n' (with 2-space indents) in output.\nGot: {:?}",
        out
    );

    // Test folded block scalar
    // Lines fold to space except when blank or more-indented
    let s2 = ">\n foo\n bar\n\n  baz\n";
    let d2 = Doc::from_str(s2).unwrap();
    let out2 = d2.to_string().unwrap();

    // For folded scalars: "foo\nbar" folds to "foo bar", blank line preserved,
    // " baz" (with extra leading space) is preserved as more-indented
    // When emitted, we get 2-space base indent for the folded content
    assert!(
        out2.contains("foo bar"),
        "Expected folding 'foo bar'.\nGot: {:?}",
        out2
    );
}
