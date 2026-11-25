//! The API, and behaviour toml-test doesn't pin down: error kinds and
//! positions, large tables (hash index), order, the encoder's output.

use forgo_lib_toml::{Document, ErrorKind, Item, Offset, ParseOptions, TomlVersion, Value};

fn parse(s: &str) -> Document {
    Document::parse(s).unwrap_or_else(|e| panic!("{s:?}: {e}"))
}

fn err(s: &str) -> forgo_lib_toml::Error {
    Document::parse(s).expect_err(s)
}

#[test]
fn tabs_and_escapes_in_one_string() {
    // A tab in a string with escapes once looped forever.
    let doc = parse("x = \"a\\tb\tc\\u00e9\td\"\n");
    assert_eq!(
        doc.root().get("x").unwrap().as_str(),
        Some("a\tb\tc\u{e9}\td")
    );
}

#[test]
fn errors_say_what_and_where() {
    let cases: &[(&str, ErrorKind, usize, usize)] = &[
        ("a = 1\na = 2\n", ErrorKind::Duplicate, 2, 1),
        ("[a]\n[a]\n", ErrorKind::Table, 2, 1),
        ("a = \"x\n", ErrorKind::String, 1, 5),
        ("a = 1_\n", ErrorKind::Number, 1, 5),
        ("a = 9223372036854775808\n", ErrorKind::Number, 1, 5),
        ("a = 2021-02-30\n", ErrorKind::Datetime, 1, 13),
        ("a = 1 b = 2\n", ErrorKind::Syntax, 1, 7),
        ("a = \"\\q\"\n", ErrorKind::String, 1, 6),
        ("x = 1\n[t]\ny = { z = 1 }\n[t.y]\n", ErrorKind::Table, 4, 1),
    ];
    for (src, kind, line, col) in cases {
        let e = err(src);
        assert_eq!(
            (e.kind(), e.line(), e.column()),
            (*kind, *line, *col),
            "{src:?}: {e}"
        );
    }
    assert_eq!(
        err("a = 1\na = 2\n").to_string(),
        "the key `a` is already defined at line 2, column 1"
    );
    let bad = Document::from_slice(b"a = \"\xff\"", &ParseOptions::default()).unwrap_err();
    assert_eq!(bad.kind(), ErrorKind::Encoding);
}

#[test]
fn large_tables_use_an_index() {
    let mut src = String::new();
    for i in 0..5000 {
        src.push_str(&format!("k{i} = {i}\n"));
    }
    let doc = parse(&src);
    let root = doc.root();
    assert_eq!(root.len(), 5000);
    for i in (0..5000).step_by(97) {
        assert_eq!(root.get(&format!("k{i}")).unwrap().as_integer(), Some(i));
    }
    assert!(root.get("k5000").is_none());
    // Duplicates are found through the index too.
    src.push_str("k4321 = 0\n");
    assert_eq!(err(&src).kind(), ErrorKind::Duplicate);
    // Quoted and bare spellings are the same key.
    let mut src: String = (0..50).map(|i| format!("k{i} = 1\n")).collect();
    src.push_str("\"k7\" = 2\n");
    assert_eq!(err(&src).kind(), ErrorKind::Duplicate);
}

#[test]
fn order_and_kinds() {
    let doc = parse("b = 1\na = [1, 2]\n[[c]]\nx = 1\n[[c]]\n[d]\ne = { f = 1 }\n");
    let root = doc.root();
    assert_eq!(root.keys().collect::<Vec<_>>(), ["b", "a", "c", "d"]);
    let c = root.get("c").unwrap().as_array().unwrap();
    assert!(c.is_array_of_tables());
    assert_eq!(c.len(), 2);
    assert!(
        !root
            .get("a")
            .unwrap()
            .as_array()
            .unwrap()
            .is_array_of_tables()
    );
    let e = doc.get(&["d", "e"]).unwrap().as_table().unwrap();
    assert!(e.is_inline());
    assert_eq!(e.get("f"), Some(Value::Integer(1)));
}

#[test]
fn versions() {
    let v10 = ParseOptions::default().with_version(TomlVersion::V1_0);
    for src in [
        "t = 07:32",
        "s = \"\\e\"",
        "s = \"\\x41\"",
        "t = { a = 1, }",
        "t = {\na = 1\n}",
    ] {
        assert!(Document::parse(src).is_ok(), "1.1: {src:?}");
        assert!(Document::parse_with(src, &v10).is_err(), "1.0: {src:?}");
    }
    assert_eq!(
        parse("s = \"\\x41\\e\"").root().get("s").unwrap().as_str(),
        Some("A\u{1b}")
    );
}

#[test]
fn datetimes() {
    let doc = parse("a = 1979-05-27 07:32:00.123456789123-07:00\nb = 1979-05-27\nc = 00:00:60\n");
    let a = doc.root().get("a").unwrap().as_datetime().unwrap();
    assert_eq!(a.time.unwrap().nanosecond, 123_456_789);
    assert_eq!(a.offset, Some(Offset::Minutes(-420)));
    assert_eq!(a.to_string(), "1979-05-27T07:32:00.123456789-07:00");
    assert_eq!(a.kind_name(), "datetime");
    let b = doc.root().get("b").unwrap().as_datetime().unwrap();
    assert_eq!(
        (b.to_string().as_str(), b.kind_name()),
        ("1979-05-27", "date-local")
    );
    assert_eq!(
        doc.root()
            .get("c")
            .unwrap()
            .as_datetime()
            .unwrap()
            .to_string(),
        "00:00:60"
    );
}

#[test]
fn encoder() {
    let item = Item::table([
        ("bare-key_1", Item::from(1)),
        ("needs quotes", Item::from("tab\there \"quoted\" \u{7f}")),
        ("float", Item::from(1.0)),
        ("big", Item::from(1e300)),
        ("nan", Item::from(f64::NAN)),
        ("inf", Item::from(f64::NEG_INFINITY)),
        (
            "mixed",
            Item::array([Item::from(1), Item::table([("a", Item::from(true))])]),
        ),
        ("", Item::from("empty key")),
    ]);
    assert_eq!(
        item.to_toml_string().unwrap(),
        "bare-key_1 = 1\n\
         \"needs quotes\" = \"tab\\there \\\"quoted\\\" \\u007F\"\n\
         float = 1.0\n\
         big = 1e300\n\
         nan = nan\n\
         inf = -inf\n\
         mixed = [1, { a = true }]\n\
         \"\" = \"empty key\"\n"
    );
    assert!(Item::from(1).to_toml_string().is_err());
    // Round trip through a document.
    let doc = parse(&item.to_toml_string().unwrap());
    assert_eq!(
        doc.root().get("needs quotes").unwrap().as_str(),
        Some("tab\there \"quoted\" \u{7f}")
    );
}
