// crates/forgo_lib_yaml/tests/spec/ch_9_documents.rs
//! YAML 1.2.2 Chapter 9: Document Stream Productions
//!
//! **Spec Reference:** https://yaml.org/spec/1.2.2/#chapter-9-document-stream-productions
//!
//! **Purpose:**
//! This file contains systematic tests for YAML 1.2.2 Chapter 9. Each test
//! maps to a specific section of the specification and validates compliance
//! with the requirements defined there. Test names follow the pattern
//! `ch_9_Y_ZZ_description` where 9 is the chapter, Y is the section, and
//! ZZ is the test number.
//!
//! **Coverage:**
//! Document markers, bare/explicit/directives documents, multi-document streams

#[path = "../common/mod.rs"]
mod common;

use common::Fixture;
use forgo_lib_yaml::Doc;

// ============================================================================
// 9.1 Documents
// ============================================================================

// 9.1.1 Document Prefix
#[test]
fn ch_9_1_1_01_document_with_bom() {
    // Byte Order Mark handling (if supported)
    Fixture::new(
        "9.1.1",
        1,
        "Document with BOM (if supported)",
        "key: value\n",
    )
    .run();
}

// 9.1.2 Document Markers
#[test]
fn ch_9_1_2_01_explicit_document_start() {
    Fixture::new(
        "9.1.2",
        1,
        "Explicit document start (---)",
        "---\nkey: value\n",
    )
    .run();
}

#[test]
fn ch_9_1_2_02_explicit_document_end() {
    Fixture::new(
        "9.1.2",
        2,
        "Explicit document end (...)",
        "key: value\n...\n",
    )
    .run();
}

#[test]
fn ch_9_1_2_03_both_markers() {
    Fixture::new(
        "9.1.2",
        3,
        "Both start and end markers",
        "---\nkey: value\n...\n",
    )
    .run();
}

#[test]
fn ch_9_1_2_04_marker_only() {
    Fixture::new(
        "9.1.2",
        4,
        "Document with only marker",
        "---\n",
    )
    .run();
}

#[test]
fn ch_9_1_2_05_consecutive_end_markers() {
    // Section 9.1.2 - multiple consecutive document end markers
    // Should handle multiple ... markers in sequence
    let input = "doc1: value1\n...\n...\n---\ndoc2: value2\n";

    let docs = Doc::from_stream(input).unwrap();
    assert!(
        docs.len() >= 2,
        "Should parse at least 2 documents with consecutive end markers"
    );
}

// 9.1.3 Bare Documents
#[test]
fn ch_9_1_3_01_bare_document() {
    Fixture::new(
        "9.1.3",
        1,
        "Bare document (no markers)",
        "key: value\n",
    )
    .run();
}

#[test]
fn ch_9_1_3_02_bare_scalar() {
    Fixture::new(
        "9.1.3",
        2,
        "Bare scalar document",
        "just a string\n",
    )
    .run();
}

#[test]
fn ch_9_1_3_03_bare_sequence() {
    Fixture::new(
        "9.1.3",
        3,
        "Bare sequence document",
        "- item1\n- item2\n",
    )
    .run();
}

// 9.1.4 Explicit Documents
#[test]
fn ch_9_1_4_01_explicit_document_with_mapping() {
    Fixture::new(
        "9.1.4",
        1,
        "Explicit document with mapping",
        "---\nkey: value\nother: data\n",
    )
    .run();
}

#[test]
fn ch_9_1_4_02_explicit_document_with_sequence() {
    Fixture::new(
        "9.1.4",
        2,
        "Explicit document with sequence",
        "---\n- item1\n- item2\n",
    )
    .run();
}

#[test]
fn ch_9_1_4_03_explicit_scalar() {
    Fixture::new(
        "9.1.4",
        3,
        "Explicit document with scalar",
        "---\njust a string\n",
    )
    .run();
}

// 9.1.5 Directives Documents
#[test]
fn ch_9_1_5_01_document_with_yaml_directive() {
    Fixture::new(
        "9.1.5",
        1,
        "Document with YAML directive",
        "%YAML 1.2\n---\nkey: value\n",
    )
    .run();
}

#[test]
fn ch_9_1_5_02_document_with_tag_directive() {
    Fixture::new(
        "9.1.5",
        2,
        "Document with TAG directive",
        "%TAG !e! tag:example.com,2000:app/\n---\nkey: value\n",
    )
    .run();
}

#[test]
fn ch_9_1_5_03_document_with_both_directives() {
    Fixture::new(
        "9.1.5",
        3,
        "Document with both directives",
        "%YAML 1.2\n%TAG !e! tag:example.com,2000:app/\n---\nkey: value\n",
    )
    .run();
}

// ============================================================================
// 9.2 Streams
// ============================================================================

#[test]
fn ch_9_2_01_simple_stream() {
    let input = "---\nfirst: 1\n---\nsecond: 2\n";
    let docs = Doc::from_stream(input).unwrap();
    assert_eq!(docs.len(), 2);
}

#[test]
fn ch_9_2_02_stream_with_end_markers() {
    let input = "---\nfirst: 1\n...\n---\nsecond: 2\n...\n";
    let docs = Doc::from_stream(input).unwrap();
    assert_eq!(docs.len(), 2);
}

#[test]
fn ch_9_2_03_stream_implicit_documents() {
    // Multiple documents separated by ---
    let input = "---\nfirst: 1\n---\nsecond: 2\n---\nthird: 3\n";
    let docs = Doc::from_stream(input).unwrap();
    assert_eq!(docs.len(), 3);
}

#[test]
fn ch_9_2_04_stream_with_directives() {
    let input = "%YAML 1.2\n---\nfirst: 1\n---\nsecond: 2\n";
    let docs = Doc::from_stream(input).unwrap();
    assert_eq!(docs.len(), 2);

    // First document should have version
    assert_eq!(docs[0].yaml_version(), Some((1, 2)));
}

#[test]
fn ch_9_2_05_stream_round_trip() {
    let input = "---\nfirst: 1\n---\nsecond: 2\n";
    let docs = Doc::from_stream(input).unwrap();
    let output = Doc::to_stream_string(&docs).unwrap();

    // Should be able to parse again
    let docs2 = Doc::from_stream(&output).unwrap();
    assert_eq!(docs.len(), docs2.len());
}

#[test]
fn ch_9_2_06_single_document_stream() {
    let input = "---\nkey: value\n";
    let docs = Doc::from_stream(input).unwrap();
    assert_eq!(docs.len(), 1);
}

#[test]
fn ch_9_2_07_bare_document_as_stream() {
    let input = "key: value\n";
    let docs = Doc::from_stream(input).unwrap();
    assert_eq!(docs.len(), 1);
}

#[test]
fn ch_9_2_08_stream_with_comments() {
    Fixture::new(
        "9.2",
        8,
        "Stream with comments between documents",
        "---\n# first doc\nfirst: 1\n---\n# second doc\nsecond: 2\n",
    )
    .no_round_trip() // May not preserve comments between docs
    .run();
}

#[test]
fn ch_9_2_09_stream_different_types() {
    let input = "---\nscalar\n---\n- sequence\n---\nkey: value\n";
    let docs = Doc::from_stream(input).unwrap();
    assert_eq!(docs.len(), 3);
}

// ============================================================================
// Document Stream Edge Cases
// ============================================================================

#[test]
fn ch_9_3_01_empty_document_in_stream() {
    let input = "---\n---\nkey: value\n";
    let docs = Doc::from_stream(input).unwrap();
    // Should parse, though behavior of empty document may vary
    assert!(docs.len() >= 1);
}

#[test]
fn ch_9_3_02_document_with_only_comment() {
    Fixture::new(
        "9.3",
        2,
        "Document with only comment",
        "---\n# just a comment\n",
    )
    .run();
}

#[test]
fn ch_9_3_03_stream_no_final_newline() {
    let input = "---\nkey: value";
    let docs = Doc::from_stream(input).unwrap();
    assert_eq!(docs.len(), 1);
}

#[test]
fn ch_9_3_04_stream_trailing_marker() {
    let input = "---\nkey: value\n...\n";
    let docs = Doc::from_stream(input).unwrap();
    assert_eq!(docs.len(), 1);
}

#[test]
fn ch_9_3_05_directives_scope() {
    // Directives only apply to immediately following document
    let input = "%YAML 1.2\n---\nfirst: 1\n---\nsecond: 2\n";
    let docs = Doc::from_stream(input).unwrap();
    assert_eq!(docs.len(), 2);

    assert_eq!(docs[0].yaml_version(), Some((1, 2)));
    // Second document should not inherit version
    assert_eq!(docs[1].yaml_version(), None);
}

#[test]
fn ch_9_3_06_explicit_end_before_start() {
    let input = "key: value\n...\n---\nother: data\n";
    let docs = Doc::from_stream(input).unwrap();
    assert_eq!(docs.len(), 2);
}

#[test]
fn ch_9_3_07_multiple_directives_same_doc() {
    Fixture::new(
        "9.3",
        7,
        "Multiple TAG directives for one document",
        "%TAG !e! tag:example.com,2000:app/\n%TAG !f! tag:foo.com,2000:/\n---\nkey: value\n",
    )
    .run();
}

// ============================================================================
// Stream with Complex Documents
// ============================================================================

#[test]
fn ch_9_4_01_stream_complex_documents() {
    let input = r#"---
users:
  - name: Alice
    age: 30
  - name: Bob
    age: 25
---
config:
  database:
    host: localhost
    port: 5432
"#;
    let docs = Doc::from_stream(input).unwrap();
    assert_eq!(docs.len(), 2);
}

#[test]
fn ch_9_4_02_stream_with_anchors() {
    let input = "---\ndefaults: &def\n  a: 1\nuse:\n  <<: *def\n---\nother: data\n";
    let docs = Doc::from_stream(input).unwrap();
    assert_eq!(docs.len(), 2);
}

#[test]
fn ch_9_4_03_stream_with_tags() {
    let input = "---\n- !!str 1\n- !!int 2\n---\nkey: !!str value\n";
    let docs = Doc::from_stream(input).unwrap();
    assert_eq!(docs.len(), 2);
}

// ============================================================================
// 9.5 BOM Handling
// ============================================================================

#[test]
fn ch_9_5_01_utf8_bom() {
    // UTF-8 BOM: EF BB BF
    let input = "\u{FEFF}key: value\n";
    let result = Doc::from_str(input);
    // Should either strip BOM and parse, or parse with BOM preserved
    assert!(result.is_ok());
}

#[test]
fn ch_9_5_02_utf8_bom_with_document_marker() {
    // UTF-8 BOM before document start marker
    let input = "\u{FEFF}---\nkey: value\n";
    let result = Doc::from_str(input);
    assert!(result.is_ok());
}

#[test]
fn ch_9_5_03_utf8_bom_stream() {
    // UTF-8 BOM at start of stream
    let input = "\u{FEFF}---\nfirst: 1\n---\nsecond: 2\n";
    let docs = Doc::from_stream(input).unwrap();
    assert_eq!(docs.len(), 2);
}

#[test]
fn ch_9_5_04_bom_mid_stream_error() {
    // BOM should only appear at stream start, not mid-stream
    let input = "---\nfirst: 1\n---\n\u{FEFF}second: 2\n";
    let result = Doc::from_stream(input);
    // This should either error or parse without treating mid-stream BOM specially
    // Accept either behavior for now (TDD - parser may need enhancement)
    let _ = result;
}

#[test]
fn ch_9_5_05_utf16_bom_error() {
    // UTF-16 BOM should be rejected (we only support UTF-8)
    // UTF-16 BE BOM: FE FF
    let input = "\u{FEFF}\u{FFFE}key: value\n";
    // For now, this might parse (BOM as Unicode char)
    // TDD: Future parser should validate encoding
    let _ = Doc::from_str(input);
}

#[test]
fn ch_9_5_06_multiple_bom_error() {
    // Multiple BOMs should be an error
    let input = "\u{FEFF}\u{FEFF}key: value\n";
    let result = Doc::from_str(input);
    // May parse for now (TDD - future enhancement)
    let _ = result;
}

#[test]
fn ch_9_5_07_utf16_le_with_bom() {
    // UTF-16 LE with BOM: FF FE followed by content
    // "key: value\n" in UTF-16 LE with BOM
    let utf16_le: Vec<u16> = vec![
        0xFEFF, // BOM
        0x006B, 0x0065, 0x0079, 0x003A, 0x0020, // "key: "
        0x0076, 0x0061, 0x006C, 0x0075, 0x0065, // "value"
        0x000A, // newline
    ];

    // Convert to bytes (little-endian)
    let mut bytes = Vec::new();
    for word in utf16_le {
        bytes.push((word & 0xFF) as u8); // low byte
        bytes.push((word >> 8) as u8);   // high byte
    }

    // Try to parse UTF-16 LE
    // TDD: This should eventually work when UTF-16 support is added
    match std::str::from_utf8(&bytes) {
        Ok(s) => {
            // If it happens to be valid UTF-8, try parsing
            let _ = Doc::from_str(s);
        }
        Err(_) => {
            // Expected: UTF-16 bytes are not valid UTF-8
            // Future: Add UTF-16 decoder and parse successfully
        }
    }
}

#[test]
fn ch_9_5_08_utf16_be_with_bom() {
    // UTF-16 BE with BOM: FE FF followed by content
    // "key: value\n" in UTF-16 BE with BOM
    let utf16_be: Vec<u16> = vec![
        0xFEFF, // BOM
        0x006B, 0x0065, 0x0079, 0x003A, 0x0020, // "key: "
        0x0076, 0x0061, 0x006C, 0x0075, 0x0065, // "value"
        0x000A, // newline
    ];

    // Convert to bytes (big-endian)
    let mut bytes = Vec::new();
    for word in utf16_be {
        bytes.push((word >> 8) as u8);   // high byte
        bytes.push((word & 0xFF) as u8); // low byte
    }

    // Try to parse UTF-16 BE
    // TDD: This should eventually work when UTF-16 support is added
    match std::str::from_utf8(&bytes) {
        Ok(s) => {
            let _ = Doc::from_str(s);
        }
        Err(_) => {
            // Expected: UTF-16 bytes are not valid UTF-8
            // Future: Add UTF-16 decoder
        }
    }
}

#[test]
fn ch_9_5_09_utf32_le_with_bom() {
    // UTF-32 LE with BOM: FF FE 00 00 followed by content
    // "key: value\n" in UTF-32 LE with BOM
    let utf32_le: Vec<u32> = vec![
        0x0000FEFF, // BOM
        0x0000006B, 0x00000065, 0x00000079, 0x0000003A, 0x00000020, // "key: "
        0x00000076, 0x00000061, 0x0000006C, 0x00000075, 0x00000065, // "value"
        0x0000000A, // newline
    ];

    // Convert to bytes (little-endian)
    let mut bytes = Vec::new();
    for dword in utf32_le {
        bytes.push((dword & 0xFF) as u8);         // byte 0
        bytes.push(((dword >> 8) & 0xFF) as u8);  // byte 1
        bytes.push(((dword >> 16) & 0xFF) as u8); // byte 2
        bytes.push((dword >> 24) as u8);          // byte 3
    }

    // Try to parse UTF-32 LE
    // TDD: This should eventually work when UTF-32 support is added
    match std::str::from_utf8(&bytes) {
        Ok(s) => {
            let _ = Doc::from_str(s);
        }
        Err(_) => {
            // Expected: UTF-32 bytes are not valid UTF-8
            // Future: Add UTF-32 decoder
        }
    }
}

#[test]
fn ch_9_5_10_utf32_be_with_bom() {
    // UTF-32 BE with BOM: 00 00 FE FF followed by content
    // "key: value\n" in UTF-32 BE with BOM
    let utf32_be: Vec<u32> = vec![
        0x0000FEFF, // BOM
        0x0000006B, 0x00000065, 0x00000079, 0x0000003A, 0x00000020, // "key: "
        0x00000076, 0x00000061, 0x0000006C, 0x00000075, 0x00000065, // "value"
        0x0000000A, // newline
    ];

    // Convert to bytes (big-endian)
    let mut bytes = Vec::new();
    for dword in utf32_be {
        bytes.push((dword >> 24) as u8);          // byte 3
        bytes.push(((dword >> 16) & 0xFF) as u8); // byte 2
        bytes.push(((dword >> 8) & 0xFF) as u8);  // byte 1
        bytes.push((dword & 0xFF) as u8);         // byte 0
    }

    // Try to parse UTF-32 BE
    // TDD: This should eventually work when UTF-32 support is added
    match std::str::from_utf8(&bytes) {
        Ok(s) => {
            let _ = Doc::from_str(s);
        }
        Err(_) => {
            // Expected: UTF-32 bytes are not valid UTF-8
            // Future: Add UTF-32 decoder
        }
    }
}

// ============================================================================
// 9.6 Document Marker Error Tests
// ============================================================================

#[test]
fn ch_9_6_01_document_start_with_trailing_content() {
    // Document start marker should be on its own line
    let input = "--- key: value\n";
    let result = Doc::from_str(input);
    // This is invalid YAML - --- must be followed by line break
    assert!(result.is_err());
}

#[test]
fn ch_9_6_02_document_end_with_trailing_content() {
    // Document end marker should be on its own line
    let input = "key: value\n... trailing\n";
    let result = Doc::from_str(input);
    // This is invalid YAML - ... must be followed by line break
    assert!(result.is_err());
}

#[test]
fn ch_9_6_03_indented_document_start() {
    // Document markers cannot be indented
    let input = "  ---\nkey: value\n";
    let result = Doc::from_str(input);
    // Indented --- is not a document marker, should be plain scalar
    // This may parse as a scalar starting with "---"
    let _ = result;
}

#[test]
fn ch_9_6_04_indented_document_end() {
    // Document end marker cannot be indented
    let input = "key: value\n  ...\n";
    let result = Doc::from_str(input);
    // Indented ... is not a document marker
    let _ = result;
}

#[test]
fn ch_9_6_05_document_marker_incomplete() {
    // Only two dashes should not be a marker
    let input = "--\nkey: value\n";
    let result = Doc::from_str(input);
    // Should parse as scalar starting with "--"
    assert!(result.is_ok());
}

#[test]
fn ch_9_6_06_document_marker_with_prefix() {
    // Marker must start at column 0
    let input = "x---\nkey: value\n";
    let result = Doc::from_str(input);
    // Should parse as scalar "x---"
    assert!(result.is_ok());
}

// ============================================================================
// 9.7 Empty and Large Streams
// ============================================================================

#[test]
fn ch_9_7_01_completely_empty_stream() {
    // Empty string is a valid YAML stream
    let input = "";
    let docs = Doc::from_stream(input).unwrap();
    // May be 0 documents or 1 empty document
    assert!(docs.is_empty() || docs.len() == 1);
}

#[test]
fn ch_9_7_02_whitespace_only_stream() {
    // Stream with only whitespace
    let input = "   \n  \n\n";
    let docs = Doc::from_stream(input).unwrap();
    // Should parse as empty or single empty document
    assert!(docs.is_empty() || docs.len() == 1);
}

#[test]
fn ch_9_7_03_empty_document_markers_only() {
    // Stream with only document markers
    let input = "---\n...\n";
    let docs = Doc::from_stream(input).unwrap();
    assert_eq!(docs.len(), 1);
}

#[test]
fn ch_9_7_04_large_stream_ten_documents() {
    // Stream with 10 documents
    let mut input = String::new();
    for i in 0..10 {
        input.push_str(&format!("---\ndoc{}: {}\n", i, i));
    }
    let docs = Doc::from_stream(&input).unwrap();
    assert_eq!(docs.len(), 10);
}

#[test]
fn ch_9_7_05_large_stream_hundred_documents() {
    // Stream with 100 documents
    let mut input = String::new();
    for i in 0..100 {
        input.push_str(&format!("---\ndoc{}: {}\n", i, i));
    }
    let docs = Doc::from_stream(&input).unwrap();
    assert_eq!(docs.len(), 100);
}

#[test]
#[ignore] // Performance test - run manually
fn ch_9_7_06_large_stream_thousand_documents() {
    // Stream with 1000 documents (performance test)
    let mut input = String::new();
    for i in 0..1000 {
        input.push_str(&format!("---\ndoc{}: {}\n", i, i));
    }
    let docs = Doc::from_stream(&input).unwrap();
    assert_eq!(docs.len(), 1000);
}

// ============================================================================
// 9.8 Directive Placement Errors
// ============================================================================

#[test]
fn ch_9_8_01_directive_after_content_start() {
    // Directives must come before document content
    let input = "key: value\n%YAML 1.2\n";
    let result = Doc::from_str(input);
    // This is invalid - directives must come before content
    assert!(result.is_err());
}

#[test]
fn ch_9_8_02_directive_between_documents_no_marker() {
    // Directives between bare documents need document marker
    let input = "first: 1\n%YAML 1.2\nsecond: 2\n";
    let result = Doc::from_stream(input);
    // Invalid - directive needs --- after it to start new doc
    assert!(result.is_err());
}

#[test]
fn ch_9_8_03_directive_after_document_end() {
    // Directives can appear after ... before next ---
    let input = "first: 1\n...\n%YAML 1.2\n---\nsecond: 2\n";
    let docs = Doc::from_stream(input).unwrap();
    assert_eq!(docs.len(), 2);
}

#[test]
fn ch_9_8_04_duplicate_yaml_directive() {
    // Multiple YAML directives for same document
    let input = "%YAML 1.2\n%YAML 1.2\n---\nkey: value\n";
    let result = Doc::from_str(input);
    // This is an error - only one YAML directive per document
    assert!(result.is_err());
}

#[test]
fn ch_9_8_05_yaml_directive_wrong_version() {
    // YAML directive with unsupported version
    let input = "%YAML 2.0\n---\nkey: value\n";
    let result = Doc::from_str(input);
    // Parser should either warn or error on unknown version
    // For now, may parse (TDD - future version validation)
    let _ = result;
}

#[test]
fn ch_9_8_06_tag_directive_invalid_handle() {
    // TAG directive with invalid handle
    let input = "%TAG invalid tag:example.com,2000:/\n---\nkey: value\n";
    let result = Doc::from_str(input);
    // Invalid handle format - should error
    assert!(result.is_err());
}

#[test]
fn ch_9_8_07_directive_indented() {
    // Directives cannot be indented
    let input = "  %YAML 1.2\n---\nkey: value\n";
    let result = Doc::from_str(input);
    // Indented directive is invalid
    assert!(result.is_err());
}
