use forgo_lib_yaml::Doc;

fn main() {
    let input = r#"
# hello
"abc"
"#;

    let doc = Doc::from_str(input).unwrap();
    let root = doc.root();

    println!("Root element:");
    println!("  Leading comments: {:?}", root.meta.leading_comments);
    println!("  Type: {:?}", std::mem::discriminant(&root.node));

    let out = doc.to_string().unwrap();
    println!("\nOutput:");
    println!("{}", out);
    println!("\nContains '# hello': {}", out.contains("# hello"));
}
