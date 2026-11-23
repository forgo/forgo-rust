//! Anchor on Alias Node Tests
//!
//! **Related YAML 1.2.2 Spec Sections:**
//! - §7.1: Alias Nodes - "Alias nodes are not primary nodes and cannot have anchors"
//! - §6.9.1: Node Anchors - "Each node may have at most one anchor"
//!
//! **Purpose:**
//! Per YAML 1.2.2 §7.1, alias nodes are not primary nodes and cannot have anchors.
//! The official YAML test suite (test SU74) confirms that having both an anchor
//! and an alias on the same node should fail. This test validates that we correctly
//! reject this invalid syntax.

use forgo_lib_yaml::Doc;

#[test]
fn anchor_on_alias_should_fail() {
    let input = "- &x *x\n";

    // Per YAML 1.2.2 §7.1, alias nodes cannot have anchors
    // This should be a parse error
    let result = Doc::from_str(input);
    assert!(
        result.is_err(),
        "Expected parse error for anchor on alias node, but got: {:?}",
        result
    );

    // Verify the error message mentions the issue
    let err = result.unwrap_err();
    let err_msg = err.to_string();
    assert!(
        err_msg.contains("alias") || err_msg.contains("anchor"),
        "Error message should mention alias/anchor issue: {}",
        err_msg
    );
}
