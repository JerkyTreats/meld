# Compatibility Shim Policy

Date: 2026-04-17
Status: active
Scope: temporary compatibility shims used during refactors migrations and legacy path retirement

## Intent

Allow short lived compatibility seams that preserve correctness during change.
Prevent stale indirection file bloat and unclear ownership after the migration window closes.

## Core Rule

A compatibility shim is a temporary migration tool.
It is allowed only when it reduces migration risk or preserves required behavior during an active refactor migration or compatibility window.
It must be removed once the migration condition is satisfied.

If a compatibility layer must remain long term, promote it into an explicit stable adapter or domain contract with clear ownership.
Do not leave it in place as an unnamed temporary seam.

## When A Shim Is Allowed

- preserve required behavior while ownership moves into the target domain
- bridge old call sites to a new contract during a staged migration
- preserve legacy stored data handling or wire format handling while parity evidence is being established
- isolate one narrow compatibility seam instead of spreading legacy branching through the new implementation

## Required Shape

- Keep the shim thin and local to the owning domain boundary.
- Prefer one explicit adapter seam over broad forwarding layers or repeated legacy checks.
- New code must target the post migration path directly unless it is characterization coverage for the old path.
- Do not add new public surface solely to preserve an internal transition when a private seam is sufficient.
- Avoid shim only files and re export layers unless the public surface truly requires them for the migration window.

## Removal Note Requirement

Every shim must carry a local removal note in the shim file or immediately above the shim entry point.

- Use `TODO compat-shim:` or another equally explicit marker.
- State why the shim exists.
- State what old path behavior or format it preserves.
- State what condition allows removal.
- State what tests or proof must remain green before deletion.

Example note:

```rust
// TODO compat-shim: remove after legacy frame metadata readers are deleted
// and parity tests prove the structural agent_id path covers the old blobs.
```

## Verification Rules

- Before adding a shim, capture characterization tests or parity tests for the behavior it preserves.
- Before removing a shim, prove the target path covers the same contract or document an intentional break under the compatibility policy.
- Remove shim specific tests once the shim is deleted unless they still validate an enduring contract.

## Removal Rules

- Remove the shim in the first change where supported callers data and formats no longer require it.
- Do not keep a shim for convenience speculative rollback or uncertain ownership.
- Do not stack a new shim on top of an older shim. Collapse to one seam or remove the older path first.
- If a shim survives beyond the immediate refactor series, refresh the removal note and explain the continued need in review.

## Review Guidance

- Treat a shim with no removal note as incomplete.
- Treat a shim that becomes the default path for new code as architectural drift.
- Prefer deletion over preserving a clean but unnecessary wrapper.
- Prefer explicit adapters with names and ownership over vague helper functions that quietly preserve legacy behavior.

## Related Policies

- Breaking user facing changes remain governed by [Compatibility Policy](compatibility_policy.md).
- Rust code comments for compatibility seams remain governed by [Commenting Policy](commenting_policy.md).
