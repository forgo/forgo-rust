use forgo_lib_yaml::Doc;

fn main() {
    let s = "---\n- !str foo\n";
    
    println!("Input: {:?}", s);
    
    match Doc::from_str(s) {
        Ok(doc) => {
            println!("✓ Parsed successfully!");
            match doc.to_string() {
                Ok(out) => {
                    println!("Output:");
                    println!("{}", out);
                    println!("\nContains '!str': {}", out.contains("!str"));
                }
                Err(e) => println!("Emit error: {:?}", e),
            }
        }
        Err(e) => println!("✗ Parse error: {:?}", e),
    }
}
