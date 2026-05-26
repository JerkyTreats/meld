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

## Advanced Test Tools

These tools are optional local verification aids. They are not required for the standard cargo harness.

Install mutation testing:

```bash
cargo install cargo-mutants --locked
```

Run mutation tests against the belief slice:

```bash
cargo mutants -p meld-world-model --timeout 60 --file crates/meld-world-model/src/belief/config.rs --file crates/meld-world-model/src/belief/comparator.rs --file crates/meld-world-model/src/belief/store.rs -- --all-targets
```

Run a broader world-model mutation pass:

```bash
cargo mutants -p meld-world-model --timeout 60 -- --all-targets
```

Install fuzzing:

```bash
cargo install cargo-fuzz --locked
rustup toolchain install nightly --profile minimal
```

Initialize fuzzing inside a crate when a fuzz target is added:

```bash
cd crates/meld-world-model
cargo fuzz init
```

Run a fuzz target for a bounded local pass:

```bash
cd crates/meld-world-model
cargo +nightly fuzz run fuzz_belief_config -- -max_total_time=60
```

Suggested first fuzz targets:

- `fuzz_belief_config` for `BeliefFamilyConfig` JSON loading and validation
- `fuzz_belief_record_roundtrip` for public belief records through `serde_json`
- `fuzz_belief_comparator` for bounded comparator input shapes

CI guidance:

- Keep `cargo fmt`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace --all-targets` in required CI.
- Run `cargo-mutants` in a separate optional or scheduled job because it is slower than the normal harness.
- Keep `cargo-fuzz` as local or scheduled verification unless each target has a strict time cap.

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
