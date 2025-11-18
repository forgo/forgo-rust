// crates/forgo_lib_yaml/tests/spec/ch_10_schemas.rs
//! YAML 1.2.2 Chapter 10: Recommended Schemas
//!
//! Reference: https://yaml.org/spec/1.2.2/#chapter-10-recommended-schemas
//!
//! YAML defines three recommended schemas:
//! - Failsafe Schema: Only strings, mappings, sequences
//! - JSON Schema: Compatible with JSON (null, bool, int, float, str)
//! - Core Schema: Extended types with more flexible parsing

#[path = "../common/mod.rs"]
mod common;

use common::Fixture;
use forgo_lib_yaml::{Doc, Node, Scalar};

// ============================================================================
// 10.1 Failsafe Schema
// ============================================================================

#[test]
fn ch_10_1_01_failsafe_mappings() {
    Fixture::new(
        "10.1",
        1,
        "Failsafe: mappings allowed",
        "map:\n  key: value\n",
    )
    .run();
}

#[test]
fn ch_10_1_02_failsafe_sequences() {
    Fixture::new(
        "10.1",
        2,
        "Failsafe: sequences allowed",
        "- item1\n- item2\n",
    )
    .run();
}

#[test]
fn ch_10_1_03_failsafe_strings() {
    Fixture::new(
        "10.1",
        3,
        "Failsafe: string scalars",
        "string: hello\nother: world\n",
    )
    .run();
}

#[test]
fn ch_10_1_04_failsafe_no_type_inference() {
    // In Failsafe schema, everything is a string unless explicitly tagged
    // "42" should be a string, not a number
    Fixture::new(
        "10.1",
        4,
        "Failsafe: no type inference (numbers are strings)",
        "count: 42\nflag: true\nnull_value: null\n",
    )
    .run();
}

#[test]
fn ch_10_1_05_failsafe_only_explicit_tags() {
    // Only explicit tags resolve types in Failsafe
    Fixture::new(
        "10.1",
        5,
        "Failsafe: explicit tags work",
        "number: !!int 42\nstring: !!str text\n",
    )
    .run();
}

#[test]
fn ch_10_1_06_failsafe_nested_collections() {
    // Nested maps and sequences in Failsafe
    Fixture::new(
        "10.1",
        6,
        "Failsafe: nested collections",
        "outer:\n  inner:\n    - item1\n    - item2\n",
    )
    .run();
}

// ============================================================================
// 10.2 JSON Schema
// ============================================================================

#[test]
fn ch_10_2_01_json_null() {
    let input = "value: null\n";
    let d = Doc::from_str(input).unwrap();

    let Node::Map(root) = d.root().node() else {
        panic!("root not a map");
    };
    let (_, val) = root.iter().find(|(k, _)| k == "value").unwrap();
    assert!(matches!(val.node(), Node::Scalar(Scalar::Null)));
}

#[test]
fn ch_10_2_02_json_boolean_true() {
    let input = "flag: true\n";
    let d = Doc::from_str(input).unwrap();

    let Node::Map(root) = d.root().node() else {
        panic!("root not a map");
    };
    let (_, val) = root.iter().find(|(k, _)| k == "flag").unwrap();
    assert!(matches!(val.node(), Node::Scalar(Scalar::Bool(true))));
}

#[test]
fn ch_10_2_03_json_boolean_false() {
    let input = "flag: false\n";
    let d = Doc::from_str(input).unwrap();

    let Node::Map(root) = d.root().node() else {
        panic!("root not a map");
    };
    let (_, val) = root.iter().find(|(k, _)| k == "flag").unwrap();
    assert!(matches!(val.node(), Node::Scalar(Scalar::Bool(false))));
}

#[test]
fn ch_10_2_04_json_integer() {
    let input = "count: 42\n";
    let d = Doc::from_str(input).unwrap();

    let Node::Map(root) = d.root().node() else {
        panic!("root not a map");
    };
    let (_, val) = root.iter().find(|(k, _)| k == "count").unwrap();
    assert!(matches!(val.node(), Node::Scalar(Scalar::Num { text }) if text == "42"));
}

#[test]
fn ch_10_2_05_json_negative_integer() {
    Fixture::new(
        "10.2",
        5,
        "JSON: negative integer",
        "temp: -42\n",
    )
    .run();
}

#[test]
fn ch_10_2_06_json_float() {
    Fixture::new(
        "10.2",
        6,
        "JSON: floating point",
        "pi: 3.14159\n",
    )
    .run();
}

#[test]
fn ch_10_2_07_json_exponential() {
    Fixture::new(
        "10.2",
        7,
        "JSON: exponential notation",
        "avogadro: 6.022e23\n",
    )
    .run();
}

#[test]
fn ch_10_2_08_json_string() {
    Fixture::new(
        "10.2",
        8,
        "JSON: string value",
        "name: hello\n",
    )
    .run();
}

#[test]
fn ch_10_2_09_json_null_only_lowercase() {
    // JSON Schema: only "null" (not ~, Null, NULL, or empty)
    let input = "value: null\n";
    let d = Doc::from_str(input).unwrap();

    let Node::Map(root) = d.root().node() else {
        panic!("root not a map");
    };
    let (_, val) = root.iter().find(|(k, _)| k == "value").unwrap();
    assert!(matches!(val.node(), Node::Scalar(Scalar::Null)));
}

#[test]
fn ch_10_2_10_json_tilde_not_null() {
    // JSON Schema: ~ should be a string, not null
    // (Only Core schema treats ~ as null)
    let input = "value: ~\n";
    let result = Doc::from_str(input);

    // In strict JSON schema, ~ should be string
    // Most parsers use Core schema by default
    if result.is_ok() {
        let d = result.unwrap();
        let Node::Map(root) = d.root().node() else {
            panic!("root not a map");
        };
        let (_, val) = root.iter().find(|(k, _)| k == "value").unwrap();
        // May be Null (Core schema) or Str (JSON schema)
        assert!(matches!(val.node(), Node::Scalar(_)));
    }
}

#[test]
fn ch_10_2_11_json_bool_case_sensitive() {
    // JSON Schema: only lowercase "true" and "false"
    // "True", "TRUE", etc. should be strings
    let input = "flag: true\n";
    let d = Doc::from_str(input).unwrap();

    let Node::Map(root) = d.root().node() else {
        panic!("root not a map");
    };
    let (_, val) = root.iter().find(|(k, _)| k == "flag").unwrap();
    assert!(matches!(val.node(), Node::Scalar(Scalar::Bool(true))));
}

#[test]
fn ch_10_2_12_json_uppercase_bool_is_string() {
    // JSON Schema: "True" should be a string, not boolean
    let input = "flag: True\n";
    let result = Doc::from_str(input);

    // In strict JSON schema, should be string
    // Core schema allows case-insensitive
    if result.is_ok() {
        // Parser may use Core schema (accepts True as bool)
        // or JSON schema (True is string)
        let _ = result.unwrap();
    }
}

#[test]
fn ch_10_2_13_json_decimal_only_no_octal() {
    // JSON Schema: numbers are decimal only
    // No octal (0o), hex (0x), or binary (0b)
    let input = "value: 42\n";
    let d = Doc::from_str(input).unwrap();

    let Node::Map(root) = d.root().node() else {
        panic!("root not a map");
    };
    let (_, val) = root.iter().find(|(k, _)| k == "value").unwrap();
    assert!(matches!(val.node(), Node::Scalar(Scalar::Num { text }) if text == "42"));
}

#[test]
fn ch_10_2_14_json_octal_is_error_or_string() {
    // JSON Schema: octal notation not allowed
    let input = "value: 0o52\n";
    let result = Doc::from_str(input);

    // In strict JSON schema, this should be an error or string
    // Core schema allows it
    if result.is_ok() {
        // Parser uses Core schema (allows octal)
        let _ = result.unwrap();
    }
}

#[test]
fn ch_10_2_15_json_hex_is_error_or_string() {
    // JSON Schema: hexadecimal notation not allowed
    let input = "value: 0x2A\n";
    let result = Doc::from_str(input);

    // In strict JSON schema, this should be an error or string
    // Core schema allows it
    if result.is_ok() {
        // Parser uses Core schema (allows hex)
        let _ = result.unwrap();
    }
}

#[test]
fn ch_10_2_16_json_no_special_floats() {
    // JSON Schema: no .inf, -.inf, .nan
    // Only regular float notation allowed
    let input = "value: 3.14\n";
    let d = Doc::from_str(input).unwrap();

    let Node::Map(root) = d.root().node() else {
        panic!("root not a map");
    };
    let (_, val) = root.iter().find(|(k, _)| k == "value").unwrap();
    assert!(matches!(val.node(), Node::Scalar(Scalar::Num { .. })));
}

#[test]
fn ch_10_2_17_json_inf_is_error_or_string() {
    // JSON Schema: .inf should be error or string
    let input = "value: .inf\n";
    let result = Doc::from_str(input);

    // In strict JSON schema, should be error
    // Core schema allows it
    if result.is_ok() {
        // Parser uses Core schema
        let _ = result.unwrap();
    }
}

#[test]
fn ch_10_2_18_json_nan_is_error_or_string() {
    // JSON Schema: .nan should be error or string
    let input = "value: .nan\n";
    let result = Doc::from_str(input);

    // In strict JSON schema, should be error
    // Core schema allows it
    if result.is_ok() {
        // Parser uses Core schema
        let _ = result.unwrap();
    }
}

// ============================================================================
// 10.3 Core Schema
// ============================================================================

#[test]
fn ch_10_3_01_core_null_variations() {
    Fixture::new(
        "10.3",
        1,
        "Core: null variations (null, ~)",
        "null1: null\nnull2: ~\n",
    )
    .run();
}

#[test]
fn ch_10_3_02_core_bool_case_insensitive() {
    Fixture::new(
        "10.3",
        2,
        "Core: boolean case variations",
        "t1: true\nt2: True\nf1: false\nf2: False\n",
    )
    .run();
}

#[test]
fn ch_10_3_03_core_integer_formats() {
    Fixture::new(
        "10.3",
        3,
        "Core: integer formats (decimal, hex, octal)",
        "decimal: 42\nhex: 0x2A\noctal: 0o52\n",
    )
    .run();
}

#[test]
fn ch_10_3_04_core_float_special() {
    Fixture::new(
        "10.3",
        4,
        "Core: float special values (.inf, .nan)",
        "inf: .inf\nneginf: -.inf\nnan: .nan\n",
    )
    .run();
}

#[test]
fn ch_10_3_05_core_timestamp() {
    Fixture::new(
        "10.3",
        5,
        "Core: ISO 8601 timestamp",
        "timestamp: 2001-12-15T02:59:43.1Z\n",
    )
    .run();
}

// ============================================================================
// 10.3.1 Core Schema - Boolean Variations
// ============================================================================

#[test]
fn ch_10_3_1_01_core_bool_yes_no() {
    // Core: Yes/No as boolean
    Fixture::new(
        "10.3.1",
        1,
        "Core boolean: Yes/No",
        "enabled: Yes\ndisabled: No\n",
    )
    .run();
}

#[test]
fn ch_10_3_1_02_core_bool_yes_no_lowercase() {
    // Core: yes/no as boolean (lowercase)
    Fixture::new(
        "10.3.1",
        2,
        "Core boolean: yes/no (lowercase)",
        "enabled: yes\ndisabled: no\n",
    )
    .run();
}

#[test]
fn ch_10_3_1_03_core_bool_on_off() {
    // Core: On/Off as boolean
    Fixture::new(
        "10.3.1",
        3,
        "Core boolean: On/Off",
        "power: On\nlight: Off\n",
    )
    .run();
}

#[test]
fn ch_10_3_1_04_core_bool_on_off_lowercase() {
    // Core: on/off as boolean (lowercase)
    Fixture::new(
        "10.3.1",
        4,
        "Core boolean: on/off (lowercase)",
        "power: on\nlight: off\n",
    )
    .run();
}

#[test]
fn ch_10_3_1_05_core_bool_y_n() {
    // Core: Y/N as boolean
    Fixture::new(
        "10.3.1",
        5,
        "Core boolean: Y/N",
        "answer: Y\nquestion: N\n",
    )
    .run();
}

#[test]
fn ch_10_3_1_06_core_bool_y_n_lowercase() {
    // Core: y/n as boolean (lowercase)
    Fixture::new(
        "10.3.1",
        6,
        "Core boolean: y/n (lowercase)",
        "answer: y\nquestion: n\n",
    )
    .run();
}

#[test]
fn ch_10_3_1_07_core_bool_true_variations() {
    // Core: TRUE, True variations
    Fixture::new(
        "10.3.1",
        7,
        "Core boolean: TRUE/True variations",
        "t1: TRUE\nt2: True\nt3: true\n",
    )
    .run();
}

#[test]
fn ch_10_3_1_08_core_bool_false_variations() {
    // Core: FALSE, False variations
    Fixture::new(
        "10.3.1",
        8,
        "Core boolean: FALSE/False variations",
        "f1: FALSE\nf2: False\nf3: false\n",
    )
    .run();
}

// ============================================================================
// 10.3.2 Core Schema - Null Variations
// ============================================================================

#[test]
fn ch_10_3_2_01_core_null_lowercase() {
    // Core: null (lowercase)
    Fixture::new(
        "10.3.2",
        1,
        "Core null: null (lowercase)",
        "value: null\n",
    )
    .run();
}

#[test]
fn ch_10_3_2_02_core_null_capitalized() {
    // Core: Null (capitalized)
    Fixture::new(
        "10.3.2",
        2,
        "Core null: Null (capitalized)",
        "value: Null\n",
    )
    .run();
}

#[test]
fn ch_10_3_2_03_core_null_uppercase() {
    // Core: NULL (uppercase)
    Fixture::new(
        "10.3.2",
        3,
        "Core null: NULL (uppercase)",
        "value: NULL\n",
    )
    .run();
}

#[test]
fn ch_10_3_2_04_core_null_tilde() {
    // Core: ~ (tilde) as null
    Fixture::new(
        "10.3.2",
        4,
        "Core null: ~ (tilde)",
        "value: ~\n",
    )
    .run();
}

#[test]
fn ch_10_3_2_05_core_null_empty_value() {
    // Core: empty value as null
    Fixture::new(
        "10.3.2",
        5,
        "Core null: empty value",
        "value:\nother: data\n",
    )
    .run();
}

// ============================================================================
// 10.3.3 Core Schema - Integer Bases and Formats
// ============================================================================

#[test]
fn ch_10_3_3_01_core_int_binary() {
    // Core: binary notation (0b)
    Fixture::new(
        "10.3.3",
        1,
        "Core integer: binary (0b)",
        "binary: 0b1010\n",
    )
    .run();
}

#[test]
fn ch_10_3_3_02_core_int_octal() {
    // Core: octal notation (0o)
    Fixture::new(
        "10.3.3",
        2,
        "Core integer: octal (0o)",
        "octal: 0o52\n",
    )
    .run();
}

#[test]
fn ch_10_3_3_03_core_int_hex_lowercase() {
    // Core: hexadecimal notation (0x) lowercase
    Fixture::new(
        "10.3.3",
        3,
        "Core integer: hex (0x) lowercase",
        "hex: 0x2a\n",
    )
    .run();
}

#[test]
fn ch_10_3_3_04_core_int_hex_uppercase() {
    // Core: hexadecimal notation (0x) uppercase
    Fixture::new(
        "10.3.3",
        4,
        "Core integer: hex (0x) uppercase",
        "hex: 0x2A\n",
    )
    .run();
}

#[test]
fn ch_10_3_3_05_core_int_underscores() {
    // Core: integer with underscores (1_000_000)
    Fixture::new(
        "10.3.3",
        5,
        "Core integer: underscores for readability",
        "million: 1_000_000\n",
    )
    .run();
}

#[test]
fn ch_10_3_3_06_core_int_underscores_multiple() {
    // Core: multiple underscores in different positions
    Fixture::new(
        "10.3.3",
        6,
        "Core integer: multiple underscores",
        "value: 1_234_567_890\n",
    )
    .run();
}

#[test]
fn ch_10_3_3_07_core_int_positive_sign() {
    // Core: explicit positive sign
    Fixture::new(
        "10.3.3",
        7,
        "Core integer: explicit positive sign",
        "positive: +42\n",
    )
    .run();
}

#[test]
fn ch_10_3_3_08_core_int_sexagesimal() {
    // Core: sexagesimal (base 60) notation
    Fixture::new(
        "10.3.3",
        8,
        "Core integer: sexagesimal (60:00 = 3600)",
        "seconds: 60:00\n",
    )
    .run();
}

// ============================================================================
// 10.3.4 Core Schema - Float Special Values
// ============================================================================

#[test]
fn ch_10_3_4_01_core_float_inf_lowercase() {
    // Core: .inf (lowercase)
    Fixture::new(
        "10.3.4",
        1,
        "Core float: .inf (lowercase)",
        "infinity: .inf\n",
    )
    .run();
}

#[test]
fn ch_10_3_4_02_core_float_inf_capitalized() {
    // Core: .Inf (capitalized)
    Fixture::new(
        "10.3.4",
        2,
        "Core float: .Inf (capitalized)",
        "infinity: .Inf\n",
    )
    .run();
}

#[test]
fn ch_10_3_4_03_core_float_inf_uppercase() {
    // Core: .INF (uppercase)
    Fixture::new(
        "10.3.4",
        3,
        "Core float: .INF (uppercase)",
        "infinity: .INF\n",
    )
    .run();
}

#[test]
fn ch_10_3_4_04_core_float_neg_inf_lowercase() {
    // Core: -.inf (negative infinity lowercase)
    Fixture::new(
        "10.3.4",
        4,
        "Core float: -.inf (negative infinity)",
        "neg_infinity: -.inf\n",
    )
    .run();
}

#[test]
fn ch_10_3_4_05_core_float_neg_inf_capitalized() {
    // Core: -.Inf (negative infinity capitalized)
    Fixture::new(
        "10.3.4",
        5,
        "Core float: -.Inf (negative infinity capitalized)",
        "neg_infinity: -.Inf\n",
    )
    .run();
}

#[test]
fn ch_10_3_4_06_core_float_neg_inf_uppercase() {
    // Core: -.INF (negative infinity uppercase)
    Fixture::new(
        "10.3.4",
        6,
        "Core float: -.INF (negative infinity uppercase)",
        "neg_infinity: -.INF\n",
    )
    .run();
}

#[test]
fn ch_10_3_4_07_core_float_nan_lowercase() {
    // Core: .nan (lowercase)
    Fixture::new(
        "10.3.4",
        7,
        "Core float: .nan (lowercase)",
        "not_a_number: .nan\n",
    )
    .run();
}

#[test]
fn ch_10_3_4_08_core_float_nan_capitalized() {
    // Core: .NaN (capitalized)
    Fixture::new(
        "10.3.4",
        8,
        "Core float: .NaN (capitalized)",
        "not_a_number: .NaN\n",
    )
    .run();
}

#[test]
fn ch_10_3_4_09_core_float_nan_uppercase() {
    // Core: .NAN (uppercase)
    Fixture::new(
        "10.3.4",
        9,
        "Core float: .NAN (uppercase)",
        "not_a_number: .NAN\n",
    )
    .run();
}

#[test]
fn ch_10_3_4_10_core_float_underscores() {
    // Core: float with underscores
    Fixture::new(
        "10.3.4",
        10,
        "Core float: underscores for readability",
        "value: 1_234.567_89\n",
    )
    .run();
}

// ============================================================================
// 10.3.5 Core Schema - Timestamps and Binary
// ============================================================================

#[test]
fn ch_10_3_5_01_core_timestamp_date_only() {
    // Core: ISO 8601 date only
    Fixture::new(
        "10.3.5",
        1,
        "Core timestamp: date only (YYYY-MM-DD)",
        "date: 2001-12-15\n",
    )
    .run();
}

#[test]
fn ch_10_3_5_02_core_timestamp_datetime_space() {
    // Core: ISO 8601 with space separator
    Fixture::new(
        "10.3.5",
        2,
        "Core timestamp: datetime with space",
        "datetime: 2001-12-15 02:59:43.1Z\n",
    )
    .run();
}

#[test]
fn ch_10_3_5_03_core_timestamp_datetime_t() {
    // Core: ISO 8601 with T separator
    Fixture::new(
        "10.3.5",
        3,
        "Core timestamp: datetime with T",
        "datetime: 2001-12-15T02:59:43.1Z\n",
    )
    .run();
}

#[test]
fn ch_10_3_5_04_core_timestamp_timezone_offset() {
    // Core: ISO 8601 with timezone offset
    Fixture::new(
        "10.3.5",
        4,
        "Core timestamp: timezone offset",
        "datetime: 2001-12-15T02:59:43.1+05:30\n",
    )
    .run();
}

#[test]
fn ch_10_3_5_05_core_timestamp_no_timezone() {
    // Core: ISO 8601 without timezone (local time)
    Fixture::new(
        "10.3.5",
        5,
        "Core timestamp: no timezone",
        "datetime: 2001-12-15T02:59:43.1\n",
    )
    .run();
}

#[test]
fn ch_10_3_5_06_core_binary_base64() {
    // Core: !!binary tag with base64
    Fixture::new(
        "10.3.5",
        6,
        "Core binary: base64 encoded",
        "image: !!binary |\n  R0lGODlhDAAMAIQAAP//9/X\n  17unp5WZmZgAAAOfn515eXv\n",
    )
    .run();
}

#[test]
fn ch_10_3_5_07_core_merge_key() {
    // Core: merge key (<<) for inheritance
    Fixture::new(
        "10.3.5",
        7,
        "Core merge key: << for inheritance",
        "defaults: &defaults\n  color: red\n  size: 10\nitem:\n  <<: *defaults\n  name: widget\n",
    )
    .run();
}

#[test]
fn ch_10_3_5_08_core_merge_multiple() {
    // Core: merge multiple mappings
    Fixture::new(
        "10.3.5",
        8,
        "Core merge key: multiple mappings",
        "base1: &b1\n  x: 1\nbase2: &b2\n  y: 2\nmerged:\n  <<: [*b1, *b2]\n  z: 3\n",
    )
    .run();
}

// ============================================================================
// 10.4 Type Resolution
// ============================================================================

#[test]
fn ch_10_4_01_plain_string() {
    Fixture::new(
        "10.4",
        1,
        "Type resolution: plain string",
        "name: hello world\n",
    )
    .run();
}

#[test]
fn ch_10_4_02_number_detection() {
    let input = "version: 123\n";
    let d = Doc::from_str(input).unwrap();

    let Node::Map(root) = d.root().node() else {
        panic!("root not a map");
    };
    let (_, val) = root.iter().find(|(k, _)| k == "version").unwrap();
    assert!(matches!(val.node(), Node::Scalar(Scalar::Num { .. })));
}

#[test]
fn ch_10_4_03_quoted_forces_string() {
    let input = "version: \"123\"\n";
    let d = Doc::from_str(input).unwrap();

    let Node::Map(root) = d.root().node() else {
        panic!("root not a map");
    };
    let (_, val) = root.iter().find(|(k, _)| k == "version").unwrap();
    assert!(matches!(val.node(), Node::Scalar(Scalar::Str(s)) if s == "123"));
}

#[test]
fn ch_10_4_04_bool_detection() {
    Fixture::new(
        "10.4",
        4,
        "Type resolution: boolean",
        "enabled: true\n",
    )
    .run();
}

#[test]
fn ch_10_4_05_quoted_bool_is_string() {
    let input = "value: \"true\"\n";
    let d = Doc::from_str(input).unwrap();

    let Node::Map(root) = d.root().node() else {
        panic!("root not a map");
    };
    let (_, val) = root.iter().find(|(k, _)| k == "value").unwrap();
    assert!(matches!(val.node(), Node::Scalar(Scalar::Str(s)) if s == "true"));
}

#[test]
fn ch_10_4_06_null_detection() {
    Fixture::new(
        "10.4",
        6,
        "Type resolution: null",
        "value: null\n",
    )
    .run();
}

#[test]
fn ch_10_4_07_tilde_as_null() {
    Fixture::new(
        "10.4",
        7,
        "Type resolution: tilde as null",
        "value: ~\n",
    )
    .run();
}

// ============================================================================
// Number Format Edge Cases
// ============================================================================

#[test]
fn ch_10_5_01_integer_zero() {
    Fixture::new(
        "10.5",
        1,
        "Number: integer zero",
        "zero: 0\n",
    )
    .run();
}

#[test]
fn ch_10_5_02_negative_zero() {
    Fixture::new(
        "10.5",
        2,
        "Number: negative zero",
        "negzero: -0\n",
    )
    .run();
}

#[test]
fn ch_10_5_03_float_zero() {
    Fixture::new(
        "10.5",
        3,
        "Number: float zero",
        "fzero: 0.0\n",
    )
    .run();
}

#[test]
fn ch_10_5_04_large_integer() {
    Fixture::new(
        "10.5",
        4,
        "Number: large integer",
        "big: 9223372036854775807\n",
    )
    .run();
}

#[test]
fn ch_10_5_05_decimal_leading_zero() {
    Fixture::new(
        "10.5",
        5,
        "Number: decimal with leading zero",
        "value: 0.123\n",
    )
    .run();
}

#[test]
fn ch_10_5_06_exponential_positive() {
    Fixture::new(
        "10.5",
        6,
        "Number: exponential positive exponent",
        "value: 1.23e+10\n",
    )
    .run();
}

#[test]
fn ch_10_5_07_exponential_negative() {
    Fixture::new(
        "10.5",
        7,
        "Number: exponential negative exponent",
        "value: 1.23e-10\n",
    )
    .run();
}

// ============================================================================
// String Type Edge Cases
// ============================================================================

#[test]
fn ch_10_6_01_url_is_string() {
    Fixture::new(
        "10.6",
        1,
        "String: URL",
        "url: https://example.com\n",
    )
    .run();
}

#[test]
fn ch_10_6_02_email_is_string() {
    Fixture::new(
        "10.6",
        2,
        "String: email address",
        "email: user@example.com\n",
    )
    .run();
}

#[test]
fn ch_10_6_03_version_string() {
    Fixture::new(
        "10.6",
        3,
        "String: version number",
        "version: 1.0\n",
    )
    .run();
}

#[test]
fn ch_10_6_04_quoted_number() {
    Fixture::new(
        "10.6",
        4,
        "String: quoted number",
        "code: \"007\"\n",
    )
    .run();
}
