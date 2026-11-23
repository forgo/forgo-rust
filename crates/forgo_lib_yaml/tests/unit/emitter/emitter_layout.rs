//! Layout and Formatting Emission Tests
//!
//! **Related YAML 1.2.2 Spec Sections:**
//! - §6.2: Indentation Spaces
//! - §8.2: Block Collection Styles
//! - §7.4: Flow Collection indentation
//! - §6.5: Line Folding
//!
//! **Purpose:**
//! These are unit tests for layout and formatting emission. They test implementation-specific
//! behavior and edge cases not explicitly covered by the spec. Unlike spec
//! tests which validate compliance, these tests validate internal correctness
//! and error handling.

use forgo_lib_yaml::Doc;

fn doc(s: &str) -> Doc {
    Doc::from_str(s).expect("parse")
}

#[test]
fn seq_as_map_value_is_indented_two_spaces_from_key() {
    let d = doc("k:\n  - 1\n  - 2\n");
    let out = d.to_string().unwrap();
    // Key line ends with colon+newline; list items start at key-indent + 2
    assert!(out.contains("k:\n  - 1\n  - 2\n"), "{out}");
}

#[test]
fn nested_map_as_map_value_is_indented_two_spaces_from_key() {
    let d = doc("root:\n  child: 1\n");
    let out = d.to_string().unwrap();
    assert!(out.contains("root:\n  child: 1\n"), "{out}");
}
