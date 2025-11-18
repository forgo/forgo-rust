use forgo_lib_yaml;

fn main() {
    let inputs = vec![
        ("|2-", "pipe then 2 then minus"),
        ("|-2", "pipe then minus then 2"),
        (">+1", "gt then plus then 1"),
        (">1+", "gt then 1 then plus"),
    ];

    for (input, desc) in inputs {
        println!("\n=== {} ===", desc);
        println!("Input: {:?}", input);
        println!("\nTokens:");
        let tokens = forgo_lib_yaml::debug_token_dump(input);
        println!("{}", tokens);
    }
}
