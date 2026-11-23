//! Adapters: each library's parse, load and emit APIs behind one shape.

pub mod forgo;
pub mod libyaml;
pub mod others;
pub mod rustyaml;
pub mod serdes;
pub mod yamledit;

use crate::data::Data;
use crate::suite::Ev;

/// Join emitted documents into one stream, adding `---` separators
/// where the emitter didn't write them.
pub fn join_docs(
    parts: impl IntoIterator<Item = Result<String, String>>,
) -> Result<String, String> {
    let mut out = String::new();
    for (i, p) in parts.into_iter().enumerate() {
        let p = p?;
        if i > 0 {
            if !out.ends_with('\n') {
                out.push('\n');
            }
            if !p.starts_with("---") {
                out.push_str("---\n");
            }
        }
        out.push_str(&p);
    }
    Ok(out)
}

/// Stands in for a node a loader couldn't represent (yaml-rust's
/// `BadValue`): the document loads, but its data is wrong.
pub const BAD_VALUE: &str = "\u{0}<bad value>";

/// Emits a loaded tree again; returns the output length in bytes.
pub type Emitter = Box<dyn Fn() -> Result<usize, String>>;

/// Parse or load and return a count (events or documents).
pub type CountFn = fn(&str) -> Result<usize, String>;
/// Load, then write with the library's own emitter.
pub type EmitFn = fn(&str) -> Result<String, String>;
/// Load once; the returned closure emits.
pub type EmitterFn = fn(&str) -> Result<Emitter, String>;

/// A parser with an event (pull or push) API.
pub struct EventLib {
    pub name: &'static str,
    /// crates.io package, for the version column.
    pub krate: &'static str,
    /// Events carry everything `test.event` needs (anchor names, flow
    /// markers, explicit document markers).
    pub exact: bool,
    pub events: fn(&str) -> Result<Vec<Ev>, String>,
    /// Parse and count events (benchmarks).
    pub count: CountFn,
}

/// A loader into the library's own tree.
pub struct LoadLib {
    pub name: &'static str,
    pub krate: &'static str,
    /// The tree it loads into, for the notes.
    pub tree: &'static str,
    pub load: fn(&str) -> Result<Vec<Data>, String>,
    /// Load and drop the tree (benchmarks); returns the document count.
    pub bench: CountFn,
    /// Load, then emit with the library's own emitter.
    pub emit: Option<EmitFn>,
    /// Load once; the returned closure emits (benchmarks).
    pub emitter: Option<EmitterFn>,
    /// The tree is lossless: emitting writes the source text back, so its
    /// speed is a write-back, not a render.
    pub lossless: bool,
}

pub const EVENT_LIBS: &[EventLib] = &[
    EventLib {
        name: "forgo_lib_yaml",
        krate: "forgo_lib_yaml",
        exact: true,
        events: forgo::events,
        count: forgo::count_events,
    },
    EventLib {
        name: "libyaml",
        krate: "unsafe-libyaml",
        exact: true,
        events: libyaml::libyaml_events,
        count: libyaml::libyaml_count,
    },
    EventLib {
        name: "libyaml-safer",
        krate: "libyaml-safer",
        exact: true,
        events: libyaml::safer_events,
        count: libyaml::safer_count,
    },
    EventLib {
        name: "granit-parser",
        krate: "granit-parser",
        exact: true,
        events: rustyaml::granit_events,
        count: rustyaml::granit_count,
    },
    EventLib {
        name: "saphyr-parser",
        krate: "saphyr-parser",
        exact: false,
        events: rustyaml::saphyr_events,
        count: rustyaml::saphyr_count,
    },
    EventLib {
        name: "yaml-rust2",
        krate: "yaml-rust2",
        exact: false,
        events: rustyaml::yaml_rust2_events,
        count: rustyaml::yaml_rust2_count,
    },
    EventLib {
        name: "yaml-rust",
        krate: "yaml-rust",
        exact: false,
        events: rustyaml::yaml_rust_events,
        count: rustyaml::yaml_rust_count,
    },
];

pub const LOAD_LIBS: &[LoadLib] = &[
    LoadLib {
        name: "forgo_lib_yaml",
        krate: "forgo_lib_yaml",
        tree: "`Doc` (with comments, styles, spans)",
        load: forgo::load,
        bench: forgo::load_bench,
        emit: Some(forgo::emit),
        emitter: Some(forgo::emitter),
        lossless: false,
    },
    LoadLib {
        name: "serde_yaml",
        krate: "serde_yaml",
        tree: "`serde_yaml::Value`",
        load: serdes::serde_yaml_09::load,
        bench: serdes::serde_yaml_09::bench,
        emit: Some(serdes::serde_yaml_09::emit),
        emitter: Some(serdes::serde_yaml_09::emitter),
        lossless: false,
    },
    LoadLib {
        name: "serde_yaml_ng",
        krate: "serde_yaml_ng",
        tree: "`serde_yaml_ng::Value`",
        load: serdes::serde_yaml_ng_010::load,
        bench: serdes::serde_yaml_ng_010::bench,
        emit: Some(serdes::serde_yaml_ng_010::emit),
        emitter: Some(serdes::serde_yaml_ng_010::emitter),
        lossless: false,
    },
    LoadLib {
        name: "serde_norway",
        krate: "serde_norway",
        tree: "`serde_norway::Value`",
        load: serdes::serde_norway_09::load,
        bench: serdes::serde_norway_09::bench,
        emit: Some(serdes::serde_norway_09::emit),
        emitter: Some(serdes::serde_norway_09::emitter),
        lossless: false,
    },
    LoadLib {
        name: "yaml_serde",
        krate: "yaml_serde",
        tree: "`yaml_serde::Value`",
        load: serdes::yaml_serde_010::load,
        bench: serdes::yaml_serde_010::bench,
        emit: Some(serdes::yaml_serde_010::emit),
        emitter: Some(serdes::yaml_serde_010::emitter),
        lossless: false,
    },
    LoadLib {
        name: "serde-saphyr",
        krate: "serde-saphyr",
        tree: "none (a generic serde value)",
        load: serdes::serde_saphyr_2::load,
        bench: serdes::serde_saphyr_2::bench,
        emit: Some(serdes::serde_saphyr_2::emit),
        emitter: Some(serdes::serde_saphyr_2::emitter),
        lossless: false,
    },
    LoadLib {
        name: "noyalib",
        krate: "noyalib",
        tree: "`noyalib::Value`",
        load: serdes::noyalib_0::load,
        bench: serdes::noyalib_0::bench,
        emit: Some(serdes::noyalib_0::emit),
        emitter: Some(serdes::noyalib_0::emitter),
        lossless: false,
    },
    LoadLib {
        name: "noyalib (CST)",
        krate: "noyalib",
        tree: "lossless syntax tree",
        load: serdes::noyalib_cst::load,
        bench: serdes::noyalib_cst::bench,
        emit: Some(serdes::noyalib_cst::emit),
        emitter: Some(serdes::noyalib_cst::emitter),
        lossless: true,
    },
    LoadLib {
        name: "saphyr",
        krate: "saphyr",
        tree: "`saphyr::Yaml`",
        load: rustyaml::saphyr_load,
        bench: rustyaml::saphyr_bench,
        emit: Some(rustyaml::saphyr_emit),
        emitter: Some(rustyaml::saphyr_emitter),
        lossless: false,
    },
    LoadLib {
        name: "yaml-rust2",
        krate: "yaml-rust2",
        tree: "`yaml_rust2::Yaml`",
        load: rustyaml::yaml_rust2_load,
        bench: rustyaml::yaml_rust2_bench,
        emit: Some(rustyaml::yaml_rust2_emit),
        emitter: Some(rustyaml::yaml_rust2_emitter),
        lossless: false,
    },
    LoadLib {
        name: "yaml-rust",
        krate: "yaml-rust",
        tree: "`yaml_rust::Yaml`",
        load: rustyaml::yaml_rust_load,
        bench: rustyaml::yaml_rust_bench,
        emit: Some(rustyaml::yaml_rust_emit),
        emitter: Some(rustyaml::yaml_rust_emitter),
        lossless: false,
    },
    LoadLib {
        name: "yaml-edit",
        krate: "yaml-edit",
        tree: "lossless syntax tree",
        load: yamledit::load,
        bench: yamledit::bench,
        emit: Some(yamledit::emit),
        emitter: Some(yamledit::emitter),
        lossless: true,
    },
    LoadLib {
        name: "rust-yaml",
        krate: "rust-yaml",
        tree: "`rust_yaml::Value`",
        load: others::rust_yaml_load,
        bench: others::rust_yaml_bench,
        emit: Some(others::rust_yaml_emit),
        emitter: Some(others::rust_yaml_emitter),
        lossless: false,
    },
    LoadLib {
        name: "yaml-peg",
        krate: "yaml-peg",
        tree: "`yaml_peg::Node`",
        load: others::yaml_peg_load,
        bench: others::yaml_peg_bench,
        emit: Some(others::yaml_peg_emit),
        emitter: Some(others::yaml_peg_emitter),
        lossless: false,
    },
];
