use forgo_lib_yaml;

fn show(s: &str) -> String {
    // Make whitespace/newlines visible so we can see trailing spaces, etc.
    let mut out = String::new();
    for (_, ch) in s.chars().enumerate() {
        match ch {
            '\n' => out.push_str("\\n\n"),
            '\t' => out.push_str("\\t"),
            ' ' => out.push('·'), // middle dot for spaces
            _ => out.push(ch),
        }
    }
    // Also include byte dump (handy if some non-printable sneaks in)
    let bytes = s
        .as_bytes()
        .iter()
        .map(|b| format!("{:02X}", b))
        .collect::<Vec<_>>()
        .join(" ");
    format!(
        "--- EMITTED YAML (visible) ---\n{}\n--- BYTES ---\n{}\n",
        out, bytes
    )
}

#[test]
fn literal_block_pipe_preserves_newlines_default_one_final() {
    let input = "msg: |\n  line 1\n  line 2\n";
    let d = forgo_lib_yaml::Doc::from_str(input).unwrap();
    let out = d.to_string().unwrap();

    if !out.contains("msg: |") {
        panic!("expected header not found\n{}", show(&out));
    }
    if !out.contains("line 1\n") {
        panic!("expected 'line 1\\n' not found\n{}", show(&out));
    }
    if !out.contains("line 2\n") {
        panic!("expected 'line 2\\n' not found\n{}", show(&out));
    }
}

#[test]
fn literal_block_chomp_strip() {
    let input = "m: |-\n  a\n";
    let d = forgo_lib_yaml::Doc::from_str(input).unwrap();
    let out = d.to_string().unwrap();

    if !out.contains("m: |") {
        panic!("expected 'm: |' header not found\n{}", show(&out));
    }
    if !out.contains("a\n") {
        panic!("expected 'a\\n' in body\n{}", show(&out));
    }
}

#[test]
fn folded_block_gt_folds_single_newlines() {
    let input = "wrap: >\n  this\n  is\n  one\n  paragraph\n";
    let d = forgo_lib_yaml::Doc::from_str(input).unwrap();
    let out = d.to_string().unwrap();

    if !out.contains("wrap: |") {
        panic!("expected canonical '|'\n{}", show(&out));
    }
    if !out.contains("this is one paragraph\n") {
        panic!("expected folded paragraph line\n{}", show(&out));
    }
}

#[test]
fn block_body_has_no_trailing_spaces_before_newline() {
    let input = "msg: |\n  line 1  \n  line 2\t \n";
    let d = forgo_lib_yaml::Doc::from_str(input).unwrap();
    let out = d.to_string().unwrap();

    // print once for this stricter test if anything fails
    for (i, l) in out.lines().enumerate() {
        if l.ends_with(' ') || l.ends_with('\t') {
            panic!("line {} ends with trailing whitespace\n{}", i, show(&out));
        }
    }
    if !out.contains("msg: |") || !out.contains("line 1\n") || !out.contains("line 2\n") {
        panic!("expected substrings missing\n{}", show(&out));
    }
}
