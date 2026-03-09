#!/usr/bin/env bash

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

SINCE_TAG="${1:-v1.1.0}"
TEMP_ENTRIES="$(mktemp)"
TEMP_CHANGELOG="$(mktemp)"
trap 'rm -f "$TEMP_ENTRIES" "$TEMP_CHANGELOG"' EXIT

if ! command -v gh >/dev/null 2>&1; then
  echo "error: gh is required" >&2
  exit 1
fi

if ! git rev-parse --verify "$SINCE_TAG" >/dev/null 2>&1; then
  echo "error: tag not found: $SINCE_TAG" >&2
  exit 1
fi

git fetch origin --tags >/dev/null 2>&1 || true

mapfile -t TAGS < <(git tag --list 'v[0-9]*.[0-9]*.[0-9]*' --sort=version:refname)
declare -A INCLUDED_TAGS=()
after_since=false

for tag in "${TAGS[@]}"; do
  if [[ "$tag" == "$SINCE_TAG" ]]; then
    after_since=true
    continue
  fi
  if [[ "$after_since" == true ]]; then
    INCLUDED_TAGS["$tag"]=1
  fi
done

find_previous_tag() {
  local target="$1"
  local previous=""
  local tag
  for tag in "${TAGS[@]}"; do
    if [[ "$tag" == "$target" ]]; then
      printf '%s\n' "$previous"
      return 0
    fi
    previous="$tag"
  done
  return 1
}

has_entry() {
  local version="$1"
  grep -Fq "## [$version]" CHANGELOG.md || grep -Fq "## $version" CHANGELOG.md
}

while IFS=$'\t' read -r tag published_at; do
  [[ -n "$tag" ]] || continue
  if [[ -z "${INCLUDED_TAGS[$tag]:-}" ]]; then
    continue
  fi

  previous_tag="$(find_previous_tag "$tag")"
  if [[ -z "$previous_tag" ]]; then
    continue
  fi

  version="${tag#v}"
  release_date="${published_at%%T*}"

  if has_entry "$version"; then
    continue
  fi

  ./scripts/release_dry_run.sh \
    --target-ref "$tag" \
    --base-ref "$previous_tag" \
    --release-version "$version" \
    --release-date "$release_date" \
    --changelog-only >> "$TEMP_ENTRIES"
  printf '\n' >> "$TEMP_ENTRIES"
done < <(gh release list --limit 100 --json tagName,publishedAt --jq '.[] | [.tagName, .publishedAt] | @tsv')

if [[ ! -s "$TEMP_ENTRIES" ]]; then
  echo "No missing changelog entries found."
  exit 0
fi

{
  sed -n '1p' CHANGELOG.md
  printf '\n'
  cat "$TEMP_ENTRIES"
  sed -n '2,$p' CHANGELOG.md
} > "$TEMP_CHANGELOG"

mv "$TEMP_CHANGELOG" CHANGELOG.md

echo "Backfilled changelog entries from releases after $SINCE_TAG."
