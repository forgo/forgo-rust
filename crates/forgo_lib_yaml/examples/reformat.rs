//! Parse a YAML stream and print it in the emitter's layout (two-space
//! indentation, compact `- key: value` items), keeping comments and
//! styles. Without `reformat()`, writing would give back the input.
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
    let written = forgo_lib_yaml::Doc::from_stream(&text).and_then(|mut docs| {
        for d in &mut docs {
            d.reformat();
        }
        forgo_lib_yaml::Doc::to_stream_string(&docs)
    });
    match written {
        Ok(out) => print!("{out}"),
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
}
