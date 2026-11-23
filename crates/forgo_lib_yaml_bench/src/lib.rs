//! Inputs and adapters for comparing YAML parsers.
//!
//! Every adapter returns a count (events or documents) so the work can't
//! be optimized away, and `Err` when the parser rejects the input.

use std::fmt::Write as _;
use std::path::Path;

pub struct Input {
    pub name: String,
    pub text: String,
}

// ---------------------------------------------------------------------------
// Synthetic inputs
// ---------------------------------------------------------------------------

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
        let _ = write!(s, "folded{i}: >\n");
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
            name: "k8s-manifests".into(),
            text: k8s_manifests(800),
        },
        Input {
            name: "ci-workflows".into(),
            text: ci_workflows(1500),
        },
        Input {
            name: "deep-nesting".into(),
            text: deep_nesting(120, 40),
        },
        Input {
            name: "long-scalars".into(),
            text: long_scalars(40, 8000),
        },
        Input {
            name: "flow-heavy".into(),
            text: flow_heavy(8000),
        },
    ]
}

/// Valid inputs of the official yaml-test-suite (fetched by
/// `crates/forgo_lib_yaml/scripts/fetch-yaml-test-suite.sh`).
pub fn suite_inputs() -> Vec<Input> {
    let root =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../forgo_lib_yaml/tests/yaml-test-suite-data");
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(&root) else {
        return out;
    };
    let mut dirs: Vec<_> = entries.filter_map(Result::ok).map(|e| e.path()).collect();
    dirs.sort();
    for d in dirs {
        let id = d.file_name().unwrap().to_string_lossy().to_string();
        if id == "name" || id == "tags" || id.starts_with('.') || !d.is_dir() {
            continue;
        }
        let mut cases = vec![(id.clone(), d.clone())];
        if !d.join("in.yaml").exists() {
            cases.clear();
            let mut subs: Vec<_> = std::fs::read_dir(&d)
                .unwrap()
                .filter_map(Result::ok)
                .map(|e| e.path())
                .collect();
            subs.sort();
            for s in subs {
                cases.push((
                    format!("{id}/{}", s.file_name().unwrap().to_string_lossy()),
                    s,
                ));
            }
        }
        for (name, dir) in cases {
            if dir.join("error").exists() {
                continue;
            }
            if let Ok(text) = std::fs::read_to_string(dir.join("in.yaml")) {
                out.push(Input { name, text });
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Adapters
// ---------------------------------------------------------------------------

pub type Adapter = fn(&str) -> Result<usize, String>;

/// forgo_lib_yaml event stream.
pub fn forgo_events(s: &str) -> Result<usize, String> {
    let mut n = 0;
    for r in forgo_lib_yaml::events(s) {
        r.map_err(|e| e.to_string())?;
        n += 1;
    }
    Ok(n)
}

/// forgo_lib_yaml documents (`Doc`).
pub fn forgo_load(s: &str) -> Result<usize, String> {
    forgo_lib_yaml::Doc::from_stream(s)
        .map(|d| d.len())
        .map_err(|e| e.to_string())
}

/// libyaml (the C parser under serde_yaml, via its Rust transpilation).
pub fn libyaml_events(s: &str) -> Result<usize, String> {
    use std::mem::MaybeUninit;
    use unsafe_libyaml as y;
    // SAFETY: standard libyaml parser lifecycle; `s` outlives the parser.
    unsafe {
        let mut parser = MaybeUninit::<y::yaml_parser_t>::uninit();
        if !y::yaml_parser_initialize(parser.as_mut_ptr()).ok {
            return Err("libyaml init failed".into());
        }
        let mut parser = parser.assume_init();
        y::yaml_parser_set_input_string(&mut parser, s.as_ptr(), s.len() as _);
        let mut n = 0usize;
        let result = loop {
            let mut event = MaybeUninit::<y::yaml_event_t>::uninit();
            if !y::yaml_parser_parse(&mut parser, event.as_mut_ptr()).ok {
                break Err("libyaml rejected the input".to_string());
            }
            let mut event = event.assume_init();
            let done = event.type_ == y::YAML_STREAM_END_EVENT;
            y::yaml_event_delete(&mut event);
            n += 1;
            if done {
                break Ok(n);
            }
        };
        y::yaml_parser_delete(&mut parser);
        result
    }
}

/// serde_yaml into `serde_yaml::Value`, one per document.
pub fn serde_yaml_load(s: &str) -> Result<usize, String> {
    use serde::Deserialize as _;
    let mut n = 0;
    for doc in serde_yaml::Deserializer::from_str(s) {
        serde_yaml::Value::deserialize(doc).map_err(|e| e.to_string())?;
        n += 1;
    }
    Ok(n)
}

/// yaml-rust2 event stream.
pub fn yaml_rust2_events(s: &str) -> Result<usize, String> {
    let mut p = yaml_rust2::parser::Parser::new_from_str(s);
    let mut n = 0;
    loop {
        let (ev, _) = p.next_token().map_err(|e| e.to_string())?;
        n += 1;
        if ev == yaml_rust2::parser::Event::StreamEnd {
            return Ok(n);
        }
    }
}

/// yaml-rust2 documents.
pub fn yaml_rust2_load(s: &str) -> Result<usize, String> {
    yaml_rust2::YamlLoader::load_from_str(s)
        .map(|d| d.len())
        .map_err(|e| e.to_string())
}

/// saphyr-parser event stream.
pub fn saphyr_events(s: &str) -> Result<usize, String> {
    let mut n = 0;
    for r in saphyr_parser::Parser::new_from_str(s) {
        r.map_err(|e| e.to_string())?;
        n += 1;
    }
    Ok(n)
}

/// saphyr documents.
pub fn saphyr_load(s: &str) -> Result<usize, String> {
    use saphyr::LoadableYamlNode as _;
    saphyr::Yaml::load_from_str(s)
        .map(|d| d.len())
        .map_err(|e| e.to_string())
}

pub const EVENT_PARSERS: &[(&str, Adapter)] = &[
    ("forgo_lib_yaml", forgo_events),
    ("libyaml", libyaml_events),
    ("yaml-rust2", yaml_rust2_events),
    ("saphyr-parser", saphyr_events),
];

pub const LOADERS: &[(&str, Adapter)] = &[
    ("forgo_lib_yaml", forgo_load),
    ("serde_yaml", serde_yaml_load),
    ("yaml-rust2", yaml_rust2_load),
    ("saphyr", saphyr_load),
];
