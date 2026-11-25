//! `cargo x`: see the library documentation (`forgo_dev`) for the commands.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match forgo_dev::run(&args, &repo_root()) {
        Ok(report) => {
            print!("{report}");
            ExitCode::SUCCESS
        }
        Err(forgo_dev::Error::Unmet(report)) => {
            print!("{report}");
            ExitCode::FAILURE
        }
        Err(e) => {
            eprintln!("error: {e}");
            if e.is_usage() {
                eprintln!("(run `cargo x help` for usage)");
                ExitCode::from(2)
            } else {
                ExitCode::FAILURE
            }
        }
    }
}

/// The workspace this binary belongs to: `crates/forgo_dev/../..`.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("forgo_dev lives at <workspace>/crates/forgo_dev")
        .to_path_buf()
}
