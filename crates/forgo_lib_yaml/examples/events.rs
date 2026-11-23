//! Print the parse events of a YAML file in yaml-test-suite notation.
//!
//! ```sh
//! cargo run -p forgo_lib_yaml --example events -- path/to/file.yaml
//! cargo run -p forgo_lib_yaml --example events -- tests/yaml-test-suite-data/2SXE/in.yaml \
//!   | diff - tests/yaml-test-suite-data/2SXE/test.event
//! ```
//!
//! Reads standard input when no path is given. Exits with status 1 after
//! printing the events that precede a syntax error.

use forgo_lib_yaml::syntax::{input::decode, test_event::to_test_event};
use std::io::Read;

fn main() {
    let bytes = match std::env::args().nth(1) {
        Some(path) => std::fs::read(&path).unwrap_or_else(|e| {
            eprintln!("{path}: {e}");
            std::process::exit(2);
        }),
        None => {
            let mut buf = Vec::new();
            std::io::stdin()
                .read_to_end(&mut buf)
                .expect("read standard input");
            buf
        }
    };
    let text = decode(&bytes).unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(1);
    });
    for r in forgo_lib_yaml::events(&text) {
        match r {
            Ok((ev, _)) => println!("{}", to_test_event(&ev)),
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(1);
            }
        }
    }
}
