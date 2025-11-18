// crates/forgo_lib_yaml/src/editor.rs
use std::fs;
use std::path::Path;

use crate::ast::{Doc, Elem, Error, Node, Scalar, Seg};

/// Normalize a sequence to strings, ASCII sort + dedup. Returns true if mutated.
/// For non-string scalars, stringify with a stable representation.
pub fn normalize_string_list(seq: &mut Vec<Elem>) -> bool {
    let mut items: Vec<String> = seq
        .iter()
        .map(|e| match &e.node {
            Node::Scalar(Scalar::Str(s)) => s.clone(),
            Node::Scalar(Scalar::Bool(b)) => {
                if *b {
                    "true".into()
                } else {
                    "false".into()
                }
            }
            Node::Scalar(Scalar::Num { text }) => text.clone(),
            Node::Scalar(Scalar::Null) => "null".into(),
            Node::Map(_) => "[map]".into(),
            Node::Seq(_) => "[seq]".into(),
            Node::Alias(a) => format!("*{a}"),
        })
        .collect();

    let before = items.clone();
    items.sort();
    items.dedup();
    let changed = items != before
        || seq
            .iter()
            .any(|e| !matches!(e.node, Node::Scalar(Scalar::Str(_))));
    if changed {
        *seq = items
            .into_iter()
            .map(|s| Elem::new(Node::Scalar(Scalar::Str(s))))
            .collect();
    }
    changed
}

/// Edit file in place by visiting sequences at `seg_path`, applying `updater`,
/// then normalizing and writing if changed.
pub fn edit_file_in_place<P, F>(path: P, seg_path: &[Seg], mut updater: F) -> Result<bool, Error>
where
    P: AsRef<Path>,
    F: FnMut(&mut Vec<Elem>) -> bool,
{
    let p = path.as_ref();
    let original = fs::read_to_string(p).map_err(Error::Io)?;
    let mut doc = Doc::from_str(&original)?;

    let mut changed_any = false;
    let visited = doc.visit_sequences_mut(seg_path, |seq| {
        let did = updater(seq);
        let norm = normalize_string_list(seq);
        changed_any |= did || norm;
        did || norm
    });

    if visited && changed_any {
        let new_text = doc.to_string()?;
        if new_text != original {
            fs::write(p, new_text).map_err(Error::Io)?;
            return Ok(true);
        }
    }
    Ok(false)
}
