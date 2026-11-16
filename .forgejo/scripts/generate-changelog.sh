#!/usr/bin/env bash
set -euo pipefail

TAG="${1:?tag required}"
REPO="${2:?owner/repo required}"
BASE_URL="${3:-https://codeberg.org}"

PREV_TAG="$(git tag --merged HEAD --sort=-v:refname | grep -Fxv "$TAG" | head -n1 || true)"

if [ -n "$PREV_TAG" ]; then
  RANGE="${PREV_TAG}..HEAD"
else
  RANGE="HEAD"
fi

features=""
fixes=""
refactors=""
docs=""
maintenance=""
others=""

while IFS=$'\t' read -r short_sha subject || [ -n "${subject:-}" ]; do
  [ -n "$subject" ] || continue
  entry="- ${subject} (\`${short_sha}\`)"
  case "$subject" in
    feat:*|feat\(*\):*|feat!:*)
      features+="${entry}"$'\n'
      ;;
    fix:*|fix\(*\):*|fix!:*)
      fixes+="${entry}"$'\n'
      ;;
    refactor:*|refactor\(*\):*|perf:*|perf\(*\):*)
      refactors+="${entry}"$'\n'
      ;;
    docs:*|docs\(*\):*)
      docs+="${entry}"$'\n'
      ;;
    build:*|build\(*\):*|ci:*|ci\(*\):*|chore:*|chore\(*\):*|style:*|style\(*\):*|test:*|test\(*\):*)
      maintenance+="${entry}"$'\n'
      ;;
    *)
      others+="${entry}"$'\n'
      ;;
  esac
done < <(git log --no-merges --pretty=format:'%h%x09%s' "$RANGE")

printf '## What'\''s Changed\n\n'

append_section() {
  local title="$1" items="$2"
  if [ -n "$items" ]; then
    printf '### %s\n%s\n' "$title" "$items"
  fi
}

append_section "Features" "$features"
append_section "Bug Fixes" "$fixes"
append_section "Refactoring & Performance" "$refactors"
append_section "Documentation" "$docs"
append_section "Build, CI & Maintenance" "$maintenance"
append_section "Other Changes" "$others"

if [ -n "$PREV_TAG" ]; then
  printf '**Full Changelog:** %s/%s/compare/%s...%s\n' "$BASE_URL" "$REPO" "$PREV_TAG" "$TAG"
fi
