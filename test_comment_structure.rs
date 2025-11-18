use forgo_lib_yaml::Doc;

fn main() {
    let input = r#"
# top banner
root: # trailing on root line ignored (root is a map)
  # before key
  key: value  # after value
"#;

    let doc = Doc::from_str(input).unwrap();
    let root = doc.root();

    println!("Root element:");
    println!("  Type: {:?}", std::mem::discriminant(&root.node));
    println!("  Leading comments: {:?}", root.meta.leading_comments);
    println!("  Trailing comment: {:?}", root.meta.trailing_comment);

    if let forgo_lib_yaml::Node::Map(entries) = &root.node {
        println!("\nRoot map has {} entries:", entries.len());
        for (i, (k, v)) in entries.iter().enumerate() {
            println!("\n  Entry {}:", i);
            println!("    Key: {:?}", k);
            println!("    Value leading comments: {:?}", v.meta.leading_comments);
            println!("    Value trailing comment: {:?}", v.meta.trailing_comment);
            println!("    Value type: {:?}", std::mem::discriminant(&v.node));

            if let forgo_lib_yaml::Node::Map(nested_entries) = &v.node {
                println!("    Nested map has {} entries:", nested_entries.len());
                for (j, (nk, nv)) in nested_entries.iter().enumerate() {
                    println!("      Nested entry {}:", j);
                    println!("        Key: {:?}", nk);
                    println!("        Value leading comments: {:?}", nv.meta.leading_comments);
                    println!("        Value trailing comment: {:?}", nv.meta.trailing_comment);
                }
            }
        }
    }
}
