//! forgo_lib_toml as a toml-test encoder: toml-test's tagged JSON on
//! stdin, TOML on stdout (written by `Item::to_toml_string`). Run by
//! scripts/official-toml-test.sh.
use forgo_lib_toml::{Document, Item};
use serde_json::Value as Json;
use std::io::Read;

fn item(j: &Json) -> Result<Item, String> {
    match j {
        Json::Object(m)
            if m.len() == 2
                && m.get("type").is_some_and(Json::is_string)
                && m.contains_key("value") =>
        {
            let ty = m["type"].as_str().unwrap_or_default();
            let Some(v) = m["value"].as_str() else {
                // `{"type": "array", "value": [...]}` in older suites.
                return item(&m["value"]);
            };
            Ok(match ty {
                "string" => Item::String(v.to_string()),
                "integer" => Item::Integer(v.parse().map_err(|_| format!("integer {v:?}"))?),
                "float" => Item::Float(match v {
                    "nan" | "+nan" => f64::NAN,
                    "-nan" => -f64::NAN,
                    "inf" | "+inf" => f64::INFINITY,
                    "-inf" => f64::NEG_INFINITY,
                    _ => v.parse().map_err(|_| format!("float {v:?}"))?,
                }),
                "bool" => Item::Boolean(v == "true"),
                _ => {
                    let doc = Document::parse(&format!("x = {v}"))
                        .map_err(|e| format!("{ty} {v:?}: {e}"))?;
                    Item::Datetime(
                        doc.root()
                            .get("x")
                            .and_then(|x| x.as_datetime())
                            .ok_or("not a date-time")?,
                    )
                }
            })
        }
        Json::Object(m) => Ok(Item::Table(
            m.iter()
                .map(|(k, v)| Ok((k.clone(), item(v)?)))
                .collect::<Result<_, String>>()?,
        )),
        Json::Array(a) => Ok(Item::Array(a.iter().map(item).collect::<Result<_, _>>()?)),
        _ => Err("unexpected JSON".into()),
    }
}

fn main() {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input).expect("stdin");
    let json: Json = serde_json::from_str(&input).unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(1)
    });
    match item(&json).and_then(|i| i.to_toml_string().map_err(|e| e.to_string())) {
        Ok(text) => print!("{text}"),
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
}
