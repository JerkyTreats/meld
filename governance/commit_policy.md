# Commit Policy

Date: 2026-03-01
Status: active

## Intent

Define commit message and scoping rules used in this repository.

## Conventional Commit Use

Use conventional commits when instructed to commit.

Approved commit `type` values:
- `feat`
- `fix`
- `perf`
- `refactor`
- `docs`
- `design`
- `test`
- `build`
- `ci`
- `chore`
- `policy`

## Type Selection

- Use `design` for changes under `design/` that update plans specs architecture docs or workflow docs.
- When one commit mixes `design/` updates with runtime code changes, keep the runtime focused commit type and describe design impact in the commit body.
- Use `policy` for repository governance updates such as standards process rules and enforcement workflow changes.

## Governance Trace For Policy Commits

For `policy` commits include at least one governance trace footer such as `Policy-Ref:` or `Discussion:`.

## Subject Rules

- Write the subject as a declarative summary of what changed.
- Describe concrete behavior or ownership changes not process context.
- Do not use contextual labels such as phase names in the subject.
- Keep the subject focused and specific to the diff.
- Prefer type and scope with this shape `type(scope): summary`.

Examples:
- good `refactor(provider): split provider ownership into profile repository diagnostics commands and generation`
- bad `refactor(provider): implement phase2`
- breaking `refactor(context)!: remove legacy frame metadata compatibility path`
- breaking footer `BREAKING CHANGE: frame metadata no longer accepts legacy prompt key`
- policy `policy(agents): require policy trace footer for governance changes`

## Push Guard

Verify with user before push.
