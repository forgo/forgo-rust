use forgo_lib_yaml;

fn main() {
    let input = "|2\n  a\n    b\n c\n    d\n";
    println!("Input (with visible spaces):");
    let vis: String = input.chars().map(|c| if c == ' ' { '·' } else { c }).collect();
    println!("{}", vis);
    println!("\nTokens:");
    let tokens = forgo_lib_yaml::debug_token_dump(input);
    println!("{}", tokens);
}
