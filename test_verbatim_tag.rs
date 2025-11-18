use forgo_lib_yaml::Doc;

fn main() {
    let inputs = vec![
        ("---\n- !str foo\n", "simple tag"),
        ("---\n- !!str foo\n", "double exclaim tag"),
        ("---\n- !<tag:example.com,2000:app/other> bar\n", "verbatim tag with colon"),
        ("%YAML 1.2\n---\n- !<tag:example.com,2000:app/other> bar\n", "with YAML directive"),
    ];
    
    for (input, desc) in inputs {
        println!("\n=== {} ===", desc);
        println!("Input: {:?}", input);
        match Doc::from_stream(input) {
            Ok(_) => println!("✓ Parsed successfully"),
            Err(e) => println!("✗ Parse error: {:?}", e),
        }
    }
}
