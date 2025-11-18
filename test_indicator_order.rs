use forgo_lib_yaml::Doc;

fn main() {
    let inputs = vec![
        ("|2-\n  a\n  b\n", "|2-"),
        ("|-2\n  a\n  b\n", "|-2"),
        (">+1\n a\n b\n", ">+1"),
        (">1+\n a\n b\n", ">1+"),
    ];

    for (input, label) in inputs {
        println!("\n=== Testing: {} ===", label);
        println!("Input: {:?}", input);

        match Doc::from_str(input) {
            Ok(doc) => {
                println!("✓ Parsed successfully");
                println!("Doc: {:#?}", doc.root());

                match doc.to_string() {
                    Ok(output) => {
                        println!("Output: {:?}", output);
                    }
                    Err(e) => {
                        println!("✗ Emit error: {:?}", e);
                    }
                }
            }
            Err(e) => {
                println!("✗ Parse error: {:?}", e);
            }
        }
    }
}
