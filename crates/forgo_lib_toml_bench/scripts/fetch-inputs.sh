#!/usr/bin/env bash
# Fetch the real-world TOML files the report measures, at pinned commits.
#
# Usage: scripts/fetch-inputs.sh
#
# Files go to inputs/ next to this crate (not committed). Each is checked
# against its SHA-256 below; existing files that match are kept.
set -euo pipefail

CRATE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="$CRATE_DIR/inputs"
mkdir -p "$DEST"

ZED=f38d614b4b58bb44cc19d08ef5dd3efba918f9a3
RUFF=29e26e59b27398bd3b3b905b485811104a43d19c

# name  url  sha256 (filled in below)
FILES=(
  "zed-Cargo.toml https://raw.githubusercontent.com/zed-industries/zed/$ZED/Cargo.toml"
  "zed-Cargo.lock https://raw.githubusercontent.com/zed-industries/zed/$ZED/Cargo.lock"
  "ruff-pyproject.toml https://raw.githubusercontent.com/astral-sh/ruff/$RUFF/pyproject.toml"
)

sha() { shasum -a 256 "$1" | cut -d ' ' -f 1; }

for entry in "${FILES[@]}"; do
  read -r name url <<<"$entry"
  want="$(grep -E "^$name " "$CRATE_DIR/scripts/inputs.sha256" | cut -d ' ' -f 2 || true)"
  if [ -f "$DEST/$name" ] && [ -n "$want" ] && [ "$(sha "$DEST/$name")" = "$want" ]; then
    continue
  fi
  tmp="$(mktemp "${TMPDIR:-/tmp}/toml-input.XXXXXX")"
  curl --fail --silent --show-error --location --retry 3 -A "forgo-bench-setup/0.1" "$url" -o "$tmp"
  got="$(sha "$tmp")"
  if [ -n "$want" ] && [ "$got" != "$want" ]; then
    rm -f "$tmp"
    echo "error: $name has SHA-256 $got, expected $want" >&2
    exit 1
  fi
  mv "$tmp" "$DEST/$name"
  echo "fetched $name ($got)"
done
