use forgo_lib_yaml::Doc;

fn main() {
    let input = ">\nfoo\nbar\n";
    println!("Input: {:?}", input);

    match Doc::from_str(input) {
        Ok(doc) => {
            println!("Parsed successfully");
            println!("Doc: {:#?}", doc);

            match doc.to_string() {
                Ok(output) => {
                    println!("Output: {:?}", output);
                }
                Err(e) => {
                    println!("Emit error: {:?}", e);
                }
            }
        }
        Err(e) => {
            println!("Parse error: {:?}", e);
        }
    }
}
