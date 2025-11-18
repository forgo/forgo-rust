use forgo_lib_yaml::Doc;

fn main() {
    let s = "[+5, -3, 0]\n";
    let d = Doc::from_str(s).expect("parse");

    println!("Parsed:");
    println!("{:#?}", d.root());

    let out = d.to_string().expect("emit");
    println!("\nEmitted:");
    println!("{:?}", out);
    println!("\nFormatted:");
    println!("{}", out);
}
