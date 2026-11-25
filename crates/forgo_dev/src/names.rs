//! Crate names: one validator for every command that takes one.

use crate::cli::Kind;
use crate::error::{Error, Result};

/// Windows reserves these file names (in any case, with any extension), so
/// a crate directory can't use them.
const RESERVED: &[&str] = &[
    "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8",
    "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
];

/// Check that `name` can name a crate directory under `crates/`:
/// `[a-z][a-z0-9_]*`, at most 64 characters, and not a name Windows
/// reserves. This rules out paths (`..`, `a/b`, `a\b`), so a name can't
/// point outside `crates/`.
pub fn validate_name(name: &str) -> Result<()> {
    let bad = |why: &str| Err(Error::Usage(format!("invalid crate name `{name}`: {why}")));
    let mut chars = name.chars();
    match chars.next() {
        None => return bad("it is empty"),
        Some(c) if !c.is_ascii_lowercase() => return bad("it must start with a letter a-z"),
        Some(_) => {}
    }
    if chars.any(|c| !(c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')) {
        return bad("use only a-z, 0-9 and _");
    }
    if name.len() > 64 {
        return bad("it is longer than 64 characters");
    }
    if RESERVED.contains(&name) {
        return bad("Windows reserves this file name");
    }
    Ok(())
}

/// [`validate_name`] plus the naming rule for `kind`:
/// - `lib`: starts with `forgo_lib_`, doesn't end with `_wasm`;
/// - `wasm`: starts with `forgo_lib_` and ends with `_wasm`;
/// - `bin`: neither starts with `forgo_lib_` nor ends with `_wasm`.
pub fn validate_new(kind: Kind, name: &str) -> Result<()> {
    validate_name(name)?;
    let lib = name.starts_with("forgo_lib_");
    let wasm = name.ends_with("_wasm");
    let rule = match kind {
        Kind::Lib if !lib || wasm => {
            "lib crate names start with `forgo_lib_` and don't end with `_wasm` (that's `wasm`)"
        }
        Kind::Wasm if !lib || !wasm => {
            "wasm crate names start with `forgo_lib_` and end with `_wasm`"
        }
        Kind::Bin if lib || wasm => {
            "bin crate names don't start with `forgo_lib_` or end with `_wasm`"
        }
        _ => return Ok(()),
    };
    Err(Error::Usage(format!(
        "`{name}` isn't a {} crate name: {rule}",
        kind.as_str()
    )))
}
