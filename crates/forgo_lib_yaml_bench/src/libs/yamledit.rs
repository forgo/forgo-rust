//! yaml-edit: a lossless (rowan CST) parser and editor. Its parser is
//! error-tolerant; an input counts as rejected when parsing fails or its
//! `Validator` reports an error. Values are read the way yaml-edit's own
//! yaml-test-suite harness reads them.

use crate::data::Data;
use crate::libs::Emitter;
use rowan::ast::AstNode as _;
use std::str::FromStr as _;
use yaml_edit::anchor_resolution::AnchorRegistry;
use yaml_edit::validator::{Severity, Validator};
use yaml_edit::{Scalar, ScalarStyle, ScalarType, ScalarValue, SyntaxKind, YamlFile, YamlNode};

fn parse(s: &str) -> Result<YamlFile, String> {
    let file = YamlFile::from_str(s).map_err(|e| e.to_string())?;
    let errors: Vec<String> = Validator::new()
        .validate_syntax(file.syntax())
        .into_iter()
        .filter(|v| v.severity == Severity::Error)
        .map(|v| v.to_string())
        .collect();
    if errors.is_empty() {
        Ok(file)
    } else {
        Err(errors.join("; "))
    }
}

struct Converter {
    anchors: AnchorRegistry,
}

impl Converter {
    fn node(&self, node: &YamlNode) -> Result<Data, String> {
        match node {
            YamlNode::Scalar(s) => self.scalar(s, None),
            YamlNode::Mapping(m) => {
                let mut out = Vec::new();
                for entry in m.entries() {
                    let key = match entry.key_node() {
                        None => Data::Null,
                        Some(k) => self.node(&k)?,
                    };
                    let value = match entry.value_node() {
                        None => Data::Null,
                        Some(v) => self.node(&v)?,
                    };
                    out.push((key, value));
                }
                Ok(Data::Map(out))
            }
            YamlNode::Sequence(seq) => seq
                .values()
                .map(|i| self.node(&i))
                .collect::<Result<_, _>>()
                .map(Data::Seq),
            YamlNode::Alias(a) => self.node(&self.resolve(&a.name())?),
            YamlNode::TaggedNode(t) => {
                let tag = t.resolved_tag();
                if let Some(m) = t.as_mapping() {
                    self.node(&YamlNode::Mapping(m))
                } else if let Some(s) = t.as_sequence() {
                    self.node(&YamlNode::Sequence(s))
                } else if let Some(s) = t.value() {
                    self.scalar(&s, tag.as_deref())
                } else {
                    Err(format!("tagged node {tag:?} has no value"))
                }
            }
        }
    }

    fn resolve(&self, name: &str) -> Result<YamlNode, String> {
        let node = self
            .anchors
            .resolve(name)
            .ok_or_else(|| format!("undefined alias *{name}"))?;
        let node = if node.kind() == SyntaxKind::VALUE {
            node.first_child()
        } else {
            Some(node.clone())
        };
        node.and_then(YamlNode::from_syntax)
            .ok_or_else(|| format!("anchor &{name} has no value"))
    }

    fn scalar(&self, scalar: &Scalar, tag: Option<&str>) -> Result<Data, String> {
        let value = ScalarValue::from_scalar(scalar);
        let ty = match tag {
            Some("!!str") | Some("!") => ScalarType::String,
            Some("!!int") => ScalarType::Integer,
            Some("!!float") => ScalarType::Float,
            Some("!!bool") => ScalarType::Boolean,
            Some("!!null") => ScalarType::Null,
            _ if value.style() == ScalarStyle::Plain && value.value().is_empty() => {
                ScalarType::Null
            }
            _ => value.scalar_type(),
        };
        let invalid = || format!("{:?} is not a valid {ty:?}", value.value());
        let coerced = value.coerce_to_type(ty).ok_or_else(invalid)?;
        Ok(match ty {
            ScalarType::Null => Data::Null,
            ScalarType::Boolean => Data::Bool(coerced.to_bool().ok_or_else(invalid)?),
            ScalarType::Integer => Data::Int(coerced.to_i64().ok_or_else(invalid)? as i128),
            ScalarType::Float => Data::Float(coerced.to_f64().ok_or_else(invalid)?),
            _ => Data::Str(value.value().to_string()),
        })
    }
}

pub fn load(s: &str) -> Result<Vec<Data>, String> {
    let file = parse(s)?;
    file.documents()
        .map(|doc| {
            let c = Converter {
                anchors: AnchorRegistry::from_document(&doc),
            };
            match doc.syntax().children().find_map(YamlNode::from_syntax) {
                None => Ok(Data::Null),
                Some(root) => c.node(&root),
            }
        })
        .collect()
}

/// Loading for benchmarks: building the syntax tree.
pub fn bench(s: &str) -> Result<usize, String> {
    let file = YamlFile::from_str(s).map_err(|e| e.to_string())?;
    Ok(file.documents().count())
}

/// The tree is lossless: writing it back gives the input text.
pub fn emit(s: &str) -> Result<String, String> {
    Ok(parse(s)?.to_string())
}

pub fn emitter(s: &str) -> Result<Emitter, String> {
    let file = YamlFile::from_str(s).map_err(|e| e.to_string())?;
    Ok(Box::new(move || Ok(file.to_string().len())))
}
