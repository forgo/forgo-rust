# Changelog

Notable changes to forgo_lib_toml. Versions follow [Semantic
Versioning](https://semver.org).

## Unreleased

The first release (not yet published).

- `Document`: TOML 1.1 (the default) and 1.0, read through borrowed
  `Table`, `Array` and `Value` views. Passes every case of toml-test for
  both versions, and toml-test's own runner.
- Errors with a kind, a message, and a line and column.
- `Item`: values built in code, encoded as TOML 1.0, and displayed as
  inline TOML.
- Editing that keeps a file's layout: `Document::set`, `remove`,
  `edit_array` and `edit_string_set`, paths through tables and arrays
  (`Seg`, `IntoPath`), `edit_file_in_place`, and
  `ParseOptions::for_editing`. Each edit is checked, and an edit that
  can't keep the layout is refused.
- No dependencies, no `unsafe`. Minimum supported Rust version: 1.88.
