//! Path-Based Editing Example - Advanced Document Manipulation
//!
//! This example demonstrates the powerful path-based visitor API for editing YAML:
//! - Using IntoPath trait for ergonomic path syntax
//! - Visiting and modifying sequences, maps, and values at specific paths
//! - Using wildcards (*) to apply changes to multiple locations
//! - Practical examples of common editing patterns
//!
//! The path-based API makes it easy to modify deeply nested structures without
//! manually traversing the entire document tree.
//!
//! Run this example with: `cargo run --example path_based_editing`

use forgo_lib_yaml::{parse, stringify, Elem};

fn main() {
    println!("=== Path-Based Document Editing ===\n");

    // Example 1: Basic path navigation
    println!("1. Basic Path Navigation\n");

    let yaml = r#"
app:
  name: my-app
  version: 1.0.0
  config:
    debug: false
    timeout: 30
"#;

    let mut doc = parse(yaml).expect("Failed to parse");

    // Visit a specific value using a path
    // Paths are specified as arrays of strings: &["app", "config", "debug"]
    let modified = doc.visit_values_mut(&["app", "config", "debug"], |elem| {
        *elem = Elem::boolean(true); // Enable debug mode
        true // Return true to indicate we made changes
    });

    println!("Debug mode enabled: {}", modified);

    // Visit and modify a number value
    doc.visit_values_mut(&["app", "config", "timeout"], |elem| {
        *elem = Elem::number(60).with_comment("Increased timeout");
        true
    });

    match stringify(&doc) {
        Ok(yaml) => {
            println!("Modified configuration:");
            println!("{}", "─".repeat(50));
            println!("{}", yaml);
            println!("{}", "─".repeat(50));
        }
        Err(e) => eprintln!("Error: {}", e),
    }

    // Example 2: Working with sequences
    println!("\n2. Editing Sequences (Lists)\n");

    let yaml = r#"
dependencies:
  - yaml-parser
  - config-loader
"#;

    let mut doc = parse(yaml).expect("Failed to parse");

    // Visit a sequence and add items
    doc.visit_sequences_mut(&["dependencies"], |seq| {
        // Add new dependencies
        seq.push(Elem::string("logging"));
        seq.push(Elem::string("metrics").with_comment("For monitoring"));
        true
    });

    match stringify(&doc) {
        Ok(yaml) => {
            println!("Updated dependencies:");
            println!("{}", yaml);
        }
        Err(e) => eprintln!("Error: {}", e),
    }

    // Example 3: Working with maps
    println!("3. Editing Maps (Dictionaries)\n");

    let yaml = r#"
server:
  host: localhost
  port: 8080
"#;

    let mut doc = parse(yaml).expect("Failed to parse");

    // Visit a map and add entries
    doc.visit_maps_mut(&["server"], |map| {
        // Add new configuration keys
        map.push((
            "workers".to_string(),
            Elem::number(4).with_comment("Worker threads"),
        ));
        map.push((
            "timeout".to_string(),
            Elem::number(30),
        ));
        true
    });

    match stringify(&doc) {
        Ok(yaml) => {
            println!("Updated server config:");
            println!("{}", yaml);
        }
        Err(e) => eprintln!("Error: {}", e),
    }

    // Example 4: Wildcard paths - the killer feature!
    println!("4. Using Wildcards to Edit Multiple Locations\n");

    let yaml = r#"
jobs:
  build:
    runs-on: ubuntu-20.04
    steps:
      - checkout
      - build
  test:
    runs-on: ubuntu-20.04
    steps:
      - checkout
      - test
  deploy:
    runs-on: ubuntu-20.04
    steps:
      - checkout
      - deploy
"#;

    let mut doc = parse(yaml).expect("Failed to parse");

    // Use wildcard (*) to visit all jobs and update their OS version
    let count = doc.visit_values_mut(&["jobs", "*", "runs-on"], |elem| {
        *elem = Elem::string("ubuntu-22.04").with_comment("Updated to Ubuntu 22.04");
        true
    });

    println!("Updated {} job runners to ubuntu-22.04", if count { "all" } else { "no" });

    // Add a cleanup step to all jobs using wildcard
    doc.visit_sequences_mut(&["jobs", "*", "steps"], |steps| {
        steps.push(Elem::string("cleanup").with_comment("Clean up artifacts"));
        true
    });

    println!("Added cleanup step to all jobs");

    match stringify(&doc) {
        Ok(yaml) => {
            println!("\nUpdated CI configuration:");
            println!("{}", "─".repeat(50));
            println!("{}", yaml);
            println!("{}", "─".repeat(50));
        }
        Err(e) => eprintln!("Error: {}", e),
    }

    // Example 5: Multiple path syntaxes
    println!("\n5. IntoPath Trait - Multiple Path Syntaxes\n");

    let yaml = r#"
level1:
  level2:
    value: original
"#;

    let mut doc = parse(yaml).expect("Failed to parse");

    // All these syntaxes work thanks to IntoPath trait:

    // 1. Array literal (most common)
    doc.visit_values_mut(&["level1", "level2", "value"], |elem| {
        *elem = Elem::string("modified");
        true
    });

    // 2. Vec<&str> (useful when path is dynamic)
    let path_segments = vec!["level1", "level2", "value"];
    doc.visit_values_mut(&path_segments[..], |elem| {
        *elem = Elem::string("modified again");
        true
    });

    // 3. Vec<&str> owned (when you build the path dynamically)
    let dynamic_path = vec!["level1", "level2", "value"];
    doc.visit_values_mut(&dynamic_path[..], |elem| {
        *elem = Elem::string("final value");
        true
    });

    println!("✓ All path syntaxes work seamlessly via IntoPath trait");

    match stringify(&doc) {
        Ok(yaml) => {
            println!("\nFinal result:");
            println!("{}", yaml);
        }
        Err(e) => eprintln!("Error: {}", e),
    }

    // Example 6: Practical use case - Update all image tags
    println!("6. Practical Example: Update Container Image Tags\n");

    let yaml = r#"
services:
  frontend:
    image: myapp/frontend:v1.0.0
    replicas: 3
  backend:
    image: myapp/backend:v1.0.0
    replicas: 5
  cache:
    image: redis:6.0
    replicas: 1
  database:
    image: postgres:13
    replicas: 1
"#;

    let mut doc = parse(yaml).expect("Failed to parse");

    // Update all myapp images to v2.0.0
    let mut updated_count = 0;
    doc.visit_values_mut(&["services", "*", "image"], |elem| {
        if let Some(image) = elem.node().as_str() {
            if image.starts_with("myapp/") {
                let new_image = image.replace("v1.0.0", "v2.0.0");
                *elem = Elem::string(new_image).with_comment("Updated to v2.0.0");
                updated_count += 1;
                return true;
            }
        }
        false
    });

    println!("Updated {} application images to v2.0.0", updated_count);

    match stringify(&doc) {
        Ok(yaml) => {
            println!("\nUpdated service configuration:");
            println!("{}", "─".repeat(50));
            println!("{}", yaml);
            println!("{}", "─".repeat(50));
        }
        Err(e) => eprintln!("Error: {}", e),
    }

    // Example 7: Conditional modifications
    println!("\n7. Conditional Modifications\n");

    let yaml = r#"
environments:
  development:
    replicas: 1
    resources:
      cpu: 0.5
      memory: 512Mi
  staging:
    replicas: 2
    resources:
      cpu: 1.0
      memory: 1Gi
  production:
    replicas: 5
    resources:
      cpu: 2.0
      memory: 2Gi
"#;

    let mut doc = parse(yaml).expect("Failed to parse");

    // Scale up all non-development environments
    doc.visit_maps_mut(&["environments", "*"], |env_map| {
        // Check if this is not development
        let is_dev = env_map.iter()
            .filter(|(_, elem)| elem.node().as_i64() == Some(1))
            .count() > 0;

        if !is_dev {
            // Find and update replicas
            for (key, value) in env_map.iter_mut() {
                if key == "replicas" {
                    if let Some(current) = value.node().as_i64() {
                        *value = Elem::number(current * 2)
                            .with_comment("Scaled up 2x");
                    }
                }
            }
            return true;
        }
        false
    });

    match stringify(&doc) {
        Ok(yaml) => {
            println!("Scaled up non-development environments:");
            println!("{}", "─".repeat(50));
            println!("{}", yaml);
            println!("{}", "─".repeat(50));
        }
        Err(e) => eprintln!("Error: {}", e),
    }

    println!("\n=== Key Takeaways ===\n");
    println!("✓ Use visit_values_mut() to modify individual values at a path");
    println!("✓ Use visit_sequences_mut() to modify arrays/lists");
    println!("✓ Use visit_maps_mut() to modify objects/dictionaries");
    println!("✓ Use '*' wildcard to apply changes to all matching paths");
    println!("✓ Return true from visitor to indicate modifications were made");
    println!("✓ Return false to indicate no changes (optimization)");
    println!("✓ IntoPath trait accepts &[&str], Vec<&str>, and other formats");
    println!("✓ Combine paths and wildcards for powerful bulk edits\n");
}
