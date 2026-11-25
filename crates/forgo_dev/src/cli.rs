//! Command-line parsing.

use crate::error::{Error, Result};

/// The kind of crate `cargo x new` creates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A library: `forgo_lib_<name>`.
    Lib,
    /// A binary: any name not starting with `forgo_lib_` or ending in `_wasm`.
    Bin,
    /// A WebAssembly library: `forgo_lib_<name>_wasm`.
    Wasm,
}

impl Kind {
    /// Every kind.
    pub const ALL: [Kind; 3] = [Kind::Lib, Kind::Bin, Kind::Wasm];

    /// The kind named on the command line.
    pub fn parse(s: &str) -> Option<Kind> {
        match s {
            "lib" => Some(Kind::Lib),
            "bin" => Some(Kind::Bin),
            "wasm" => Some(Kind::Wasm),
            _ => None,
        }
    }

    /// The command-line name of the kind.
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Lib => "lib",
            Kind::Bin => "bin",
            Kind::Wasm => "wasm",
        }
    }
}

/// A parsed command line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// `new <lib|bin|wasm> <name> [--public]`
    New {
        /// What to create.
        kind: Kind,
        /// The crate's name (and directory under `crates/`).
        name: String,
        /// Publishable: `publish = true`, and listed in the release workflow.
        public: bool,
    },
    /// `rm [<kind>] <name> [--yes]`
    Rm {
        /// The kind, if given: the name must then fit it.
        kind: Option<Kind>,
        /// The crate to remove.
        name: String,
        /// Apply the changes; without it, only report them.
        yes: bool,
    },
    /// `semver ...`: the arguments after `semver`.
    Semver(Vec<String>),
    /// `version ...`: the arguments after `version`.
    Version(Vec<String>),
    /// `toml ...`: the arguments after `toml`.
    Toml(Vec<String>),
    /// `ci ...`: the arguments after `ci`.
    Ci(Vec<String>),
    /// `deps ...`: the arguments after `deps`.
    Deps(Vec<String>),
    /// `help`, `--help` or `-h`.
    Help,
}

/// Parse the arguments after `cargo x`.
pub fn parse_args(args: &[String]) -> Result<Command> {
    let usage = |m: &str| Err(Error::Usage(m.to_string()));
    let Some(cmd) = args.first() else {
        return usage("expected a command");
    };
    match cmd.as_str() {
        "help" | "--help" | "-h" => Ok(Command::Help),
        "new" => {
            let Some(kind) = args.get(1) else {
                return usage("new: expected <lib|bin|wasm>");
            };
            let Some(kind) = Kind::parse(kind) else {
                return usage(&format!(
                    "new: unknown kind `{kind}` (expected lib, bin or wasm)"
                ));
            };
            let Some(name) = args.get(2) else {
                return usage("new: expected <name>");
            };
            let mut public = false;
            for a in &args[3..] {
                match a.as_str() {
                    "--public" if !public => public = true,
                    _ => return usage(&format!("new: unexpected argument `{a}`")),
                }
            }
            Ok(Command::New {
                kind,
                name: name.clone(),
                public,
            })
        }
        "rm" => {
            // `rm <name>` or `rm <kind> <name>`.
            let mut rest = &args[1..];
            let kind = rest.first().and_then(|k| Kind::parse(k));
            if kind.is_some() {
                rest = &rest[1..];
            }
            let Some(name) = rest.first() else {
                return usage("rm: expected <name>");
            };
            let mut yes = false;
            for a in &rest[1..] {
                match a.as_str() {
                    "--yes" | "-y" if !yes => yes = true,
                    _ => return usage(&format!("rm: unexpected argument `{a}`")),
                }
            }
            Ok(Command::Rm {
                kind,
                name: name.clone(),
                yes,
            })
        }
        "semver" => Ok(Command::Semver(args[1..].to_vec())),
        "version" => Ok(Command::Version(args[1..].to_vec())),
        "toml" => Ok(Command::Toml(args[1..].to_vec())),
        "ci" => Ok(Command::Ci(args[1..].to_vec())),
        "deps" => Ok(Command::Deps(args[1..].to_vec())),
        other => usage(&format!("unknown command `{other}`")),
    }
}
