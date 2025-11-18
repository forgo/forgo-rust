use forgo_lib_yaml::Doc;

fn main() {
    let input = "|2\n  a\n    b\n c\n    d\n";
    println!("Input: {:?}", input);
    println!("Breakdown:");
    println!("  Line 1: '  a' (indent 2)");
    println!("  Line 2: '    b' (indent 4)");
    println!("  Line 3: ' c' (indent 1 < 2)");
    println!("  Line 4: '    d' (indent 4)");

    match Doc::from_str(input) {
        Ok(doc) => {
            println!("\n✓ Parsed successfully");
            let root = doc.root();
            println!("Root node type: {:?}", root.node);

            if let forgo_lib_yaml::Node::Scalar(forgo_lib_yaml::Scalar::Str(s)) = &root.node {
                println!("\nString value: {:?}", s);
                println!("String length: {}", s.len());
                println!("Bytes: {:?}", s.as_bytes());
            }

            match doc.to_string() {
                Ok(output) => {
                    println!("\nEmitted output: {:?}", output);
                }
                Err(e) => {
                    println!("\n✗ Emit error: {:?}", e);
                }
            }
        }
        Err(e) => {
            println!("\n✗ Parse error: {:?}", e);
        }
    }
}
