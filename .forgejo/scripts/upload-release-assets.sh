#!/usr/bin/env bash
set -euo pipefail

TAG="${1:?tag required}"
REPO="${2:?owner/repo required}"
shift 2

if [ -z "${GITHUB_TOKEN:-}" ]; then
  echo "GITHUB_TOKEN is not set" >&2
  exit 1
fi

API="https://codeberg.org/api/v1/repos/${REPO}"
AUTH="Authorization: token ${GITHUB_TOKEN}"

REL_JSON="$(curl -fsSL -H "$AUTH" "$API/releases/tags/$TAG" 2>/dev/null || true)"
REL_ID="$(echo "$REL_JSON" | jq -r '.id // empty')"
if [ -z "$REL_ID" ]; then
  REL_JSON="$(curl -fsSL -X POST -H "$AUTH" -H "Content-Type: application/json" \
    -d "$(jq -n --arg tag "$TAG" '{tag_name: $tag, name: $tag}')" \
    "$API/releases" 2>/dev/null || curl -fsSL -H "$AUTH" "$API/releases/tags/$TAG")"
  REL_ID="$(echo "$REL_JSON" | jq -r '.id')"
fi

for file in "$@"; do
  [ -f "$file" ] || continue
  name="$(basename "$file")"
  curl -fsSL -X POST -H "$AUTH" -F "attachment=@$file" \
    "$API/releases/${REL_ID}/assets?name=${name}" >/dev/null
  echo "Uploaded $name to release $TAG"
done
