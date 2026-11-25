#!/usr/bin/env bash
# Fetch the pinned toml-test suite used by tests/toml_test.rs.
#
# Usage: scripts/fetch-toml-test.sh [DEST]
#   DEST defaults to tests/toml-test-data next to this crate.
#
# The suite is downloaded as the source archive of the pinned commit (no
# nested git checkout), and only its tests/ directory is kept. Idempotent:
# when DEST already holds the pinned commit, nothing happens.
set -euo pipefail

COMMIT="96fd088448ec0f4d9f061c730ed7854399c46faf"
ARCHIVE_URL="https://codeload.github.com/toml-lang/toml-test/tar.gz/$COMMIT"
# Cases listed for each version (valid + invalid .toml files).
EXPECTED_1_0=717
EXPECTED_1_1=720

CRATE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="${1:-$CRATE_DIR/tests/toml-test-data}"
MARKER="$DEST/.pinned-commit"

if [ -f "$MARKER" ] && [ "$(cat "$MARKER")" = "$COMMIT" ]; then
  echo "toml-test $COMMIT already present at $DEST"
  exit 0
fi

if [ -e "$DEST" ]; then
  echo "error: $DEST exists but does not hold toml-test $COMMIT; remove it first." >&2
  exit 1
fi

tmp="$(mktemp -d "${TMPDIR:-/tmp}/toml-test.XXXXXX")"
trap 'rm -rf "$tmp"' EXIT

curl --fail --silent --show-error --location --retry 3 "$ARCHIVE_URL" -o "$tmp/toml-test.tar.gz"
mkdir "$tmp/src"
tar -xzf "$tmp/toml-test.tar.gz" -C "$tmp/src" --strip-components=1

count() { grep -c '\.toml$' "$tmp/src/tests/files-toml-$1"; }
if [ "$(count 1.0.0)" != "$EXPECTED_1_0" ] || [ "$(count 1.1.0)" != "$EXPECTED_1_1" ]; then
  echo "error: expected $EXPECTED_1_0 / $EXPECTED_1_1 cases, found $(count 1.0.0) / $(count 1.1.0)." >&2
  exit 1
fi

mkdir "$tmp/data"
mv "$tmp/src/tests" "$tmp/data/tests"
cp "$tmp/src/LICENSE" "$tmp/data/LICENSE"
echo "$COMMIT" > "$tmp/data/.pinned-commit"
mkdir -p "$(dirname "$DEST")"
mv "$tmp/data" "$DEST"
echo "toml-test $COMMIT fetched to $DEST ($EXPECTED_1_0 cases for 1.0, $EXPECTED_1_1 for 1.1)"
