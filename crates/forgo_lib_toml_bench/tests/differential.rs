//! forgo_lib_toml against toml, toml_edit and toml-spanner on documents
//! made from every valid toml-test case by truncation and one-character
//! insertions: where all of them accept a document, the values must be
//! equal; where they disagree on accepting it, the difference must be the
//! one known (and documented) kind.

use forgo_lib_toml_bench::{mutations, only_split_pairs, readings, same, toml_test_valid};

#[test]
fn forgo_lib_toml_agrees_with_the_other_toml_1_1_parsers() {
    let Some(docs) = toml_test_valid() else {
        eprintln!("skipped: no toml-test data (crates/forgo_lib_toml/scripts/fetch-toml-test.sh)");
        return;
    };
    let inputs = mutations(&docs);
    let mut differences = Vec::new();
    for s in &inputs {
        let r = readings(s);
        let ours = &r[0].1;
        for (name, theirs) in &r[1..] {
            match (ours, theirs) {
                (Some(a), Some(b)) if !same(a, b) => {
                    differences.push(format!("{name} reads different values:\n{s}"))
                }
                // TOML forbids line breaks inside an inline table's pairs;
                // the others accept them.
                (None, Some(b)) if only_split_pairs(s, b) => {}
                (None, Some(_)) => differences.push(format!("only {name} accepts:\n{s}")),
                (Some(_), None) => differences.push(format!(
                    "only forgo_lib_toml accepts ({name} rejects):\n{s}"
                )),
                _ => {}
            }
        }
    }
    assert!(
        differences.is_empty(),
        "{} of {} documents differ:\n{}",
        differences.len(),
        inputs.len(),
        differences[..differences.len().min(10)].join("\n---\n")
    );
    println!("{} documents: no unexplained differences", inputs.len());
}
