# Changelog

Notable changes to forgo_lib_yaml. Versions follow [Semantic
Versioning](https://semver.org).

## Unreleased

The first release (not yet published).

- A YAML 1.2.2 parser that passes all 402 cases of the official
  yaml-test-suite with exact results, plus the YAML 1.2.2 rules the suite
  skips.
- An emitter, and editing that keeps a file's layout: comments, blank
  lines, indentation and quoting stay where nothing changed.
- `Doc::to_string_preserving` and `Doc::to_stream_string_preserving`
  return an error rather than re-render; `try_normalize_string_list`
  refuses lists with items that aren't strings.
- Multi-document streams (`Doc::from_stream`, `Doc::to_stream_string`),
  path-based edits, and `edit_file_in_place`.
- No dependencies, no `unsafe`. Minimum supported Rust version: 1.88.
