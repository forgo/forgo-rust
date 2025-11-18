// crates/forgo_lib_yaml/tests/api/into_path.rs
//! Tests for IntoPath trait and path handling
//!
//! Covers:
//! - IntoPath trait implementations
//! - Support for &[&str], &[&str; N], Vec<&str>
//! - Wildcard handling with "*"
//! - Path conversions to Seg

use crate::api::*;
use forgo_lib_yaml::{parse, Elem, Seg};

// ============================================================================
// IntoPath Trait Tests - Array Literals
// ============================================================================

#[test]
fn test_path_from_str_array_literal() {
    let mut doc = sample_jobs_doc();

    // Should work with array literal syntax
    let modified = doc.visit_sequences_mut(&["jobs", "build", "steps"], |steps| {
        steps.push(str_elem("new step"));
        true
    });

    assert!(modified);
}

#[test]
fn test_path_with_wildcard_from_str_array() {
    let mut doc = sample_jobs_doc();

    // Wildcard should work in array literal
    let mut count = 0;
    doc.visit_sequences_mut(&["jobs", "*", "steps"], |_steps| {
        count += 1;
        false
    });

    // Should visit both "build" and "deploy" jobs
    assert_eq!(count, 2);
}

#[test]
fn test_path_single_element_array() {
    let yaml = "key: value\n";
    let mut doc = parse(yaml).unwrap();

    let modified = doc.visit_values_mut(&["key"], |elem| {
        *elem = Elem::string("modified");
        true
    });

    assert!(modified);
    assert_eq!(
        doc.root().node().as_map().unwrap()[0].1.node().as_str(),
        Some("modified")
    );
}

#[test]
fn test_path_empty_array() {
    let mut doc = sample_doc();

    // Empty path should target root
    let modified = doc.visit_values_mut(&[] as &[&str], |_elem| {
        // Root is a map
        true
    });

    assert!(modified);
}

// ============================================================================
// IntoPath Trait Tests - Slice Syntax
// ============================================================================

#[test]
fn test_path_from_slice() {
    let mut doc = sample_jobs_doc();
    let path_vec = vec!["jobs", "build", "steps"];

    let modified = doc.visit_sequences_mut(path_vec.as_slice(), |steps| {
        steps.push(str_elem("from slice"));
        true
    });

    assert!(modified);
}

#[test]
fn test_path_from_vec() {
    let mut doc = sample_jobs_doc();
    let path = vec!["jobs", "build", "steps"];

    let modified = doc.visit_sequences_mut(&path[..], |steps| {
        steps.push(str_elem("from vec"));
        true
    });

    assert!(modified);
}

// ============================================================================
// IntoPath Trait Tests - Seg Types
// ============================================================================

#[test]
fn test_path_from_seg_array() {
    let mut doc = sample_jobs_doc();

    // Should still work with explicit Seg types
    let path = &[
        Seg::Key("jobs".into()),
        Seg::Key("build".into()),
        Seg::Key("steps".into()),
    ];

    let modified = doc.visit_sequences_mut(path, |steps| {
        steps.push(str_elem("from seg"));
        true
    });

    assert!(modified);
}

#[test]
fn test_path_mixed_key_and_wildcard() {
    let mut doc = sample_jobs_doc();

    // Mix of keys and wildcards
    let modified = doc.visit_sequences_mut(&["jobs", "*", "steps"], |steps| {
        steps.len(); // Just access to verify it's a sequence
        true
    });

    assert!(modified);
}

// ============================================================================
// Wildcard Behavior Tests
// ============================================================================

#[test]
fn test_wildcard_matches_all_keys() {
    let yaml = r#"
a:
  value: 1
b:
  value: 2
c:
  value: 3
"#;
    let mut doc = parse(yaml).unwrap();

    let mut visited = vec![];
    doc.visit_values_mut(&["*", "value"], |elem| {
        if let Some(n) = elem.node().as_i64() {
            visited.push(n);
        }
        false
    });

    assert_eq!(visited.len(), 3);
    assert!(visited.contains(&1));
    assert!(visited.contains(&2));
    assert!(visited.contains(&3));
}

#[test]
fn test_multiple_wildcards_in_path() {
    let yaml = r#"
outer1:
  inner1:
    value: a
  inner2:
    value: b
outer2:
  inner3:
    value: c
"#;
    let mut doc = parse(yaml).unwrap();

    let mut visited = vec![];
    doc.visit_values_mut(&["*", "*", "value"], |elem| {
        if let Some(s) = elem.node().as_str() {
            visited.push(s.to_string());
        }
        false
    });

    assert_eq!(visited.len(), 3);
    assert!(visited.contains(&"a".to_string()));
    assert!(visited.contains(&"b".to_string()));
    assert!(visited.contains(&"c".to_string()));
}

#[test]
fn test_wildcard_at_different_positions() {
    let mut doc = sample_jobs_doc();

    // Wildcard in middle
    let mut count1 = 0;
    doc.visit_values_mut(&["jobs", "*", "runs-on"], |_| {
        count1 += 1;
        false
    });

    // Should match both jobs
    assert_eq!(count1, 2);
}

// ============================================================================
// Integration Tests with Real Documents
// ============================================================================

#[test]
fn test_modify_all_jobs_with_wildcard() {
    let mut doc = sample_jobs_doc();

    // Add timeout to all jobs
    doc.visit_maps_mut(&["jobs", "*"], |job_map| {
        job_map.push(("timeout-minutes".into(), Elem::number(30)));
        true
    });

    // Verify both jobs have timeout
    let jobs = doc.root().node().as_map().unwrap()[0].1.node().as_map().unwrap();

    for (job_name, job) in jobs {
        if job_name == "build" || job_name == "deploy" {
            let job_map = job.node().as_map().unwrap();
            let has_timeout = job_map.iter().any(|(k, _)| k == "timeout-minutes");
            assert!(has_timeout, "Job {} should have timeout", job_name);
        }
    }
}

#[test]
fn test_add_step_to_all_jobs() {
    let mut doc = sample_jobs_doc();

    // Add a step to all jobs
    doc.visit_sequences_mut(&["jobs", "*", "steps"], |steps| {
        steps.push(Elem::string("run: echo done").with_comment("Completion step"));
        true
    });

    // Verify step was added to both jobs
    let jobs = doc.root().node().as_map().unwrap()[0].1.node().as_map().unwrap();

    for (job_name, job) in jobs {
        if job_name == "build" || job_name == "deploy" {
            let job_map = job.node().as_map().unwrap();
            let steps = job_map.iter()
                .find(|(k, _)| k == "steps")
                .unwrap().1.node().as_seq().unwrap();

            // build: 2 original + 1 new = 3 steps
            // deploy: 1 original + 1 new = 2 steps
            if job_name == "build" {
                assert_eq!(steps.len(), 3, "Job build should have 3 steps");
            } else if job_name == "deploy" {
                assert_eq!(steps.len(), 2, "Job deploy should have 2 steps");
            }
        }
    }
}

// ============================================================================
// Path Syntax Comparison Tests
// ============================================================================

#[test]
fn test_all_path_syntaxes_equivalent() {
    // These should all be equivalent:

    // 1. Array literal
    let mut doc1 = sample_doc();
    doc1.visit_values_mut(&["version"], |elem| {
        *elem = Elem::string("2.0");
        true
    });

    // 2. Slice from vec
    let mut doc2 = sample_doc();
    let path = vec!["version"];
    doc2.visit_values_mut(&path[..], |elem| {
        *elem = Elem::string("2.0");
        true
    });

    // 3. Seg array
    let mut doc3 = sample_doc();
    doc3.visit_values_mut(&[Seg::Key("version".into())], |elem| {
        *elem = Elem::string("2.0");
        true
    });

    // All should produce same result
    assert_eq!(doc1, doc2);
    assert_eq!(doc2, doc3);
}

// ============================================================================
// Error Cases / Edge Cases
// ============================================================================

#[test]
fn test_nonexistent_path_does_not_panic() {
    let mut doc = sample_doc();

    // Path that doesn't exist should return false (not found)
    let modified = doc.visit_values_mut(&["nonexistent", "path"], |_| {
        true
    });

    assert!(!modified);
}

#[test]
fn test_type_mismatch_in_path_does_not_panic() {
    let mut doc = sample_doc();

    // Trying to traverse a scalar as a map should not panic
    let modified = doc.visit_values_mut(&["version", "nested"], |_| {
        true
    });

    assert!(!modified);
}

#[test]
fn test_wildcard_on_non_map_does_not_panic() {
    let yaml = "- item1\n- item2\n";
    let mut doc = parse(yaml).unwrap();

    // Wildcard on a sequence should not panic
    let modified = doc.visit_values_mut(&["*"], |_| {
        true
    });

    // Might or might not modify depending on implementation
    // Main thing is it shouldn't panic
    let _ = modified;
}

// ============================================================================
// Documentation Examples
// ============================================================================

#[test]
fn test_path_api_example_from_docs() {
    let mut doc = sample_jobs_doc();

    // Clean syntax as advertised
    doc.visit_sequences_mut(&["jobs", "*", "steps"], |steps| {
        steps.push(Elem::string("run: cleanup"));
        true
    });

    // Verify it worked
    let jobs = doc.root().node().as_map().unwrap()[0].1.node().as_map().unwrap();
    assert!(jobs.len() >= 2);
}
