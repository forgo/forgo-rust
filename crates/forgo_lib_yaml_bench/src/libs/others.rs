//! Independent pure-Rust loaders: rust-yaml and yaml-peg.

use crate::data::{Data, float_text};
use crate::libs::Emitter;

// ---------------------------------------------------------------------------
// rust-yaml 1.1
// ---------------------------------------------------------------------------

fn ry_docs(s: &str) -> Result<Vec<rust_yaml::Value>, String> {
    rust_yaml::Yaml::new()
        .load_all_str(s)
        .map_err(|e| e.to_string())
}

fn ry_data(v: &rust_yaml::Value) -> Data {
    use rust_yaml::Value as V;
    match v {
        V::Null => Data::Null,
        V::Bool(b) => Data::Bool(*b),
        V::Int(i) => Data::Int(*i as i128),
        V::Float(f) => Data::Float(*f),
        V::String(s) => Data::Str(s.clone()),
        V::Sequence(a) => Data::Seq(a.iter().map(ry_data).collect()),
        V::Mapping(m) => Data::Map(m.iter().map(|(k, v)| (ry_data(k), ry_data(v))).collect()),
    }
}

pub fn rust_yaml_load(s: &str) -> Result<Vec<Data>, String> {
    Ok(ry_docs(s)?.iter().map(ry_data).collect())
}

pub fn rust_yaml_bench(s: &str) -> Result<usize, String> {
    ry_docs(s).map(|d| d.len())
}

pub fn rust_yaml_emit(s: &str) -> Result<String, String> {
    let docs = ry_docs(s)?;
    rust_yaml::Yaml::new()
        .dump_all_str(&docs)
        .map_err(|e| e.to_string())
}

pub fn rust_yaml_emitter(s: &str) -> Result<Emitter, String> {
    let docs = ry_docs(s)?;
    let y = rust_yaml::Yaml::new();
    Ok(Box::new(move || {
        y.dump_all_str(&docs)
            .map(|t| t.len())
            .map_err(|e| e.to_string())
    }))
}

// ---------------------------------------------------------------------------
// yaml-peg 1.0
// ---------------------------------------------------------------------------

type PegNode = yaml_peg::Node<yaml_peg::repr::RcRepr>;

fn peg_docs(s: &str) -> Result<Vec<PegNode>, String> {
    yaml_peg::parse::<yaml_peg::repr::RcRepr>(s).map_err(|e| e.to_string())
}

fn peg_data(n: &PegNode) -> Data {
    use yaml_peg::Yaml as Y;
    let v = match n.yaml() {
        Y::Null => Data::Null,
        Y::Bool(b) => Data::Bool(*b),
        Y::Int(t) => match crate::data::core_int(t) {
            Some(i) => Data::Int(i),
            None => float_text(t),
        },
        Y::Float(t) => float_text(t),
        Y::Str(s) => Data::Str(s.clone()),
        Y::Seq(a) => Data::Seq(a.iter().map(peg_data).collect()),
        Y::Map(m) => Data::Map(m.iter().map(|(k, v)| (peg_data(k), peg_data(v))).collect()),
        Y::Alias(a) => Data::Str(format!("*{a}")),
    };
    match n.tag() {
        "" => v,
        t if t.starts_with("tag:yaml.org,2002:") => v,
        t => Data::Tagged(t.to_string(), Box::new(v)),
    }
}

pub fn yaml_peg_load(s: &str) -> Result<Vec<Data>, String> {
    Ok(peg_docs(s)?.iter().map(peg_data).collect())
}

pub fn yaml_peg_bench(s: &str) -> Result<usize, String> {
    peg_docs(s).map(|d| d.len())
}

pub fn yaml_peg_emit(s: &str) -> Result<String, String> {
    Ok(yaml_peg::dump(&peg_docs(s)?, &[]))
}

pub fn yaml_peg_emitter(s: &str) -> Result<Emitter, String> {
    let docs = peg_docs(s)?;
    Ok(Box::new(move || Ok(yaml_peg::dump(&docs, &[]).len())))
}
