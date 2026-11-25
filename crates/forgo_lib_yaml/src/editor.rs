//! Editing helpers.

use std::fs;
use std::path::Path;

use crate::ast::{Doc, Elem, Error, Node, Scalar, Seg};

/// An item's text: a string's value; other scalars as written (`true`,
/// `42`, `null`), `*name` for an alias, `[map]` / `[seq]` for collections.
fn item_text(e: &Elem) -> String {
    match &e.node {
        Node::Scalar(Scalar::Str(s)) => s.clone(),
        Node::Scalar(Scalar::Bool(b)) => b.to_string(),
        Node::Scalar(Scalar::Num { text }) => text.clone(),
        Node::Scalar(Scalar::Null) => "null".into(),
        Node::Map(_) => "[map]".into(),
        Node::Seq(_) => "[seq]".into(),
        Node::Alias(a) => format!("*{a}"),
    }
}

/// Make a list a sorted set of strings: sort by text (byte order), drop
/// duplicates, and turn other items into strings of their text (see
/// below). Items keep their comments and quoting, so only moved and
/// removed lines change in the file. Returns whether the list changed.
///
/// Non-string items become their text: `true`, `42`, `null`, `*name` for
/// an alias, and `[map]` / `[seq]` for collections.
///
/// ```
/// use forgo_lib_yaml::{normalize_string_list, Doc};
///
/// let mut doc = Doc::from_str("tags: [web, api, 'web', 2]  # deploy targets\n")?;
/// doc.visit_sequences_mut(&["tags"], normalize_string_list);
/// assert_eq!(doc.to_string()?, "tags: [\"2\", api, web]  # deploy targets\n");
/// # Ok::<(), forgo_lib_yaml::Error>(())
/// ```
pub fn normalize_string_list(seq: &mut Vec<Elem>) -> bool {
    let before: Vec<String> = seq.iter().map(item_text).collect();
    let mut sorted = before.clone();
    sorted.sort();
    sorted.dedup();
    let all_strings = seq
        .iter()
        .all(|e| matches!(e.node, Node::Scalar(Scalar::Str(_))));
    if sorted == before && all_strings {
        return false;
    }
    let mut items: Vec<(String, Elem)> = seq
        .drain(..)
        .map(|mut e| {
            let text = item_text(&e);
            if !matches!(e.node, Node::Scalar(Scalar::Str(_))) {
                // A string now: let the emitter pick a style that reads back
                // as one (`"42"`).
                e.node = Node::Scalar(Scalar::Str(text.clone()));
                e.meta.tag = None;
                e.meta.style = None;
                e.meta.spelling = None;
            }
            (text, e)
        })
        .collect();
    items.sort_by(|a, b| a.0.cmp(&b.0));
    items.dedup_by(|a, b| a.0 == b.0);
    *seq = items.into_iter().map(|(_, e)| e).collect();
    true
}

/// [`normalize_string_list`] for lists that must hold only strings: an
/// item that isn't a string (an alias, a number, a collection) is an error
/// and the list is left as it was, instead of being converted. Returns
/// whether the list changed.
///
/// ```
/// use forgo_lib_yaml::{try_normalize_string_list, Doc};
///
/// let mut doc = Doc::from_str("crates: [web, api]\nmixed: [b, 2]\n")?;
/// let map = doc.root_mut().node_mut().as_map_mut().unwrap();
/// let crates = map[0].1.node_mut().as_seq_mut().unwrap();
/// assert!(try_normalize_string_list(crates)?);
/// let mixed = map[1].1.node_mut().as_seq_mut().unwrap();
/// assert!(try_normalize_string_list(mixed).is_err());
/// assert_eq!(doc.to_string()?, "crates: [api, web]\nmixed: [b, 2]\n");
/// # Ok::<(), forgo_lib_yaml::Error>(())
/// ```
pub fn try_normalize_string_list(seq: &mut Vec<Elem>) -> Result<bool, Error> {
    if let Some((i, e)) = seq
        .iter()
        .enumerate()
        .find(|(_, e)| !matches!(e.node, Node::Scalar(Scalar::Str(_))))
    {
        return Err(Error::Edit(format!(
            "item {} of the list is not a string (`{}`)",
            i + 1,
            item_text(e)
        )));
    }
    Ok(normalize_string_list(seq))
}

/// Edit a YAML file in place: in every document of the file, apply
/// `updater` to each sequence at `seg_path`, then sort and dedup it with
/// [`normalize_string_list`]. The file is written only when something
/// changed, and only the edited sequences change in it (see
/// [`Doc::to_string`]). Returns whether the file was written.
pub fn edit_file_in_place<P, F>(path: P, seg_path: &[Seg], mut updater: F) -> Result<bool, Error>
where
    P: AsRef<Path>,
    F: FnMut(&mut Vec<Elem>) -> bool,
{
    let p = path.as_ref();
    let original = fs::read_to_string(p).map_err(Error::Io)?;
    let mut docs = Doc::from_stream(&original)?;

    let mut changed_any = false;
    let mut visited = false;
    for doc in &mut docs {
        visited |= doc.visit_sequences_mut(seg_path, |seq| {
            let did = updater(seq);
            let norm = normalize_string_list(seq);
            changed_any |= did || norm;
            did || norm
        });
    }

    if visited && changed_any {
        let new_text = Doc::to_stream_string(&docs)?;
        if new_text != original {
            fs::write(p, new_text).map_err(Error::Io)?;
            return Ok(true);
        }
    }
    Ok(false)
}
