# Native Docs maintenance

This package maintains README existence and configured claims supported by workspace evidence through native owner observations, Agent judgment, Strategy planning, Execution, and confirmation after publication. A current README can satisfy the maintained condition without a Task. A completed write alone does not establish correctness or Goal satisfaction.

Build and select `meld-docs-owner` using the [external owner setup](../../owners/README.md), then add the declaration below to the same Meld configuration. Bind `docs-provider` to a configured provider that supports JSON Schema response formats and keep the product storage outside the target workspace. An explicit `--config` selects the same configuration for initialization and runtime execution.

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
target/debug/meld --workspace /absolute/path/to/workspace --assignment docs world init /absolute/path/to/workspace --theory-source theory/docs_freshness --format json
target/debug/meld --workspace /absolute/path/to/workspace --assignment docs --enable-runtime execution.task_dispatch runtime run
```

Preparation remains inert. Activation opens the native owner lifecycle and admission path. The installed claim policy supplies the questions, acceptance rules and drafting instructions. The runtime observes current workspace evidence before deciding on work and returns published results through Docs observations, Curation, Belief and Agent judgment.

The policy's `response_formats` declare the provider response contracts. `x-meld-enum-from-lines` binds quotation choices from the named input text using JSON pointers, retaining exact source substrings within the declared length bound. `x-meld-if-text` enables a response alternative only when that evidence partition contains text. Native owners still verify identity, exact citations, policy acceptance and required coverage after generation. A schema-conforming response alone does not establish freshness.

`x-meld-map-from-field` requires one response slot per captured claim identity, and `x-meld-enum-from-field` limits references to the captured claim set. Required-field order supplies generation order to grammar-backed providers without changing durable JSON identity. Drafting and revision receive the frozen current README so repairs can retain correct content. Low-confidence judgments remain unaccepted evidence; they do not become successful claims through response formatting.

The installed `semantic_theory.scope` selects the managed document name, hidden-directory handling, excluded directory names and direct-directory or subtree comparison. `document_per_source_directory_v1` captures nonbinary source files and manages each containing directory and ancestor. Missing bounded text remains an explicit coverage gap. The observation, drafting, comparison and publication paths use the same selection. Historical captures without this field retain their original read contract; they cannot author new judgments under a newly selected scope.

`scope.capture_limits` declares the supported capture budget. This package selects 64 managed directories, 8192 bytes per source file, 24576 bytes of direct evidence per directory, 3072 bytes per child document supplied as evidence, and 8192 characters per revision report. Larger or unavailable source text produces explicit coverage gaps; exceeding the directory count refuses capture. These limits constrain this package's supported scope and do not assert complete coverage of arbitrary repositories.

`semantic_theory.claim_extraction` selects `markdown_sentences_v2`. It reads sentence candidates in paragraphs and lists, all heading levels, table rows and fenced code. Candidates retain source line locations and receive semantic judgment in the complete document context. Sentence boundaries do not establish atomic meaning: the judge must assess every factual clause. An unchanged document retains identical capture identities; a changed extraction or capture selection creates a distinct observation.

A README remains a document. Drafting and revision request explanatory prose and useful structure, with examples only when supported by evidence. The `document_claim_mass_v2` evaluator excludes explicitly judged `non_assertive` navigation and connective language from factual support mass. Such text cannot supply evidence or satisfy required source-claim correspondence. Factual headings, identifiers, commands and examples still require support. A document containing only navigation cannot establish accepted factual coverage. Destructive line-pruning repair is retired; installed revision repairs the complete document and rechecks it.

Existing captures without an extraction selection retain the frozen original grammar through the owner-owned historical reader. They are not reinterpreted using the new grammar and cannot authorize new judgments under a successor policy. Old policy bytes and hashes remain readable; new work requires explicit extraction and capture selections. The historical line-pruning selection can be decoded for inspection but is refused for execution.

Semantic judgments retain execution provenance supplied by the provider adapter: exact request and response identities, installed policy identity, resolved configuration hash, requested and reported model, finish reason and token usage. Endpoints and credentials are not written into these reports. The reported model may be a service alias; it does not identify backing weights. Restart retains this evidence with the original source and policy.

README judgment receives the captured document as reference context, separately from the source evidence used to establish truth. Extraction describes implementation-supported behavior rather than the existence of docstrings. Correspondence requires every independent clause, including both branches of a conditional. These instructions are package content, not Rust judgments.

The selected guards check exact literals and code lines. Semantic entailment belongs to the configured judge with exact citations. The package does not select word-overlap scoring: that heuristic rejected accurate descriptions even when every constant had an exact source quotation. Required coverage and all declared acceptance thresholds still apply.

The old bundled `docs_writer_thread_v1` Workflow and `docs_writer` task package are no longer installed by `meld init` or embedded as an Execution fallback. This package is their maintained-Docs successor. Legacy Workflow execution is retired. Existing profiles remain inspectable, but CLI, context, watch and control no longer execute their turns. Their stored output does not establish native Docs satisfaction. Arbitrary profiles are not automatically translated into this maintained intent.

You can also request an independent reconciliation of the installed intent:

```sh
meld runtime request --agent-id docs-writer --request-key docs-review-1 --format json
```

The command reaches the live owner when the foreground runtime is running, or durably records the request for the next run. Repeating the key returns the same request and its completion status. A new key creates another request under the same preparation. Intake does not create a Goal or authorize a Task: Agent judges current evidence first. A correct README can complete the request without either. When work is needed, requests retain separate Goals and decisions while Execution may share compatible actions.

The installed Strategy settlement rule selects `repeat_on_changed_owners: ["docs"]`. Changed Docs evidence permits a new repair even while an earlier repair awaits confirmation. Other products do not inherit that repetition policy. Runtime checks the selected owner basis and preserves completed history; it does not choose which domain changes warrant repeated work.

Current Docs preparation uses package `1.16.0`, including explicit extraction, capture scope, document-oriented revision and repeated-work selection. Previously prepared revisions remain historical evidence. The CLI does not migrate an existing Agent into a different genesis lineage when installing a revised package. Use a separately assigned Agent and preparation for changed theory; assignments can share the same product root. Do not delete prior records to imitate migration. Ordinary runtime reopen of the same prepared theory remains supported.

Correspondence retains valid low-confidence proposals as uncertain evidence. The report binds the selected minimum confidence; uncertain matches cannot count as represented, supported source coverage. Resuming the same capture reuses the report instead of repeating judgment solely because the answer was uncertain.

The selected response schema bounds each supported or contradicted judgment to six citations and asks for the smallest sufficient set without repetition. Maintenance requests make the smallest necessary repair to an existing document. This strict literal policy can explain source computations, but it does not validate newly invented evaluated example calls whose literals are absent from captured evidence.
