use forgo_lib_yaml::{Doc, Node, Scalar};

#[test]
fn single_and_double_quoted_escapes() {
    let d =
        Doc::from_str("a: 'it''s fine'\nb: \"line\\nbreak\"\nc: \"unicode: \\u263A\"\n").unwrap();
    if let Node::Map(m) = &d.root().node {
        match &m[0].1.node {
            Node::Scalar(Scalar::Str(s)) => assert_eq!(s, "it's fine"),
            _ => panic!(),
        }
        match &m[1].1.node {
            Node::Scalar(Scalar::Str(s)) => assert_eq!(s, "line\nbreak"),
            _ => panic!(),
        }
        match &m[2].1.node {
            Node::Scalar(Scalar::Str(s)) => assert!(s.contains("unicode: ")),
            _ => panic!(),
        }
    } else {
        panic!()
    }
}
