//! Fluent Builders Example - Creating YAML Documents from Scratch
//!
//! This example demonstrates how to programmatically build YAML documents using:
//! - Node and Elem constructors for creating basic structures
//! - Fluent builder methods for adding metadata (comments, anchors, styling)
//! - Combining builders to create complex, well-documented YAML configs
//!
//! The fluent API allows you to chain method calls for concise, readable code
//! while maintaining full control over formatting and documentation.
//!
//! Run this example with: `cargo run --example fluent_builders`

use forgo_lib_yaml::{parse, stringify, Elem, MapKey, Node};

fn main() {
    println!("=== Building YAML Documents with Fluent Builders ===\n");

    // Example 1: Simple constructors
    println!("1. Basic Constructors\n");

    // Node provides static constructors for all YAML types
    let _string_node = Node::string("hello");
    let _number_node = Node::number(42);
    let _bool_node = Node::boolean(true);
    let _null_node = Node::null();
    let _map_node = Node::map(); // Empty map
    let _seq_node = Node::seq(); // Empty sequence

    // Elem wraps Node and adds metadata (comments, anchors, styling)
    // Elem has the same constructors as Node
    let _name = Elem::string("my-application");
    let _version = Elem::number("2.1.0");
    let _enabled = Elem::boolean(true);
    let _timeout = Elem::null();

    println!("✓ Created basic elements: string, number, boolean, null\n");

    // Example 2: Adding metadata with fluent builders
    println!("2. Fluent Builders for Metadata\n");

    // The fluent API allows chaining methods to add metadata
    let _documented_name = Elem::string("production-server")
        .with_comment("Primary production server");

    let _versioned_config = Elem::number("3.0.0")
        .with_leading_comments(vec![
            "Application Version".to_string(),
            "Updated: 2024-11-17".to_string(),
        ])
        .with_comment("Semantic versioning");

    // Anchors allow you to reference values elsewhere in the document
    let _base_config = Elem::map()
        .with_anchor("base")
        .with_comment("Base configuration template");

    // Styling preferences guide the serializer
    let _multiline_text = Elem::string("This is a long text\nthat spans multiple lines\nand should use block style")
        .prefer_block(); // Hint: use | or > block scalar

    let _quoted_string = Elem::string("needs-quotes: true")
        .prefer_quoted(); // Hint: use "..." quoting

    println!("✓ Created elements with comments, anchors, and style hints\n");

    // Example 3: Building a complete configuration
    println!("3. Building a Complete Configuration Document\n");

    // Build a server configuration from scratch
    let mut server_config: Vec<(MapKey, Elem)> = vec![];

    // Add application metadata
    server_config.push((
        "name".to_string(),
        Elem::string("web-api")
            .with_comment("Application identifier"),
    ));

    server_config.push((
        "version".to_string(),
        Elem::string("1.0.0")
            .with_comment("Semantic version"),
    ));

    server_config.push((
        "environment".to_string(),
        Elem::string("production")
            .with_comment("deployment environment"),
    ));

    // Build server settings sub-map
    let mut server_settings: Vec<(MapKey, Elem)> = vec![];
    server_settings.push((
        "host".to_string(),
        Elem::string("0.0.0.0"),
    ));
    server_settings.push((
        "port".to_string(),
        Elem::number(8080),
    ));
    server_settings.push((
        "workers".to_string(),
        Elem::number(4)
            .with_comment("Number of worker threads"),
    ));

    server_config.push((
        "server".to_string(),
        Elem::new(Node::Map(server_settings))
            .with_leading_comments(vec!["Server configuration".to_string()])
            .prefer_block(),
    ));

    // Build features list
    let features = vec![
        Elem::string("authentication"),
        Elem::string("rate-limiting"),
        Elem::string("caching")
            .with_comment("Redis-backed cache"),
        Elem::string("logging"),
    ];

    server_config.push((
        "features".to_string(),
        Elem::new(Node::Seq(features))
            .with_leading_comments(vec!["Enabled features".to_string()]),
    ));

    // Build database configuration
    let mut db_config: Vec<(MapKey, Elem)> = vec![];
    db_config.push((
        "driver".to_string(),
        Elem::string("postgresql"),
    ));
    db_config.push((
        "host".to_string(),
        Elem::string("localhost"),
    ));
    db_config.push((
        "port".to_string(),
        Elem::number(5432),
    ));
    db_config.push((
        "database".to_string(),
        Elem::string("production_db"),
    ));
    db_config.push((
        "pool_size".to_string(),
        Elem::number(10)
            .with_comment("Connection pool size"),
    ));

    server_config.push((
        "database".to_string(),
        Elem::new(Node::Map(db_config))
            .with_leading_comments(vec![
                "Database configuration".to_string(),
                "PostgreSQL connection settings".to_string(),
            ])
            .with_anchor("db_config")
            .prefer_block(),
    ));

    // Create the root element and build a temporary YAML to parse into a Doc
    let root_elem = Elem::new(Node::Map(server_config))
        .with_leading_comments(vec![
            "Web API Server Configuration".to_string(),
            "Generated by forgo_lib_yaml fluent builders".to_string(),
            "".to_string(),
            "This configuration demonstrates:".to_string(),
            "- Fluent builder API for creating elements".to_string(),
            "- Adding inline and leading comments".to_string(),
            "- Using anchors for referencing".to_string(),
            "- Style preferences for serialization".to_string(),
        ]);

    // For demonstration, we'll create a minimal doc and replace its root
    // In a real application, you'd typically use parse() then modify
    let mut doc = parse("temp: value").unwrap();
    *doc.root_mut() = root_elem;

    // Serialize and display
    match stringify(&doc) {
        Ok(yaml) => {
            println!("Generated YAML configuration:");
            println!("{}", "=".repeat(60));
            println!("{}", yaml);
            println!("{}", "=".repeat(60));
        }
        Err(e) => {
            eprintln!("Failed to serialize: {}", e);
        }
    }

    // Example 4: Chaining all metadata builders
    println!("\n4. Chaining Multiple Builders\n");

    let _fully_decorated = Elem::string("important-value")
        .with_leading_comments(vec![
            "Critical configuration value".to_string(),
            "DO NOT MODIFY without approval".to_string(),
        ])
        .with_comment("See documentation for details")
        .with_anchor("critical_value")
        .prefer_quoted();

    println!("✓ Created element with all metadata types chained together");
    println!("  - Leading comments: 2 lines");
    println!("  - Trailing comment: yes");
    println!("  - Anchor: critical_value");
    println!("  - Style hint: prefer quoted\n");

    // Example 5: Building nested structures programmatically
    println!("5. Building Nested Structures\n");

    // Create a CI/CD configuration
    let mut ci_config: Vec<(MapKey, Elem)> = vec![];

    // Build jobs map
    let mut jobs: Vec<(MapKey, Elem)> = vec![];

    // Build job
    let mut build_job: Vec<(MapKey, Elem)> = vec![];
    build_job.push(("runs-on".into(), Elem::string("ubuntu-latest")));

    let build_steps = vec![
        Elem::string("checkout code"),
        Elem::string("setup rust"),
        Elem::string("cargo build --release")
            .with_comment("Build in release mode"),
        Elem::string("cargo test")
            .with_comment("Run test suite"),
    ];

    build_job.push((
        "steps".into(),
        Elem::new(Node::Seq(build_steps))
            .with_comment("Build steps"),
    ));

    jobs.push((
        "build".into(),
        Elem::new(Node::Map(build_job))
            .with_comment("Build and test job"),
    ));

    // Deploy job
    let mut deploy_job: Vec<(MapKey, Elem)> = vec![];
    deploy_job.push(("runs-on".into(), Elem::string("ubuntu-latest")));
    deploy_job.push((
        "needs".into(),
        Elem::new(Node::Seq(vec![Elem::string("build")]))
            .with_comment("Wait for build to complete"),
    ));

    let deploy_steps = vec![
        Elem::string("checkout code"),
        Elem::string("deploy to production"),
    ];

    deploy_job.push((
        "steps".into(),
        Elem::new(Node::Seq(deploy_steps)),
    ));

    jobs.push((
        "deploy".into(),
        Elem::new(Node::Map(deploy_job))
            .with_comment("Deployment job"),
    ));

    ci_config.push((
        "jobs".into(),
        Elem::new(Node::Map(jobs))
            .with_leading_comments(vec!["CI/CD Jobs".to_string()]),
    ));

    let ci_root = Elem::new(Node::Map(ci_config))
        .with_leading_comments(vec!["CI/CD Pipeline Configuration".to_string()]);

    let mut ci_doc = parse("temp: value").unwrap();
    *ci_doc.root_mut() = ci_root;

    match stringify(&ci_doc) {
        Ok(yaml) => {
            println!("Generated CI/CD configuration:");
            println!("{}", "=".repeat(60));
            println!("{}", yaml);
            println!("{}", "=".repeat(60));
        }
        Err(e) => {
            eprintln!("Failed to serialize: {}", e);
        }
    }

    println!("\n=== Key Takeaways ===\n");
    println!("✓ Use Node::* or Elem::* constructors to create elements");
    println!("✓ Chain .with_comment() for inline comments");
    println!("✓ Chain .with_leading_comments() for block comments above elements");
    println!("✓ Chain .with_anchor() to create referenceable anchors");
    println!("✓ Chain .prefer_block() or .prefer_quoted() for style hints");
    println!("✓ All builder methods return Self, allowing unlimited chaining");
    println!("✓ Build complex nested structures using Vec and Node::Map/Seq\n");
}
