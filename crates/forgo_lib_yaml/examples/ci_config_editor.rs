//! CI Config Editor - Real-World Application Example
//!
//! This example demonstrates a complete, production-ready application that:
//! - Parses a GitHub Actions-style CI/CD configuration
//! - Provides a menu-driven interface for common operations
//! - Uses path-based editing for bulk modifications
//! - Demonstrates error handling and validation
//! - Shows round-trip preservation of comments and formatting
//!
//! This showcases how forgo_lib_yaml can be used to build tools for
//! managing complex YAML configurations in real-world scenarios.
//!
//! Run this example with: `cargo run --example ci_config_editor`

use forgo_lib_yaml::{parse, stringify, Elem, Node};
use std::io::{self, Write};

fn main() {
    println!("╔═══════════════════════════════════════════════════════════╗");
    println!("║          CI/CD Configuration Editor                       ║");
    println!("║          Powered by forgo_lib_yaml                        ║");
    println!("╚═══════════════════════════════════════════════════════════╝\n");

    // Sample CI configuration
    let initial_config = r#"
# GitHub Actions CI/CD Configuration
# This file defines the continuous integration and deployment pipeline

name: CI/CD Pipeline

on:
  push:
    branches:
      - main
      - develop
  pull_request:
    branches:
      - main

jobs:
  # Build and test the application
  build:
    runs-on: ubuntu-20.04
    steps:
      - name: Checkout code
        uses: actions/checkout@v2

      - name: Setup Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

      - name: Build
        run: cargo build --release

      - name: Run tests
        run: cargo test

  # Linting and code quality
  lint:
    runs-on: ubuntu-20.04
    steps:
      - name: Checkout code
        uses: actions/checkout@v2

      - name: Setup Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          components: clippy, rustfmt

      - name: Run clippy
        run: cargo clippy -- -D warnings

      - name: Check formatting
        run: cargo fmt -- --check

  # Deploy to production (only on main branch)
  deploy:
    runs-on: ubuntu-20.04
    needs:
      - build
      - lint
    if: github.ref == 'refs/heads/main'
    steps:
      - name: Checkout code
        uses: actions/checkout@v2

      - name: Deploy to production
        run: ./scripts/deploy.sh
        env:
          DEPLOY_KEY: ${{ secrets.DEPLOY_KEY }}
"#;

    let mut doc = match parse(initial_config) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Failed to parse CI configuration: {}", e);
            return;
        }
    };

    println!("Loaded CI/CD configuration successfully!\n");
    display_summary(&doc);

    loop {
        println!("\n╔═══════════════════════════════════════════════════════════╗");
        println!("║ What would you like to do?                                ║");
        println!("╠═══════════════════════════════════════════════════════════╣");
        println!("║ 1. Upgrade all Ubuntu runners to 22.04                    ║");
        println!("║ 2. Add caching step to all jobs                           ║");
        println!("║ 3. Update Rust toolchain version                          ║");
        println!("║ 4. Add new job (security-scan)                            ║");
        println!("║ 5. Update actions versions to latest                      ║");
        println!("║ 6. View current configuration                             ║");
        println!("║ 7. Export and exit                                        ║");
        println!("╚═══════════════════════════════════════════════════════════╝");

        print!("\nEnter your choice (1-7): ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();

        match input.trim() {
            "1" => upgrade_ubuntu_runners(&mut doc),
            "2" => add_caching_to_jobs(&mut doc),
            "3" => update_rust_toolchain(&mut doc),
            "4" => add_security_scan_job(&mut doc),
            "5" => update_actions_versions(&mut doc),
            "6" => view_configuration(&doc),
            "7" => {
                export_and_exit(&doc);
                break;
            }
            _ => println!("❌ Invalid choice. Please enter 1-7."),
        }
    }
}

/// Display a summary of the CI configuration
fn display_summary(doc: &forgo_lib_yaml::Doc) {
    println!("📋 Configuration Summary:");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    if let Some(root_map) = doc.root().node().as_map() {
        // Count jobs
        if let Some((_, jobs_elem)) = root_map.iter().find(|(k, _)| k == "jobs") {
            if let Some(jobs_map) = jobs_elem.node().as_map() {
                println!("  Jobs defined: {}", jobs_map.len());
                for (job_name, _) in jobs_map {
                    println!("    • {}", job_name);
                }
            }
        }

        // Show triggers
        if let Some((_, on_elem)) = root_map.iter().find(|(k, _)| k == "on") {
            if let Some(on_map) = on_elem.node().as_map() {
                let triggers: Vec<&str> = on_map.iter().map(|(k, _)| k.as_str()).collect();
                println!("  Triggers: {}", triggers.join(", "));
            }
        }
    }
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
}

/// Upgrade all Ubuntu runners from 20.04 to 22.04
fn upgrade_ubuntu_runners(doc: &mut forgo_lib_yaml::Doc) {
    println!("\n🔄 Upgrading Ubuntu runners to 22.04...");

    let mut count = 0;
    doc.visit_values_mut(&["jobs", "*", "runs-on"], |elem| {
        if let Some(runner) = elem.node().as_str() {
            if runner.contains("ubuntu-20.04") {
                *elem = Elem::string("ubuntu-22.04")
                    .with_comment("Upgraded from 20.04");
                count += 1;
                return true;
            }
        }
        false
    });

    if count > 0 {
        println!("✅ Successfully upgraded {} job runner(s) to Ubuntu 22.04", count);
    } else {
        println!("ℹ️  No Ubuntu 20.04 runners found to upgrade");
    }
}

/// Add caching step to all jobs
fn add_caching_to_jobs(doc: &mut forgo_lib_yaml::Doc) {
    println!("\n🔄 Adding caching step to all jobs...");

    let mut count = 0;
    doc.visit_sequences_mut(&["jobs", "*", "steps"], |steps| {
        // Check if caching already exists
        let has_cache = steps.iter().any(|step| {
            step.node().as_map()
                .and_then(|m| m.iter().find(|(k, _)| k == "name"))
                .and_then(|(_, v)| v.node().as_str())
                .map(|s| s.contains("cache") || s.contains("Cache"))
                .unwrap_or(false)
        });

        if !has_cache {
            // Build cache step
            let mut cache_step = vec![];
            cache_step.push((
                "name".to_string(),
                Elem::string("Cache dependencies"),
            ));
            cache_step.push((
                "uses".to_string(),
                Elem::string("actions/cache@v3"),
            ));

            let mut with_map = vec![];
            with_map.push((
                "path".to_string(),
                Elem::string("~/.cargo\ntarget/").prefer_block(),
            ));
            with_map.push((
                "key".to_string(),
                Elem::string("${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}"),
            ));

            cache_step.push((
                "with".to_string(),
                Elem::new(Node::Map(with_map)),
            ));

            // Insert cache step after checkout (position 1)
            steps.insert(
                1,
                Elem::new(Node::Map(cache_step))
                    .with_comment("Added by CI editor")
            );

            count += 1;
            return true;
        }
        false
    });

    if count > 0 {
        println!("✅ Added caching to {} job(s)", count);
    } else {
        println!("ℹ️  All jobs already have caching configured");
    }
}

/// Update Rust toolchain version
fn update_rust_toolchain(doc: &mut forgo_lib_yaml::Doc) {
    println!("\n🔄 Updating Rust toolchain...");

    print!("Enter new toolchain version (e.g., 'stable', '1.75', 'nightly'): ");
    io::stdout().flush().unwrap();

    let mut version = String::new();
    io::stdin().read_line(&mut version).unwrap();
    let version = version.trim();

    if version.is_empty() {
        println!("❌ Invalid version");
        return;
    }

    let mut count = 0;
    doc.visit_values_mut(&["jobs", "*", "steps", "*", "with", "toolchain"], |elem| {
        *elem = Elem::string(version)
            .with_comment(&format!("Updated to {}", version));
        count += 1;
        true
    });

    if count > 0 {
        println!("✅ Updated Rust toolchain to '{}' in {} location(s)", version, count);
    } else {
        println!("ℹ️  No Rust toolchain configuration found");
    }
}

/// Add a new security scanning job
fn add_security_scan_job(doc: &mut forgo_lib_yaml::Doc) {
    println!("\n🔄 Adding security scan job...");

    doc.visit_maps_mut(&["jobs"], |jobs_map| {
        // Check if security-scan already exists
        if jobs_map.iter().any(|(k, _)| k == "security-scan") {
            println!("ℹ️  Security scan job already exists");
            return false;
        }

        // Build security scan job
        let mut security_job = vec![];
        security_job.push((
            "runs-on".to_string(),
            Elem::string("ubuntu-22.04"),
        ));

        let steps = vec![
            {
                let mut step = vec![];
                step.push(("name".to_string(), Elem::string("Checkout code")));
                step.push(("uses".to_string(), Elem::string("actions/checkout@v2")));
                Elem::new(Node::Map(step))
            },
            {
                let mut step = vec![];
                step.push(("name".to_string(), Elem::string("Run cargo audit")));
                step.push(("run".to_string(), Elem::string("cargo install cargo-audit && cargo audit")));
                Elem::new(Node::Map(step))
            },
            {
                let mut step = vec![];
                step.push(("name".to_string(), Elem::string("Run cargo deny")));
                step.push(("run".to_string(), Elem::string("cargo install cargo-deny && cargo deny check")));
                Elem::new(Node::Map(step))
            },
        ];

        security_job.push((
            "steps".to_string(),
            Elem::new(Node::Seq(steps)),
        ));

        jobs_map.push((
            "security-scan".to_string(),
            Elem::new(Node::Map(security_job))
                .with_leading_comments(vec![
                    "Security scanning job".to_string(),
                    "Added by CI editor".to_string(),
                ])
        ));

        println!("✅ Added security-scan job successfully");
        true
    });
}

/// Update GitHub Actions versions
fn update_actions_versions(doc: &mut forgo_lib_yaml::Doc) {
    println!("\n🔄 Updating GitHub Actions to latest versions...");

    let updates = vec![
        ("actions/checkout@v2", "actions/checkout@v4"),
        ("actions-rs/toolchain@v1", "dtolnay/rust-toolchain@stable"),
        ("actions/cache@v2", "actions/cache@v3"),
    ];

    let mut total_updates = 0;

    for (old, new) in updates {
        let mut count = 0;
        doc.visit_values_mut(&["jobs", "*", "steps", "*", "uses"], |elem| {
            if let Some(action) = elem.node().as_str() {
                if action == old {
                    *elem = Elem::string(new)
                        .with_comment(&format!("Updated from {}", old));
                    count += 1;
                    return true;
                }
            }
            false
        });

        if count > 0 {
            println!("  ✓ Updated {} → {} ({} occurrences)", old, new, count);
            total_updates += count;
        }
    }

    if total_updates > 0 {
        println!("✅ Updated {} action(s) to latest versions", total_updates);
    } else {
        println!("ℹ️  All actions are already up to date");
    }
}

/// View the current configuration
fn view_configuration(doc: &forgo_lib_yaml::Doc) {
    println!("\n📄 Current Configuration:");
    println!("═══════════════════════════════════════════════════════════");

    match stringify(doc) {
        Ok(yaml) => {
            println!("{}", yaml);
        }
        Err(e) => {
            eprintln!("❌ Failed to serialize configuration: {}", e);
        }
    }

    println!("═══════════════════════════════════════════════════════════");
}

/// Export configuration and exit
fn export_and_exit(doc: &forgo_lib_yaml::Doc) {
    println!("\n💾 Exporting configuration...");

    match stringify(doc) {
        Ok(yaml) => {
            println!("\n📄 Final Configuration:");
            println!("═══════════════════════════════════════════════════════════");
            println!("{}", yaml);
            println!("═══════════════════════════════════════════════════════════");

            println!("\n✅ Configuration exported successfully!");
            println!("📝 Copy the above YAML to your .github/workflows/ci.yml file");
            println!("\n🎉 Thank you for using forgo_lib_yaml CI Config Editor!");
        }
        Err(e) => {
            eprintln!("❌ Failed to export configuration: {}", e);
        }
    }
}
