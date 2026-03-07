#!/usr/bin/env bash

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

TARGET_REF="${1:-HEAD}"
FORCE_RELEASE="${FORCE_RELEASE:-false}"
TODAY="$(date +%Y-%m-%d)"
COMMIT_PATTERN='^([a-z]+)(\(([^)]+)\))?(!)?:[[:space:]](.+)$'

declare -a BREAKING_ENTRIES=()
declare -a FEATURE_ENTRIES=()
declare -a FIX_ENTRIES=()
declare -a PERF_ENTRIES=()
declare -a REFACTOR_ENTRIES=()
declare -a DOCS_ENTRIES=()
declare -a TEST_ENTRIES=()
declare -a BUILD_ENTRIES=()
declare -a CI_ENTRIES=()
declare -a CHORE_ENTRIES=()
declare -a DESIGN_ENTRIES=()
declare -a POLICY_ENTRIES=()

if ! git rev-parse --verify "$TARGET_REF" >/dev/null 2>&1; then
  echo "error: target ref not found: $TARGET_REF" >&2
  exit 1
fi

crate_name="$(sed -n 's/^name = "\(.*\)"/\1/p' Cargo.toml | head -n1)"
cargo_version="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n1)"

if [[ -z "$crate_name" || -z "$cargo_version" ]]; then
  echo "error: unable to read crate name and version from Cargo.toml" >&2
  exit 1
fi

parse_version() {
  local value="$1"
  local major minor patch
  IFS='.' read -r major minor patch <<<"$value"
  if [[ -z "${major:-}" || -z "${minor:-}" || -z "${patch:-}" ]]; then
    echo "error: invalid semver value: $value" >&2
    exit 1
  fi
  if ! [[ "$major" =~ ^[0-9]+$ && "$minor" =~ ^[0-9]+$ && "$patch" =~ ^[0-9]+$ ]]; then
    echo "error: non numeric semver value: $value" >&2
    exit 1
  fi
  printf '%s %s %s\n' "$major" "$minor" "$patch"
}

normalize_repo_url() {
  local raw_url="$1"
  if [[ -z "$raw_url" ]]; then
    return 0
  fi

  if [[ "$raw_url" =~ ^git@github.com:(.+)\.git$ ]]; then
    printf 'https://github.com/%s\n' "${BASH_REMATCH[1]}"
    return 0
  fi

  if [[ "$raw_url" =~ ^https://github.com/(.+)\.git$ ]]; then
    printf 'https://github.com/%s\n' "${BASH_REMATCH[1]}"
    return 0
  fi

  if [[ "$raw_url" =~ ^https://github.com/.+ ]]; then
    printf '%s\n' "$raw_url"
    return 0
  fi

  return 0
}

append_entry() {
  local category="$1"
  local value="$2"
  case "$category" in
    feat) FEATURE_ENTRIES+=("$value") ;;
    fix) FIX_ENTRIES+=("$value") ;;
    perf) PERF_ENTRIES+=("$value") ;;
    refactor) REFACTOR_ENTRIES+=("$value") ;;
    docs) DOCS_ENTRIES+=("$value") ;;
    test) TEST_ENTRIES+=("$value") ;;
    build) BUILD_ENTRIES+=("$value") ;;
    ci) CI_ENTRIES+=("$value") ;;
    chore) CHORE_ENTRIES+=("$value") ;;
    design) DESIGN_ENTRIES+=("$value") ;;
    policy) POLICY_ENTRIES+=("$value") ;;
    *)
      echo "error: unsupported category: $category" >&2
      exit 1
      ;;
  esac
}

emit_section() {
  local title="$1"
  shift
  local entries=("$@")
  if [[ "${#entries[@]}" -eq 0 ]]; then
    return 0
  fi

  printf '### %s\n\n' "$title"
  printf '%s\n' "${entries[@]}"
  printf '\n'
}

repo_url="$(normalize_repo_url "$(git config --get remote.origin.url || true)")"
last_tag="$(git tag --list 'v[0-9]*.[0-9]*.[0-9]*' --sort=-version:refname | head -n1)"

if [[ -n "$last_tag" ]]; then
  base_version="${last_tag#v}"
  range_expr="${last_tag}..${TARGET_REF}"
else
  base_version="$cargo_version"
  range_expr="$TARGET_REF"
fi

required_rank=0
required_name="none"
saw_commit=false

while IFS= read -r sha; do
  [[ -n "$sha" ]] || continue
  saw_commit=true
  subject="$(git log -1 --format=%s "$sha")"
  body="$(git log -1 --format=%b "$sha")"
  short_sha="$(git rev-parse --short "$sha")"

  if ! [[ "$subject" =~ $COMMIT_PATTERN ]]; then
    echo "error: commit $sha is not a valid conventional commit: $subject" >&2
    exit 1
  fi

  commit_type="${BASH_REMATCH[1]}"
  scope="${BASH_REMATCH[3]:-}"
  bang="${BASH_REMATCH[4]:-}"
  summary="${BASH_REMATCH[5]}"

  case "$commit_type" in
    feat|fix|perf|refactor|docs|design|test|build|ci|chore|policy) ;;
    *)
      echo "error: commit $sha uses unsupported type '$commit_type'" >&2
      exit 1
      ;;
  esac

  if [[ "$commit_type" == "policy" ]]; then
    if ! grep -Eq '^(Policy-Ref|Discussion):[[:space:]].+' <<<"$body"; then
      echo "error: policy commit $sha is missing Policy-Ref or Discussion footer" >&2
      exit 1
    fi
  fi

  entry="* $summary"
  if [[ -n "$scope" ]]; then
    entry="* **$scope:** $summary"
  fi
  if [[ -n "$repo_url" ]]; then
    entry="$entry ([$short_sha]($repo_url/commit/$sha))"
  fi

  append_entry "$commit_type" "$entry"

  if [[ -n "$bang" ]] || grep -Eq '^BREAKING CHANGE:[[:space:]].+' <<<"$body"; then
    required_rank=3
    required_name="major"
    breaking_note="$(grep -E '^BREAKING CHANGE:[[:space:]].+' <<<"$body" | head -n1 | sed -E 's/^BREAKING CHANGE:[[:space:]]*//')"
    if [[ -z "$breaking_note" ]]; then
      breaking_note="$summary"
    fi
    BREAKING_ENTRIES+=("* $breaking_note")
  elif [[ "$required_rank" -lt 2 && "$commit_type" == "feat" ]]; then
    required_rank=2
    required_name="minor"
  elif [[ "$required_rank" -lt 1 ]]; then
    required_rank=1
    required_name="patch"
  fi
done < <(git rev-list --no-merges --reverse "$range_expr")

if [[ "$saw_commit" != "true" && "$FORCE_RELEASE" == "true" ]]; then
  required_rank=1
  required_name="patch"
fi

if [[ "$saw_commit" != "true" && "$FORCE_RELEASE" != "true" ]]; then
  echo "release_required=false"
  echo "target_ref=$TARGET_REF"
  echo "crate_name=$crate_name"
  echo "cargo_version=$cargo_version"
  echo "base_version=$base_version"
  echo "last_tag=${last_tag:-none}"
  echo "required_bump=none"
  echo "next_version=$cargo_version"
  echo "release_tag=v$cargo_version"
  echo
  echo "No release needed for $TARGET_REF."
  exit 0
fi

read -r base_major base_minor base_patch <<<"$(parse_version "$base_version")"

effective_rank="$required_rank"
if [[ "$base_major" == "0" && "$required_rank" -eq 3 ]]; then
  effective_rank=2
  required_name="minor"
fi

case "$effective_rank" in
  3)
    next_major=$((base_major + 1))
    next_minor=0
    next_patch=0
    ;;
  2)
    next_major=$base_major
    next_minor=$((base_minor + 1))
    next_patch=0
    ;;
  1)
    next_major=$base_major
    next_minor=$base_minor
    next_patch=$((base_patch + 1))
    ;;
  *)
    echo "error: unable to determine required version bump" >&2
    exit 1
    ;;
esac

next_version="${next_major}.${next_minor}.${next_patch}"
release_tag="v$next_version"
ancestor_status="unknown"

if [[ -n "$last_tag" ]]; then
  if git merge-base --is-ancestor "$last_tag" "$TARGET_REF"; then
    ancestor_status="yes"
  else
    ancestor_status="no"
  fi
fi

compare_url=""
if [[ -n "$repo_url" && -n "$last_tag" ]]; then
  compare_url="$repo_url/compare/${last_tag}...${release_tag}"
fi

echo "release_required=true"
echo "target_ref=$TARGET_REF"
echo "crate_name=$crate_name"
echo "cargo_version=$cargo_version"
echo "base_version=$base_version"
echo "last_tag=${last_tag:-none}"
echo "last_tag_ancestor_of_target=$ancestor_status"
echo "required_bump=$required_name"
echo "next_version=$next_version"
echo "release_tag=$release_tag"
echo
echo "CHANGELOG_PREVIEW<<EOF"

if [[ -n "$compare_url" ]]; then
  printf '## [%s](%s) %s\n\n' "$next_version" "$compare_url" "$TODAY"
else
  printf '## %s %s\n\n' "$next_version" "$TODAY"
fi

if [[ "${#BREAKING_ENTRIES[@]}" -gt 0 ]]; then
  printf '### ⚠ BREAKING CHANGES\n\n'
  printf '%s\n' "${BREAKING_ENTRIES[@]}"
  printf '\n'
fi

emit_section "Features" "${FEATURE_ENTRIES[@]}"
emit_section "Bug Fixes" "${FIX_ENTRIES[@]}"
emit_section "Performance" "${PERF_ENTRIES[@]}"
emit_section "Refactors" "${REFACTOR_ENTRIES[@]}"
emit_section "Documentation" "${DOCS_ENTRIES[@]}"
emit_section "Tests" "${TEST_ENTRIES[@]}"
emit_section "Build" "${BUILD_ENTRIES[@]}"
emit_section "CI" "${CI_ENTRIES[@]}"
emit_section "Chores" "${CHORE_ENTRIES[@]}"
emit_section "Design" "${DESIGN_ENTRIES[@]}"
emit_section "Policy" "${POLICY_ENTRIES[@]}"

echo "EOF"
