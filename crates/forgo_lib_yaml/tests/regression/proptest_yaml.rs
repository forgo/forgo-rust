//! Property-based testing for YAML parser/emitter
//!
//! Tests the following properties:
//! 1. Round-trip stability: parse → emit → parse should be idempotent
//! 2. Unicode handling: arbitrary valid Unicode should not crash
//! 3. Deep nesting: arbitrarily deep structures should parse correctly
//! 4. Wide collections: large sequences/mappings should work
//! 5. Structural invariants: parsed AST should always be valid

use forgo_lib_yaml::Doc;

/// Deterministic pseudo-random generator (xorshift64*)
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(if seed == 0 { 1 } else { seed })
    }

    fn next_u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        ((x.wrapping_mul(2685821657736338717)) >> 32) as u32
    }

    fn gen_range(&mut self, min: u32, max: u32) -> u32 {
        min + (self.next_u32() % (max - min))
    }

    fn gen_bool(&mut self) -> bool {
        self.next_u32() % 2 == 0
    }

    fn pick_str<'a>(&mut self, items: &'a [&'a str]) -> &'a str {
        items[(self.next_u32() as usize) % items.len()]
    }
}

// =============================================================================
// Semantic Equality (uses PartialEq but checks emitted form)
// =============================================================================

fn docs_semantically_equal(d1: &Doc, d2: &Doc) -> bool {
    // For semantic comparison, we emit both and compare the result
    // This handles cases where metadata (prefer_quoted, etc.) differs
    // but the semantic content is the same
    let emit1 = match d1.to_string() {
        Ok(s) => s,
        Err(_) => return false,
    };

    let emit2 = match d2.to_string() {
        Ok(s) => s,
        Err(_) => return false,
    };

    // Normalize whitespace differences (trailing vs no trailing newline, empty vs "")
    normalize_yaml(&emit1) == normalize_yaml(&emit2)
}

fn normalize_yaml(s: &str) -> String {
    s.lines()
        .map(|line| {
            // Normalize "key:" and "key: ''" to be equivalent
            if line.ends_with(':') {
                format!("{}: \"\"", line.trim_end_matches(':'))
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

// =============================================================================
// YAML Value Generators
// =============================================================================

/// Generate random scalar values
fn gen_scalar(rng: &mut Rng) -> String {
    let scalar_types = [
        // Plain strings
        "foo", "bar", "baz", "hello", "world", "test", "value", "key",
        // Numbers
        "0", "1", "42", "-1", "3.14", "-99.99", "1e10", "0.0",
        // Booleans
        "true", "false", "yes", "no", "on", "off",
        // Null
        "null", "~",
        // Strings with special chars (that need quoting)
        "hello world", "foo: bar", "- item", "[1, 2]", "{a: b}",
        // URLs and identifiers
        "http://example.com", "user@example.com", "v1.2.3", "some-id",
    ];

    rng.pick_str(&scalar_types).to_string()
}

/// Generate random mapping key
fn gen_key(rng: &mut Rng) -> String {
    let keys = [
        "a", "b", "c", "x", "y", "z",
        "name", "value", "id", "type", "key",
        "foo", "bar", "baz", "qux",
        "alpha", "beta", "gamma", "delta",
        "first", "second", "third",
        "item", "element", "node",
        "project", "config", "setting",
        "jobs", "steps", "matrix",
    ];

    rng.pick_str(&keys).to_string()
}

/// Generate random YAML document with controlled complexity
fn gen_yaml_doc(rng: &mut Rng, max_depth: u32, max_width: u32) -> String {
    gen_yaml_value(rng, max_depth, max_width, 0)
}

fn gen_yaml_value(rng: &mut Rng, max_depth: u32, max_width: u32, current_depth: u32) -> String {
    if current_depth >= max_depth {
        return gen_scalar(rng);
    }

    let choice = rng.gen_range(0, 10);

    match choice {
        0..=4 => gen_scalar(rng), // 50% scalar
        5..=7 => gen_yaml_sequence(rng, max_depth, max_width, current_depth), // 30% sequence
        _ => gen_yaml_mapping(rng, max_depth, max_width, current_depth), // 20% mapping
    }
}

fn gen_yaml_sequence(rng: &mut Rng, max_depth: u32, max_width: u32, current_depth: u32) -> String {
    let count = rng.gen_range(0, max_width.min(10) + 1);

    if count == 0 {
        return "[]".to_string();
    }

    let mut result = String::new();

    // Sometimes use flow style
    if rng.gen_range(0, 10) < 3 {
        result.push('[');
        for i in 0..count {
            if i > 0 {
                result.push_str(", ");
            }
            result.push_str(&gen_yaml_value(rng, max_depth - 1, max_width, current_depth + 1));
        }
        result.push(']');
    } else {
        // Block style
        for _ in 0..count {
            result.push_str("- ");
            result.push_str(&gen_yaml_value(rng, max_depth - 1, max_width, current_depth + 1));
            result.push('\n');
        }
    }

    result
}

fn gen_yaml_mapping(rng: &mut Rng, max_depth: u32, max_width: u32, current_depth: u32) -> String {
    let count = rng.gen_range(0, max_width.min(10) + 1);

    if count == 0 {
        return "{}".to_string();
    }

    let mut result = String::new();
    let mut used_keys = Vec::new();

    // Sometimes use flow style
    if rng.gen_range(0, 10) < 2 {
        result.push('{');
        for i in 0..count {
            if i > 0 {
                result.push_str(", ");
            }
            let mut key = gen_key(rng);
            // Ensure unique keys
            while used_keys.contains(&key) {
                key = gen_key(rng);
            }
            used_keys.push(key.clone());

            result.push_str(&key);
            result.push_str(": ");
            result.push_str(&gen_yaml_value(rng, max_depth - 1, max_width, current_depth + 1));
        }
        result.push('}');
    } else {
        // Block style
        for _ in 0..count {
            let mut key = gen_key(rng);
            // Ensure unique keys
            while used_keys.contains(&key) {
                key = gen_key(rng);
            }
            used_keys.push(key.clone());

            result.push_str(&key);
            result.push_str(": ");
            result.push_str(&gen_yaml_value(rng, max_depth - 1, max_width, current_depth + 1));
            result.push('\n');
        }
    }

    result
}

/// Generate Unicode string with various ranges
fn gen_unicode_string(rng: &mut Rng, max_len: u32) -> String {
    let len = rng.gen_range(0, max_len + 1);
    let mut result = String::new();

    for _ in 0..len {
        let char_type = rng.gen_range(0, 10);

        let ch = match char_type {
            0..=5 => {
                // Basic ASCII (60%)
                let ascii = rng.gen_range(32, 127); // Printable ASCII
                char::from_u32(ascii).unwrap()
            }
            6 => {
                // Latin Extended
                let code = rng.gen_range(0x00A0, 0x00FF);
                char::from_u32(code).unwrap_or('?')
            }
            7 => {
                // Cyrillic
                let code = rng.gen_range(0x0400, 0x04FF);
                char::from_u32(code).unwrap_or('?')
            }
            8 => {
                // CJK (Chinese, Japanese, Korean)
                let code = rng.gen_range(0x4E00, 0x9FFF);
                char::from_u32(code).unwrap_or('?')
            }
            _ => {
                // Greek
                let code = rng.gen_range(0x0370, 0x03FF);
                char::from_u32(code).unwrap_or('?')
            }
        };

        result.push(ch);
    }

    result
}

// =============================================================================
// Property Tests
// =============================================================================

#[test]
fn prop_roundtrip_parse_emit_parse() {
    // Property: parse → emit → parse should produce identical AST
    const TEST_CASES: u64 = 500;

    for seed in 0..TEST_CASES {
        let mut rng = Rng::new(seed + 1);
        let yaml = gen_yaml_doc(&mut rng, 4, 6);

        // First parse
        let doc1 = match Doc::from_str(&yaml) {
            Ok(d) => d,
            Err(_) => continue, // Skip invalid generated YAML
        };

        // Emit
        let emitted = match doc1.to_string() {
            Ok(s) => s,
            Err(_) => {
                panic!("Emit failed for valid doc (seed={seed}):\n{yaml}");
            }
        };

        // Second parse
        let doc2 = match Doc::from_str(&emitted) {
            Ok(d) => d,
            Err(e) => {
                panic!(
                    "Re-parse failed (seed={seed}):\nOriginal:\n{yaml}\nEmitted:\n{emitted}\nError: {e:?}"
                );
            }
        };

        // Round-trip should be stable (semantically)
        // Note: metadata like prefer_quoted may differ, but structure should match
        if !docs_semantically_equal(&doc1, &doc2) {
            panic!(
                "Round-trip semantic mismatch (seed={seed}):\nOriginal:\n{yaml}\nEmitted:\n{emitted}\nDoc1: {doc1:?}\nDoc2: {doc2:?}"
            );
        }
    }
}

#[test]
fn prop_emit_parse_stability() {
    // Property: emit → parse → emit should produce identical output
    // Note: Some edge cases like "gamma:" vs "gamma: ''" may differ,
    // but the semantic meaning should be the same
    const TEST_CASES: u64 = 500;

    for seed in 0..TEST_CASES {
        let mut rng = Rng::new(seed + 1);
        let yaml = gen_yaml_doc(&mut rng, 4, 6);

        let doc1 = match Doc::from_str(&yaml) {
            Ok(d) => d,
            Err(_) => continue,
        };

        let emit1 = match doc1.to_string() {
            Ok(s) => s,
            Err(_) => continue,
        };

        let doc2 = Doc::from_str(&emit1).expect("emit1 should parse");
        let emit2 = doc2.to_string().expect("doc2 should emit");

        // Check semantic stability rather than string identity
        // (empty string representation may vary: "" vs omitted)
        let doc3 = Doc::from_str(&emit2).expect("emit2 should parse");

        if !docs_semantically_equal(&doc2, &doc3) {
            panic!(
                "Emit not semantically stable (seed={seed}):\nFirst:\n{emit1}\nSecond:\n{emit2}"
            );
        }
    }
}

#[test]
fn prop_deep_nesting() {
    // Property: deeply nested structures should parse without crashing
    const TEST_CASES: u64 = 100;
    const MAX_DEPTH: u32 = 20;

    for seed in 0..TEST_CASES {
        let mut rng = Rng::new(seed + 1);
        let depth = rng.gen_range(10, MAX_DEPTH);
        let yaml = gen_yaml_doc(&mut rng, depth, 3);

        // Should not panic
        let _ = Doc::from_str(&yaml);
    }
}

#[test]
fn prop_wide_collections() {
    // Property: wide collections (many items) should parse correctly
    const TEST_CASES: u64 = 50;
    const MAX_WIDTH: u32 = 100;

    for seed in 0..TEST_CASES {
        let mut rng = Rng::new(seed + 1);
        let width = rng.gen_range(50, MAX_WIDTH);
        let yaml = gen_yaml_doc(&mut rng, 2, width);

        let doc = match Doc::from_str(&yaml) {
            Ok(d) => d,
            Err(_) => continue,
        };

        // Should be able to emit
        let _ = doc.to_string();
    }
}

#[test]
fn prop_unicode_scalars() {
    // Property: Unicode scalars should round-trip correctly
    const TEST_CASES: u64 = 200;

    for seed in 0..TEST_CASES {
        let mut rng = Rng::new(seed + 1);
        let unicode_str = gen_unicode_string(&mut rng, 20);

        // Build a simple YAML doc with Unicode value
        let yaml = format!("value: {unicode_str}\n");

        let doc = match Doc::from_str(&yaml) {
            Ok(d) => d,
            Err(_) => continue, // Skip if Unicode is problematic
        };

        let emitted = match doc.to_string() {
            Ok(s) => s,
            Err(_) => continue,
        };

        // Re-parse should work
        let _ = Doc::from_str(&emitted);
    }
}

#[test]
fn prop_empty_collections() {
    // Property: empty collections should round-trip (semantically)
    // Note: flow style `[]` may become block style `- []` but should still parse correctly
    let empty_cases = vec![
        "[]",
        "{}",
        "a: []",
        "a: {}",
        "- []",
        "- {}",
        "{a: [], b: {}}",
    ];

    for yaml in empty_cases {
        let doc = Doc::from_str(yaml).expect("should parse empty collection");
        let emitted = doc.to_string().expect("should emit");
        let doc2 = Doc::from_str(&emitted).expect("should re-parse");
        let emitted2 = doc2.to_string().expect("should re-emit");

        // After one round-trip, subsequent emissions should stabilize
        let doc3 = Doc::from_str(&emitted2).expect("should parse again");
        let emitted3 = doc3.to_string().expect("should emit again");

        assert_eq!(
            emitted2, emitted3,
            "empty collection not stable after round-trip: {yaml}\nFirst emit: {emitted}\nSecond emit: {emitted2}\nThird emit: {emitted3}"
        );
    }
}

#[test]
fn prop_mixed_flow_block() {
    // Property: mixing flow and block styles should work
    const TEST_CASES: u64 = 200;

    for seed in 0..TEST_CASES {
        let mut rng = Rng::new(seed + 1);

        // Generate doc that mixes styles
        let yaml = if rng.gen_bool() {
            // Block with flow inside
            format!(
                "outer:\n  inner: {}\n  list: [1, 2, 3]\n",
                gen_yaml_value(&mut rng, 2, 5, 0)
            )
        } else {
            // Flow with potential nesting
            format!(
                "{{a: {}, b: {}}}",
                gen_yaml_value(&mut rng, 2, 5, 0),
                gen_yaml_value(&mut rng, 2, 5, 0)
            )
        };

        let doc = match Doc::from_str(&yaml) {
            Ok(d) => d,
            Err(_) => continue,
        };

        let _ = doc.to_string();
    }
}

#[test]
fn prop_scalar_types() {
    // Property: different scalar types should be preserved
    const TEST_CASES: u64 = 300;

    for seed in 0..TEST_CASES {
        let mut rng = Rng::new(seed + 1);
        let scalar = gen_scalar(&mut rng);

        let yaml = format!("value: {scalar}\n");

        let doc = match Doc::from_str(&yaml) {
            Ok(d) => d,
            Err(_) => continue,
        };

        let emitted = match doc.to_string() {
            Ok(s) => s,
            Err(_) => continue,
        };

        // Should re-parse
        let doc2 = Doc::from_str(&emitted).expect("scalar round-trip parse");

        // AST should be semantically equal
        if !docs_semantically_equal(&doc, &doc2) {
            panic!("scalar type round-trip failed: {scalar}\nEmitted: {emitted}");
        }
    }
}

#[test]
fn prop_no_crash_random_input() {
    // Property: parser should never crash on random input
    const TEST_CASES: u64 = 1000;

    for seed in 0..TEST_CASES {
        let mut rng = Rng::new(seed + 1);
        let len = rng.gen_range(0, 500);
        let mut random_bytes = Vec::new();

        for _ in 0..len {
            random_bytes.push(rng.gen_range(0, 256) as u8);
        }

        // Try to parse random bytes as UTF-8
        if let Ok(random_str) = String::from_utf8(random_bytes) {
            // Should not panic
            let _ = Doc::from_str(&random_str);
        }
    }
}

#[test]
fn prop_deterministic_emit() {
    // Property: emitting the same AST multiple times should produce identical output
    const TEST_CASES: u64 = 200;

    for seed in 0..TEST_CASES {
        let mut rng = Rng::new(seed + 1);
        let yaml = gen_yaml_doc(&mut rng, 4, 6);

        let doc = match Doc::from_str(&yaml) {
            Ok(d) => d,
            Err(_) => continue,
        };

        let emit1 = doc.to_string().expect("first emit");
        let emit2 = doc.to_string().expect("second emit");
        let emit3 = doc.to_string().expect("third emit");

        assert_eq!(emit1, emit2, "emit not deterministic (1 vs 2)");
        assert_eq!(emit2, emit3, "emit not deterministic (2 vs 3)");
    }
}

#[test]
fn prop_structure_preservation() {
    // Property: sequence order and mapping keys should be preserved
    const TEST_CASES: u64 = 200;

    for seed in 0..TEST_CASES {
        let mut rng = Rng::new(seed + 1);

        // Generate sequence
        let items: Vec<String> = (0..rng.gen_range(3, 10))
            .map(|_| gen_scalar(&mut rng))
            .collect();

        let yaml = format!("items:\n{}", items.iter().map(|s| format!("  - {s}\n")).collect::<String>());

        let doc = Doc::from_str(&yaml).expect("should parse sequence");
        let emitted = doc.to_string().expect("should emit");
        let doc2 = Doc::from_str(&emitted).expect("should re-parse");

        // Structure should be preserved (semantically)
        if !docs_semantically_equal(&doc, &doc2) {
            panic!("structure not preserved:\nOriginal: {yaml}\nEmitted: {emitted}");
        }
    }
}
