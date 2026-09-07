# Native Docs maintenance

This package maintains README existence and configured claims supported by workspace evidence through native owner observations, Agent judgment, Strategy planning, Execution, and confirmation after publication. A current README can satisfy the maintained condition without a Task. A completed write alone does not establish correctness or Goal satisfaction.

Add a declaration to your Meld configuration. Bind `docs-provider` to an existing configured provider and keep the product storage outside the target workspace.

```toml
[system.storage]
product_root = "/absolute/path/to/docs-product"

[stewardship.declarations.docs]
expression = "documentation_maintenance"
target_root = "/absolute/path/to/workspace"
subject = "docs"
agent_id = "docs-writer"
principal_id = "workspace-owner"
provider_id = "docs-provider"

[stewardship.declarations.docs.theory]
belief_family_id = "docs_freshness"
evidence_mapping_id = "docs_freshness_outcome_interpretation_v1"
curation_rule_id = "docs_freshness"
maintained_condition_id = "docs_freshness"
strategy_theory_id = "docs_freshness"
authority_policy_id = "docs_workspace_local"
claim_policy_id = "docs-claims-strict-v1"
```

From the repository root, install and prepare the product, then activate it:

```sh
cargo run --bin meld -- world init --theory-source theory/docs_freshness --format json
cargo run --bin meld -- runtime run
```

Preparation remains inert. Activation opens the native owner lifecycle and admission path. The installed claim policy supplies the questions, acceptance rules and drafting instructions. The runtime observes current workspace evidence before deciding on work and returns published results through Docs observations, Curation, Belief and Agent judgment.

The old bundled `docs_writer_thread_v1` Workflow and `docs_writer` task package are no longer installed by `meld init` or embedded as an Execution fallback. This package is their maintained-Docs successor. Existing user-supplied Workflow profiles and package documents remain explicit compatibility inputs; their turn completion and frame-head output do not establish native Docs satisfaction.
