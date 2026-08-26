# AGENTS.md

## Domain Architecture Rule

- Do not use `mod.rs`. Use the modern Rust convention: a module is either a single file `parent.rs` or a file `parent.rs` that declares submodules with `mod child;` and children live in `parent/child.rs`. Prefer `parent.rs` plus `parent/child.rs` over `parent/mod.rs` plus `parent/child.rs`.
- Organize code by domain first.
- Keep each domain concern under `src/<domain>/`.
- Inside a domain, name submodules by behavior, for example `query`, `mutation`, `orchestration`, `queue`, `sessions`, `sinks`.
- Keep adapters thin. `tooling` and `api` may parse route format and delegate only.
- Cross domain calls must use explicit domain contracts.
- Do not reach into another domain internal modules.
- Avoid generic primary folders named by technical layer.
- Use compatibility readers or forwarding adapters only for demonstrated migration needs. They must route into the canonical authority and retain characterization or parity evidence.

## Canonical Runtime Path Rule

- One semantic responsibility has one canonical runtime authority in completed source.
- A replacement must route real callers through its successor and remove the superseded authority in the completed change.
- Compatibility readers and forwarding adapters do not authorize a second writer, planner, actor, selector, or decision authority.
- Follow [Runtime Invariants](governance/runtime_invariants.md).

## Governance Index

- [Runtime Invariants](governance/runtime_invariants.md)
- [Contribution Policy](governance/contribution_policy.md)

## Commit Governance Rule

- For every user request that asks for a commit, review [Contribution Policy](governance/contribution_policy.md) before running `git commit` or `git commit --amend`.

## Response Style Rule

- For architecture assessments and redesign syntheses, use the [Architecture Response skill](.agents/skills/architecture-response/SKILL.md).

## Comment Policy 

Use comments appropriately under the [Contribution Policy](governance/contribution_policy.md).
