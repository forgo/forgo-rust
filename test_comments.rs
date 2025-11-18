use forgo_lib_yaml::Doc;

fn main() {
    let input = r#"
# top banner
root: # trailing on root line ignored (root is a map)
  # before key
  key: value  # after value
  list:
    # before item
    - a  # after item a
    - b
"#;

    println!("Input:");
    println!("{}", input);

    match Doc::from_str(input) {
        Ok(doc) => {
            println!("\n✓ Parsed successfully");

            match doc.to_string() {
                Ok(output) => {
                    println!("\nOutput:");
                    println!("{}", output);

                    println!("\nFirst 50 chars: {:?}", &output.chars().take(50).collect::<String>());
                    println!("Starts with '# top banner': {}", output.starts_with("# top banner"));
                }
                Err(e) => println!("\n✗ Emit error: {:?}", e),
            }
        }
        Err(e) => println!("\n✗ Parse error: {:?}", e),
    }
}
