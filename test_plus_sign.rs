use forgo_lib_yaml;

fn main() {
    let input = "[+5, -3]";
    println!("Input: {:?}", input);
    println!("\nTokens:");
    let tokens = forgo_lib_yaml::debug_token_dump(input);
    println!("{}", tokens);
}
