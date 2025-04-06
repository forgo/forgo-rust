# How this repo was created

- Used [rustup](https://rustup.rs/) to install Rust. It will help you manage toolchain versions.
- Created a `rust-toolchain.toml` file in repo root to ensure everyone uses the same version.
- Created the `forgo-rust` repo on GitHub
  - Added a `.gitignore` based on the Rust template
  - Chose a license like `MIT`
- Created a root `Cargo.toml` to include workspace member projects
- Create baseline projects for repo

  - `cargo new forgo-lib-cli --lib` (use `--lib` for a library)
  - `cargo new forgo --bin` (use `--bin` for an executable)

- **Create Your First Crate:**

  Inside the `crates` directory, create a new crate:

```bash
mkdir -p crates
cd crates
cargo new project1 --lib  # Use --bin if you need an executable
cd ..
```

Repeat for additional projects...

---

## 4.Task Automation with cargo-make

In addition to Cargo aliases, this repository leverages [cargo-make](https://github.com/sagiegurari/cargo-make) to streamline and chain common tasks such as formatting checks, linting, and testing. Cargo-make is a versatile task runner that lets you define multi-step workflows in a single configuration file.

### Installing cargo-make

To install cargo-make, run:

```bash
cargo install cargo-make
```

---

## 5. Set Up Development Best Practice

### a. Code Formatting

- **`rustfmt`**

  Ensure code consistency by running:

```bash
cargo fmt --all
```

Optionally, add a rustfmt.toml in your root to enforce style rules.

### b. Linting

- **Clippy**

  Use Clippy to catch common mistakes:

```bash
cargo clippy --all -- -D warnings
```

The `-D warnings` flag makes clippy treat warnings as errors, which is great for keeping code clean.

### c. Testing

- **Unit and Integration Tests:**

  Write tests within modules (using `#[cfg(test)]`) or in a dedicated `tests/` directory within each crate. Run tests with:

```bash
cargo test --all
```

### d. Building

- **Build and Check:**

  Always verify that your code builds:

```bash
cargo build --all
cargo check --all
```

---

## 6. Set Up Continuous Integration with GitHub Actions

Automate testing, linting, and formatting checks by creating a GitHub Actions workflow.

- **Create a Workflow File:**

  Create `.github/workflows/ci.yml` with contents like:

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  build:
    runs-on: ubuntu-latest

    steps:
      - name: Checkout repository
        uses: actions/checkout@v3

      - name: Set up Rust toolchain
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          override: true
          components: clippy, rustfmt

      - name: Cache cargo registry
        uses: actions/cache@v3
        with:
          path: ~/.cargo/registry
          key: ${{ runner.os }}-cargo-registry-${{ hashFiles('**/Cargo.lock') }}

      - name: Cache cargo build artifacts
        uses: actions/cache@v3
        with:
          path: target
          key: ${{ runner.os }}-cargo-build-${{ hashFiles('**/Cargo.lock') }}

      - name: Cache Cargo bin (including cargo-make)
        uses: actions/cache@v3
        with:
          path: ~/.cargo/bin
          key: cargo-bin-${{ runner.os }}-cargo-make-0.36.7

      - name: Install cargo-make if not already installed
        run: |
          if ! command -v cargo-make > /dev/null; then
            echo "Installing cargo-make..."
            cargo install cargo-make --version 0.36.7
          else
            echo "cargo-make is already installed."
          fi

      - name: Run CI tasks via cargo-make
        run: cargo make ci
```

This workflow will ensure that every push or pull request runs a build, lints your code, checks formatting, and runs tests.

---

## 7. Additional Best Practices

- **Documentation:**

  Write clear documentation and run `cargo doc --open` to generate docs for your crates.

- **Security and Dependencies:**

  Use tools like `cargo audit` to check for security vulnerabilities in your dependencies.

- **Versioning and Releases:**

  Follow [semantic versioning](https://semver.org/) for your crates. Consider tools like `cargo-release` to automate version bumps and tagging.

- **Commit Messages and Branching:**

  Use clear, descriptive commit messages (e.g., conventional commits) and maintain a branching strategy (like GitFlow or trunk-based development) to streamline collaboration and releases.

- **Modularity:**

  As your monorepo grows, keep each crate small and focused. This modularity will help manage dependencies and facilitate testing.
