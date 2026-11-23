//! Benchmark inputs: large synthetic documents of common shapes, one small
//! config document, and the valid files of the yaml-test-suite.

use std::fmt::Write as _;

pub struct Input {
    pub name: &'static str,
    /// What the input exercises, for the report.
    pub about: &'static str,
    /// Documents in the stream, to catch a loader that stops early.
    pub docs: usize,
    pub text: String,
}

fn k8s_manifests(n: usize) -> String {
    let mut s = String::new();
    for i in 0..n {
        let _ = write!(
            s,
            r#"---
# Deployment {i}
apiVersion: apps/v1
kind: Deployment
metadata:
  name: service-{i}
  namespace: production
  labels: {{app: service-{i}, tier: backend, team: "platform"}}
  annotations:
    deployment.kubernetes.io/revision: "{i}"
spec:
  replicas: 3
  selector:
    matchLabels:
      app: service-{i}
  template:
    metadata:
      labels:
        app: service-{i}
    spec:
      containers:
        - name: app
          image: registry.example.com/service-{i}:1.{i}.0
          ports:
            - containerPort: 8080
              protocol: TCP
          env:
            - name: LOG_LEVEL
              value: info
            - name: CONFIG
              value: '{{"retries": 3, "timeout": "30s"}}'
          resources:
            limits: {{cpu: 500m, memory: 512Mi}}
            requests: {{cpu: 250m, memory: 256Mi}}
          command: ["/bin/app", "--port", "8080"]
"#
        );
    }
    s
}

fn ci_workflows(n: usize) -> String {
    let mut s = String::new();
    for i in 0..n {
        let _ = write!(
            s,
            r#"---
name: CI {i}
on:
  push:
    branches: [main, "release/**"]
  pull_request:
defaults: &defaults
  runs-on: ubuntu-latest
  timeout-minutes: 30
jobs:
  build-{i}:
    <<: *defaults
    steps:
      - uses: actions/checkout@v4
      - name: Build
        run: |
          cargo build --workspace --all-targets
          cargo test --workspace -- --nocapture
      - name: Summary
        run: >
          echo "build {i} finished"
          && echo done
        env:
          RUSTFLAGS: -D warnings # fail on warnings
"#
        );
    }
    s
}

fn deep_nesting(depth: usize, repeats: usize) -> String {
    let mut s = String::new();
    for r in 0..repeats {
        let _ = writeln!(s, "doc{r}:");
        for d in 0..depth {
            let _ = writeln!(s, "{}level{d}:", "  ".repeat(d + 1));
        }
        let _ = writeln!(s, "{}- leaf", "  ".repeat(depth + 1));
        let _ = writeln!(s, "flow{r}: {}{}", "[".repeat(depth), "]".repeat(depth));
    }
    s
}

fn long_scalars(n: usize, len: usize) -> String {
    let line = "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor.";
    let mut s = String::new();
    for i in 0..n {
        let _ = writeln!(s, "literal{i}: |");
        let mut written = 0;
        while written < len {
            let _ = writeln!(s, "  {line}");
            written += line.len();
        }
        let _ = writeln!(s, "folded{i}: >");
        written = 0;
        while written < len {
            let _ = writeln!(s, "  {line}");
            written += line.len();
        }
        let quoted = line.repeat(len / line.len()).replace('o', "\\u00f6");
        let _ = writeln!(s, "quoted{i}: \"{quoted}\"");
        let _ = writeln!(s, "plain{i}: {}", line.repeat(len / line.len()));
    }
    s
}

fn flow_heavy(n: usize) -> String {
    let mut s = String::from("records:\n");
    for i in 0..n {
        let _ = writeln!(
            s,
            "  - {{id: {i}, name: \"record {i}\", tags: [a, b, c], score: {i}.5, active: true, meta: {{k: v, n: null}}}}"
        );
    }
    s
}

/// Representative large inputs, each around 0.5–1.5 MB.
pub fn synthetic_inputs() -> Vec<Input> {
    vec![
        Input {
            name: "k8s",
            docs: 800,
            about: "800 Kubernetes Deployments (block maps, flow maps, quoted strings)",
            text: k8s_manifests(800),
        },
        Input {
            name: "CI",
            docs: 1500,
            about: "1,500 CI workflows (anchors, merge keys, block scalars, comments)",
            text: ci_workflows(1500),
        },
        Input {
            name: "deep",
            docs: 1,
            about: "40 × 120-level nested mappings and flow sequences",
            text: deep_nesting(120, 40),
        },
        Input {
            name: "long",
            docs: 1,
            about: "long literal, folded, double-quoted and plain scalars",
            text: long_scalars(40, 8000),
        },
        Input {
            name: "flow",
            docs: 1,
            about: "8,000 flow mappings with nested flow collections",
            text: flow_heavy(8000),
        },
    ]
}

/// One Kubernetes Deployment, about 1 KB: a typical small config file.
pub fn small_document() -> Input {
    Input {
        name: "1 KB",
        docs: 1,
        about: "one Kubernetes Deployment",
        text: k8s_manifests(1),
    }
}

/// The valid inputs of the official yaml-test-suite, as `(id, text)`.
pub fn suite_valid() -> Vec<(String, String)> {
    crate::suite::discover()
        .into_iter()
        .filter(|c| !c.error)
        .map(|c| (c.id, c.yaml))
        .collect()
}
