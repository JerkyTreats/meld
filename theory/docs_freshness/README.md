# Native Docs maintenance

This package maintains README existence and configured claims supported by workspace evidence through native owner observations, Agent judgment, Strategy planning, Execution, and confirmation after publication. A current README can satisfy the maintained condition without a Task. A completed write alone does not establish correctness or Goal satisfaction.

Add a declaration to your Meld configuration. Bind `docs-provider` to a configured provider that supports JSON Schema response formats and keep the product storage outside the target workspace. An explicit `--config` selects the same configuration for initialization and runtime execution.

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

The policy's `response_formats` declare the provider response contracts. `x-meld-enum-from-lines` binds quotation choices from the named input text using JSON pointers, retaining exact source substrings within the declared length bound. `x-meld-if-text` enables a response alternative only when that evidence partition contains text. Native owners still verify identity, exact citations, policy acceptance and required coverage after generation. A schema-conforming response alone does not establish freshness.

`x-meld-map-from-field` requires one response slot per captured claim identity, and `x-meld-enum-from-field` limits references to the captured claim set. Required-field order supplies generation order to grammar-backed providers without changing durable JSON identity. Drafting and revision receive the frozen current README so repairs can retain correct content. Low-confidence judgments remain unaccepted evidence; they do not become successful claims through response formatting.

The old bundled `docs_writer_thread_v1` Workflow and `docs_writer` task package are no longer installed by `meld init` or embedded as an Execution fallback. This package is their maintained-Docs successor. Legacy Workflow execution is retired. Existing profiles remain inspectable, but CLI, context, watch and control no longer execute their turns. Their stored output does not establish native Docs satisfaction. Arbitrary profiles are not automatically translated into this maintained intent.

You can also request an independent reconciliation of the installed intent:

```sh
meld runtime request --agent-id docs-writer --request-key docs-review-1 --format json
```

The command reaches the live owner when the foreground runtime is running, or durably records the request for the next run. Repeating the key returns the same request and its completion status. A new key creates another request under the same preparation. Intake does not create a Goal or authorize a Task: Agent judges current evidence first. A correct README can complete the request without either. When work is needed, requests retain separate Goals and decisions while Execution may share compatible actions.
