# Contribution Policy

Status: active

## Scope And Authority

This policy governs repository authoring, documentation, comments, commits, and governance changes. The [Runtime Invariants](runtime_invariants.md) govern source and runtime meaning. Neither policy overrides the other.

Governance files change only after an explicit user request or approval. A governance amendment must update or remove conflicting active guidance in the same change. Newer dates do not silently establish precedence.

Prefer deleting duplicated, obsolete, or procedure-shaped governance over adding cross-references that preserve multiple authorities.

A clean change removes obsolete code and supporting surface made unnecessary by the change. Adding a successor without removing redundant implementation is incomplete.

## Comments

Comments explain meaning, ownership, invariants, and non-obvious intent. They do not narrate syntax or restate field names.

Use concise Rustdoc for public contracts and domain boundaries when their semantic role is not obvious. Use local comments for ordering, persistence, replay, compatibility, recovery, and other reasoning that the code shape cannot express safely.

Compatibility adapters state why they remain and the condition that permits removal. When behavior changes, update or delete stale comments in the same change.

## Markdown And Design

Do not use literal `(` or `)` in Markdown prose. They are allowed only when Markdown syntax requires them and inside inline or fenced code.

Files under `design/cognitive_architecture` state evergreen intended design. They do not contain implementation status, evidence dates, readiness judgments, migration state, delivery sequencing, or open implementation questions.

Put implementation evidence, readiness, migration, and sequencing under `design/plan` or `design/completed`.

## Commits

Commit only when requested. Use conventional commit types from this set:

`feat`, `fix`, `perf`, `refactor`, `docs`, `design`, `test`, `build`, `ci`, `chore`, `policy`

Use `design` for design-only changes. Use `policy` when governance changes are the primary purpose, including supporting design updates in the same commit. Runtime changes retain their runtime-focused type.

Write a declarative subject that names the concrete behavior or ownership change. Do not use phase labels as the subject.

Mark breaking changes with `!` and a `BREAKING CHANGE:` footer that states migration impact.

Confirm with the user before every push.
