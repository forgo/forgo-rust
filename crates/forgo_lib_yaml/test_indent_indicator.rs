use forgo_lib_yaml::Doc;

fn main() {
    let s = "|2\n  a\n    b\n c\n    d\n";

    println!("Input (visible):");
    let vis: String = s.chars().map(|c| if c == ' ' { '·' } else { c }).collect();
    println!("{}", vis);

    println!("\nParsing...");
    match Doc::from_str(s) {
        Ok(d) => {
            println!("Parsed successfully!");

            match d.to_string() {
                Ok(out) => {
                    println!("\nOutput (visible):");
                    let vis: String = out.chars().map(|c| if c == ' ' { '·' } else { c }).collect();
                    println!("{}", vis);

                    println!("\nExpected to contain: {:?}", "  a\n    b\n  \n    d\n");
                    println!("Actually contains it: {}", out.contains("  a\n    b\n  \n    d\n"));

                    println!("\nFull output:");
                    println!("{:?}", out);
                }
                Err(e) => println!("Emit error: {:?}", e),
            }
        }
        Err(e) => println!("Parse error: {:?}", e),
    }
}
