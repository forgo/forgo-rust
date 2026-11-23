//! Flow Sequence Quote Preservation Tests
//!
//! **Related YAML 1.2.2 Spec Sections:**
//! - §7.4.1: Flow Sequences
//! - §7.3: Flow Scalar Styles (quoted strings in flow context)
//! - §6.7: Escape Sequences
//!
//! **Purpose:**
//! These tests validate that explicit quotes on flow sequence items are
//! preserved during round-trip, and that flow sequences stay flow. Strings
//! requiring quotes (e.g., `"01"`, `"true"`) keep their quoted form so they
//! aren't read back as numbers or booleans.

use forgo_lib_yaml::Doc;

#[test]
fn preserves_quotes_in_flow_seq_items() {
    let d = Doc::from_str("l: [\"01\", \"true\", 3]\n").unwrap();
    let out = d.to_string().unwrap();
    // The flow style and the quoted items survive.
    assert_eq!(out, "l: [\"01\", \"true\", 3]\n");
}
