use forgo_lib_yaml::Doc;

fn main() {
    let s = "%YAML 1.2\n%TAG !e! tag:example.com,2000:app/\n---\n- !e!thing 1\n- !!str foo\n- !<tag:example.com,2000:app/other> bar\n";

    println!("Input:\n{}", s);
    println!("\n=== Parsing ===");

    match Doc::from_stream(s) {
        Ok(docs) => {
            println!("✓ Parsed {} document(s)", docs.len());
            for (i, doc) in docs.iter().enumerate() {
                println!("\nDocument {}:", i);
                println!("{:#?}", doc.root());
            }

            match Doc::to_stream_string(&docs) {
                Ok(output) => {
                    println!("\n=== Emitted ===\n{}", output);
                }
                Err(e) => {
                    println!("\n✗ Emit error: {:?}", e);
                }
            }
        }
        Err(e) => {
            println!("✗ Parse error: {:?}", e);
        }
    }
}
