use forgo_lib_yaml::Doc;

fn main() {
    // Test with tags on sequence items
    let inputs = vec![
        ("Simple tag", "- !str foo\n"),
        ("Double-bang tag", "- !!str foo\n"),
        ("Verbatim tag", "- !<tag:example.com> foo\n"),
        ("With TAG directive", "%TAG !e! tag:example.com,2000:app/\n---\n- !e!thing 1\n"),
    ];

    for (label, input) in inputs {
        println!("\n=== {} ===", label);
        println!("Input: {:?}", input);
        match Doc::from_stream(input) {
            Ok(docs) => {
                println!("✓ Parsed {} docs", docs.len());
                if let Some(doc) = docs.first() {
                    println!("Root: {:#?}", doc.root());
                }
            }
            Err(e) => println!("✗ Error: {:?}", e),
        }
    }
}
