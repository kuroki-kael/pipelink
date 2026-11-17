#!/usr/bin/env bash
set -euo pipefail

TAG="${1:?tag required}"
REPO="${2:?owner/repo required}"
BASE_URL="${3:-https://codeberg.org}"

if ! command -v git-cliff >/dev/null 2>&1; then
  echo "git-cliff is required to generate release notes" >&2
  exit 1
fi

ARGS=(--config cliff.toml --strip header)
if git rev-parse -q --verify "refs/tags/${TAG}" >/dev/null; then
  TAG_COMMIT="$(git rev-list -n1 "refs/tags/${TAG}")"
  HEAD_COMMIT="$(git rev-parse HEAD)"
  if [ "$TAG_COMMIT" = "$HEAD_COMMIT" ]; then
    ARGS+=(--current)
  else
    PREV_TAG="$(git tag --merged "refs/tags/${TAG}" --sort=-v:refname | grep -Fxv "$TAG" | head -n1 || true)"
    if [ -n "$PREV_TAG" ]; then
      ARGS+=("${PREV_TAG}..${TAG}")
    else
      ARGS+=("${TAG}")
    fi
  fi
else
  ARGS+=(--unreleased --tag "$TAG")
fi

git-cliff "${ARGS[@]}" \
  | sed "s|https://codeberg.org/kuroki-kael/pipelink|${BASE_URL}/${REPO}|g"
