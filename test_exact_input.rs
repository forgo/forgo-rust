use forgo_lib_yaml::Doc;

fn main() {
    let s = "%YAML 1.2\n%TAG !e! tag:example.com,2000:app/\n---\n- !e!thing 1\n- !!str foo\n- !<tag:example.com,2000:app/other> bar\n";
    
    println!("Input:");
    println!("{}", s);
    
    println!("\nParsing...");
    match Doc::from_stream(s) {
        Ok(docs) => {
            println!("✓ Parsed {} document(s) successfully!", docs.len());
            for (i, doc) in docs.iter().enumerate() {
                println!("\nDocument {}:", i + 1);
                match doc.to_string() {
                    Ok(out) => {
                        println!("{}", out);
                        println!("\nChecking for tags...");
                        println!("Contains '!e!thing': {}", out.contains("!e!thing"));
                        println!("Contains '!!str': {}", out.contains("!!str"));
                        println!("Contains '!<tag:example': {}", out.contains("!<tag:example"));
                    }
                    Err(e) => println!("Emit error: {:?}", e),
                }
            }
        }
        Err(e) => {
            println!("✗ Parse error: {:?}", e);
        }
    }
}
