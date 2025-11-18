# forgo_lib_yaml

Zero-dependency YAML subset parser/editor.

## Supported subset

- Mappings: `key: value` (string keys, string values)
- Sequences:
  - Dash lists:
    ```yaml
    list:
      - a
      - b
    ```
  - Inline arrays (single or multi-line):
    ```yaml
    list: [a, b, c]
    list: [
      "a",
      "b",
    ]
    ```
- Scalars: bare or `"quoted"` strings
- Indentation determines structure; spaces only

## Non-goals (v0)

- Comments/anchors/tags, block scalars (`|`/`>`), multi-doc (`---`)
- Type semantics (everything is string)
- Numeric/bool/null typing

## Public API

```rust
use forgo_lib_yaml::{Doc, Node, Seg, normalize_string_list};

// Parse
let mut doc = Doc::from_str("jobs: { build: { strategy: { matrix: { project: [b, a] } } } }")?;

// Edit + normalize
let path = &["jobs".into(), "build".into(), "strategy".into(), "matrix".into(), "project".into()];
doc.visit_sequences_mut(path, |seq| {
    if !seq.iter().any(|n| n.as_str() == Some("tooling")) {
        seq.push(Node::Str("tooling".into()));
    }
    normalize_string_list(seq)
});

// Emit canonical YAML
println!("{}", doc.to_string()?);
# Ok::<(), Box<dyn std::error::Error>>(())
```
