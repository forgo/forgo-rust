//! Flow Sequence Quote Preservation Tests
//!
//! **Related YAML 1.2.2 Spec Sections:**
//! - §7.4.1: Flow Sequences
//! - §7.3: Flow Scalar Styles (quoted strings in flow context)
//! - §6.7: Escape Sequences
//!
//! **Purpose:**
//! These tests validate that explicit quotes on flow sequence items are
//! preserved during round-trip, even when the emitter normalizes flow
//! sequences to block style (dash notation). This ensures that strings
//! requiring quotes (e.g., `"01"`, `"true"`) maintain their quoted form
//! to preserve their string type rather than being interpreted as numbers
//! or booleans.

use forgo_lib_yaml::Doc;

#[test]
fn preserves_quotes_in_flow_seq_items() {
    let d = Doc::from_str("l: [\"01\", \"true\", 3]\n").unwrap();
    let out = d.to_string().unwrap();
    // We normalize to dash style, but items that were explicitly quoted should remain quoted
    assert!(out.contains(r#"- "01""#));
    assert!(out.contains(r#"- "true""#));
    assert!(out.contains("\n  - 3\n"));
}
