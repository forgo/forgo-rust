//! serde-based loaders: the serde_yaml family (all on libyaml), noyalib
//! and serde-saphyr.

use crate::data::Data;
use crate::libs::{Emitter, join_docs};

/// The serde_yaml forks share serde_yaml's `Value` and API.
macro_rules! serde_yaml_like {
    ($m:ident, $krate:ident) => {
        pub mod $m {
            use super::*;
            use serde::Deserialize as _;
            use $krate::Value;

            fn docs(s: &str) -> Result<Vec<Value>, String> {
                let mut out = Vec::new();
                for doc in $krate::Deserializer::from_str(s) {
                    out.push(Value::deserialize(doc).map_err(|e| e.to_string())?);
                }
                Ok(out)
            }

            fn data(v: &Value) -> Data {
                match v {
                    Value::Null => Data::Null,
                    Value::Bool(b) => Data::Bool(*b),
                    Value::Number(n) => match (n.as_i64(), n.as_u64(), n.as_f64()) {
                        (Some(i), _, _) => Data::Int(i as i128),
                        (_, Some(u), _) => Data::Int(u as i128),
                        (_, _, f) => Data::Float(f.unwrap_or(f64::NAN)),
                    },
                    Value::String(s) => Data::Str(s.clone()),
                    Value::Sequence(a) => Data::Seq(a.iter().map(data).collect()),
                    Value::Mapping(m) => {
                        Data::Map(m.iter().map(|(k, v)| (data(k), data(v))).collect())
                    }
                    Value::Tagged(t) => Data::Tagged(t.tag.to_string(), Box::new(data(&t.value))),
                }
            }

            pub fn load(s: &str) -> Result<Vec<Data>, String> {
                Ok(docs(s)?.iter().map(data).collect())
            }

            pub fn bench(s: &str) -> Result<usize, String> {
                docs(s).map(|d| d.len())
            }

            fn dump(docs: &[Value]) -> Result<String, String> {
                join_docs(
                    docs.iter()
                        .map(|d| $krate::to_string(d).map_err(|e| e.to_string())),
                )
            }

            pub fn emit(s: &str) -> Result<String, String> {
                dump(&docs(s)?)
            }

            pub fn emitter(s: &str) -> Result<Emitter, String> {
                let d = docs(s)?;
                Ok(Box::new(move || dump(&d).map(|t| t.len())))
            }
        }
    };
}

serde_yaml_like!(serde_yaml_09, serde_yaml);
serde_yaml_like!(serde_yaml_ng_010, serde_yaml_ng);
serde_yaml_like!(serde_norway_09, serde_norway);
serde_yaml_like!(yaml_serde_010, yaml_serde);

// ---------------------------------------------------------------------------
// noyalib
// ---------------------------------------------------------------------------

pub mod noyalib_0 {
    use super::*;
    use noyalib::Value;

    fn docs(s: &str) -> Result<Vec<Value>, String> {
        let mut out = Vec::new();
        for d in noyalib::load_all(s).map_err(|e| e.to_string())? {
            out.push(d.map_err(|e| e.to_string())?);
        }
        Ok(out)
    }

    pub(super) fn data(v: &Value) -> Data {
        match v {
            Value::Null => Data::Null,
            Value::Bool(b) => Data::Bool(*b),
            Value::Number(n) => match n {
                noyalib::Number::Integer(i) => Data::Int(*i as i128),
                noyalib::Number::Float(f) => Data::Float(*f),
                #[allow(unreachable_patterns)]
                other => crate::data::float_text(&other.to_string()),
            },
            Value::String(s) => Data::Str(s.clone()),
            Value::Sequence(a) => Data::Seq(a.iter().map(data).collect()),
            Value::Mapping(m) => Data::Map(
                m.iter()
                    .map(|(k, v)| (Data::Str(k.clone()), data(v)))
                    .collect(),
            ),
            Value::Tagged(t) => Data::Tagged(t.tag().to_string(), Box::new(data(t.value()))),
        }
    }

    pub fn load(s: &str) -> Result<Vec<Data>, String> {
        Ok(docs(s)?.iter().map(data).collect())
    }

    /// Benchmarks lift the default limit of 1,000 documents per stream.
    fn docs_unlimited(s: &str) -> Result<Vec<Value>, String> {
        let cfg = noyalib::ParserConfig::default().max_documents(usize::MAX);
        let mut out = Vec::new();
        for d in noyalib::load_all_with_config(s, &cfg).map_err(|e| e.to_string())? {
            out.push(d.map_err(|e| e.to_string())?);
        }
        Ok(out)
    }

    pub fn bench(s: &str) -> Result<usize, String> {
        docs_unlimited(s).map(|d| d.len())
    }

    fn dump(docs: &[Value]) -> Result<String, String> {
        join_docs(
            docs.iter()
                .map(|d| noyalib::to_string(d).map_err(|e| e.to_string())),
        )
    }

    pub fn emit(s: &str) -> Result<String, String> {
        dump(&docs(s)?)
    }

    pub fn emitter(s: &str) -> Result<Emitter, String> {
        let d = docs_unlimited(s)?;
        Ok(Box::new(move || dump(&d).map(|t| t.len())))
    }
}

/// noyalib's lossless syntax tree (`noyalib::cst`), its format-preserving
/// editing API. Writing it back gives the source text.
pub mod noyalib_cst {
    use super::*;

    fn docs(s: &str, unlimited: bool) -> Result<Vec<noyalib::cst::Document>, String> {
        if unlimited {
            let cfg = noyalib::ParserConfig::default().max_documents(usize::MAX);
            noyalib::cst::parse_stream_with_config(s, &cfg).map_err(|e| e.to_string())
        } else {
            noyalib::cst::parse_stream(s).map_err(|e| e.to_string())
        }
    }

    pub fn load(s: &str) -> Result<Vec<Data>, String> {
        Ok(docs(s, false)?
            .iter()
            .map(|d| super::noyalib_0::data(&d.as_value()))
            .collect())
    }

    pub fn bench(s: &str) -> Result<usize, String> {
        docs(s, true).map(|d| d.len())
    }

    pub fn emit(s: &str) -> Result<String, String> {
        Ok(docs(s, false)?.iter().map(|d| d.to_string()).collect())
    }

    pub fn emitter(s: &str) -> Result<Emitter, String> {
        let d = docs(s, true)?;
        Ok(Box::new(move || {
            Ok(d.iter().map(|d| d.to_string().len()).sum())
        }))
    }
}

// ---------------------------------------------------------------------------
// serde-saphyr: no tree type of its own, so it loads into `AnyValue`
// ---------------------------------------------------------------------------

/// A minimal generic serde value (any keys, any numbers), for serde
/// deserializers that don't ship a `Value` type.
#[derive(Clone, Debug)]
pub enum AnyValue {
    Null,
    Bool(bool),
    Int(i128),
    Float(f64),
    Str(String),
    Bytes(Vec<u8>),
    Seq(Vec<AnyValue>),
    Map(Vec<(AnyValue, AnyValue)>),
    Tagged(String, Box<AnyValue>),
}

impl AnyValue {
    pub fn data(&self) -> Data {
        match self {
            AnyValue::Null => Data::Null,
            AnyValue::Bool(b) => Data::Bool(*b),
            AnyValue::Int(i) => Data::Int(*i),
            AnyValue::Float(f) => Data::Float(*f),
            AnyValue::Str(s) => Data::Str(s.clone()),
            AnyValue::Bytes(b) => Data::Str(String::from_utf8_lossy(b).into_owned()),
            AnyValue::Seq(a) => Data::Seq(a.iter().map(AnyValue::data).collect()),
            AnyValue::Map(m) => Data::Map(m.iter().map(|(k, v)| (k.data(), v.data())).collect()),
            AnyValue::Tagged(t, v) => Data::Tagged(t.clone(), Box::new(v.data())),
        }
    }
}

struct AnyVisitor;

impl<'de> serde::de::Visitor<'de> for AnyVisitor {
    type Value = AnyValue;

    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("any value")
    }
    fn visit_unit<E>(self) -> Result<AnyValue, E> {
        Ok(AnyValue::Null)
    }
    fn visit_none<E>(self) -> Result<AnyValue, E> {
        Ok(AnyValue::Null)
    }
    fn visit_some<D: serde::Deserializer<'de>>(self, d: D) -> Result<AnyValue, D::Error> {
        serde::Deserialize::deserialize(d)
    }
    fn visit_newtype_struct<D: serde::Deserializer<'de>>(self, d: D) -> Result<AnyValue, D::Error> {
        serde::Deserialize::deserialize(d)
    }
    fn visit_bool<E>(self, v: bool) -> Result<AnyValue, E> {
        Ok(AnyValue::Bool(v))
    }
    fn visit_i64<E>(self, v: i64) -> Result<AnyValue, E> {
        Ok(AnyValue::Int(v as i128))
    }
    fn visit_u64<E>(self, v: u64) -> Result<AnyValue, E> {
        Ok(AnyValue::Int(v as i128))
    }
    fn visit_i128<E>(self, v: i128) -> Result<AnyValue, E> {
        Ok(AnyValue::Int(v))
    }
    fn visit_u128<E>(self, v: u128) -> Result<AnyValue, E> {
        Ok(AnyValue::Int(v as i128))
    }
    fn visit_f64<E>(self, v: f64) -> Result<AnyValue, E> {
        Ok(AnyValue::Float(v))
    }
    fn visit_str<E>(self, v: &str) -> Result<AnyValue, E> {
        Ok(AnyValue::Str(v.to_string()))
    }
    fn visit_string<E>(self, v: String) -> Result<AnyValue, E> {
        Ok(AnyValue::Str(v))
    }
    fn visit_bytes<E>(self, v: &[u8]) -> Result<AnyValue, E> {
        Ok(AnyValue::Bytes(v.to_vec()))
    }
    fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut a: A) -> Result<AnyValue, A::Error> {
        let mut out = Vec::new();
        while let Some(v) = a.next_element()? {
            out.push(v);
        }
        Ok(AnyValue::Seq(out))
    }
    fn visit_map<A: serde::de::MapAccess<'de>>(self, mut a: A) -> Result<AnyValue, A::Error> {
        let mut out = Vec::new();
        while let Some((k, v)) = a.next_entry()? {
            out.push((k, v));
        }
        Ok(AnyValue::Map(out))
    }
    fn visit_enum<A: serde::de::EnumAccess<'de>>(self, a: A) -> Result<AnyValue, A::Error> {
        use serde::de::VariantAccess as _;
        let (tag, v): (String, _) = a.variant()?;
        Ok(AnyValue::Tagged(tag, Box::new(v.newtype_variant()?)))
    }
}

impl<'de> serde::Deserialize<'de> for AnyValue {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<AnyValue, D::Error> {
        d.deserialize_any(AnyVisitor)
    }
}

impl serde::Serialize for AnyValue {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use serde::ser::{SerializeMap as _, SerializeSeq as _};
        match self {
            AnyValue::Null => s.serialize_unit(),
            AnyValue::Bool(b) => s.serialize_bool(*b),
            AnyValue::Int(i) => match i64::try_from(*i) {
                Ok(i) => s.serialize_i64(i),
                Err(_) => s.serialize_i128(*i),
            },
            AnyValue::Float(f) => s.serialize_f64(*f),
            AnyValue::Str(v) => s.serialize_str(v),
            AnyValue::Bytes(b) => s.serialize_bytes(b),
            AnyValue::Seq(items) => {
                let mut seq = s.serialize_seq(Some(items.len()))?;
                for it in items {
                    seq.serialize_element(it)?;
                }
                seq.end()
            }
            AnyValue::Map(entries) => {
                let mut map = s.serialize_map(Some(entries.len()))?;
                for (k, v) in entries {
                    map.serialize_entry(k, v)?;
                }
                map.end()
            }
            AnyValue::Tagged(_, v) => v.serialize(s),
        }
    }
}

pub mod serde_saphyr_2 {
    use super::*;

    fn docs(s: &str) -> Result<Vec<AnyValue>, String> {
        serde_saphyr::from_multiple::<AnyValue>(s).map_err(|e| e.to_string())
    }

    pub fn load(s: &str) -> Result<Vec<Data>, String> {
        Ok(docs(s)?.iter().map(AnyValue::data).collect())
    }

    /// Benchmarks turn off the default budget (1,024 documents, depth 64).
    fn docs_unlimited(s: &str) -> Result<Vec<AnyValue>, String> {
        let mut o = serde_saphyr::Options::default();
        o.budget = None;
        serde_saphyr::from_multiple_with_options::<AnyValue>(s, o).map_err(|e| e.to_string())
    }

    pub fn bench(s: &str) -> Result<usize, String> {
        docs_unlimited(s).map(|d| d.len())
    }

    fn dump(docs: &[AnyValue]) -> Result<String, String> {
        join_docs(
            docs.iter()
                .map(|d| serde_saphyr::to_string(d).map_err(|e| e.to_string())),
        )
    }

    pub fn emit(s: &str) -> Result<String, String> {
        dump(&docs(s)?)
    }

    pub fn emitter(s: &str) -> Result<Emitter, String> {
        let d = docs_unlimited(s)?;
        Ok(Box::new(move || dump(&d).map(|t| t.len())))
    }
}
