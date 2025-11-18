use forgo_lib_yaml::{Doc, Node};

#[test]
fn flow_mapping_and_nested_flow_seq() {
    let d = Doc::from_str("{k1: [1, 2, 3], k2: {x: true}}\n").unwrap();
    if let Node::Map(pairs) = d.root().node() {
        assert_eq!(pairs.len(), 2);
        match &pairs[0].1.node {
            Node::Seq(v) => assert_eq!(v.len(), 3),
            _ => panic!(),
        }
        match &pairs[1].1.node {
            Node::Map(m) => assert_eq!(m.len(), 1),
            _ => panic!(),
        }
    } else {
        panic!()
    }
}
