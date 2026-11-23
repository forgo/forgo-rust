//! YAML 1.2.2 rules the official yaml-test-suite doesn't exercise: input
//! encodings and byte order marks, line break characters, spec examples
//! missing from the suite, version handling and implicit key limits.
//!
//! Expected events are derived from the spec text and use the suite's
//! `test.event` notation.

use forgo_lib_yaml::syntax::test_event::to_test_event;

#[path = "yaml_test_suite/normalize.rs"]
mod normalize;

enum Expect {
    Events(&'static str),
    Error,
}

fn run_bytes(bytes: &[u8]) -> Result<String, String> {
    let evs = forgo_lib_yaml::events_from_bytes(bytes).map_err(|e| e.to_string())?;
    Ok(evs
        .iter()
        .map(|(e, _)| to_test_event(e))
        .collect::<Vec<_>>()
        .join("\n"))
}

fn check(name: &str, input: &[u8], expect: Expect) -> Option<String> {
    let got = run_bytes(input);
    match (expect, got) {
        (Expect::Events(exp), Ok(got)) => {
            let exp = exp.trim();
            if got != exp {
                return Some(format!("{name}: expected\n{exp}\n--- got\n{got}"));
            }
            roundtrip(name, input)
        }
        (Expect::Events(_), Err(e)) => Some(format!("{name}: unexpected error: {e}")),
        (Expect::Error, Ok(got)) => Some(format!("{name}: expected an error, got\n{got}")),
        (Expect::Error, Err(_)) => None,
    }
}

/// Parse → emit → parse must keep the content (normalized events).
fn roundtrip(name: &str, input: &[u8]) -> Option<String> {
    let text = match forgo_lib_yaml::syntax::input::decode(input) {
        Ok(t) => t,
        Err(e) => return Some(format!("{name}: decode failed: {e}")),
    };
    let emitted = match forgo_lib_yaml::Doc::from_stream(&text)
        .and_then(|docs| forgo_lib_yaml::Doc::to_stream_string(&docs))
    {
        Ok(out) => out,
        Err(e) => return Some(format!("{name}: compose/emit failed: {e}")),
    };
    let a = normalize::normalized_events(&text);
    let b = normalize::normalized_events(&emitted);
    (a != b).then(|| {
        format!("{name}: round trip changed content\n{a:?}\n--- emitted {emitted:?}\n{b:?}")
    })
}

fn utf16(s: &str, le: bool, bom: bool) -> Vec<u8> {
    let units = bom
        .then_some(0xFEFF_u16)
        .into_iter()
        .chain(s.encode_utf16());
    units
        .flat_map(|u| if le { u.to_le_bytes() } else { u.to_be_bytes() })
        .collect()
}

fn utf32(s: &str, le: bool, bom: bool) -> Vec<u8> {
    let chars = bom.then_some('\u{FEFF}').into_iter().chain(s.chars());
    chars
        .flat_map(|c| {
            let u = c as u32;
            if le { u.to_le_bytes() } else { u.to_be_bytes() }
        })
        .collect()
}

const MAP_A_E: &str = "+STR\n+DOC\n+MAP\n=VAL :a\n=VAL :é\n-MAP\n-DOC\n-STR";

#[test]
fn spec_gaps() {
    let mut cases: Vec<(String, Vec<u8>, Expect)> = Vec::new();
    let mut text = |name: &str, input: &str, expect: Expect| {
        cases.push((name.to_string(), input.as_bytes().to_vec(), expect));
    };

    // --- §5.2 byte order marks: allowed at the start of each document only.
    text(
        "bom-per-document",
        "\u{FEFF}a\n...\n\u{FEFF}b\n",
        Expect::Events("+STR\n+DOC\n=VAL :a\n-DOC ...\n+DOC\n=VAL :b\n-DOC\n-STR"),
    );
    text(
        "bom-before-explicit-document",
        "a\n\u{FEFF}--- b\n",
        Expect::Events("+STR\n+DOC\n=VAL :a\n-DOC\n+DOC ---\n=VAL :b\n-DOC\n-STR"),
    );
    text(
        "bom-inside-document-content",
        "a:\n\u{FEFF}  b\n",
        Expect::Error,
    );
    text("bom-inside-plain-scalar", "a: \u{FEFF}b\n", Expect::Error);

    // --- §5.4 line breaks: CRLF and CR are breaks and are normalized to LF.
    text(
        "crlf",
        "a: b\r\nc: |\r\n  x\r\n  y\r\nd: 'p\r\n  q'\r\n",
        Expect::Events(
            "+STR\n+DOC\n+MAP\n=VAL :a\n=VAL :b\n=VAL :c\n=VAL |x\\ny\\n\n=VAL :d\n=VAL 'p q\n-MAP\n-DOC\n-STR",
        ),
    );
    text(
        "cr-only",
        "a: b\rc: \"x\r\r  y\"\r",
        Expect::Events(
            "+STR\n+DOC\n+MAP\n=VAL :a\n=VAL :b\n=VAL :c\n=VAL \"x\\ny\n-MAP\n-DOC\n-STR",
        ),
    );
    // NEL, LS and PS are not line breaks in YAML 1.2.
    text(
        "nel-ls-ps-are-content",
        "a: x\u{85}y\u{2028}z\u{2029}w\n",
        Expect::Events(
            "+STR\n+DOC\n+MAP\n=VAL :a\n=VAL :x\u{85}y\u{2028}z\u{2029}w\n-MAP\n-DOC\n-STR",
        ),
    );

    // --- §5.1 character set.
    text("nul-in-plain-scalar", "a: x\0y\n", Expect::Error);
    text("del-in-plain-scalar", "a: x\u{7F}y\n", Expect::Error);
    text(
        "del-allowed-in-double-quotes",
        "a: \"x\u{7F}y\"\n",
        Expect::Events("+STR\n+DOC\n+MAP\n=VAL :a\n=VAL \"x\u{7F}y\n-MAP\n-DOC\n-STR"),
    );

    // --- Example 5.10: reserved indicators can't start a plain scalar.
    text("example-5.10-at", "commercial-at: @text\n", Expect::Error);
    text(
        "example-5.10-backtick",
        "grave-accent: `text\n",
        Expect::Error,
    );

    // --- Example 5.13: escaped characters.
    text(
        "example-5.13",
        "- \"Fun with \\\\\"\n- \"\\\" \\a \\b \\e \\f\"\n- \"\\n \\r \\t \\v \\0\"\n- \"\\  \\_ \\N \\L \\P \\\n  \\x41 \\u0041 \\U00000041\"\n",
        Expect::Events(
            "+STR\n+DOC\n+SEQ\n=VAL \"Fun with \\\\\n=VAL \"\" \u{7} \\b \u{1B} \u{C}\n=VAL \"\\n \\r \\t \u{B} \\0\n=VAL \"  \u{A0} \u{85} \u{2028} \u{2029} A A A\n-SEQ\n-DOC\n-STR",
        ),
    );
    // --- Example 5.14: invalid escapes.
    text(
        "example-5.14-c",
        "Bad escapes:\n  \"\\c\n  \\xq-\"\n",
        Expect::Error,
    );
    text(
        "example-5.14-xq",
        "Bad escapes:\n  \"\\xq-\"\n",
        Expect::Error,
    );
    text(
        "single-quote-escape-is-not-yaml",
        "\"\\'\"\n",
        Expect::Error,
    );
    text(
        "json-surrogate-pair-escape",
        "\"\\uD83D\\uDE00\"\n",
        Expect::Events("+STR\n+DOC\n=VAL \"\u{1F600}\n-DOC\n-STR"),
    );
    text("lone-surrogate-escape", "\"\\uD800\"\n", Expect::Error);

    // --- Example 6.17: repeated %TAG handle.
    text(
        "example-6.17",
        "%TAG ! !foo\n%TAG ! !foo\n--- bar\n",
        Expect::Error,
    );
    // --- §6.8.1 %YAML: a higher minor version is accepted, major 2 is not.
    text(
        "yaml-1.3-accepted",
        "%YAML 1.3\n--- x\n",
        Expect::Events("+STR\n+DOC ---\n=VAL :x\n-DOC\n-STR"),
    );
    text("yaml-2.0-rejected", "%YAML 2.0\n--- x\n", Expect::Error);

    // --- Example 6.25: invalid verbatim tags.
    text("example-6.25-bang", "- !<!> foo\n", Expect::Error);
    text("example-6.25-not-uri", "- !<$:?> bar\n", Expect::Error);
    // --- Example 6.27: invalid tag shorthands.
    text(
        "example-6.27-empty-suffix",
        "%TAG !e! tag:example,2000:app/\n---\n- !e! foo\n",
        Expect::Error,
    );
    text(
        "example-6.27-undefined-handle",
        "%TAG !e! tag:example,2000:app/\n---\n- !h!bar baz\n",
        Expect::Error,
    );

    // --- Example 7.22: implicit keys are single-line and at most 1024 chars.
    text(
        "example-7.22-multiline",
        "[ foo\n bar: invalid ]\n",
        Expect::Error,
    );
    let key_1024 = "k".repeat(1024);
    let key_1025 = "k".repeat(1025);
    cases.push((
        "implicit-key-1024-chars".into(),
        format!("{key_1024}: v\n").into_bytes(),
        Expect::Events(Box::leak(
            format!("+STR\n+DOC\n+MAP\n=VAL :{key_1024}\n=VAL :v\n-MAP\n-DOC\n-STR")
                .into_boxed_str(),
        )),
    ));
    cases.push((
        "example-7.22-1025-chars".into(),
        format!("{key_1025}: v\n").into_bytes(),
        Expect::Error,
    ));
    cases.push((
        "flow-sequence-pair-1025-chars".into(),
        format!("[{key_1025}: v]\n").into_bytes(),
        Expect::Error,
    ));

    // --- Top-level block scalars: content may start at column 0, and an
    // explicit indentation indicator counts from column 0 (as in libyaml,
    // yaml-rust2 and saphyr; the grammar read literally would give one less).
    let mut text = |name: &str, input: &str, expect: Expect| {
        cases.push((name.to_string(), input.as_bytes().to_vec(), expect));
    };
    text(
        "top-level-indicator-1",
        "--- |1\n  foo\n",
        Expect::Events("+STR\n+DOC ---\n=VAL | foo\\n\n-DOC\n-STR"),
    );
    text(
        "top-level-indicator-2",
        "--- |2\n  foo\n",
        Expect::Events("+STR\n+DOC ---\n=VAL |foo\\n\n-DOC\n-STR"),
    );
    // A line that only closes a flow collection may sit at column 0 under a
    // block key (as in yaml-rust2 and saphyr); content may not.
    text(
        "flow-closing-bracket-at-column-0",
        "key: [\n  a\n]\n",
        Expect::Events("+STR\n+DOC\n+MAP\n=VAL :key\n+SEQ []\n=VAL :a\n-SEQ\n-MAP\n-DOC\n-STR"),
    );
    text("flow-content-at-column-0", "key: [a,\nb]\n", Expect::Error);
    text(
        "top-level-auto-detect-column-0",
        "--- >\nfoo\nbar\n",
        Expect::Events("+STR\n+DOC ---\n=VAL >foo bar\\n\n-DOC\n-STR"),
    );

    // --- §5.2 encodings.
    for (name, bytes) in [
        ("utf16le", utf16("a: é\n", true, false)),
        ("utf16le-bom", utf16("a: é\n", true, true)),
        ("utf16be", utf16("a: é\n", false, false)),
        ("utf16be-bom", utf16("a: é\n", false, true)),
        ("utf32le", utf32("a: é\n", true, false)),
        ("utf32le-bom", utf32("a: é\n", true, true)),
        ("utf32be", utf32("a: é\n", false, false)),
        ("utf32be-bom", utf32("a: é\n", false, true)),
        ("utf8-bom", b"\xEF\xBB\xBFa: \xC3\xA9\n".to_vec()),
    ] {
        cases.push((name.to_string(), bytes, Expect::Events(MAP_A_E)));
    }
    cases.push(("invalid-utf8".into(), b"a: \xFF\n".to_vec(), Expect::Error));

    let failures: Vec<String> = cases
        .into_iter()
        .filter_map(|(name, input, expect)| check(&name, &input, expect))
        .collect();
    assert!(
        failures.is_empty(),
        "{} spec-gap failures:\n\n{}",
        failures.len(),
        failures.join("\n\n")
    );
}
