//! forgo_dev: repo-native developer utilities for the forgo-rust monorepo.
//!
//! Commands:
//!   cargo x new lib  <name> [--public]   # library crate; adds to workspace + CI; if --public, also Release options
//!   cargo x new bin  <name> [--public]   # binary crate;  adds to workspace + CI; if --public, also Release options
//!   cargo x new wasm <name> [--public]   # WASM lib crate with boilerplate; same integrations
//!   cargo x rm [lib|bin|wasm] <name>     # remove crate folder, and scrub workspace, CI, and Release references
//!
//! Notes:
//! - Library crates must start with `forgo_lib_` and must NOT end with `_wasm`
//! - WASM  crates must start with `forgo_lib_` and must end with `_wasm`
//! - Binary crates must NOT start with `forgo_lib_` and must NOT end with `_wasm`

mod ci;
mod commands;
mod release;
mod templates;
mod util;
mod workspace;

use std::env;
use util::{die, help};

fn main() {
    let mut args = env::args().skip(1);
    let cmd = take_arg(args.next(), "usage: cargo x <new|rm|help> ...");
    match cmd.as_str() {
        "new" => {
            let kind = take_arg(args.next(), "new: expected <lib|bin|wasm>");
            let name = take_arg(args.next(), "new: expected <name>");
            let rest: Vec<String> = args.collect();
            let make_public = rest.iter().any(|s| s == "--public");
            commands::new(&kind, &name, make_public);
        }
        "rm" => {
            // Accept either: `cargo x rm <name>` OR `cargo x rm <kind> <name>`
            let first = take_arg(args.next(), "rm: expected <name> or <kind> <name>");
            let name = match first.as_str() {
                "lib" | "bin" | "wasm" => take_arg(args.next(), "rm: expected <name> after kind"),
                _ => first,
            };
            commands::rm(&name);
        }
        "help" | "--help" | "-h" => {
            help();
            std::process::exit(0);
        }
        other => {
            eprintln!("Unknown command: {other}\n");
            util::help_and_exit();
        }
    }
}

/// Like unwrap_or_else, but works with functions that return `!`.
fn take_arg(opt: Option<String>, msg: &str) -> String {
    match opt {
        Some(s) => s,
        None => die(msg),
    }
}
