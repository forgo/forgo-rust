use forgo_lib_yaml::Doc;

fn main() {
    let input = r#"
# top banner
root: # trailing on root line ignored (root is a map)
  # before key
  key: value  # after value
  list:
    # before item
    - a  # after item a
    - b
"#;

    let doc = Doc::from_str(input).unwrap();
    let root = doc.root();

    if let forgo_lib_yaml::Node::Map(entries) = &root.node {
        for (k, v) in entries.iter() {
            if k == "root" {
                if let forgo_lib_yaml::Node::Map(nested) = &v.node {
                    for (nk, nv) in nested.iter() {
                        if nk == "list" {
                            println!("list value:");
                            println!("  Leading comments: {:?}", nv.meta.leading_comments);
                            if let forgo_lib_yaml::Node::Seq(items) = &nv.node {
                                println!("  Sequence has {} items:", items.len());
                                for (i, item) in items.iter().enumerate() {
                                    println!("    Item {}:", i);
                                    println!("      Leading comments: {:?}", item.meta.leading_comments);
                                    println!("      Trailing comment: {:?}", item.meta.trailing_comment);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let out = doc.to_string().unwrap();
    println!("\nOutput:");
    println!("{}", out);
}
