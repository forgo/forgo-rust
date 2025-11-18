// crates/forgo_lib_yaml/tests/api/scalar_api.rs
//! Tests for Scalar type guards and conversions
//!
//! Covers:
//! - Scalar::is_str(), is_bool(), is_num(), is_null()
//! - Scalar::as_str(), as_bool(), as_i64(), as_f64(), as_u64()
//! - Scalar::num_text(), to_string()

use forgo_lib_yaml::Scalar;

// ============================================================================
// Type Guards Tests
// ============================================================================

#[test]
fn test_is_str_returns_true_for_string() {
    let scalar = Scalar::Str("hello".into());
    assert!(scalar.is_str());
    assert!(!scalar.is_bool());
    assert!(!scalar.is_num());
    assert!(!scalar.is_null());
}

#[test]
fn test_is_bool_returns_true_for_boolean() {
    let scalar = Scalar::Bool(true);
    assert!(scalar.is_bool());
    assert!(!scalar.is_str());
    assert!(!scalar.is_num());
    assert!(!scalar.is_null());
}

#[test]
fn test_is_num_returns_true_for_number() {
    let scalar = Scalar::Num { text: "42".into() };
    assert!(scalar.is_num());
    assert!(!scalar.is_str());
    assert!(!scalar.is_bool());
    assert!(!scalar.is_null());
}

#[test]
fn test_is_null_returns_true_for_null() {
    let scalar = Scalar::Null;
    assert!(scalar.is_null());
    assert!(!scalar.is_str());
    assert!(!scalar.is_bool());
    assert!(!scalar.is_num());
}

// ============================================================================
// Type Conversion Tests
// ============================================================================

#[test]
fn test_as_str_returns_string_value() {
    let scalar = Scalar::Str("hello".into());
    assert_eq!(scalar.as_str(), Some("hello"));
}

#[test]
fn test_as_str_returns_none_for_non_string() {
    assert!(Scalar::Bool(true).as_str().is_none());
    assert!(Scalar::Num { text: "42".into() }.as_str().is_none());
    assert!(Scalar::Null.as_str().is_none());
}

#[test]
fn test_as_bool_returns_boolean_value() {
    assert_eq!(Scalar::Bool(true).as_bool(), Some(true));
    assert_eq!(Scalar::Bool(false).as_bool(), Some(false));
}

#[test]
fn test_as_bool_returns_none_for_non_boolean() {
    assert!(Scalar::Str("true".into()).as_bool().is_none());
    assert!(Scalar::Num { text: "1".into() }.as_bool().is_none());
    assert!(Scalar::Null.as_bool().is_none());
}

// ============================================================================
// Number Parsing Tests
// ============================================================================

#[test]
fn test_as_i64_parses_positive_integer() {
    let scalar = Scalar::Num { text: "42".into() };
    assert_eq!(scalar.as_i64(), Some(42));
}

#[test]
fn test_as_i64_parses_negative_integer() {
    let scalar = Scalar::Num { text: "-123".into() };
    assert_eq!(scalar.as_i64(), Some(-123));
}

#[test]
fn test_as_i64_parses_zero() {
    let scalar = Scalar::Num { text: "0".into() };
    assert_eq!(scalar.as_i64(), Some(0));
}

#[test]
fn test_as_i64_returns_none_for_invalid() {
    let scalar = Scalar::Num { text: "not_a_number".into() };
    assert!(scalar.as_i64().is_none());
}

#[test]
fn test_as_i64_returns_none_for_float() {
    let scalar = Scalar::Num { text: "3.14".into() };
    assert!(scalar.as_i64().is_none());
}

#[test]
fn test_as_i64_returns_none_for_non_number() {
    assert!(Scalar::Str("42".into()).as_i64().is_none());
}

#[test]
fn test_as_f64_parses_float() {
    let scalar = Scalar::Num { text: "3.14".into() };
    assert_eq!(scalar.as_f64(), Some(3.14));
}

#[test]
fn test_as_f64_parses_integer_as_float() {
    let scalar = Scalar::Num { text: "42".into() };
    assert_eq!(scalar.as_f64(), Some(42.0));
}

#[test]
fn test_as_f64_parses_negative_float() {
    let scalar = Scalar::Num { text: "-2.5".into() };
    assert_eq!(scalar.as_f64(), Some(-2.5));
}

#[test]
fn test_as_f64_parses_scientific_notation() {
    let scalar = Scalar::Num { text: "1.5e2".into() };
    assert_eq!(scalar.as_f64(), Some(150.0));
}

#[test]
fn test_as_f64_returns_none_for_invalid() {
    let scalar = Scalar::Num { text: "not_a_number".into() };
    assert!(scalar.as_f64().is_none());
}

#[test]
fn test_as_u64_parses_unsigned_integer() {
    let scalar = Scalar::Num { text: "42".into() };
    assert_eq!(scalar.as_u64(), Some(42));
}

#[test]
fn test_as_u64_returns_none_for_negative() {
    let scalar = Scalar::Num { text: "-42".into() };
    assert!(scalar.as_u64().is_none());
}

#[test]
fn test_as_u64_parses_large_number() {
    let scalar = Scalar::Num { text: "18446744073709551615".into() }; // u64::MAX
    assert_eq!(scalar.as_u64(), Some(u64::MAX));
}

// ============================================================================
// Raw Number Text Tests
// ============================================================================

#[test]
fn test_num_text_returns_original_text() {
    let scalar = Scalar::Num { text: "042".into() };
    assert_eq!(scalar.num_text(), Some("042"));
}

#[test]
fn test_num_text_preserves_formatting() {
    let scalar = Scalar::Num { text: "1_000_000".into() };
    assert_eq!(scalar.num_text(), Some("1_000_000"));
}

#[test]
fn test_num_text_returns_none_for_non_number() {
    assert!(Scalar::Str("42".into()).num_text().is_none());
    assert!(Scalar::Bool(true).num_text().is_none());
    assert!(Scalar::Null.num_text().is_none());
}

// ============================================================================
// to_string Tests
// ============================================================================

#[test]
fn test_to_string_for_string_scalar() {
    let scalar = Scalar::Str("hello".into());
    assert_eq!(scalar.to_string(), "hello");
}

#[test]
fn test_to_string_for_bool_true() {
    let scalar = Scalar::Bool(true);
    assert_eq!(scalar.to_string(), "true");
}

#[test]
fn test_to_string_for_bool_false() {
    let scalar = Scalar::Bool(false);
    assert_eq!(scalar.to_string(), "false");
}

#[test]
fn test_to_string_for_number() {
    let scalar = Scalar::Num { text: "42".into() };
    assert_eq!(scalar.to_string(), "42");
}

#[test]
fn test_to_string_for_null() {
    let scalar = Scalar::Null;
    assert_eq!(scalar.to_string(), "null");
}

#[test]
fn test_to_string_preserves_number_formatting() {
    let scalar = Scalar::Num { text: "3.14159".into() };
    assert_eq!(scalar.to_string(), "3.14159");
}
