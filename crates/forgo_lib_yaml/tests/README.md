# YAML 1.2.2 Test Suite

This directory contains comprehensive tests for `forgo_lib_yaml` organized into two categories:

1. **Spec-Aligned Tests** - YAML 1.2.2 specification compliance
2. **Implementation Tests** - Parser, emitter, editor, and lexer specifics

**Reference:** https://yaml.org/spec/1.2.2/

---

## 📊 Current Status (Updated 2025-11-16)

**YAML 1.2.2 Specification Compliance: 90.5%** (607 passing / 671 total)

| Metric | Count | Status |
|--------|-------|--------|
| Total Spec Tests | 671 | 607 ✅ / 63 ❌ / 1 ⏭️ |
| Implementation Tests | ~9 | 3 ✅ / 6 ❌ |
| **Overall Compliance** | **90.5%** | 🟨 Good progress, work needed |

**Strengths:**
- ✅ Chapter 6 (Structural) - 100% passing
- ✅ Chapter 10 (Schemas) - 100% passing
- ⚡ Chapter 4 (Syntax) - 98%+ passing

**Areas Needing Work:**
- 🔴 Block scalars (11 failures) - Core feature requiring fixes
- 🔴 Parser error detection (26 failures) - Should reject invalid input
- 🟠 Document streams (16 failures) - Multi-doc support gaps
- 🟠 Tag system (5 failures) - Tag preservation issues

See [Known Failures](#known-failures-63-spec-tests--6-implementation-tests) for detailed breakdown.

---

## Quick Start

```bash
# Run all tests
cargo test

# Run only spec compliance tests
cargo test --test yaml_spec_1_2_2

# Run specific chapter
cargo test --test yaml_spec_1_2_2 ch_7

# Run implementation tests
cargo test editor
cargo test emitter
cargo test parser
```

## Test Organization

### Directory Structure

```
tests/
├── yaml_spec_1_2_2.rs          # Main spec test suite
├── common/mod.rs               # Test infrastructure (Fixture pattern)
├── spec/
│   ├── ch_2_overview.rs        # Chapter 2: Language Overview (31 tests)
│   ├── ch_3_processes.rs       # Chapter 3: Processes and Models (31 tests)
│   ├── ch_4_syntax.rs          # Chapter 4: Syntax Conventions (35 tests)
│   ├── ch_5_characters.rs      # Chapter 5: Character Productions (57 tests)
│   ├── ch_6_structural.rs      # Chapter 6: Structural Productions (30 tests)
│   ├── ch_7_flow_styles.rs     # Chapter 7: Flow Style Productions (57 tests)
│   ├── ch_8_block_styles.rs    # Chapter 8: Block Style Productions (34 tests)
│   ├── ch_9_documents.rs       # Chapter 9: Document Streams (30 tests)
│   └── ch_10_schemas.rs        # Chapter 10: Recommended Schemas (26 tests)
└── [implementation tests]      # Editor, emitter, parser, lexer tests
```

### Test Naming Convention

All spec tests follow the pattern: **`ch_X_Y_ZZ_description`**

- **X**: Chapter number (6, 7, 8, 9, 10)
- **Y**: Section within chapter (1, 2, 3, ...)
- **ZZ**: Test case number (01, 02, 03, ...)
- **description**: Snake_case description

**Examples:**

```rust
ch_7_3_1_01_double_quoted_basic()        // Chapter 7, Section 3.1, Test 01
ch_8_1_1_1_02_indent_indicator()         // Chapter 8, Section 1.1.1, Test 02
ch_10_2_04_json_integer()                // Chapter 10, Section 2, Test 04
```

## Spec-Aligned Tests

### Chapter-Based Test Files

Each test file corresponds directly to a chapter in the YAML 1.2.2 specification:

| File                   | Chapter | Description                 | Tests    |
| ---------------------- | ------- | --------------------------- | -------- |
| `ch_2_overview.rs`     | 2       | Language Overview           | 63 tests |
| `ch_3_processes.rs`    | 3       | Processes and Models        | 79 tests |
| `ch_4_syntax.rs`       | 4       | Syntax Conventions          | 62 tests |
| `ch_5_characters.rs`   | 5       | Character Productions       | 96 tests |
| `ch_6_structural.rs`   | 6       | Structural Productions      | 70 tests |
| `ch_7_flow_styles.rs`  | 7       | Flow Style Productions      | 67 tests |
| `ch_8_block_styles.rs` | 8       | Block Style Productions     | 78 tests |
| `ch_9_documents.rs`    | 9       | Document Stream Productions | 60 tests |
| `ch_10_schemas.rs`     | 10      | Recommended Schemas         | 86 tests |

**Total:** 659 spec-aligned tests (595 passing, 63 failing + 1 ignored = **90.3% compliance**)

### Chapter 1: Introduction to YAML

**Reference:** https://yaml.org/spec/1.2.2/#chapter-1-introduction-to-yaml

Chapter 1 is introductory material (no testable requirements).

### Chapter 2: Language Overview

**Reference:** https://yaml.org/spec/1.2.2/#chapter-2-language-overview

Tests based on spec examples demonstrating YAML features:

- **2.1 Collections** - Sequences, mappings, and nested structures
- **2.2 Structures** - Document streams, comments, anchors, aliases
- **2.3 Scalars** - Literal, folded, quoted, and multi-line scalars
- **2.4 Tags** - Type tags, explicit tags, application-specific tags
- **2.5 Full Length Example** - Invoice document with complex structure
- **2.6 Additional Features** - Mixed styles, deep nesting, complex keys
- **2.7 Edge Cases** - Empty collections, Unicode keys, deep nesting (10 levels), document markers, indicators in strings, flow spacing variations, comment placement
- **2.8 Error Conditions** - Reserved indicators (@, `), trailing commas, unmatched delimiters, unterminated quotes

### Chapter 3: Processes and Models

**Reference:** https://yaml.org/spec/1.2.2/#chapter-3-processes-and-models

Tests for the three-stage YAML processing model:

- **3.1 Processes** - Load (Parse → Compose → Construct) and Dump (Represent → Serialize → Present)
- **3.2 Information Models** - Representation graph, serialization tree, presentation stream
  - 3.2.1 Representation Graph (nodes, tags, content)
  - 3.2.2 Serialization Tree (anchor uniqueness, scope, circular references)
  - 3.2.3 Presentation Stream (directives, BOM handling, encoding)
- **3.3 Loading Failure Points** - Ill-formed input, undefined aliases, duplicate keys, tabs in indentation, invalid UTF-8
- **3.4 Node Comparison & Tag Resolution** - Equality, identity, tag resolution priority
- **3.5 Round-Trip Preservation** - Content, structure, aliases, and complex graph preservation
- **3.6 Presentation Independence** - Whitespace, comments, and style independence
- **3.7 Edge Cases & Exotic Scenarios** - Comprehensive edge case coverage:
  - 3.7.1 Anchor/Alias Edge Cases (orphaned anchors, multiple identical anchors, aliases to empty/null)
  - 3.7.2 Duplicate Key Detection (canonical comparison, numeric formats, semantic equality)
  - 3.7.3 Tag Resolution Edge Cases (non-specific tags, sibling independence)
  - 3.7.4 Multi-Document Streams (per-document directives, BOM in streams, encoding consistency)
  - 3.7.5 Comment Placement (comments vs scalars, interleaved comments, directive comments)
  - 3.7.6 Partial Representations (unresolved tags)
  - 3.7.7 Node Equality (tag matching requirements, content matching)
  - 3.7.8 Construction Constraints (comment independence, whitespace independence, key order independence)
  - 3.7.9 Encoding Edge Cases (UTF-8 BOM variations, BOM in quoted scalars, ASCII fallback)
  - 3.7.10 Cyclic Reference Equality (self-referential nodes, mutual references, duplicate detection limits)

### Chapter 4: Syntax Conventions

**Reference:** https://yaml.org/spec/1.2.2/#chapter-4-syntax-conventions

Tests for BNF notation edge cases and formal syntax conventions:

- **4.1 Production Syntax** - Character ranges, concatenation, alternation
- **4.2 Quantification** - Optional (`?`), zero-or-more (`*`), one-or-more (`+`), greedy matching
- **4.3 Special Productions** - `<start-of-line>`, `<end-of-input>`, `<empty>`
- **4.4 Lookaround Assertions** - Lookahead and lookbehind in productions
- **4.5 Parameterized Productions** - Indentation parameters (n, n+1, n=-1)
- **4.6 Context Parameters** - BLOCK-IN, BLOCK-OUT, BLOCK-KEY, FLOW-IN, FLOW-OUT, FLOW-KEY
- **4.7 Operator Precedence** - Parenthesization, quantification, concatenation, alternation
- **4.8 Edge Cases** - Complex nested structures mixing contexts
- **4.9 Indentation Edge Cases** - Maximum depth (20 levels), inconsistent indentation, mixed sizes, single-space, large jumps
- **4.10 Context Transitions** - Block↔Flow transitions, nested contexts, rapid switching, document boundaries
- **4.11 Production Limits** - Very long lines/keys (1000+ chars), large collections (100+ items), deep nesting (10+ levels)
- **4.12 Quantification Edge Cases** - Boundary conditions, zero occurrences, many occurrences
- **4.13 Special Characters** - Printable ASCII range, Unicode, combining characters

### Chapter 5: Character Productions

**Reference:** https://yaml.org/spec/1.2.2/#chapter-5-character-productions

Tests for character-level requirements:

- **5.1 Character Set** - Printable Unicode subset, allowed/disallowed characters
- **5.2 Character Encodings** - UTF-8, UTF-16, UTF-32 support and BOM detection
- **5.3 Indicator Characters** - Special YAML characters (`-`, `?`, `:`, `#`, `&`, `*`, `!`, `|`, `>`, etc.)
- **5.4 Line Break Characters** - LF, CR, CRLF normalization
- **5.5 White Space Characters** - Space and tab handling
- **5.6 Miscellaneous Characters** - Decimal/hex digits, letters, word characters, URI characters
- **5.7 Escaped Characters** - Escape sequences in double-quoted scalars (`\n`, `\t`, `\uNNNN`, etc.)
- **5.8 Character Edge Cases & Unicode** - Comprehensive edge case coverage:
  - 5.8.1 Unicode Range Tests (Basic Latin, Latin Extended, Cyrillic, Greek, CJK, Arabic, Hebrew, Emoji, combining characters, zero-width)
  - 5.8.2 Unprintable Control Character Errors (NULL, backspace, vertical tab, form feed, delete, C1 controls)
  - 5.8.3 All Line Break Types (CR-only, NEL, mixed breaks, line separator, paragraph separator)
  - 5.8.4 Invalid Escape Sequences (invalid letters, incomplete hex/unicode escapes, invalid hex digits)
  - 5.8.5 Unicode Surrogate Pair Errors (high/low surrogates alone, reversed pairs, valid pairs)
  - 5.8.6 Indicators in Different Contexts (colons in URLs, hashes in quotes, dashes/asterisks/brackets/braces in plain scalars)

### Chapter 6: Structural Productions

**Reference:** https://yaml.org/spec/1.2.2/#chapter-6-structural-productions

Tests for fundamental YAML structure:

- **6.1 Indentation Spaces** - s-indent(n), s-indent-less-than(n), exact/relative indentation rules
- **6.2 Separation Spaces** - s-separate-in-line, required spaces after colons/dashes, flow vs block separation
- **6.3 Line Prefixes** - s-line-prefix, s-block-line-prefix, s-flow-line-prefix in different contexts
- **6.4 Empty Lines** - l-empty(n,c), empty lines between entries, whitespace-only lines
- **6.5 Line Folding** - b-l-folded, b-as-space, folded scalars (>), plain scalar folding, chomping indicators
- **6.6 Comments** - Leading, trailing, and inline comments
- **6.7 Separation Lines** - s-separate-lines, blank line separation, comment-based separation
- **6.8 Directives** - YAML and TAG directives
  - 6.8.1 "YAML" Directives (`%YAML`)
  - 6.8.2 "TAG" Directives (`%TAG`)
    - 6.8.2.1 Tag Handles (`!`, `!!`, `!e!`)
    - 6.8.2.2 Tag Prefixes (`!<...>`)
- **6.9 Node Properties**
  - 6.9.1 Node Tags (`!!str`, `!!int`, etc.)
  - 6.9.2 Node Anchors (`&anchor`, `*alias`, `<<`)

### Chapter 7: Flow Style Productions

**Reference:** https://yaml.org/spec/1.2.2/#chapter-7-flow-style-productions

Tests for flow (inline) style:

- **7.1 Alias Nodes** - Alias references (`*name`)
- **7.2 Empty Nodes** - Empty scalars and values
- **7.3 Flow Scalar Styles**
  - 7.3.1 Double-Quoted Style (`"..."`)
  - 7.3.2 Single-Quoted Style (`'...'`)
  - 7.3.3 Plain Style (unquoted)
- **7.4 Flow Collection Styles**
  - 7.4.1 Flow Sequences (`[...]`)
  - 7.4.2 Flow Mappings (`{...}`)
- **7.5 Flow Nodes** - Nodes with anchors/tags
- **7.6 Flow Style Edge Cases** - Mixed styles, comments, anchors, Unicode
- **7.7 Flow Collection Errors and Advanced Cases** - Comprehensive edge case coverage:
  - 7.7.1 Flow Collection Error Conditions (missing comma, trailing comma, missing colon, unclosed delimiters, extra delimiters)
  - 7.7.2 Multiline Flow Collections (multiline sequences/mappings, nested multiline, quoted scalars, empty lines)
  - 7.7.3 Complex Keys in Flow Mappings (quoted keys, special characters, explicit indicators, flow sequence keys)
  - 7.7.4 Deeply Nested Flow Collections (10+ levels of nesting, mixed nesting)
  - 7.7.5 Duplicate Keys Error Handling (simple duplicates, different quoting, nested duplicates)

### Chapter 8: Block Style Productions

**Reference:** https://yaml.org/spec/1.2.2/#chapter-8-block-style-productions

Tests for block (indentation-based) style:

- **8.1 Block Scalar Styles**
  - 8.1.1 Block Scalar Headers
    - 8.1.1.1 Block Indentation Indicator (`|2`)
    - 8.1.1.2 Block Chomping Indicator (`+`, `-`, clip)
  - 8.1.2 Literal Style (`|`)
  - 8.1.3 Folded Style (`>`)
- **8.2 Block Collection Styles**
  - 8.2.1 Block Sequences (`- item`)
  - 8.2.2 Block Mappings (`key: value`)
  - 8.2.3 Block Nodes (with anchors/tags)
- **8.3 Block Style Edge Cases** - Deeply nested blocks, mixed collections, comments, empty scalars
- **8.4 Chomping Indicator Combinations** - Comprehensive chomping tests:
  - All combinations of literal/folded with strip/keep/clip
  - Indicator order variations (|2-, |-2, >2+, >+2)
- **8.5 Full Indentation Indicator Range (1-9)** - Complete range of valid indent indicators
- **8.6 Explicit Key/Value Indicators** - Comprehensive `?` and `:` usage:
  - Simple explicit keys
  - Multiline keys and values
  - Complex values with explicit keys
  - Multiple explicit keys in single mapping
- **8.7 Complex Keys (Sequences and Mappings as Keys)** - Advanced key types:
  - Block sequences as keys
  - Block mappings as keys
  - Nested sequences/mappings as keys
  - Flow collections as block mapping keys
- **8.8 Multiline Plain Scalar Keys** - Keys spanning multiple lines with line folding
- **8.9 Block Scalar Edge Cases** - Boundary conditions:
  - Empty content scalars
  - Whitespace-only lines
  - Single character content
  - Very long lines (500+ chars)
  - Mixed empty and content lines
  - Trailing space preservation

### Chapter 9: Document Stream Productions

**Reference:** https://yaml.org/spec/1.2.2/#chapter-9-document-stream-productions

Tests for document streams:

- **9.1 Documents**
  - 9.1.1 Document Prefix (BOM)
  - 9.1.2 Document Markers (`---`, `...`)
  - 9.1.3 Bare Documents (no markers)
  - 9.1.4 Explicit Documents (with `---`)
  - 9.1.5 Directives Documents (with `%`)
- **9.2 Streams** - Multiple documents
- **9.5 BOM Handling** - Comprehensive BOM tests:
  - UTF-8 BOM at stream start (U+FEFF)
  - BOM with document markers
  - BOM in multi-document streams
  - Mid-stream BOM errors
  - UTF-16/32 BOM detection
  - Multiple BOM errors
- **9.6 Document Marker Error Tests** - Invalid marker usage:
  - Document start marker with trailing content (`--- key: value`)
  - Document end marker with trailing content (`... trailing`)
  - Indented markers (not at column 0)
  - Incomplete markers (only 2 dashes)
  - Markers with prefix characters
- **9.7 Empty and Large Streams** - Boundary conditions:
  - Completely empty streams
  - Whitespace-only streams
  - Streams with only markers
  - Large streams (10, 100, 1000+ documents)
- **9.8 Directive Placement Errors** - Invalid directive usage:
  - Directives after document content (must precede content)
  - Directives between bare documents (need `---` marker)
  - Duplicate YAML directives (only one per document)
  - Invalid YAML versions
  - Invalid TAG directive handles
  - Indented directives

### Chapter 10: Recommended Schemas

**Reference:** https://yaml.org/spec/1.2.2/#chapter-10-recommended-schemas

Tests for type schemas:

- **10.1 Failsafe Schema** - Comprehensive Failsafe tests:
  - No type inference (everything is string by default)
  - Only explicit tags (`!!int`, `!!str`) resolve types
  - Mappings, sequences, and strings only
  - Nested collections support
- **10.2 JSON Schema** - Strict JSON compatibility tests:
  - Null: only lowercase `null` (not `~`, `Null`, `NULL`, or empty)
  - Boolean: only lowercase `true`/`false` (case-sensitive)
  - Numbers: decimal only (no octal `0o`, hex `0x`, or binary `0b`)
  - No special floats (`.inf`, `-.inf`, `.nan` not allowed)
  - Regular float notation only
- **10.3 Core Schema** - Extended type variations:
  - **10.3.1 Boolean Variations** - Comprehensive boolean formats:
    - Yes/No, yes/no, YES/NO
    - On/Off, on/off, ON/OFF
    - Y/N, y/n
    - True/False (all case variations)
  - **10.3.2 Null Variations** - All null representations:
    - null, Null, NULL (case variations)
    - `~` (tilde)
    - Empty values
  - **10.3.3 Integer Bases and Formats** - Complete integer support:
    - Binary notation (`0b1010`)
    - Octal notation (`0o52`)
    - Hexadecimal notation (`0x2A`, `0x2a`)
    - Underscores for readability (`1_000_000`)
    - Explicit positive sign (`+42`)
    - Sexagesimal (base 60) notation (`60:00`)
  - **10.3.4 Float Special Values** - All float variations:
    - Infinity: `.inf`, `.Inf`, `.INF`
    - Negative infinity: `-.inf`, `-.Inf`, `-.INF`
    - Not-a-Number: `.nan`, `.NaN`, `.NAN`
    - Underscores in floats (`1_234.567_89`)
  - **10.3.5 Timestamps and Binary** - Advanced types:
    - ISO 8601 date formats (date-only, datetime with T, datetime with space)
    - Timezone offsets and UTC
    - Local time (no timezone)
    - Binary type (`!!binary` with base64 encoding)
    - Merge key (`<<`) for mapping inheritance
    - Multiple merge sources
- **10.4 Type Resolution** - Automatic type detection
- **10.5 Number Format Edge Cases** - Boundary conditions
- **10.6 String Type Edge Cases** - URLs, emails, versions

## Implementation Tests

Tests for specific implementation features (not in spec):

### Editor Tests

- `editor.rs` - Editor API (4 tests)
- `editor_strings.rs` - String editing (2 tests)

### Emitter Tests

- `emitter.rs` - General emitter (6 tests)
- `emitter_alias_anchor.rs` - Anchor/alias emission (3 tests)
- `emitter_block_scalars.rs` - Block scalar emission (3 tests)
- `emitter_comments.rs` - Comment emission (6 tests)
- `emitter_layout.rs` - Layout formatting (2 tests)
- `emitter_quoting.rs` - Quote handling (4 tests)
- `emitter_scalars.rs` - Scalar emission (2 tests)
- `emitter_top_level.rs` - Top-level emission (2 tests)

### Parser Tests

- `parser.rs` - Parser specifics (15 tests)
- `lexer_edges.rs` - Lexer edge cases (4 tests)

### Other Tests

- `ast.rs` - AST utilities (10 tests)
- `fuzzish.rs` - Fuzz testing (2 tests)
- `typing.rs` - Type detection (2 tests)
- `schema_core.rs` - Schema tests (1 test)
- `document_streams.rs`, `indented_doc_markers.rs`, etc.

**Total:** ~55 implementation-specific tests

## Test Results (Updated 2025-11-16)

### Overall Test Status

```
Total Tests: 680
├── YAML 1.2.2 Spec Tests: 671
│   ├── Passing: 607
│   ├── Failing: 63
│   └── Ignored: 1
└── Implementation Tests: ~9
    ├── Passing: 3
    └── Failing: 6
```

**YAML 1.2.2 Compliance: 90.5% (607/671 tests passing)**

### YAML 1.2.2 Spec Compliance by Chapter

| Chapter | Description           | Estimated Tests | Status               |
| ------- | --------------------- | --------------- | -------------------- |
| 2       | Overview              | ~63             | ⚠️ Multiple failures |
| 3       | Processes             | ~79             | ⚠️ Multiple failures |
| 4       | Syntax                | ~62             | ⚠️ 1 failure         |
| 5       | Characters            | ~96             | ⚠️ 13 failures       |
| 6       | Structural            | ~70             | ✅ All passing       |
| 7       | Flow Styles           | ~67             | ⚠️ 11 failures       |
| 8       | Block Styles          | ~78             | ⚠️ 3 failures        |
| 9       | Documents             | ~60             | ⚠️ 15 failures       |
| 10      | Schemas               | ~86             | ✅ All passing       |

### Implementation Test Results

| Test File                  | Status  | Notes                                  |
| -------------------------- | ------- | -------------------------------------- |
| `alias_anchor_weird.rs`    | ✅ Pass | 1/1 tests passing                      |
| `anchors.rs`               | ✅ Pass | 2/2 tests passing                      |
| `ast.rs`                   | ✅ Pass | 10/10 tests passing                    |
| `block_scalars.rs`         | ❌ Fail | 0/1 - Indent indicator & folding issue |
| `block_scalars_strict.rs`  | ❌ Fail | 4/9 - Block scalar handling issues     |
| `flow.rs`                  | ❌ Fail | 0/1 - Flow mapping nested seq issue    |
| `merge_key_roundtrip.rs`   | ✅ Pass | 1/1 tests passing                      |
| `schema_core.rs`           | ❌ Fail | 0/1 - Case-sensitive bool/null issue   |
| `single_double_quotes.rs`  | ❌ Fail | 0/1 - Quote escape handling            |
| `tags.rs`                  | ❌ Fail | 0/1 - Tag parsing error                |

### Known Failures (63 spec tests + 6 implementation tests)

#### YAML 1.2.2 Spec Test Failures (63 tests)

**Chapter 2: Overview (20 failures)**

1. `ch_2_2_01_two_documents_in_stream` - Document stream handling
2. `ch_2_3_01_literal_block_scalar` - Literal block scalar formatting
3. `ch_2_3_05_quoted_scalars` - Quoted scalar escape sequences
4. `ch_2_4_05_explicit_tags` - Explicit tag preservation (!!str, !!binary)
5. `ch_2_4_06_application_specific_tags` - Custom tag handling
6. `ch_2_5_01_invoice_document` - Complex document with anchors/aliases
7. `ch_2_edge_01_empty_sequence` - Empty flow sequence `[]`
8. `ch_2_edge_03_empty_collections_nested` - Nested empty collections
9. `ch_2_edge_20_single_vs_double_quotes` - Quote style handling
10. `ch_2_error_03_trailing_comma_flow_sequence` - Should reject `[a,]`
11. `ch_2_error_04_trailing_comma_flow_mapping` - Should reject `{a:1,}`
12. `ch_2_error_05_unmatched_bracket_extra_close` - Should reject `[a]]`
13. `ch_2_error_07_unmatched_brace_extra_close` - Should reject `{a:1}}`
14. `ch_2_error_08_unmatched_brace_missing_close` - Should reject `{a:1`
15. `ch_2_error_11_unterminated_double_quote` - Should reject unterminated `"`
16. `ch_2_error_12_unterminated_single_quote` - Should reject unterminated `'`

**Chapter 3: Processes and Models (8 failures)**

17. `ch_3_2_2_02_anchor_scope_does_not_cross_documents` - Anchor scope validation
18. `ch_3_2_3_05_duplicate_yaml_directive` - Should reject duplicate `%YAML`
19. `ch_3_2_3_06_duplicate_tag_handle` - Should reject duplicate `%TAG`
20. `ch_3_2_3_07_directive_after_content` - Directives must precede content
21. `ch_3_3_02_ill_formed_rejected` - Should reject ill-formed input
22. `ch_3_3_03_unidentified_alias` - Undefined alias validation
23. `ch_3_3_09_undefined_tag_handle` - Tag handle declaration required
24. `ch_3_7_2_02_duplicate_keys_canonical_comparison` - Duplicate key detection

**Chapter 4: Syntax Conventions (1 failure)**

25. `ch_4_10_02_flow_to_block_to_flow` - Complex context transitions

**Chapter 5: Characters (13 failures)**

26. `ch_5_7_22_multiple_escapes` - Escape sequence handling
27. `ch_5_8_2_01_null_character_error` - Should reject NULL (U+0000)
28. `ch_5_8_2_02_backspace_character_error` - Should reject backspace (U+0008)
29. `ch_5_8_2_03_vertical_tab_error` - Should reject vertical tab (U+000B)
30. `ch_5_8_2_04_form_feed_error` - Should reject form feed (U+000C)
31. `ch_5_8_2_05_delete_character_error` - Should reject DELETE (U+007F)
32. `ch_5_8_2_06_c1_control_characters_error` - Should reject C1 controls
33. `ch_5_8_4_01_invalid_escape_letter` - Should reject `\q`
34. `ch_5_8_4_02_incomplete_hex_escape` - Should reject `\x4`
35. `ch_5_8_4_03_incomplete_unicode16_escape` - Should reject `\u004`
36. `ch_5_8_4_04_incomplete_unicode32_escape` - Should reject incomplete `\U`
37. `ch_5_8_4_05_invalid_hex_digits` - Should reject invalid hex in escapes
38. `ch_5_8_5_01_high_surrogate_alone` - Surrogate pair validation
39. `ch_5_8_5_02_low_surrogate_alone` - Surrogate pair validation
40. `ch_5_8_5_03_reversed_surrogate_pair` - Surrogate pair order

**Chapter 7: Flow Styles (11 failures)**

41. `ch_7_4_1_05_flow_sequence_empty` - Empty flow sequence handling
42. `ch_7_7_1_01_missing_comma_in_sequence` - Should reject missing comma
43. `ch_7_7_1_02_trailing_comma_in_sequence` - Should reject trailing comma
44. `ch_7_7_1_03_missing_comma_in_mapping` - Should reject missing comma
45. `ch_7_7_1_04_trailing_comma_in_mapping` - Should reject trailing comma
46. `ch_7_7_1_05_missing_colon_in_mapping` - Should reject missing colon
47. `ch_7_7_1_07_missing_closing_brace` - Should reject unclosed `{`
48. `ch_7_7_1_08_extra_closing_bracket` - Should reject extra `]`
49. `ch_7_7_5_01_duplicate_keys_simple` - Duplicate key detection
50. `ch_7_7_5_02_duplicate_keys_different_quotes` - Canonical duplicate detection
51. `ch_7_7_5_03_duplicate_keys_nested` - Nested duplicate keys

**Chapter 8: Block Styles (3 failures)**

52. `ch_8_1_1_1_04_indent_indicator_zero_invalid` - Should reject indent `0`
53. `ch_8_2_2_06_block_mapping_quoted_keys` - Quoted key preservation
54. `ch_8_2_3_03_block_scalar_in_mapping` - Block scalar in mapping values

**Chapter 9: Document Streams (15 failures)**

55. `ch_9_3_01_empty_document_in_stream` - Empty document handling
56. `ch_9_6_01_document_start_with_trailing_content` - Should reject `--- content`
57. `ch_9_6_02_document_end_with_trailing_content` - Should reject `... content`
58. `ch_9_7_03_empty_document_markers_only` - Marker-only documents
59. `ch_9_8_01_directive_after_content_start` - Directive placement
60. `ch_9_8_02_directive_between_documents_no_marker` - Directive validation
61. `ch_9_8_04_duplicate_yaml_directive` - Should reject duplicate `%YAML`
62. `ch_9_8_06_tag_directive_invalid_handle` - TAG directive validation
63. `ch_9_8_07_directive_indented` - Should reject indented directives

#### Implementation Test Failures (6 tests)

**Block Scalar Issues:**
- `block_scalars.rs::indent_indicator_and_folding` - Block scalar content not preserved
- `block_scalars_strict.rs` (5 failures):
  - `chomping_plus_keeps_all_trailing_newlines` - Chomping indicator handling
  - `content_lines_must_meet_required_indent_when_indicator_present` - Indent validation
  - `folded_preserves_paragraphs_and_more_indented_runs` - Folding logic
  - `folded_single_newline_becomes_single_space_no_double_spaces` - Line folding
  - `literal_with_indent_indicator_removes_exact_k_spaces` - Indent stripping

**Flow Collection Issues:**
- `flow.rs::flow_mapping_and_nested_flow_seq` - Nested flow structure parsing

**Schema Issues:**
- `schema_core.rs::core_case_sensitive_bools_null` - Core schema type resolution

**Quote Handling:**
- `single_double_quotes.rs::single_and_double_quoted_escapes` - Quote escape processing

**Tag Parsing:**
- `tags.rs::simple_short_and_verbatim_tags_parse_and_emit` - Tag syntax parsing

## Test Infrastructure

### Fixture Pattern

The test suite uses a `Fixture` builder for clean, readable tests:

```rust
use common::Fixture;

Fixture::new(
    "7.3.1",  // Spec section
    1,         // Test number
    "Double-quoted basic",
    "key: \"value\"\n",  // Input YAML
)
.run();  // Execute test
```

### Direct Spec Traceability

Each test can be traced directly to the spec:

```rust
// Test: ch_8_1_2_01_literal_preserves_newlines
// Links to: https://yaml.org/spec/1.2.2/#812-literal-style
Fixture::new(
    "8.1.2",  // ← Spec section
    1,         // ← Test number
    "Literal preserves all line breaks",
    "|\nline 1\nline 2\n\nline 4\n",
)
.run();
```

### Key Features

- ✅ **Zero dependencies** - Pure Rust, no third-party test frameworks
- ✅ **Round-trip testing** - Parse → emit → parse → emit stability
- ✅ **Spec traceability** - Every test maps to a specific spec section
- ✅ **Professional structure** - Industry-standard approach to standards compliance
- ✅ **Direct spec links** - Chapter and section numbers match the spec document

## Running Tests

### Run All Tests

```bash
# Everything
cargo test

# Only spec tests
cargo test --test yaml_spec_1_2_2

# Only implementation tests
cargo test editor
cargo test emitter
cargo test parser
```

### Run Specific Chapter

```bash
cargo test --test yaml_spec_1_2_2 ch_6   # Chapter 6: Structural
cargo test --test yaml_spec_1_2_2 ch_7   # Chapter 7: Flow styles
cargo test --test yaml_spec_1_2_2 ch_8   # Chapter 8: Block styles
cargo test --test yaml_spec_1_2_2 ch_9   # Chapter 9: Documents
cargo test --test yaml_spec_1_2_2 ch_10  # Chapter 10: Schemas
```

### Run Specific Section

```bash
cargo test --test yaml_spec_1_2_2 ch_7_3   # Flow scalars
cargo test --test yaml_spec_1_2_2 ch_8_1   # Block scalars
cargo test --test yaml_spec_1_2_2 ch_9_2   # Document streams
```

### Run Specific Test

```bash
cargo test --test yaml_spec_1_2_2 ch_7_3_1_01_double_quoted_basic
```

## Contributing Tests

### Add Spec Test

1. Identify spec chapter/section from https://yaml.org/spec/1.2.2/
2. Add to appropriate `ch_X_*.rs` file
3. Use chapter-based naming: `ch_X_Y_ZZ_description`
4. Reference spec section in `Fixture::new("X.Y", ZZ, ...)`

**Example:**

```rust
#[test]
fn ch_7_3_1_05_double_quoted_escapes() {
    Fixture::new(
        "7.3.1",
        5,
        "Double-quoted with escape sequences",
        "key: \"line 1\\nline 2\"\n",
    )
    .run();
}
```

### Add Implementation Test

1. Create or add to relevant file (e.g., `emitter.rs`)
2. Use descriptive test names
3. Add comments explaining what's being tested

## Design Principles

1. **Spec-Driven** - Primary tests map directly to YAML 1.2.2
2. **Implementation-Specific** - Separate tests for library features
3. **Zero Dependencies** - No third-party test frameworks
4. **Maintainable** - Clear organization and naming
5. **Comprehensive** - ~385 tests covering all major features
6. **Educational** - Learn YAML by reading tests in spec order

## Compliance Path to 100%

To reach 100% compliance, the following implementation gaps need to be addressed:

### Critical Issues (High Impact)

**1. Parser Error Detection (26 tests)**
   - Flow collection validation: trailing commas, missing delimiters, unmatched brackets/braces
   - Character validation: reject control characters, invalid escapes, surrogate pair errors
   - Directive validation: duplicate directives, invalid placement, indented directives
   - Document marker validation: reject trailing content after `---` or `...`
   - **Impact:** Ch 2 (11), Ch 3 (3), Ch 5 (9), Ch 7 (8), Ch 9 (5)
   - **Priority:** HIGH - These are correctness issues that violate spec

**2. Block Scalar Implementation (11 tests)**
   - Block scalar content preservation and round-trip stability
   - Chomping indicators (`+`, `-`, clip)
   - Indent indicators (especially validation of indicator `0`)
   - Folding logic for `>` style
   - Line break handling and paragraph preservation
   - **Impact:** Ch 2 (2), Ch 8 (3), Implementation tests (6)
   - **Priority:** HIGH - Core YAML feature

**3. Document Stream Handling (16 tests)**
   - Empty document handling
   - Multi-document streams
   - Anchor scope (must not cross documents)
   - Document marker parsing
   - Directive placement rules
   - **Impact:** Ch 2 (1), Ch 3 (1), Ch 9 (14)
   - **Priority:** MEDIUM - Important for multi-doc workflows

**4. Tag System (5 tests)**
   - Tag preservation (explicit tags like `!!str`, `!!binary`)
   - Custom/application-specific tags
   - Tag directive handling and validation
   - Tag handle resolution
   - **Impact:** Ch 2 (2), Ch 3 (2), Implementation tests (1)
   - **Priority:** MEDIUM - Advanced feature

**5. Flow Collections (3 tests)**
   - Empty flow sequence `[]` handling
   - Nested flow structures
   - Duplicate key detection in flow mappings
   - **Impact:** Ch 2 (1), Ch 7 (1), Implementation tests (1)
   - **Priority:** MEDIUM

**6. Quote and Escape Handling (4 tests)**
   - Single vs double quote selection
   - Escape sequence processing
   - Quoted key preservation in block mappings
   - **Impact:** Ch 2 (2), Ch 8 (1), Implementation tests (1)
   - **Priority:** LOW - Edge cases

**7. Other Issues (4 tests)**
   - Complex context transitions (flow↔block)
   - Duplicate key detection with canonical comparison
   - Core schema type resolution (case-sensitive bools/nulls)
   - Complex documents with anchors/aliases
   - **Impact:** Ch 2 (1), Ch 3 (1), Ch 4 (1), Implementation tests (1)
   - **Priority:** LOW - Advanced edge cases

### Implementation Strategy

**Phase 1: Error Detection & Validation (40% of failures)**
- Add comprehensive input validation to reject ill-formed YAML
- Implement proper error messages for all spec violations
- Focus on Ch 5, 7, 9 error tests

**Phase 2: Block Scalars (16% of failures)**
- Rewrite or fix block scalar parsing and emission
- Implement proper chomping and indent indicator support
- Ensure round-trip stability

**Phase 3: Document Streams (23% of failures)**
- Fix multi-document stream handling
- Implement anchor scope validation
- Handle empty documents and markers correctly

**Phase 4: Tags & Advanced Features (21% of failures)**
- Implement tag preservation and custom tag support
- Fix flow collection edge cases
- Improve quote/escape handling

Each category maps directly to specific YAML 1.2.2 spec sections!

## Future Work

### Additional Coverage

- [ ] More edge cases for each chapter
- [ ] Unicode and encoding tests (Chapter 5)
- [ ] Complex nested structure tests
- [ ] Performance benchmarks per chapter

## Benefits of This Test Suite

1. **Direct Traceability**: Every test maps to a specific spec section
2. **Easy Navigation**: Chapter and section numbers match the spec document
3. **Complete Coverage**: Organized systematically through the spec
4. **Professional**: Industry-standard approach to standards compliance
5. **Maintainable**: Clear structure for adding new tests
6. **Educational**: Learn YAML by reading tests in spec order
7. **Zero Dependencies**: Pure Rust implementation
8. **TDD-Ready**: Comprehensive test coverage drives implementation toward full compliance
