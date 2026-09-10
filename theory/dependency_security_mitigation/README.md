# Declared Security mitigation

This package extends the native Cargo Security reconciliation path with an explicitly granted, declared intervention. Agent first seeks current inventory, advisory assessment and independent verification. When that work leaves the desired posture unsatisfied and current coverage is established, Strategy can select the installed intervention Method.

The Method constructs two complete Tasks. The first reads the declared proposal and materializes its exact replacements for existing files. The second independently acquires inventory and advisories, assesses them, and verifies the assessment. The installed settlement rule requires Agent to accept the mutation Task's operational return before authorizing successor verification. Planned Curation, Belief assessment and Agent's Goal judgment remain separate from both Tasks.

Missing or incomplete coverage cannot authorize the mutation. Version `2.0.0` selects Security policy V2: complete transitive inventory, coverage of every component, and independent recalculation. It retains the same bounded Cargo and declared-advisory semantics as the read-only Security package. Old boolean policy declarations are incompatible and require explicit reinstallation; retained history is not rewritten. It does not generate patches or treat an attempted intervention as proof of resolution.

The proposal is a canonical serialized `CodeChangeSet` with exact prior content hashes, replacement text and declared reason references. It is read during execution from `code-change.proposal`. The native file materializer currently requires Unix. This policy grants six exact actions to `workspace-owner` for `workspace_fs/node/dependency-graph`; another principal or logical subject requires its corresponding policy.

Build and select `meld-dependency-security-owner` using the [external owner setup](../../owners/README.md). Grant its Cargo and advisory bindings explicitly, then add this declaration to the same configuration.

```toml
[system.storage]
product_root = "/absolute/path/to/security-product"

[stewardship.declarations.security]
expression = "dependency_security_mitigation"
target_root = "/absolute/path/to/workspace"
subject = { domain_id = "workspace_fs", object_kind = "node", object_id = "dependency-graph" }
agent_id = "security-agent"
principal_id = "workspace-owner"

[stewardship.declarations.security.bindings]
"dependency-security.cargo" = { executable_ref = "/absolute/path/to/cargo" }
"dependency-security.advisories" = { endpoint_ref = "/absolute/path/to/advisories.json" }
"code-change.proposal" = { endpoint_ref = "/absolute/path/to/proposal.json" }

[stewardship.declarations.security.theory]
belief_family_id = "dependency_security_posture"
evidence_mapping_id = "dependency_security_outcome_mapping_v1"
curation_rule_id = "dependency_security_posture"
maintained_condition_id = "dependency_security_posture"
strategy_theory_id = "dependency_security_mitigation"
authority_policy_id = "dependency_security_declared_mitigation"
```

From the repository root, install and prepare the selected product, then activate it. Preparation does not execute the intervention.

```sh
target/debug/meld --workspace /absolute/path/to/workspace --assignment security world init /absolute/path/to/workspace --theory-source theory/dependency_security_mitigation --format json
target/debug/meld --workspace /absolute/path/to/workspace --assignment security --enable-runtime execution.task_dispatch runtime run
```

Native owners retain the proposal, mutation intent, materialization and independent Security products. A completed mutation remains causal history when Strategy constructs successor verification. If the generation closes after materialization and before verification, the replacement generation can finish the same Goal using the retained mutation and fresh Security evidence. The original mutation authorization and acceptance keep their original generation; successor verification and Goal judgment use current authority. Recovery does not require the proposal file to remain available and does not repeat the completed write.

New admitted Security evidence can select another declared intervention after a successful repair. Within the same admission epoch, Agent retains the maintenance Goal and its earlier decisions, constructs successor Plans, and separately authorizes the new mutation and verification Tasks. Current standing observations can establish the new exposure without another acquisition Task before mutation.

Changing the proposal file alone does not create another admitted request. This package responds to Security evidence changes; proposal-only request intake remains a separate behavior. The standalone code-change package retains its prepared-request semantics.
