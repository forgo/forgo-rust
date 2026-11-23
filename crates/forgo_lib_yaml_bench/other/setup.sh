#!/bin/sh
# Install the non-Rust parsers compared by `report --other-languages`
# into target/other/, never globally. Versions are pinned.
set -eu
PYYAML_VERSION=6.0.4
RUAMEL_VERSION=0.19.1
YAML_JS_VERSION=2.9.1
JS_YAML_VERSION=5.5.0
# go-yaml: go.yaml.in/yaml/v3, pinned in go/go.mod
here=$(cd "$(dirname "$0")" && pwd)
dest="$here/../target/other"
mkdir -p "$dest"

if command -v python3 >/dev/null; then
  python3 -m venv "$dest/venv"
  "$dest/venv/bin/pip" install --quiet --disable-pip-version-check --no-cache-dir \
    "PyYAML==$PYYAML_VERSION" "ruamel.yaml==$RUAMEL_VERSION"
fi

if command -v npm >/dev/null; then
  npm install --silent --no-audit --no-fund --prefix "$dest/node" \
    "yaml@$YAML_JS_VERSION" "js-yaml@$JS_YAML_VERSION"
fi

if command -v go >/dev/null; then
  export GOPATH="$dest/go" GOMODCACHE="$dest/go/mod" GOFLAGS=-modcacherw GOTELEMETRY=off
  (cd "$here/go" && go mod tidy && go build -o "$dest/go-driver" .)
fi
