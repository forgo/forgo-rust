use forgo_lib_yaml::Doc;

fn main() {
    let s = "%YAML 1.2\n%TAG !e! tag:example.com,2000:app/\n---\n- foo\n";
    
    println!("Input:");
    println!("{}", s);
    
    println!("\nParsing...");
    match Doc::from_stream(s) {
        Ok(docs) => {
            println!("✓ Parsed {} document(s) successfully!", docs.len());
            for (i, doc) in docs.iter().enumerate() {
                println!("\nDocument {}:", i + 1);
                match doc.to_string() {
                    Ok(out) => println!("{}", out),
                    Err(e) => println!("Emit error: {:?}", e),
                }
            }
        }
        Err(e) => {
            println!("✗ Parse error: {:?}", e);
        }
    }
}
