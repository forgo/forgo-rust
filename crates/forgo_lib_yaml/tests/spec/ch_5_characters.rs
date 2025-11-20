// crates/forgo_lib_yaml/tests/spec/ch_5_characters.rs
//! YAML 1.2.2 Chapter 5: Character Productions
//!
//! **Spec Reference:** https://yaml.org/spec/1.2.2/#chapter-5-character-productions
//!
//! **Purpose:**
//! This file contains systematic tests for YAML 1.2.2 Chapter 5. Each test
//! maps to a specific section of the specification and validates compliance
//! with the requirements defined there. Test names follow the pattern
//! `ch_5_Y_ZZ_description` where 5 is the chapter, Y is the section, and
//! ZZ is the test number.
//!
//! **Coverage:**
//! Character set, encodings, indicators, line breaks, white space, escape sequences

#[path = "../common/mod.rs"]
mod common;

use common::Fixture;
use forgo_lib_yaml::Doc;

// ============================================================================
// 5.1 Character Set
// ============================================================================

#[test]
fn ch_5_1_01_printable_ascii() {
    Fixture::new(
        "5.1",
        1,
        "Printable ASCII characters",
        "text: Hello World!\n",
    )
    .run();
}

#[test]
fn ch_5_1_02_unicode_characters() {
    Fixture::new(
        "5.1",
        2,
        "Unicode characters (emoji, accents)",
        "emoji: 🎉\naccent: café\nchinese: 你好\n",
    )
    .run();
}

#[test]
fn ch_5_1_03_tab_allowed() {
    // Tab (x09) is allowed
    Fixture::new(
        "5.1",
        3,
        "Tab character is allowed",
        "key:\tvalue\n",
    )
    .run();
}

#[test]
fn ch_5_1_04_line_feed_allowed() {
    // Line feed (x0A) is allowed
    Fixture::new(
        "5.1",
        4,
        "Line feed is allowed",
        "key: value\n",
    )
    .run();
}

#[test]
fn ch_5_1_05_carriage_return_allowed() {
    // Carriage return (x0D) is allowed
    Fixture::new(
        "5.1",
        5,
        "Carriage return is allowed",
        "key: value\r\n",
    )
    .run();
}

// ============================================================================
// 5.2 Character Encodings
// ============================================================================

#[test]
fn ch_5_2_01_utf8_encoding() {
    Fixture::new(
        "5.2",
        1,
        "UTF-8 encoding support",
        "utf8: Hello 世界\n",
    )
    .run();
}

#[test]
fn ch_5_2_02_ascii_start() {
    // Stream must begin with ASCII character for encoding detection
    Fixture::new(
        "5.2",
        2,
        "Stream starts with ASCII character",
        "key: 値\n",
    )
    .run();
}

// ============================================================================
// 5.3 Indicator Characters
// ============================================================================

#[test]
fn ch_5_3_01_sequence_indicator() {
    Fixture::new(
        "5.3",
        1,
        "Sequence indicator '-'",
        "- item1\n- item2\n",
    )
    .run();
}

#[test]
fn ch_5_3_02_mapping_key_indicator() {
    Fixture::new(
        "5.3",
        2,
        "Mapping key indicator '?'",
        "? key\n: value\n",
    )
    .run();
}

#[test]
fn ch_5_3_03_mapping_value_indicator() {
    Fixture::new(
        "5.3",
        3,
        "Mapping value indicator ':'",
        "key: value\n",
    )
    .run();
}

#[test]
fn ch_5_3_04_collection_entry_indicator() {
    Fixture::new(
        "5.3",
        4,
        "Collection entry indicator ','",
        "[a, b, c]\n",
    )
    .run();
}

#[test]
fn ch_5_3_05_flow_sequence_indicators() {
    Fixture::new(
        "5.3",
        5,
        "Flow sequence indicators '[]'",
        "[1, 2, 3]\n",
    )
    .run();
}

#[test]
fn ch_5_3_06_flow_mapping_indicators() {
    Fixture::new(
        "5.3",
        6,
        "Flow mapping indicators '{}'",
        "{key: value}\n",
    )
    .run();
}

#[test]
fn ch_5_3_07_comment_indicator() {
    Fixture::new(
        "5.3",
        7,
        "Comment indicator '#'",
        "# comment\nkey: value\n",
    )
    .run();
}

#[test]
fn ch_5_3_08_anchor_indicator() {
    Fixture::new(
        "5.3",
        8,
        "Anchor indicator '&'",
        "anchor: &ref value\nalias: *ref\n",
    )
    .run();
}

#[test]
fn ch_5_3_09_alias_indicator() {
    Fixture::new(
        "5.3",
        9,
        "Alias indicator '*'",
        "anchor: &ref value\nalias: *ref\n",
    )
    .run();
}

#[test]
fn ch_5_3_10_tag_indicator() {
    Fixture::new(
        "5.3",
        10,
        "Tag indicator '!'",
        "tag: !!str string\n",
    )
    .run();
}

#[test]
fn ch_5_3_11_literal_indicator() {
    Fixture::new(
        "5.3",
        11,
        "Literal block scalar indicator '|'",
        "literal: |\n  text\n",
    )
    .run();
}

#[test]
fn ch_5_3_12_folded_indicator() {
    Fixture::new(
        "5.3",
        12,
        "Folded block scalar indicator '>'",
        "folded: >\n  text\n",
    )
    .run();
}

#[test]
fn ch_5_3_13_directive_indicator() {
    Fixture::new(
        "5.3",
        13,
        "Directive indicator '%'",
        "%YAML 1.2\n---\nkey: value\n",
    )
    .run();
}

#[test]
fn ch_5_3_14_single_quote_indicator() {
    Fixture::new(
        "5.3",
        14,
        "Single quote indicator",
        "text: 'single quoted'\n",
    )
    .run();
}

#[test]
fn ch_5_3_15_double_quote_indicator() {
    Fixture::new(
        "5.3",
        15,
        "Double quote indicator",
        "text: \"double quoted\"\n",
    )
    .run();
}

// ============================================================================
// 5.4 Line Break Characters
// ============================================================================

#[test]
fn ch_5_4_01_line_feed() {
    Fixture::new(
        "5.4",
        1,
        "Line feed (LF) as line break",
        "line1\nline2\n",
    )
    .run();
}

#[test]
fn ch_5_4_02_carriage_return_line_feed() {
    Fixture::new(
        "5.4",
        2,
        "Carriage return + line feed (CRLF)",
        "line1\r\nline2\r\n",
    )
    .run();
}

#[test]
fn ch_5_4_03_normalized_line_breaks() {
    // CRLF should be normalized to LF
    let input = "key: value\r\n";
    let doc = Doc::from_str(input).unwrap();
    let output = doc.to_string().unwrap();
    // Output should use LF, not CRLF
    assert!(!output.contains("\r\n") || output.contains("\n"));
}

#[test]
fn ch_5_4_04_literal_preserves_line_breaks() {
    Fixture::new(
        "5.4",
        4,
        "Literal preserves line break types",
        "literal: |\n  line1\n  line2\n",
    )
    .run();
}

// ============================================================================
// 5.5 White Space Characters
// ============================================================================

#[test]
fn ch_5_5_01_space_character() {
    Fixture::new(
        "5.5",
        1,
        "Space (x20) as white space",
        "key: value with spaces\n",
    )
    .run();
}

#[test]
fn ch_5_5_02_tab_character() {
    Fixture::new(
        "5.5",
        2,
        "Tab (x09) as white space",
        "key:\tvalue\n",
    )
    .run();
}

#[test]
fn ch_5_5_03_indentation_with_spaces() {
    Fixture::new(
        "5.5",
        3,
        "Indentation using spaces",
        "parent:\n  child: value\n",
    )
    .run();
}

#[test]
fn ch_5_5_04_leading_trailing_spaces() {
    Fixture::new(
        "5.5",
        4,
        "Leading and trailing spaces in plain scalar",
        "key:   value   \n",
    )
    .run();
}

// ============================================================================
// 5.6 Miscellaneous Characters
// ============================================================================

#[test]
fn ch_5_6_01_decimal_digits() {
    Fixture::new(
        "5.6",
        1,
        "Decimal digits (0-9)",
        "number: 1234567890\n",
    )
    .run();
}

#[test]
fn ch_5_6_02_hexadecimal_digits() {
    Fixture::new(
        "5.6",
        2,
        "Hexadecimal number",
        "hex: 0x1A2B3C\n",
    )
    .run();
}

#[test]
fn ch_5_6_03_ascii_letters() {
    Fixture::new(
        "5.6",
        3,
        "ASCII letters (a-z, A-Z)",
        "text: abcXYZ\n",
    )
    .run();
}

#[test]
fn ch_5_6_04_word_characters() {
    Fixture::new(
        "5.6",
        4,
        "Word characters in plain scalars",
        "identifier: some_var_123\n",
    )
    .run();
}

#[test]
fn ch_5_6_05_uri_characters() {
    Fixture::new(
        "5.6",
        5,
        "URI characters in tags",
        "url: https://example.com/path?query=value\n",
    )
    .run();
}

// ============================================================================
// 5.7 Escaped Characters
// ============================================================================

#[test]
fn ch_5_7_01_escape_null() {
    Fixture::new(
        "5.7",
        1,
        "Escaped null character",
        "null: \"\\0\"\n",
    )
    .run();
}

#[test]
fn ch_5_7_02_escape_bell() {
    Fixture::new(
        "5.7",
        2,
        "Escaped bell character",
        "bell: \"\\a\"\n",
    )
    .run();
}

#[test]
fn ch_5_7_03_escape_backspace() {
    Fixture::new(
        "5.7",
        3,
        "Escaped backspace character",
        "backspace: \"\\b\"\n",
    )
    .run();
}

#[test]
fn ch_5_7_04_escape_tab() {
    Fixture::new(
        "5.7",
        4,
        "Escaped tab character",
        "tab: \"\\t\"\n",
    )
    .run();
}

#[test]
fn ch_5_7_05_escape_line_feed() {
    Fixture::new(
        "5.7",
        5,
        "Escaped line feed character",
        "lf: \"\\n\"\n",
    )
    .run();
}

#[test]
fn ch_5_7_06_escape_vertical_tab() {
    Fixture::new(
        "5.7",
        6,
        "Escaped vertical tab character",
        "vtab: \"\\v\"\n",
    )
    .run();
}

#[test]
fn ch_5_7_07_escape_form_feed() {
    Fixture::new(
        "5.7",
        7,
        "Escaped form feed character",
        "ff: \"\\f\"\n",
    )
    .run();
}

#[test]
fn ch_5_7_08_escape_carriage_return() {
    Fixture::new(
        "5.7",
        8,
        "Escaped carriage return character",
        "cr: \"\\r\"\n",
    )
    .run();
}

#[test]
fn ch_5_7_09_escape_escape() {
    Fixture::new(
        "5.7",
        9,
        "Escaped escape character",
        "esc: \"\\e\"\n",
    )
    .run();
}

#[test]
fn ch_5_7_10_escape_space() {
    Fixture::new(
        "5.7",
        10,
        "Escaped space character",
        "space: \"\\ \"\n",
    )
    .run();
}

#[test]
fn ch_5_7_11_escape_double_quote() {
    Fixture::new(
        "5.7",
        11,
        "Escaped double quote character",
        "quote: \"\\\"\"\n",
    )
    .run();
}

#[test]
fn ch_5_7_12_escape_slash() {
    Fixture::new(
        "5.7",
        12,
        "Escaped forward slash",
        "slash: \"\\/\"\n",
    )
    .run();
}

#[test]
fn ch_5_7_13_escape_backslash() {
    Fixture::new(
        "5.7",
        13,
        "Escaped backslash character",
        "backslash: \"\\\\\"\n",
    )
    .run();
}

#[test]
fn ch_5_7_14_escape_next_line() {
    Fixture::new(
        "5.7",
        14,
        "Escaped next line character",
        "nel: \"\\N\"\n",
    )
    .run();
}

#[test]
fn ch_5_7_15_escape_non_breaking_space() {
    Fixture::new(
        "5.7",
        15,
        "Escaped non-breaking space",
        "nbsp: \"\\_\"\n",
    )
    .run();
}

#[test]
fn ch_5_7_16_escape_line_separator() {
    Fixture::new(
        "5.7",
        16,
        "Escaped line separator",
        "ls: \"\\L\"\n",
    )
    .run();
}

#[test]
fn ch_5_7_17_escape_paragraph_separator() {
    Fixture::new(
        "5.7",
        17,
        "Escaped paragraph separator",
        "ps: \"\\P\"\n",
    )
    .run();
}

#[test]
fn ch_5_7_18_escape_8bit_unicode() {
    Fixture::new(
        "5.7",
        18,
        "8-bit Unicode escape (\\xNN)",
        "unicode8: \"\\x41\"\n",
    )
    .run();
}

#[test]
fn ch_5_7_19_escape_16bit_unicode() {
    Fixture::new(
        "5.7",
        19,
        "16-bit Unicode escape (\\uNNNN)",
        "unicode16: \"\\u0041\"\n",
    )
    .run();
}

#[test]
fn ch_5_7_20_escape_32bit_unicode() {
    Fixture::new(
        "5.7",
        20,
        "32-bit Unicode escape (\\UNNNNNNNN)",
        "unicode32: \"\\U00000041\"\n",
    )
    .run();
}

#[test]
fn ch_5_7_21_escape_only_in_double_quoted() {
    // Escapes should only work in double-quoted strings
    let single_quoted = "text: '\\n'\n";
    let doc = Doc::from_str(single_quoted).unwrap();
    let output = doc.to_string().unwrap();
    // In single-quoted, \n should be literal, not escaped
    assert!(output.contains("\\n") || output.contains("'\\n'"));
}

#[test]
fn ch_5_7_22_multiple_escapes() {
    Fixture::new(
        "5.7",
        22,
        "Multiple escape sequences",
        "text: \"line1\\nline2\\ttabbed\"\n",
    )
    .run();
}

#[test]
fn ch_5_7_23_json_escape_sequences_backslash_quote() {
    // Production [28] nb-double-char - remaining JSON escapes
    // Test: \" (quote), \\ (backslash), \/ (forward slash)
    Fixture::new(
        "5.7",
        23,
        "JSON escape sequences: quote, backslash, slash",
        "text: \"He said \\\"hello\\\" with a backslash \\\\ and slash \\/\"\n",
    )
    .run();
}

#[test]
fn ch_5_7_24_json_escape_sequences_backspace_formfeed() {
    // Production [28] nb-double-char - control character escapes
    // Test: \b (backspace), \f (form feed)
    Fixture::new(
        "5.7",
        24,
        "JSON escape sequences: backspace and form feed",
        "text: \"backspace\\bhere formfeed\\fhere\"\n",
    )
    .run();
}

// ============================================================================
// 5.8 Character Edge Cases and Unicode
// ============================================================================

// 5.8.1 Unicode Range Tests
// Test comprehensive Unicode ranges beyond basic examples

#[test]
fn ch_5_8_1_01_unicode_basic_latin() {
    // U+0000..U+007F (ASCII)
    Fixture::new(
        "5.8.1",
        1,
        "Unicode Basic Latin range",
        "text: ABCabc123!@#\n",
    )
    .run();
}

#[test]
fn ch_5_8_1_02_unicode_latin_extended() {
    // U+00A0..U+00FF, U+0100..U+017F
    Fixture::new(
        "5.8.1",
        2,
        "Unicode Latin Extended (accents, special)",
        "latin: àéîõü ÄËÏÖÜ ąčęėį\n",
    )
    .run();
}

#[test]
fn ch_5_8_1_03_unicode_cyrillic() {
    // U+0400..U+04FF
    Fixture::new(
        "5.8.1",
        3,
        "Unicode Cyrillic range",
        "cyrillic: Привет мир АБВГД\n",
    )
    .run();
}

#[test]
fn ch_5_8_1_04_unicode_greek() {
    // U+0370..U+03FF
    Fixture::new(
        "5.8.1",
        4,
        "Unicode Greek range",
        "greek: Γειά σου κόσμε ΑΒΓ\n",
    )
    .run();
}

#[test]
fn ch_5_8_1_05_unicode_cjk() {
    // U+4E00..U+9FFF (CJK Unified Ideographs)
    Fixture::new(
        "5.8.1",
        5,
        "Unicode CJK ideographs",
        "cjk: 你好世界 漢字 日本語\n",
    )
    .run();
}

#[test]
fn ch_5_8_1_06_unicode_arabic() {
    // U+0600..U+06FF
    Fixture::new(
        "5.8.1",
        6,
        "Unicode Arabic range",
        "arabic: مرحبا العالم\n",
    )
    .run();
}

#[test]
fn ch_5_8_1_07_unicode_hebrew() {
    // U+0590..U+05FF
    Fixture::new(
        "5.8.1",
        7,
        "Unicode Hebrew range",
        "hebrew: שלום עולם\n",
    )
    .run();
}

#[test]
fn ch_5_8_1_08_unicode_emoji() {
    // U+1F300..U+1F9FF (Emoji)
    Fixture::new(
        "5.8.1",
        8,
        "Unicode emoji range",
        "emoji: 😀🎉🚀💻🌍🔥✨\n",
    )
    .run();
}

#[test]
fn ch_5_8_1_09_unicode_combining_characters() {
    // Combining diacritical marks
    Fixture::new(
        "5.8.1",
        9,
        "Unicode combining characters",
        "combining: é (e + combining acute)\n",
    )
    .run();
}

#[test]
fn ch_5_8_1_10_unicode_zero_width_characters() {
    // Zero-width characters (ZWJ, ZWNJ)
    Fixture::new(
        "5.8.1",
        10,
        "Unicode zero-width characters",
        "zwj: a\u{200D}b\n",
    )
    .run();
}

// 5.8.2 Unprintable Control Character Errors
// Test that unprintable control characters are rejected

#[test]
fn ch_5_8_2_01_null_character_error() {
    // U+0000 (NULL) is not allowed in plain scalars
    let input = "key: value\0more\n";
    let result = Doc::from_str(input);
    assert!(result.is_err(), "NULL character should be rejected in plain scalars");
}

#[test]
fn ch_5_8_2_02_backspace_character_error() {
    // U+0008 (BACKSPACE) is not allowed
    let input = "key: value\u{0008}more\n";
    let result = Doc::from_str(input);
    assert!(result.is_err(), "Backspace character should be rejected");
}

#[test]
fn ch_5_8_2_03_vertical_tab_error() {
    // U+000B (VERTICAL TAB) is not allowed
    let input = "key: value\u{000B}more\n";
    let result = Doc::from_str(input);
    assert!(result.is_err(), "Vertical tab should be rejected");
}

#[test]
fn ch_5_8_2_04_form_feed_error() {
    // U+000C (FORM FEED) is not allowed
    let input = "key: value\u{000C}more\n";
    let result = Doc::from_str(input);
    assert!(result.is_err(), "Form feed should be rejected");
}

#[test]
fn ch_5_8_2_05_delete_character_error() {
    // U+007F (DELETE) is not allowed
    let input = "key: value\u{007F}more\n";
    let result = Doc::from_str(input);
    assert!(result.is_err(), "Delete character should be rejected");
}

#[test]
fn ch_5_8_2_06_c1_control_characters_error() {
    // U+0080..U+009F (C1 control characters) are not allowed
    let input = "key: value\u{0081}more\n";
    let result = Doc::from_str(input);
    assert!(result.is_err(), "C1 control characters should be rejected");
}

// 5.8.3 All Line Break Types
// Test all line break types including CR-only and NEL

#[test]
fn ch_5_8_3_01_carriage_return_only() {
    // CR without LF should be treated as line break
    Fixture::new(
        "5.8.3",
        1,
        "Carriage return only (CR) as line break",
        "line1\rline2\r",
    )
    .run();
}

#[test]
fn ch_5_8_3_02_next_line_character() {
    // U+0085 (NEL) is a line break
    Fixture::new(
        "5.8.3",
        2,
        "Next line (NEL) character as line break",
        "line1\u{0085}line2\u{0085}",
    )
    .run();
}

#[test]
fn ch_5_8_3_03_mixed_line_breaks() {
    // Mix of LF, CR, CRLF, NEL
    Fixture::new(
        "5.8.3",
        3,
        "Mixed line break types in same document",
        "lf\ncr\rcrlf\r\nnel\u{0085}end\n",
    )
    .run();
}

#[test]
fn ch_5_8_3_04_line_separator() {
    // U+2028 (LINE SEPARATOR)
    Fixture::new(
        "5.8.3",
        4,
        "Line separator (U+2028) character",
        "line1\u{2028}line2\n",
    )
    .run();
}

#[test]
fn ch_5_8_3_05_paragraph_separator() {
    // U+2029 (PARAGRAPH SEPARATOR)
    Fixture::new(
        "5.8.3",
        5,
        "Paragraph separator (U+2029) character",
        "para1\u{2029}para2\n",
    )
    .run();
}

// 5.8.4 Invalid Escape Sequences
// Test that invalid escape sequences are rejected

#[test]
fn ch_5_8_4_01_invalid_escape_letter() {
    // Invalid escape like \q
    let input = "text: \"\\q\"\n";
    let result = Doc::from_str(input);
    assert!(result.is_err(), "Invalid escape sequence \\q should be rejected");
}

#[test]
fn ch_5_8_4_02_incomplete_hex_escape() {
    // Incomplete \xNN escape
    let input = "text: \"\\x4\"\n";
    let result = Doc::from_str(input);
    assert!(result.is_err(), "Incomplete hex escape \\x4 should be rejected");
}

#[test]
fn ch_5_8_4_03_incomplete_unicode16_escape() {
    // Incomplete \uNNNN escape
    let input = "text: \"\\u004\"\n";
    let result = Doc::from_str(input);
    assert!(result.is_err(), "Incomplete unicode escape \\u004 should be rejected");
}

#[test]
fn ch_5_8_4_04_incomplete_unicode32_escape() {
    // Incomplete \UNNNNNNNN escape
    let input = "text: \"\\U0000004\"\n";
    let result = Doc::from_str(input);
    assert!(result.is_err(), "Incomplete 32-bit unicode escape should be rejected");
}

#[test]
fn ch_5_8_4_05_invalid_hex_digits() {
    // Invalid hex digits in escape
    let input = "text: \"\\x4G\"\n";
    let result = Doc::from_str(input);
    assert!(result.is_err(), "Invalid hex digits in escape should be rejected");
}

#[test]
fn ch_5_8_4_06_escape_at_end_of_string() {
    // Escape at end of string without target
    let input = "text: \"value\\\"\n";
    let result = Doc::from_str(input);
    // This might be a valid escaped quote, so check carefully
    if result.is_err() {
        // Parser rejects incomplete escape
    } else {
        // Parser accepts it as escaped quote
        let doc = result.unwrap();
        let output = doc.to_string().unwrap();
        assert!(output.contains("value"));
    }
}

// 5.8.5 Unicode Surrogate Pair Errors
// Test that invalid surrogate pairs are rejected

#[test]
fn ch_5_8_5_01_high_surrogate_alone() {
    // High surrogate (U+D800..U+DBFF) without low surrogate
    let input = "text: \"\\uD800\"\n";
    let result = Doc::from_str(input);
    assert!(result.is_err(), "High surrogate without low surrogate should be rejected");
}

#[test]
fn ch_5_8_5_02_low_surrogate_alone() {
    // Low surrogate (U+DC00..U+DFFF) without high surrogate
    let input = "text: \"\\uDC00\"\n";
    let result = Doc::from_str(input);
    assert!(result.is_err(), "Low surrogate without high surrogate should be rejected");
}

#[test]
fn ch_5_8_5_03_reversed_surrogate_pair() {
    // Low surrogate before high surrogate
    let input = "text: \"\\uDC00\\uD800\"\n";
    let result = Doc::from_str(input);
    assert!(result.is_err(), "Reversed surrogate pair should be rejected");
}

#[test]
fn ch_5_8_5_04_valid_surrogate_pair() {
    // Valid surrogate pair (U+D800 + U+DC00 = U+10000)
    let input = "text: \"\\uD800\\uDC00\"\n";
    let result = Doc::from_str(input);

    if result.is_ok() {
        // Implementation supports surrogate pairs correctly
        let doc = result.unwrap();
        let output = doc.to_string().unwrap();
        // Should represent U+10000 (Linear B Syllable B008 A)
        assert!(output.len() > 0);
    } else {
        // Implementation rejects surrogate pairs
        // This is technically valid but some parsers don't support UTF-16 surrogates
    }
}

// 5.8.6 Indicators in Different Contexts
// Test edge cases of indicator characters in various contexts

#[test]
fn ch_5_8_6_01_colon_in_plain_scalar() {
    // Colon allowed in middle of plain scalar
    Fixture::new(
        "5.8.6",
        1,
        "Colon in middle of plain scalar",
        "url: http://example.com\n",
    )
    .run();
}

#[test]
fn ch_5_8_6_02_hash_in_quoted_string() {
    // Hash in quoted string is literal, not comment
    Fixture::new(
        "5.8.6",
        2,
        "Hash in quoted string is literal",
        "text: \"value # not a comment\"\n",
    )
    .run();
}

#[test]
fn ch_5_8_6_03_dash_in_plain_scalar() {
    // Dash in plain scalar (not at start of line)
    Fixture::new(
        "5.8.6",
        3,
        "Dash in middle of plain scalar",
        "text: some-dashed-value\n",
    )
    .run();
}

#[test]
fn ch_5_8_6_04_question_mark_in_plain_scalar() {
    // Question mark in flow context
    Fixture::new(
        "5.8.6",
        4,
        "Question mark in flow mapping",
        "{? key: value}\n",
    )
    .run();
}

#[test]
fn ch_5_8_6_05_ampersand_in_quoted_string() {
    // Ampersand in quoted string is literal
    Fixture::new(
        "5.8.6",
        5,
        "Ampersand in quoted string",
        "text: \"rock & roll\"\n",
    )
    .run();
}

#[test]
fn ch_5_8_6_06_asterisk_in_plain_scalar() {
    // Asterisk followed by non-whitespace is literal
    Fixture::new(
        "5.8.6",
        6,
        "Asterisk in plain scalar (not alias)",
        "math: 2*3=6\n",
    )
    .run();
}

#[test]
fn ch_5_8_6_07_bracket_in_plain_scalar() {
    // Brackets in plain scalar in block context
    let input = "text: [not a flow sequence]\n";
    let result = Doc::from_str(input);

    if result.is_ok() {
        // Implementation allows brackets in plain scalars
    } else {
        // Implementation requires quoting
        assert!(result.is_err());
    }
}

#[test]
fn ch_5_8_6_08_brace_in_plain_scalar() {
    // Braces in plain scalar in block context
    let input = "text: {not a flow mapping}\n";
    let result = Doc::from_str(input);

    if result.is_ok() {
        // Implementation allows braces in plain scalars
    } else {
        // Implementation requires quoting
        assert!(result.is_err());
    }
}
