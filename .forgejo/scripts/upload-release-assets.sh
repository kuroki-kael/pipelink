#!/usr/bin/env bash
set -euo pipefail

TAG="${1:?tag required}"
REPO="${2:?owner/repo required}"
shift 2

if [ -z "${GITHUB_TOKEN:-}" ]; then
  echo "GITHUB_TOKEN is not set" >&2
  exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
NOTES="$("$SCRIPT_DIR/generate-changelog.sh" "$TAG" "$REPO" "https://codeberg.org")"

API="https://codeberg.org/api/v1/repos/${REPO}"
AUTH="Authorization: token ${GITHUB_TOKEN}"

REL_JSON="$(curl -fsSL -H "$AUTH" "$API/releases/tags/$TAG" 2>/dev/null || true)"
REL_ID="$(echo "$REL_JSON" | jq -r '.id // empty')"
if [ -z "$REL_ID" ]; then
  REL_JSON="$(curl -fsSL -X POST -H "$AUTH" -H "Content-Type: application/json" \
    -d "$(jq -n --arg tag "$TAG" --arg body "$NOTES" '{tag_name: $tag, name: $tag, body: $body}')" \
    "$API/releases" 2>/dev/null || curl -fsSL -H "$AUTH" "$API/releases/tags/$TAG")"
  REL_ID="$(echo "$REL_JSON" | jq -r '.id')"
else
  REL_BODY="$(echo "$REL_JSON" | jq -r '.body // ""')"
  if [ -z "${REL_BODY//[[:space:]]/}" ] && [ -n "$NOTES" ]; then
    curl -fsSL -X PATCH -H "$AUTH" -H "Content-Type: application/json" \
      -d "$(jq -n --arg body "$NOTES" '{body: $body}')" \
      "$API/releases/${REL_ID}" >/dev/null || true
  fi
fi

for file in "$@"; do
  [ -f "$file" ] || continue
  name="$(basename "$file")"
  curl -fsSL -X POST -H "$AUTH" -F "attachment=@$file" \
    "$API/releases/${REL_ID}/assets?name=${name}" >/dev/null
  echo "Uploaded $name to release $TAG"
done
