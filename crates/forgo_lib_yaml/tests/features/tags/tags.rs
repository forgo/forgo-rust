use forgo_lib_yaml::Doc;

#[test]
fn simple_short_and_verbatim_tags_parse_and_emit() {
    let s = "%YAML 1.2\n%TAG !e! tag:example.com,2000:app/\n---\n- !e!thing 1\n- !!str foo\n- !<tag:example.com,2000:app/other> bar\n";
    let docs = Doc::from_stream(s).unwrap();
    let out = Doc::to_stream_string(&docs).unwrap();
    assert!(out.contains("!e!thing"));
    assert!(out.contains("!!str"));
    assert!(out.contains("!<tag:example.com,2000:app/other>"));
}
