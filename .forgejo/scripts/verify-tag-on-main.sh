#!/usr/bin/env bash
set -euo pipefail

git fetch origin main
if ! git merge-base --is-ancestor HEAD origin/main; then
  echo "Refusing to release: tag must point to a commit on main" >&2
  exit 1
fi
