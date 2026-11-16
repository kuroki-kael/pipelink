#!/usr/bin/env bash
set -euo pipefail

TAG="${1:?tag required}"
REPO="${2:?owner/repo required}"
shift 2

if [ -z "${GH_RELEASE_TOKEN:-}" ]; then
  echo "GH_RELEASE_TOKEN not set; skipping GitHub release mirror"
  exit 0
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
NOTES="$("$SCRIPT_DIR/generate-changelog.sh" "$TAG" "$REPO" "https://github.com")"

API="https://api.github.com/repos/${REPO}"
AUTH="Authorization: Bearer ${GH_RELEASE_TOKEN}"
ACCEPT="Accept: application/vnd.github+json"
VER="X-GitHub-Api-Version: 2022-11-28"

REL_JSON="$(curl -fsSL -H "$AUTH" -H "$ACCEPT" -H "$VER" "$API/releases/tags/$TAG" 2>/dev/null || true)"
REL_ID="$(echo "$REL_JSON" | jq -r '.id // empty')"
if [ -z "$REL_ID" ]; then
  REL_JSON="$(curl -fsSL -X POST -H "$AUTH" -H "$ACCEPT" -H "$VER" \
    -d "$(jq -n --arg tag "$TAG" --arg body "$NOTES" '{tag_name: $tag, target_commitish: "main", name: $tag, body: $body}')" \
    "$API/releases" 2>/dev/null || curl -fsSL -H "$AUTH" -H "$ACCEPT" -H "$VER" "$API/releases/tags/$TAG")"
else
  REL_BODY="$(echo "$REL_JSON" | jq -r '.body // ""')"
  if [ -z "${REL_BODY//[[:space:]]/}" ] && [ -n "$NOTES" ]; then
    curl -fsSL -X PATCH -H "$AUTH" -H "$ACCEPT" -H "$VER" \
      -d "$(jq -n --arg body "$NOTES" '{body: $body}')" \
      "$API/releases/${REL_ID}" >/dev/null || true
  fi
fi

UPLOAD_URL="$(echo "$REL_JSON" | jq -r '.upload_url' | sed -e 's/{?name,label}//')"
if [ -z "$UPLOAD_URL" ] || [ "$UPLOAD_URL" = "null" ]; then
  echo "Failed to resolve GitHub upload_url for release $TAG" >&2
  exit 1
fi

for file in "$@"; do
  [ -f "$file" ] || continue
  name="$(basename "$file")"
  curl -fsSL -X POST -H "$AUTH" -H "$ACCEPT" -H "$VER" \
    -H "Content-Type: application/octet-stream" \
    --data-binary "@$file" \
    "${UPLOAD_URL}?name=${name}" >/dev/null
  echo "Mirrored $name to GitHub release $TAG ($REPO)"
done
