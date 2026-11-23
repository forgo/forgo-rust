// crates/forgo_lib_yaml/src/util.rs
//! Public helpers.

use crate::emitter_helpers::{Ctx, plain_ok};

/// Does `s` need quotes to be written as a YAML string mapping value? True
/// unless `s` can be a plain scalar there that reads back as the same
/// string: no leading/trailing white space, no `: ` or ` #`, no leading
/// indicator, not a document marker, and not something the core schema
/// would read as null, a boolean or a number (`~`, `True`, `0x1F`, `.inf`).
pub fn needs_quotes(s: &str) -> bool {
    !plain_ok(
        s,
        Ctx {
            flow: false,
            key: false,
            block_ok: true,
            empty_ok: true,
            line_start: true,
        },
    )
}
