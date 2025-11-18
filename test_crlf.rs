use forgo_lib_yaml::Doc;

fn main() {
    let s = "a: 1\r\nb: 2\r\n";
    println!("Input bytes: {:?}", s.as_bytes());

    let d = Doc::from_str(s).unwrap();
    let out = d.to_string().unwrap();

    println!("\nOutput: {:?}", out);
    println!("Output bytes: {:?}", out.as_bytes());
    println!("\nFormatted output:");
    println!("{}", out);

    println!("\nChecking assertions:");
    println!("Contains '\\na: 1\\n': {}", out.contains("\na: 1\n"));
    println!("Contains '\\nb: 2\\n': {}", out.contains("\nb: 2\n"));
}
