use forgo_lib_yaml::Doc;

fn main() {
    let s = "---\n- !str foo\n";
    
    println!("Input:");
    println!("{}", s);
    
    println!("\nParsing...");
    match Doc::from_stream(s) {
        Ok(docs) => {
            println!("✓ Parsed {} document(s) successfully!", docs.len());
        }
        Err(e) => {
            println!("✗ Parse error: {:?}", e);
        }
    }
}
