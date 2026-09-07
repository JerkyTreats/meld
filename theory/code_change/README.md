# Declared code changes

This package asks a native Agent to acquire a declared proposal, construct and authorize its complete read-and-apply Task, and confirm the code-change owner's materialization account through Graph, Curation and Belief. The proposal contains exact prior content hashes and replacement text for existing workspace files. It is acquired during execution; preparation does not read or change those files.

The account records what was materialized. It does not assert that later workspace content is unchanged or that Security posture is clean. Security independently observes, assesses and verifies the resulting source under its own policy and authority.

Select this package with one declaration and an absolute proposal path. The selected policy grants the two code-change actions for `workspace_fs/node/repo` to `workspace-owner`. A different logical subject or principal requires a corresponding explicit policy selection. The proposal must be a canonical serialized `CodeChangeSet`, constructed by the code-change owner API.

```toml
[system.storage]
product_root = "/absolute/path/to/code-change-product"

[stewardship.declarations.code_change]
expression = "code_change"
target_root = "/absolute/path/to/workspace"
subject = { domain_id = "workspace_fs", object_kind = "node", object_id = "repo" }
agent_id = "code-agent"
principal_id = "workspace-owner"

[stewardship.declarations.code_change.bindings]
"code-change.proposal" = { endpoint_ref = "/absolute/path/to/proposal.json" }

[stewardship.declarations.code_change.theory]
belief_family_id = "code_materialization"
evidence_mapping_id = "code_materialization_v1"
curation_rule_id = "code_materialization"
maintained_condition_id = "code_materialization"
strategy_theory_id = "code_materialization"
authority_policy_id = "code_change_local"
```

From the repository root, `cargo run --bin meld -- world init --theory-source theory/code_change --format json` installs and prepares the product. The ordinary `runtime run` command activates it. It requires no model provider. The native file materializer currently requires Unix.

Version 1.1.0 scopes observation to the exact prepared request. The Goal, frozen owner products and materialization account survive generation changes. A delayed Task callback or process restart can recover the retained proposal and materialization without repeating a write, even after the proposal disappears or the user edits the files. Confirmation and further authorization use the current generation's fence. Restarting an already satisfied request leaves it satisfied.

To request another change under the same preparation, update the declared proposal and submit a new caller key:

```sh
meld runtime request --agent-id code-agent --request-key change-2 --format json
```

The live runtime accepts the request through its native Agent owner. With no foreground runtime, intake is retained for the next run. Repeating the key returns the same request and completion status. Each new key has its own Goal, observation and materialization account. Intake does not capture proposal bytes or grant mutation authority; the Task acquires the declared source when it executes. Once acquired, that exact proposal and any materialized effect remain recoverable under the original request. Lifecycle-scoped Startup observations cannot be duplicated through this command.

Existing version 1.0.0 preparations retain their epoch semantics and original receipt identities. Selecting version 1.1.0 requires a new preparation; this change does not rewrite earlier Agent genesis. This standalone package confirms materialization. The [Security mitigation package](../dependency_security_mitigation/README.md) connects declared intervention selection to independent Security verification.
