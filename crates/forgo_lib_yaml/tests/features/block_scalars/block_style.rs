//! Block Scalar Style Preservation Tests
//!
//! **Related YAML 1.2.2 Spec Sections:**
//! - §8.1.1.2: Block Chomping Indicator (`-`, `+`, clip)
//! - §8.1.2: Literal Style (`|`)
//! - §8.1.3: Folded Style (`>`)
//!
//! **Purpose:**
//! These tests validate that block scalar style indicators (literal vs folded)
//! and chomping indicators are preserved during round-trip. While parsing
//! behavior is tested in `block_scalars.rs` and `block_scalars_strict.rs`,
//! these tests ensure that the emitter maintains the original style choices
//! rather than normalizing them.

use forgo_lib_yaml::Doc;

#[test]
fn preserves_literal_and_chomp() {
    // trailing + should be preserved on re-emit
    let s = "k: |+\n  a\n  b\n";
    let d = Doc::from_str(s).unwrap();
    let out = d.to_string().unwrap();
    assert!(out.starts_with("k: |+"));
    assert!(out.contains("\n  a\n"));
    assert!(out.contains("\n  b\n"));
}

#[test]
fn preserves_folded_header() {
    let s = "k: >-\n  a\n  b\n";
    let d = Doc::from_str(s).unwrap();
    let out = d.to_string().unwrap();
    assert!(out.starts_with("k: >-"), "got:\n{out}");
}
