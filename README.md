# forgo-rust

A monorepo of Rust projects.

## Projects Overview

| Project     | Type       | Description                                                                           | Location           |
| ----------- | ---------- | ------------------------------------------------------------------------------------- | ------------------ |
| `cli-utils` | Library    | A reusable library of utilities for CLI applications (e.g., device code flow)         | `crates/cli-utils` |
| `cli-forgo` | Executable | A CLI tool that leverages `cli-utils` to implement device code flow (binary: `forgo`) | `crates/cli-forgo` |

## Repository Structure

```bash
forgo-rust/
├── Cargo.toml          # Workspace manifest
├── rust-toolchain.toml # Toolchain config
├── .gitignore          # Git ignore file
├── README.md           # Project readme
├── crates/
│   ├── cli-forgo/      # `forgo` CLI tool executable
│   │   ├── Cargo.toml
│   │   └── src/
│   └── cli-utils/      # Reusable CLI utilities library
│       ├── Cargo.toml
│       └── src/
└── .github/
    └── workflows/
         └── ci.yml     # GitHub Actions workflow (CI)

```

## Prerequisites

- **Rust Toolchain:**  
  Install Rust using [rustup](https://rustup.rs/):

  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```

- **Pinned Toolchain (Optional):**  
  The repository includes a `rust-toolchain.toml` file to ensure consistency across your development environment.

- **[cargo-make](https://github.com/sagiegurari/cargo-make):**

  Rust task runner and build tool.

  - Why Use cargo-make?
    - Chaining Tasks
    - More complex workflows can be defined beyond simple Cargo aliases.
    - Consistent interface for common tasks across different environments.

## Building the Workspace

From the root of the repository, run:

```bash
cargo build --all
```

This command compiles every crate in the workspace.

## Verifying and Checking

### 1. Syntax and Dependency Checks

Ensure your code compiles and all dependencies resolve correctly:

```bash
cargo check --all
```

### 2. Formatting

Enforce consistent code style with:

```bash
cargo fmt --all
```

To check formatting without modifying files, run:

```bash
cargo fmt -- --check
```

### 3. Linting

Catch common mistakes and enforce best practices with Clippy:

```bash
cargo clippy --all -- -D warnings
```

The `-D warnings` flag treats all warnings as errors, ensuring clean code.

### 4. Testing

Run unit and integration tests for all projects:

```bash
cargo test --all
```

## Continuous Integration

This repository includes a GitHub Actions workflow located at `.github/workflows/ci.yml`. The CI workflow automatically builds, lints, tests, and checks formatting on every push and pull request, helping maintain code quality and consistency.

## Additional Tools and Best Practices

- **Documentation:**  
  Generate and view documentation for your crates with:

  ```bash
  cargo doc --open
  ```

- **Security Audits:**  
  Check for vulnerabilities in your dependencies using:

  ```bash
  cargo audit
  ```

- **Version Control & Releases:**  
  Maintain clear, descriptive commit messages and use semantic versioning for your releases.

- **Code Reviews:**  
  Incorporate peer reviews and CI checks to ensure long-term maintainability and code quality.

## Getting Started

1. **Clone the Repository:**

   ```bash
   git clone git@github.com:yourusername/rust-monorepo.git
   cd rust-monorepo
   ```

2. **Build the Projects:**

   ```bash
   cargo build --all
   ```

3. **Run the CLI Tool:**

   Since the binary is named `forgo`, run:

   ```bash
   ./target/debug/forgo --help
   ```

4. **Clean Artifacts:**

   Remove generated artifacts from the target directory, effectively clearing the build cache for the project or workspace.

   ```bash
   cargo clean
   ```

## TODOs:

- [] test caching of GitHub workflow `ci.yaml`
