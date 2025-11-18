// crates/forgo_lib_yaml/tests/yaml_spec_1_2_2.rs
//! YAML 1.2.2 Specification Compliance Test Suite
//!
//! This test suite is organized to precisely follow the structure of the YAML 1.2.2 specification.
//! Reference: https://yaml.org/spec/1.2.2/
//!
//! Test naming convention: ch_X_Y_ZZ_description
//! - X: Chapter number (e.g., 5, 6, 7, 8, 9, 10)
//! - Y: Section within chapter (e.g., 1, 2, 3)
//! - ZZ: Test case number (01, 02, 03, ...)
//! - description: Brief test description in snake_case
//!
//! ## Spec Structure:
//!
//! Chapter 5: Character Productions
//! Chapter 6: Structural Productions (Indentation, Comments, Directives, Node Properties)
//! Chapter 7: Flow Style Productions (Aliases, Scalars, Collections)
//! Chapter 8: Block Style Productions (Block Scalars, Block Collections)
//! Chapter 9: Document Stream Productions (Documents, Streams)
//! Chapter 10: Recommended Schemas (Failsafe, JSON, Core)

#[path = "common/mod.rs"]
mod common;

// ============================================================================
// Chapter 2: Language Overview
// ============================================================================

#[path = "spec/ch_2_overview.rs"]
mod ch_2_overview;

// ============================================================================
// Chapter 3: Processes and Models
// ============================================================================

#[path = "spec/ch_3_processes.rs"]
mod ch_3_processes;

// ============================================================================
// Chapter 4: Syntax Conventions
// ============================================================================

#[path = "spec/ch_4_syntax.rs"]
mod ch_4_syntax;

// ============================================================================
// Chapter 5: Character Productions
// ============================================================================

#[path = "spec/ch_5_characters.rs"]
mod ch_5_characters;

// ============================================================================
// Chapter 6: Structural Productions
// ============================================================================

#[path = "spec/ch_6_structural.rs"]
mod ch_6_structural;

// ============================================================================
// Chapter 7: Flow Style Productions
// ============================================================================

#[path = "spec/ch_7_flow_styles.rs"]
mod ch_7_flow_styles;

// ============================================================================
// Chapter 8: Block Style Productions
// ============================================================================

#[path = "spec/ch_8_block_styles.rs"]
mod ch_8_block_styles;

// ============================================================================
// Chapter 9: Document Stream Productions
// ============================================================================

#[path = "spec/ch_9_documents.rs"]
mod ch_9_documents;

// ============================================================================
// Chapter 10: Recommended Schemas
// ============================================================================

#[path = "spec/ch_10_schemas.rs"]
mod ch_10_schemas;
