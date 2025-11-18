use forgo_lib_yaml::Doc;

fn main() {
    // Test 1: Just the TAG directive
    let s1 = "%TAG !e! tag:example.com,2000:app/\n---\nfoo\n";
    println!("Test 1:\n{}", s1);
    match Doc::from_stream(s1) {
        Ok(docs) => println!("✓ Parsed {} docs\n", docs.len()),
        Err(e) => println!("✗ Error: {:?}\n", e),
    }

    // Test 2: Without TAG directive
    let s2 = "---\nfoo\n";
    println!("Test 2:\n{}", s2);
    match Doc::from_stream(s2) {
        Ok(docs) => println!("✓ Parsed {} docs\n", docs.len()),
        Err(e) => println!("✗ Error: {:?}\n", e),
    }

    // Test 3: YAML directive only
    let s3 = "%YAML 1.2\n---\nfoo\n";
    println!("Test 3:\n{}", s3);
    match Doc::from_stream(s3) {
        Ok(docs) => println!("✓ Parsed {} docs\n", docs.len()),
        Err(e) => println!("✗ Error: {:?}\n", e),
    }
}
