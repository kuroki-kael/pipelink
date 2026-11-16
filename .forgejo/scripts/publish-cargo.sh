#!/usr/bin/env bash
set -euo pipefail

OWNER="${1:?owner required}"
USER_AGENT="pipelink-ci (https://codeberg.org/${OWNER}/pipelink)"
CRATES=(
  pipelink-common
  pipelink-resolver
  pipelink-jack
  pipelink-pw
  pipelink-engine
  pipelink
)

if [ -z "${CARGO_REGISTRY_TOKEN:-}" ]; then
  echo "CARGO_REGISTRY_TOKEN secret is not set" >&2
  exit 1
fi
if [ -z "${PACKAGE_TOKEN:-}" ]; then
  echo "PACKAGE_TOKEN secret is not set" >&2
  exit 1
fi

VERSION="$(sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -n1)"

crate_version_of() {
  cargo metadata --no-deps --format-version 1 \
    | jq -r --arg name "$1" '.packages[] | select(.name == $name) | .version'
}

exists_on_crates_io() {
  local name="$1" version="$2" code
  code="$(curl -sS -o /dev/null -w '%{http_code}' -A "$USER_AGENT" \
    "https://crates.io/api/v1/crates/${name}/${version}")"
  [ "$code" = "200" ]
}

exists_on_codeberg() {
  local name="$1" version="$2" code
  code="$(curl -sS -o /dev/null -w '%{http_code}' \
    -H "Authorization: token ${PACKAGE_TOKEN}" \
    "https://codeberg.org/api/v1/packages/${OWNER}/cargo/${name}/${version}")"
  [ "$code" = "200" ]
}

publish_crates_io() {
  local name="$1" version="$2" attempt
  if exists_on_crates_io "$name" "$version"; then
    echo "Skipping ${name} ${version} on crates.io (already published)"
    return 0
  fi

  for attempt in 1 2 3; do
    if cargo publish -p "$name" --locked; then
      echo "Published ${name} ${version} to crates.io"
      sleep 5
      return 0
    fi
    echo "Attempt ${attempt} failed for ${name} on crates.io; waiting 15s for index propagation..."
    sleep 15
  done

  echo "Failed to publish ${name} ${version} to crates.io" >&2
  exit 1
}

publish_codeberg() {
  local name="$1" version="$2"
  if exists_on_codeberg "$name" "$version"; then
    echo "Skipping ${name} ${version} on Codeberg Cargo registry (already published)"
    return 0
  fi

  CARGO_REGISTRIES_CODEBERG_INDEX="sparse+https://codeberg.org/api/packages/${OWNER}/cargo/" \
  CARGO_REGISTRIES_CODEBERG_TOKEN="Bearer ${PACKAGE_TOKEN}" \
    cargo publish -p "$name" --registry codeberg --locked
  echo "Published ${name} ${version} to Codeberg Cargo registry"
}

echo "Publishing workspace crates (workspace version ${VERSION})..."
for crate in "${CRATES[@]}"; do
  crate_ver="$(crate_version_of "$crate")"
  publish_crates_io "$crate" "$crate_ver"
done

for crate in "${CRATES[@]}"; do
  crate_ver="$(crate_version_of "$crate")"
  publish_codeberg "$crate" "$crate_ver"
done
