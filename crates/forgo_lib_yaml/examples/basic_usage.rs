//! Basic Usage Example - Getting Started with forgo_lib_yaml
//!
//! This example demonstrates the fundamental features of the forgo_lib_yaml crate:
//! - Parsing YAML documents from strings
//! - Navigating the document structure using type guards
//! - Extracting values with type-safe conversions
//! - Serializing documents back to YAML strings
//!
//! Run this example with: `cargo run --example basic_usage`

use forgo_lib_yaml::{parse, stringify};

fn main() {
    println!("=== Basic YAML Parsing and Navigation ===\n");

    // Parse a YAML document from a string
    // The parse() function is a convenient wrapper around Doc::from_str()
    let yaml_input = r#"
# Application configuration
name: my-awesome-app
version: 1.2.3
enabled: true

# Dependencies list
dependencies:
  - yaml-parser
  - config-loader
  - logging

# Nested configuration
database:
  host: localhost
  port: 5432
  credentials:
    username: admin
    password: null  # To be set via environment
"#;

    let doc = parse(yaml_input).expect("Failed to parse YAML");
    println!("✓ Successfully parsed YAML document\n");

    // Access the root node - YAML documents always have a root
    let root = doc.root();
    println!("Document structure:");

    // Use type guards to safely check the node type
    // Type guards: is_map(), is_seq(), is_scalar(), is_null(), is_alias()
    if root.node().is_map() {
        println!("  Root is a map (dictionary/object)");

        // Use as_map() to get type-safe access to the map entries
        // Returns Option<&Vec<(String, Elem)>>
        if let Some(entries) = root.node().as_map() {
            println!("  Map contains {} top-level keys:", entries.len());

            for (key, value) in entries {
                // Each value is an Elem (Element) which wraps a Node
                let value_type = if value.node().is_scalar() {
                    "scalar"
                } else if value.node().is_map() {
                    "map"
                } else if value.node().is_seq() {
                    "sequence"
                } else {
                    "other"
                };
                println!("    - {}: {}", key, value_type);
            }
        }
    }

    println!("\n=== Extracting Values ===\n");

    // Extract scalar values using type-safe conversions
    // The Node type provides convenient as_* methods for type conversion
    if let Some(map) = root.node().as_map() {
        // Find and extract string value
        if let Some((_, name_elem)) = map.iter().find(|(k, _)| k == "name") {
            if let Some(name) = name_elem.node().as_str() {
                println!("Application name: {}", name);
            }
        }

        // Find and extract number value
        // Numbers can be extracted as i64, u64, or f64
        if let Some((_, version_elem)) = map.iter().find(|(k, _)| k == "version") {
            if let Some(version) = version_elem.node().as_str() {
                println!("Version: {}", version);
            }
        }

        // Find and extract boolean value
        if let Some((_, enabled_elem)) = map.iter().find(|(k, _)| k == "enabled") {
            if let Some(enabled) = enabled_elem.node().as_bool() {
                println!("Enabled: {}", enabled);
            }
        }

        // Navigate into nested structures
        if let Some((_, deps_elem)) = map.iter().find(|(k, _)| k == "dependencies") {
            if let Some(deps) = deps_elem.node().as_seq() {
                println!("\nDependencies ({} total):", deps.len());
                for (i, dep) in deps.iter().enumerate() {
                    if let Some(dep_name) = dep.node().as_str() {
                        println!("  {}. {}", i + 1, dep_name);
                    }
                }
            }
        }

        // Navigate nested maps
        if let Some((_, db_elem)) = map.iter().find(|(k, _)| k == "database") {
            if let Some(db_map) = db_elem.node().as_map() {
                println!("\nDatabase configuration:");

                if let Some((_, host)) = db_map.iter().find(|(k, _)| k == "host") {
                    if let Some(host_str) = host.node().as_str() {
                        println!("  Host: {}", host_str);
                    }
                }

                if let Some((_, port)) = db_map.iter().find(|(k, _)| k == "port") {
                    if let Some(port_num) = port.node().as_i64() {
                        println!("  Port: {}", port_num);
                    }
                }

                // Access deeply nested values
                if let Some((_, creds)) = db_map.iter().find(|(k, _)| k == "credentials") {
                    if let Some(creds_map) = creds.node().as_map() {
                        println!("  Credentials:");

                        if let Some((_, username)) = creds_map.iter().find(|(k, _)| k == "username")
                        {
                            if let Some(user) = username.node().as_str() {
                                println!("    Username: {}", user);
                            }
                        }

                        if let Some((_, password)) = creds_map.iter().find(|(k, _)| k == "password")
                        {
                            // Check for null values
                            if password.node().is_null() {
                                println!("    Password: <not set>");
                            }
                        }
                    }
                }
            }
        }
    }

    println!("\n=== Serializing Back to YAML ===\n");

    // Serialize the document back to a YAML string
    // The stringify() function is a convenient wrapper around Doc::to_string()
    match stringify(&doc) {
        Ok(yaml_output) => {
            println!("Successfully serialized document:");
            println!("{}", "─".repeat(50));
            println!("{}", yaml_output);
            println!("{}", "─".repeat(50));
        }
        Err(e) => {
            eprintln!("Failed to serialize: {}", e);
        }
    }

    println!("\n=== Parsing Multiple Documents ===\n");

    // YAML supports multiple documents in a single stream, separated by ---
    use forgo_lib_yaml::parse_all;

    let multi_doc = r#"
---
document: first
type: config
---
document: second
type: data
---
document: third
type: metadata
"#;

    match parse_all(multi_doc) {
        Ok(docs) => {
            println!("Parsed {} documents from stream:", docs.len());
            for (i, doc) in docs.iter().enumerate() {
                if let Some(map) = doc.root().node().as_map() {
                    if let Some((_, doc_name)) = map.iter().find(|(k, _)| k == "document") {
                        if let Some(name) = doc_name.node().as_str() {
                            println!("  Document {}: {}", i + 1, name);
                        }
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to parse documents: {}", e);
        }
    }

    println!("\n=== Key Takeaways ===\n");
    println!("✓ Use parse() for single documents, parse_all() for streams");
    println!("✓ Use type guards (is_map, is_seq, is_scalar) to check types");
    println!("✓ Use as_* methods for type-safe value extraction");
    println!("✓ All conversions return Option<T> for safety");
    println!("✓ Use stringify() to serialize documents back to YAML");
    println!("✓ The crate preserves comments and formatting during round-trips\n");
}
