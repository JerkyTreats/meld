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
- For migrations, use compatibility wrappers and require characterization and parity tests before removing old paths.

## Governance Index

- [Commit Policy](governance/commit_policy.md)
- [Compatibility Policy](governance/compatibility_policy.md)
- [CLI Targeting Policy](governance/cli_targeting_policy.md)
- [Docs Style Policy](governance/docs_style_policy.md)
- [Policy Proposal Flow](governance/policy_proposal_flow.md)
- [Complex Change Workflow Governance](governance/complex_change_workflow.md)

## Complex Workflow Note

- Complex workflow mode is user triggered.
- CI ignores complex workflow governance and does not enforce workflow artifacts.
