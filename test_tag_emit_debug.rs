use forgo_lib_yaml::Doc;

fn main() {
    let s = "%YAML 1.2\n%TAG !e! tag:example.com,2000:app/\n---\n- !e!thing 1\n- !!str foo\n- !<tag:example.com,2000:app/other> bar\n";
    
    println!("Input:");
    println!("{}", s);
    println!();
    
    match Doc::from_stream(s) {
        Ok(docs) => {
            println!("✓ Parsed {} document(s)", docs.len());
            
            for (i, doc) in docs.iter().enumerate() {
                println!("\nDocument {}:", i + 1);
                match doc.to_string() {
                    Ok(output) => {
                        println!("Output:");
                        println!("{}", output);
                        println!("\nChecking for tags...");
                        println!("Contains '!e!thing': {}", output.contains("!e!thing"));
                        println!("Contains '!!str': {}", output.contains("!!str"));
                        println!("Contains '!<tag:example': {}", output.contains("!<tag:example"));
                    }
                    Err(e) => println!("Emit error: {:?}", e),
                }
            }
        }
        Err(e) => println!("✗ Parse error: {:?}", e),
    }
}
