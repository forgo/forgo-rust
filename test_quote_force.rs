use forgo_lib_yaml::Doc;

fn main() {
    let input = "# before key\nkey: value # after value\n";

    let doc = Doc::from_str(input).unwrap();
    let root = doc.root();

    println!("Root element:");
    println!("  Leading comments: {:?}", root.meta.leading_comments);

    if let forgo_lib_yaml::Node::Map(entries) = &root.node {
        for (k, v) in entries.iter() {
            println!("\nMap entry:");
            println!("  Key: {:?}", k);
            println!("  Value leading comments: {:?}", v.meta.leading_comments);
            println!("  Value trailing comment: {:?}", v.meta.trailing_comment);
        }
    }

    let out = doc.to_string().unwrap();
    println!("\nOutput:");
    println!("{}", out);
}
