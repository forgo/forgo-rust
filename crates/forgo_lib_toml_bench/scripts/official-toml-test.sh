#!/usr/bin/env bash
# Run toml-test's own runner, built from the pinned commit, on
# forgo_lib_toml (src/bin/toml-test-decoder.rs and toml-test-encoder.rs),
# for TOML 1.1 and 1.0. Needs Go.
#
# Usage: scripts/official-toml-test.sh
set -euo pipefail

COMMIT="96fd088448ec0f4d9f061c730ed7854399c46faf"
CRATE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WORK="$CRATE_DIR/target/toml-test-$COMMIT"
RUNNER="$WORK/toml-test"

if [ ! -x "$RUNNER" ]; then
  rm -rf "$WORK/src"
  mkdir -p "$WORK/src"
  curl --fail --silent --show-error --location --retry 3 -A "forgo-bench-setup/0.1" \
    "https://codeload.github.com/toml-lang/toml-test/tar.gz/$COMMIT" | tar -xz -C "$WORK/src" --strip-components=1
  # The runner embeds the test cases of the commit it is built from.
  (cd "$WORK/src" && go build -o "$RUNNER" ./cmd/toml-test)
fi

cargo build --release --quiet --manifest-path "$CRATE_DIR/Cargo.toml" --bin toml-test-decoder --bin toml-test-encoder
BIN="$CRATE_DIR/target/release"

status=0
for version in 1.1 1.0; do
  flag=""
  [ "$version" = 1.0 ] && flag=" --toml-1.0"
  echo "== TOML $version"
  "$RUNNER" test -toml="$version" -decoder="$BIN/toml-test-decoder$flag" -encoder="$BIN/toml-test-encoder" || status=1
done
exit $status
