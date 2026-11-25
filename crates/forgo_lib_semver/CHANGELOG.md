# Changelog

Notable changes to forgo_lib_semver. Versions follow [Semantic
Versioning](https://semver.org).

## Unreleased

The first release (not yet published).

- `Version`: Semantic Versioning 2.0.0, parsed exactly as the
  specification defines it, with numbers of any size. Precedence
  (§11), bumping (§7–8 and npm's `inc`), `diff`, and lenient parsing
  and coercion when asked for.
- `Req`: version requirements in Cargo's syntax (the `semver` crate's
  rules, without its limits) and npm's (node-semver's results).
- Exact set operations on requirements, across both syntaxes:
  intersection, inclusion, equivalence, lowest version, simplification,
  each answer with the version that proves it.
- `npm`: node-semver's functions, with node-semver's results.
- No dependencies, no `unsafe`, `no_std` with `alloc`. Minimum supported
  Rust version: 1.88.
