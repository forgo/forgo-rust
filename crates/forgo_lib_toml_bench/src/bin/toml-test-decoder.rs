//! forgo_lib_toml as a toml-test decoder: TOML on stdin, toml-test's
//! tagged JSON on stdout; exit code 1 for invalid TOML.
//!
//! `--toml-1.0` parses TOML 1.0 (1.1 otherwise). Run by
//! scripts/official-toml-test.sh.
use forgo_lib_toml::{Document, ParseOptions, TomlVersion, Value};
use serde_json::{Map, Value as Json, json};
use std::io::Read;

fn describe(v: Value<'_>) -> Json {
    let tagged = |ty: &str, value: String| json!({"type": ty, "value": value});
    match v {
        Value::String(s) => tagged("string", s.to_string()),
        Value::Integer(i) => tagged("integer", i.to_string()),
        Value::Float(f) if f.is_nan() => tagged("float", "nan".into()),
        Value::Float(f) if f.is_infinite() => {
            tagged("float", if f > 0.0 { "inf" } else { "-inf" }.into())
        }
        Value::Float(f) => tagged("float", format!("{f:?}")),
        Value::Boolean(b) => tagged("bool", b.to_string()),
        Value::Datetime(d) => tagged(d.kind_name(), d.to_string()),
        Value::Array(a) => Json::Array(a.iter().map(describe).collect()),
        Value::Table(t) => Json::Object(
            t.iter()
                .map(|(k, v)| (k.to_string(), describe(v)))
                .collect::<Map<_, _>>(),
        ),
    }
}

fn main() {
    let version = if std::env::args().any(|a| a == "--toml-1.0") {
        TomlVersion::V1_0
    } else {
        TomlVersion::V1_1
    };
    let mut input = Vec::new();
    std::io::stdin().read_to_end(&mut input).expect("stdin");
    match Document::from_slice(&input, &ParseOptions::default().with_version(version)) {
        Ok(doc) => println!("{}", describe(Value::Table(doc.root()))),
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
}
