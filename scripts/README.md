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
FORCE_RELEASE=true ./scripts/release_dry_run.sh
```

The default target ref is `HEAD`. The script follows the release planning rules in `.github/workflows/ci.yml` and warns when the latest semver tag is not an ancestor of the target ref.
