#!/usr/bin/env bash

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

TARGET_REF="HEAD"
BASE_REF=""
RELEASE_VERSION=""
RELEASE_DATE="$(date +%Y-%m-%d)"
FORCE_RELEASE="${FORCE_RELEASE:-false}"
CHANGELOG_ONLY=false
COMMIT_PATTERN='^([a-z]+)(\(([^)]+)\))?(!)?:[[:space:]](.+)$'
RELEASE_COMMIT_PATTERN='^ci\(release\): bump version to v[0-9]+\.[0-9]+\.[0-9]+ \[skip ci\]$'

usage() {
  cat <<'USAGE'
Usage:
  ./scripts/release_dry_run.sh
  ./scripts/release_dry_run.sh HEAD
  ./scripts/release_dry_run.sh --target-ref v2.1.0 --base-ref v2.0.0 --release-version 2.1.0 --release-date 2026-03-08 --changelog-only
USAGE
}

while [[ "$#" -gt 0 ]]; do
  case "$1" in
    --target-ref)
      TARGET_REF="$2"
      shift 2
      ;;
    --base-ref)
      BASE_REF="$2"
      shift 2
      ;;
    --release-version)
      RELEASE_VERSION="$2"
      shift 2
      ;;
    --release-date)
      RELEASE_DATE="$2"
      shift 2
      ;;
    --changelog-only)
      CHANGELOG_ONLY=true
      shift
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      if [[ "$1" == -* ]]; then
        echo "error: unknown option: $1" >&2
        exit 1
      fi
      TARGET_REF="$1"
      shift
      ;;
  esac
done

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

if [[ -n "$BASE_REF" ]] && ! git rev-parse --verify "$BASE_REF" >/dev/null 2>&1; then
  echo "error: base ref not found: $BASE_REF" >&2
  exit 1
fi

read_manifest_field() {
  local ref="$1"
  local field="$2"
  git show "$ref:Cargo.toml" | sed -n "s/^${field} = \"\\(.*\\)\"/\\1/p" | head -n1
}

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

resolve_default_base_ref() {
  local ref="$1"
  local exact_tag reachable_tags candidate
  exact_tag="$(git describe --tags --exact-match "$ref" 2>/dev/null || true)"
  reachable_tags="$(git tag --merged "$ref" --list 'v[0-9]*.[0-9]*.[0-9]*' --sort=-version:refname)"

  if [[ -z "$reachable_tags" ]]; then
    return 0
  fi

  if [[ -n "$exact_tag" && "$exact_tag" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    candidate="$(printf '%s\n' "$reachable_tags" | grep -vx "$exact_tag" | head -n1 || true)"
  else
    candidate="$(printf '%s\n' "$reachable_tags" | head -n1)"
  fi

  printf '%s\n' "$candidate"
}

crate_name="$(read_manifest_field "$TARGET_REF" name)"
target_cargo_version="$(read_manifest_field "$TARGET_REF" version)"

if [[ -z "$crate_name" || -z "$target_cargo_version" ]]; then
  echo "error: unable to read crate name and version from Cargo.toml at $TARGET_REF" >&2
  exit 1
fi

repo_url="$(normalize_repo_url "$(git config --get remote.origin.url || true)")"

if [[ -z "$BASE_REF" ]]; then
  BASE_REF="$(resolve_default_base_ref "$TARGET_REF")"
fi

if [[ -n "$BASE_REF" ]]; then
  base_version="$(read_manifest_field "$BASE_REF" version)"
  range_expr="${BASE_REF}..${TARGET_REF}"
  compare_base_tag=""
  if [[ "$BASE_REF" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    compare_base_tag="$BASE_REF"
  fi
else
  base_version="$target_cargo_version"
  range_expr="$TARGET_REF"
  compare_base_tag=""
fi

required_rank=0
required_name="none"
saw_commit=false
saw_release_relevant_change=false

while IFS= read -r sha; do
  [[ -n "$sha" ]] || continue
  subject="$(git log -1 --format=%s "$sha")"
  if [[ "$subject" =~ $RELEASE_COMMIT_PATTERN ]]; then
    continue
  fi

  saw_commit=true
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
    feat|fix|perf|refactor|docs|design|test|build|ci|chore|policy|eval) ;;
    *)
      echo "error: commit $sha uses unsupported type '$commit_type'" >&2
      exit 1
      ;;
  esac

  if [[ "$commit_type" == "eval" ]]; then
    continue
  fi

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
    entry="$entry [$short_sha]($repo_url/commit/$sha)"
  fi

  append_entry "$commit_type" "$entry"

  if git diff-tree --no-commit-id --name-only -r "$sha" | grep -Eq '^(src/|Cargo\.toml$|Cargo\.lock$|build\.rs$)'; then
    saw_release_relevant_change=true
  else
    continue
  fi

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
    case "$commit_type" in
      docs|design)
        ;;
      *)
        required_rank=1
        required_name="patch"
        ;;
    esac
  fi
done < <(git rev-list --no-merges --reverse "$range_expr")

if [[ "$saw_commit" != "true" && "$FORCE_RELEASE" == "true" ]]; then
  required_rank=1
  required_name="patch"
  saw_release_relevant_change=true
fi

if [[ "$saw_commit" != "true" && "$FORCE_RELEASE" != "true" ]]; then
  if [[ "$CHANGELOG_ONLY" == true ]]; then
    exit 0
  fi
  echo "release_required=false"
  echo "target_ref=$TARGET_REF"
  echo "base_ref=${BASE_REF:-none}"
  echo "crate_name=$crate_name"
  echo "cargo_version=$target_cargo_version"
  echo "base_version=$base_version"
  echo "required_bump=none"
  echo "next_version=$target_cargo_version"
  echo "release_tag=v$target_cargo_version"
  echo
  echo "No release needed for $TARGET_REF."
  exit 0
fi

if [[ "$saw_release_relevant_change" != "true" ]]; then
  if [[ "$CHANGELOG_ONLY" == true ]]; then
    exit 0
  fi
  echo "release_required=false"
  echo "target_ref=$TARGET_REF"
  echo "base_ref=${BASE_REF:-none}"
  echo "crate_name=$crate_name"
  echo "cargo_version=$target_cargo_version"
  echo "base_version=$base_version"
  echo "required_bump=none"
  echo "next_version=$target_cargo_version"
  echo "release_tag=v$target_cargo_version"
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

if [[ -n "$RELEASE_VERSION" ]]; then
  next_version="$RELEASE_VERSION"
else
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
fi

release_tag="v$next_version"
compare_url=""
if [[ -n "$repo_url" && -n "$compare_base_tag" ]]; then
  compare_url="$repo_url/compare/${compare_base_tag}...${release_tag}"
fi

render_changelog() {
  if [[ -n "$compare_url" ]]; then
    printf '## [%s](%s) — %s\n\n' "$next_version" "$compare_url" "$RELEASE_DATE"
  else
    printf '## %s — %s\n\n' "$next_version" "$RELEASE_DATE"
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
}

if [[ "$CHANGELOG_ONLY" == true ]]; then
  render_changelog
  exit 0
fi

echo "release_required=true"
echo "target_ref=$TARGET_REF"
echo "base_ref=${BASE_REF:-none}"
echo "crate_name=$crate_name"
echo "cargo_version=$target_cargo_version"
echo "base_version=$base_version"
echo "required_bump=$required_name"
echo "next_version=$next_version"
echo "release_tag=$release_tag"
echo
echo "CHANGELOG_PREVIEW<<EOF"
render_changelog
echo "EOF"
