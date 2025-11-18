// crates/forgo_lib_yaml/tests/api/mod.rs
//! Common utilities and test helpers for API tests

use forgo_lib_yaml::{Doc, Elem, Node, Scalar};

// Re-export for convenience in test modules
pub use forgo_lib_yaml;

// Test modules
mod elem_builders;
mod into_path;
mod layer1_simple;
mod node_constructors;
mod node_guards;
mod scalar_api;

/// Create a simple test document for use in tests
pub fn sample_doc() -> Doc {
    Doc::from_str(
        r#"
version: 1.2
name: test-app
dependencies:
  - yaml
  - json
config:
  debug: true
  timeout: 30
"#,
    )
    .unwrap()
}

/// Create a document with jobs (CI-like structure)
pub fn sample_jobs_doc() -> Doc {
    Doc::from_str(
        r#"
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - run: cargo build
      - run: cargo test
  deploy:
    runs-on: ubuntu-latest
    steps:
      - run: deploy.sh
"#,
    )
    .unwrap()
}

/// Helper to create a simple string element
pub fn str_elem(s: &str) -> Elem {
    Elem::new(Node::Scalar(Scalar::Str(s.to_string())))
}

/// Helper to create a number element
pub fn num_elem(n: &str) -> Elem {
    Elem::new(Node::Scalar(Scalar::Num {
        text: n.to_string(),
    }))
}

/// Helper to create a bool element
pub fn bool_elem(b: bool) -> Elem {
    Elem::new(Node::Scalar(Scalar::Bool(b)))
}

/// Helper to create a null element
pub fn null_elem() -> Elem {
    Elem::new(Node::Scalar(Scalar::Null))
}

/// Helper to create an empty map element
pub fn map_elem() -> Elem {
    Elem::new(Node::Map(vec![]))
}

/// Helper to create an empty seq element
pub fn seq_elem() -> Elem {
    Elem::new(Node::Seq(vec![]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sample_doc_valid() {
        let doc = sample_doc();
        assert!(matches!(doc.root().node(), Node::Map(_)));
    }

    #[test]
    fn test_sample_jobs_doc_valid() {
        let doc = sample_jobs_doc();
        assert!(matches!(doc.root().node(), Node::Map(_)));
    }

    #[test]
    fn test_helpers_create_valid_elements() {
        assert!(matches!(str_elem("test").node(), Node::Scalar(Scalar::Str(_))));
        assert!(matches!(num_elem("42").node(), Node::Scalar(Scalar::Num { .. })));
        assert!(matches!(bool_elem(true).node(), Node::Scalar(Scalar::Bool(true))));
        assert!(matches!(null_elem().node(), Node::Scalar(Scalar::Null)));
        assert!(matches!(map_elem().node(), Node::Map(_)));
        assert!(matches!(seq_elem().node(), Node::Seq(_)));
    }
}
