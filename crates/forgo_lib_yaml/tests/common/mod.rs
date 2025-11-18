// crates/forgo_lib_yaml/tests/common/mod.rs
//! Common test utilities for spec-driven testing without third-party dependencies.

use forgo_lib_yaml::Doc;
use std::fs;
use std::path::Path;

/// Test fixture representing a YAML test case.
#[allow(dead_code)]
pub struct Fixture {
    /// Spec section reference (e.g., "8.1")
    pub section: &'static str,
    /// Test case number within section
    pub case: usize,
    /// Human-readable description
    pub description: &'static str,
    /// Input YAML content
    pub input: &'static str,
    /// Expected output after round-trip (parse → emit)
    pub expected_output: Option<&'static str>,
    /// Expected parse error message substring (if this should fail to parse)
    pub expected_error: Option<&'static str>,
    /// Whether round-trip should be stable (parse → emit → parse → emit gives same result)
    pub round_trip_stable: bool,
}

#[allow(dead_code)]
impl Fixture {
    /// Create a new test fixture for successful parsing.
    pub fn new(
        section: &'static str,
        case: usize,
        description: &'static str,
        input: &'static str,
    ) -> Self {
        Self {
            section,
            case,
            description,
            input,
            expected_output: None,
            expected_error: None,
            round_trip_stable: true,
        }
    }

    /// Set expected output for round-trip test.
    pub fn expect_output(mut self, output: &'static str) -> Self {
        self.expected_output = Some(output);
        self
    }

    /// Set expected error for parse failure test.
    pub fn expect_error(mut self, error: &'static str) -> Self {
        self.expected_error = Some(error);
        self.round_trip_stable = false;
        self
    }

    /// Disable round-trip stability check.
    pub fn no_round_trip(mut self) -> Self {
        self.round_trip_stable = false;
        self
    }

    /// Run the test fixture.
    pub fn run(&self) {
        println!(
            "\n[Spec §{}.{:02}] {}",
            self.section, self.case, self.description
        );

        if let Some(expected_err) = self.expected_error {
            // This test should fail to parse
            match Doc::from_str(self.input) {
                Ok(doc) => {
                    panic!(
                        "Expected parse error containing '{}', but parsing succeeded.\nDoc: {:?}",
                        expected_err, doc
                    );
                }
                Err(e) => {
                    let err_msg = format!("{:?}", e);
                    assert!(
                        err_msg.contains(expected_err),
                        "Parse error '{}' does not contain expected substring '{}'",
                        err_msg,
                        expected_err
                    );
                    println!("✓ Parse failed as expected: {}", err_msg);
                }
            }
            return;
        }

        // Parse input
        let doc = Doc::from_str(self.input).unwrap_or_else(|e| {
            panic!(
                "Failed to parse input:\n{}\nError: {:?}",
                self.input, e
            );
        });

        // Emit to string
        let output = doc.to_string().unwrap_or_else(|e| {
            panic!("Failed to emit document: {:?}", e);
        });

        println!("Input:\n{}", self.input);
        println!("Output:\n{}", output);

        // Check expected output if provided
        if let Some(expected) = self.expected_output {
            assert_eq!(
                output.trim(),
                expected.trim(),
                "Output does not match expected"
            );
            println!("✓ Output matches expected");
        }

        // Round-trip stability test
        if self.round_trip_stable {
            let doc2 = Doc::from_str(&output).unwrap_or_else(|e| {
                panic!(
                    "Failed to parse emitted output:\n{}\nError: {:?}",
                    output, e
                );
            });

            let output2 = doc2.to_string().unwrap_or_else(|e| {
                panic!("Failed to emit document on second pass: {:?}", e);
            });

            assert_eq!(
                output.trim(),
                output2.trim(),
                "Round-trip not stable: parse → emit → parse → emit changed output"
            );
            println!("✓ Round-trip stable");
        }
    }
}

/// Load a test fixture file from disk.
#[allow(dead_code)]
pub fn load_fixture(path: impl AsRef<Path>) -> String {
    fs::read_to_string(path.as_ref()).unwrap_or_else(|e| {
        panic!("Failed to read fixture file {:?}: {}", path.as_ref(), e);
    })
}

/// Helper to create a test with expected output.
#[allow(dead_code)]
pub fn test_parse_emit_expect(
    section: &'static str,
    case: usize,
    desc: &'static str,
    input: &'static str,
    expected: &'static str,
) {
    Fixture::new(section, case, desc, input)
        .expect_output(expected)
        .run();
}

/// Helper to create a test expecting parse error.
#[allow(dead_code)]
pub fn test_parse_error(
    section: &'static str,
    case: usize,
    desc: &'static str,
    input: &'static str,
    error_substring: &'static str,
) {
    Fixture::new(section, case, desc, input)
        .expect_error(error_substring)
        .run();
}
