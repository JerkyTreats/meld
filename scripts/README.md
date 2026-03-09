# Scripts

## check_domain_boundaries.sh

Ensures no cross-domain internal reach-through after Phase 10. Run from repo root:

```bash
./scripts/check_domain_boundaries.sh
```

**Rules enforced:**

- `src/cli/` must not use `context::frame::storage`; use `context::frame::open_storage` or api.
- No use of removed `crate::composition::`; use `crate::context::query::composition`.
- No use of removed `crate::tooling::`; use `crate::cli`, `crate::workspace`, `crate::agent`.

Add this script to your CI pipeline (e.g. in the same job as `cargo test` or a dedicated step). To extend the boundary matrix, edit the script and document new rules here and in `design/refactor/PLAN.md` Phase 10.

## release_dry_run.sh

Previews the local release plan for a target ref and prints:

- release requirement
- computed bump type
- next crate version and tag
- a changelog entry preview for the commits in scope

Run from repo root:

```bash
./scripts/release_dry_run.sh
./scripts/release_dry_run.sh HEAD
./scripts/release_dry_run.sh --target-ref v2.1.0 --base-ref v2.0.0 --release-version 2.1.0 --release-date 2026-03-08 --changelog-only
FORCE_RELEASE=true ./scripts/release_dry_run.sh
```

The default target ref is `HEAD`. The script follows the release planning rules in `.github/workflows/ci.yml` and warns when the latest semver tag is not an ancestor of the target ref.

Useful flags:

- `--target-ref` to inspect a different ref
- `--base-ref` to pin the compare base for historical releases
- `--release-version` to force the rendered changelog version
- `--release-date` to pin the rendered release date
- `--changelog-only` to print only the changelog entry

## backfill_changelog.sh

Backfills missing changelog entries from published GitHub releases after a starting tag.

```bash
./scripts/backfill_changelog.sh
./scripts/backfill_changelog.sh v1.1.0
```

This script uses `gh release list` for release tags and publish dates, then renders matching changelog entries with `./scripts/release_dry_run.sh`.
