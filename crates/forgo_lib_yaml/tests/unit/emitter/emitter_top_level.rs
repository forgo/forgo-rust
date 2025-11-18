// tests/emitter_top_level.rs
use forgo_lib_yaml::Doc;

#[test]
fn top_level_block_scalar_is_emitted_with_pipe() {
    let d = Doc::from_str("|\n  hi\n  there\n").unwrap();
    let out = d.to_string().unwrap();
    assert!(out.starts_with("|\n"));
    assert!(out.contains("  hi\n"));
    assert!(out.contains("  there\n"));
}

#[test]
fn top_level_quoted_scalar_stays_quoted_when_preferred() {
    let d = Doc::from_str("\"k: v\"\n").unwrap();
    let out = d.to_string().unwrap();
    assert!(out.starts_with("\"k: v\"\n"), "{out}");
}
