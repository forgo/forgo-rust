#!/usr/bin/env bash
# Fetch node-semver at a pinned commit and record what it does on the
# inputs of its own test suite, for tests/npm.rs. Needs Node.js.
#
# Usage: scripts/fetch-node-semver.sh [DEST]
#   DEST defaults to tests/node-semver-data next to this crate.
#
# node-semver is downloaded as the source archive of the pinned commit (no
# nested git checkout), and scripts/capture-node-semver.js runs its tests
# (test/functions and test/ranges) with a stand-in for `tap`, recording
# every call they make and its result into DEST/calls.json. Idempotent:
# when DEST already holds the pinned commit, nothing happens.
set -euo pipefail

VERSION="7.8.6"
COMMIT="b37b033b585ad5e575afeccadb426ea41bc6cea4"
ARCHIVE_URL="https://codeload.github.com/npm/node-semver/tar.gz/$COMMIT"
MIN_CALLS=5000

CRATE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="${1:-$CRATE_DIR/tests/node-semver-data}"
MARKER="$DEST/.pinned-commit"

if [ -f "$MARKER" ] && [ "$(cat "$MARKER")" = "$COMMIT" ] && [ -s "$DEST/calls.json" ]; then
  echo "node-semver $VERSION already recorded at $DEST"
  exit 0
fi

if [ -e "$DEST" ]; then
  echo "error: $DEST exists but does not hold node-semver $VERSION ($COMMIT); remove it first." >&2
  exit 1
fi

command -v node >/dev/null || { echo "error: Node.js is needed to run node-semver." >&2; exit 1; }

tmp="$(mktemp -d "${TMPDIR:-/tmp}/node-semver.XXXXXX")"
trap 'rm -rf "$tmp"' EXIT

curl --fail --silent --show-error --location --retry 3 "$ARCHIVE_URL" -o "$tmp/node-semver.tar.gz"
mkdir -p "$tmp/data/node-semver"
tar -xzf "$tmp/node-semver.tar.gz" -C "$tmp/data/node-semver" --strip-components=1

found="$(node -p "require('$tmp/data/node-semver/package.json').version")"
if [ "$found" != "$VERSION" ]; then
  echo "error: expected node-semver $VERSION at $COMMIT, found $found." >&2
  exit 1
fi

node "$CRATE_DIR/scripts/capture-node-semver.js" "$tmp/data/node-semver" > "$tmp/data/calls.json"
calls="$(node -p "require('$tmp/data/calls.json').calls.length")"
if [ "$calls" -lt "$MIN_CALLS" ]; then
  echo "error: recorded only $calls calls (expected at least $MIN_CALLS)." >&2
  exit 1
fi

echo "$COMMIT" > "$tmp/data/.pinned-commit"
mkdir -p "$(dirname "$DEST")"
mv "$tmp/data" "$DEST"
echo "node-semver $VERSION recorded to $DEST ($calls calls)"
