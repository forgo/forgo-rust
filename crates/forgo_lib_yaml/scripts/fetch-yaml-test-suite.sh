#!/usr/bin/env bash
# Fetch the pinned official yaml-test-suite data release used by
# tests/yaml_test_suite.rs.
#
# Usage: scripts/fetch-yaml-test-suite.sh [DEST]
#   DEST defaults to tests/yaml-test-suite-data next to this crate.
#
# The data is downloaded as the source archive of the pinned commit (the
# commit the tag points to), so the content is fixed even if the tag moves,
# and no nested git checkout is created. Idempotent: when DEST already
# holds the pinned commit, nothing happens.
set -euo pipefail

TAG="data-2022-01-17"
COMMIT="6e6c296ae9c9d2d5c4134b4b64d01b29ac19ff6f"
ARCHIVE_URL="https://codeload.github.com/yaml/yaml-test-suite/tar.gz/$COMMIT"
EXPECTED_CASES=402

CRATE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="${1:-$CRATE_DIR/tests/yaml-test-suite-data}"
MARKER="$DEST/.pinned-commit"

if [ -f "$MARKER" ] && [ "$(cat "$MARKER")" = "$COMMIT" ]; then
  echo "yaml-test-suite $TAG already present at $DEST"
  exit 0
fi

if [ -e "$DEST" ]; then
  echo "error: $DEST exists but does not hold $TAG ($COMMIT); remove it first." >&2
  exit 1
fi

tmp="$(mktemp -d "${TMPDIR:-/tmp}/yaml-test-suite.XXXXXX")"
trap 'rm -rf "$tmp"' EXIT

curl --fail --silent --show-error --location --retry 3 "$ARCHIVE_URL" -o "$tmp/suite.tar.gz"
mkdir "$tmp/data"
tar -xzf "$tmp/suite.tar.gz" -C "$tmp/data" --strip-components=1

cases="$(find "$tmp/data" -name in.yaml -not -path '*/name/*' -not -path '*/tags/*' | wc -l | tr -d ' ')"
if [ "$cases" != "$EXPECTED_CASES" ]; then
  echo "error: expected $EXPECTED_CASES cases in $TAG, found $cases." >&2
  exit 1
fi

echo "$COMMIT" > "$tmp/data/.pinned-commit"
mkdir -p "$(dirname "$DEST")"
mv "$tmp/data" "$DEST"
echo "yaml-test-suite $TAG fetched to $DEST ($cases cases)"
