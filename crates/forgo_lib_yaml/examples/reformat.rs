//! Parse a YAML stream and print it as forgo_lib_yaml emits it.
//!
//! ```sh
//! cargo run -p forgo_lib_yaml --example reformat -- path/to/file.yaml
//! ```
//!
//! Reads standard input when no path is given.

use std::io::Read;

fn main() {
    let text = match std::env::args().nth(1) {
        Some(path) => std::fs::read_to_string(&path).unwrap_or_else(|e| {
            eprintln!("{path}: {e}");
            std::process::exit(2);
        }),
        None => {
            let mut s = String::new();
            std::io::stdin()
                .read_to_string(&mut s)
                .expect("read standard input");
            s
        }
    };
    match forgo_lib_yaml::Doc::from_stream(&text)
        .and_then(|d| forgo_lib_yaml::Doc::to_stream_string(&d))
    {
        Ok(out) => print!("{out}"),
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
}
