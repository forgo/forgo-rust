//! The crate lists in the GitHub workflows, edited with forgo_lib_yaml.
//!
//! - `ci.yaml`: every `jobs.<job>.strategy.matrix.project` list.
//! - `release.yaml`: `on.workflow_dispatch.inputs.project.options`, the
//!   crates a release can be started for, and its `default`.
//!
//! Lists stay sorted and deduplicated and keep their style (`[a, b]` or
//! `- a` lines); everything else in the file is kept byte for byte, or
//! the edit fails ([`Doc::to_stream_string_preserving`]). An item that
//! isn't a plain string (an alias, a `${{ … }}` expression list) is an
//! error rather than something to rewrite.

use crate::error::{Error, Result, failed};
use forgo_lib_yaml::{Doc, Elem, Node, try_normalize_string_list};

/// The CI workflow, relative to the workspace root.
pub const CI_YAML: &str = ".github/workflows/ci.yaml";
/// The release workflow, relative to the workspace root.
pub const RELEASE_YAML: &str = ".github/workflows/release.yaml";

const CI_MATRIX: &[&str] = &["jobs", "*", "strategy", "matrix", "project"];
const RELEASE_OPTIONS: &[&str] = &["on", "workflow_dispatch", "inputs", "project", "options"];
const RELEASE_DEFAULT: &[&str] = &["on", "workflow_dispatch", "inputs", "project", "default"];

/// What to do with a crate name in a list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Change<'a> {
    /// Add it (no effect if it's there).
    Add(&'a str),
    /// Remove it (no effect if it's not there).
    Remove(&'a str),
}

/// `ci.yaml` with the change applied to every CI matrix project list.
/// `Ok(None)` if nothing changes; an error if the file has no such list.
pub fn edit_ci_matrix(text: &str, change: Change<'_>) -> Result<Option<String>> {
    let mut docs = parse(text, CI_YAML)?;
    let lists = edit_lists(&mut docs, CI_MATRIX, change, CI_YAML)?;
    if lists.matched == 0 {
        return failed(format!(
            "{CI_YAML}: no `jobs.<job>.strategy.matrix.project` list to edit"
        ));
    }
    write(&docs, lists.changed, CI_YAML)
}

/// `release.yaml` with the change applied to the release project choices.
///
/// When a removal takes away the choice `default` names, `default` moves to
/// the first remaining choice (it is never changed when adding). With
/// `required`, a missing list is an error; otherwise there's nothing to
/// change.
pub fn edit_release_options(
    text: &str,
    change: Change<'_>,
    required: bool,
) -> Result<Option<String>> {
    let mut docs = parse(text, RELEASE_YAML)?;
    let lists = edit_lists(&mut docs, RELEASE_OPTIONS, change, RELEASE_YAML)?;
    if lists.matched == 0 {
        if required {
            return failed(format!(
                "{RELEASE_YAML}: no `on.workflow_dispatch.inputs.project.options` list to edit"
            ));
        }
        return Ok(None);
    }
    let mut changed = lists.changed;
    if let Change::Remove(name) = change {
        let mut problem = None;
        for doc in &mut docs {
            doc.visit_values_mut(RELEASE_DEFAULT, |v| {
                if v.node().as_str() != Some(name) {
                    return false;
                }
                match lists.remaining.first() {
                    Some(first) => {
                        *v.node_mut() = Node::string(first.as_str());
                        changed = true;
                        true
                    }
                    None => {
                        problem = Some(format!(
                            "{RELEASE_YAML}: removing `{name}` leaves no project for `default`"
                        ));
                        false
                    }
                }
            });
        }
        if let Some(p) = problem {
            return failed(p);
        }
    }
    write(&docs, changed, RELEASE_YAML)
}

fn parse(text: &str, file: &str) -> Result<Vec<Doc>> {
    Doc::from_stream(text).map_err(|e| Error::Failed(format!("{file}: {e}")))
}

/// What editing the lists at a path did.
struct Lists {
    /// How many lists the path matched.
    matched: usize,
    changed: bool,
    /// The items of the last list matched, after the edit.
    remaining: Vec<String>,
}

fn edit_lists(docs: &mut [Doc], path: &[&str], change: Change<'_>, file: &str) -> Result<Lists> {
    let mut lists = Lists {
        matched: 0,
        changed: false,
        remaining: Vec::new(),
    };
    let mut problem = None;
    for doc in docs.iter_mut() {
        doc.visit_sequences_mut(path, |seq| {
            lists.matched += 1;
            // Only plain strings: check before touching the list.
            let mut probe = seq.clone();
            if let Err(e) = try_normalize_string_list(&mut probe) {
                problem.get_or_insert(format!("{file}: {e}"));
                return false;
            }
            let before = seq.clone();
            match change {
                Change::Add(name) => {
                    if !seq.iter().any(|e| e.node().as_str() == Some(name)) {
                        seq.push(Elem::string(name));
                    }
                }
                Change::Remove(name) => seq.retain(|e| e.node().as_str() != Some(name)),
            }
            if let Err(e) = try_normalize_string_list(seq) {
                problem.get_or_insert(format!("{file}: {e}"));
            }
            lists.remaining = seq
                .iter()
                .filter_map(|e| e.node().as_str().map(str::to_string))
                .collect();
            let did = *seq != before;
            lists.changed |= did;
            did
        });
    }
    match problem {
        Some(p) => failed(p),
        None => Ok(lists),
    }
}

fn write(docs: &[Doc], changed: bool, file: &str) -> Result<Option<String>> {
    if !changed {
        return Ok(None);
    }
    Doc::to_stream_string_preserving(docs)
        .map(Some)
        .map_err(|e| Error::Failed(format!("{file}: {e}; edit it by hand")))
}

/// Lines of `text` that still mention `name` as a whole word, as
/// `(line number, line)`.
pub fn mentions<'t>(text: &'t str, name: &str) -> Vec<(usize, &'t str)> {
    let is_word = |c: char| c.is_ascii_alphanumeric() || c == '_' || c == '-';
    text.lines()
        .enumerate()
        .filter(|(_, line)| {
            line.match_indices(name).any(|(i, _)| {
                let before = line[..i].chars().next_back();
                let after = line[i + name.len()..].chars().next();
                !before.is_some_and(is_word) && !after.is_some_and(is_word)
            })
        })
        .map(|(i, line)| (i + 1, line.trim()))
        .collect()
}
