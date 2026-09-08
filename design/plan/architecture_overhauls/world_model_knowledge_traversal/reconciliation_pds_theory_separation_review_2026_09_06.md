# Executive Report: PDS Theory Must Be Separate From Rust Runtime

> Superseded for current reconciliation direction. [Canonical Flywheel Alignment And Runtime Soft Freeze](flywheel_remediation.md) alone owns the issue register, priorities, next work, and branch acceptance. This document retains historical design or evidence; its prior status and authorization statements do not govern current delivery.

Date: 2026-09-06

Status: architectural violation report for executive-agent disposition; source-inspected, not execution-verified

Reviewed snapshot: `360b8120a5e015a0d6832046c53387d9d7787da0`

Branch: `design/world-model-reconciliation`

Active implementation account: [Reconciliation Outcome Audit](reconciliation_outcome_audit.md)

Related review: [Reconciliation Recovery And Startup Proof](reconciliation_executive_review_2026_09_06.md)

## User Direction And Conclusion

The user identified `src/docs` as violating the separation of domain theory from Rust runtime and directed submission of this report: **PDS theory must be separate.**

The current runtime contains product-specific decisions about claim support, acceptance, source selection, comparison scope, and repair. Installed policy supplies some parameters, but compiled code still supplies additional theory that the package cannot select or replace. A directory named after the semantic owner does not resolve that authority conflict.

**Docs owning its theory does not authorize Rust runtime to hardcode Docs theory.** PDS carries the owner-issued declarative product meaning. Native runtime owners implement mechanisms, evaluate admitted theory through their contracts, and enforce the state, authority, effects, and durability they own.

The user direction is the governing boundary. The finding identifiers and proposed tests in this report are evidence and review aids, not a replacement delivery program. Record disposition in the active outcome audit. Do not restore withdrawn slice restrictions, create a second ledger, or reinterpret this report as permission to waive existing runtime invariants.

## Architectural Basis And Correction To Earlier Assessments

The [canonical PDS model][pds] calls PDS the declarative product material supplying purpose, scope, semantic resources, and standing responsibility. Its immutable theory packages can contain directives, schemas, comparator policies, graph publication contracts, Strategy knowledge, Curation templates, Capability references, and satisfaction contracts. Installation establishes owner admission and integrity, not the truth of the package's propositions. PDS is not itself a planner, Curation worker, belief engine, scheduler, or Task Network.

The earlier chat assessments correctly separated supported assertions from required coverage but did not initially classify compiled Docs policy as a violation. Saying that correctness is distributed across theory, Rust, and the provider was descriptive, not an acceptable architecture. Likewise, adding independent source claims and correspondence can improve functional behavior while extending this violation.

The current [outcome audit][audit] records independent source extraction and observed correspondence, and explicitly leaves required coverage and no-action reconciliation unfinished. Those claims are narrower than architectural completion. The next correction must remove the competing compiled theory, not simply add more correctness logic under `src/docs`.

## TS-01: Compiled Acceptance Overrides Admitted Policy

Classification: demonstrated source-level policy conflict.

The [installed claim policy][policy] declares minimum claim confidence, minimum groundedness, permitted unsupported and contradicted mass, revision budget, and Markdown-form weights. The [policy validator][policy-type] permits finite mass limits throughout the zero-to-one range. Yet [assess_readme][judgment] adds an unconditional per-claim conjunction after evaluating the aggregate thresholds:

```rust
assessments.iter().all(|assessment| {
    assessment.verdict == ClaimVerdict::Supported
        && assessment.confidence >= context.policy.minimum_claim_confidence
})
```

An installed policy with nonzero unsupported-mass allowance therefore cannot make a document with an unsupported claim acceptable. The runtime adds a decision beyond the admitted parameters. The shipped strict policy hides the conflict by selecting zero unsupported and contradicted mass.

Disposition required: express the applicable acceptance predicate in admitted theory and evaluate it through one owner contract. A deliberately strict theory may still reject every unsupported claim. What must end is the runtime silently imposing that choice on every policy. Merely deleting one conjunction without specifying the complete acceptance semantics would also be insufficient.

Suggested discriminating test: install two valid policy revisions over the same captured evidence and deterministic judgment proposals. One rejects all unsupported claims; another explicitly tolerates a bounded unresolved mass without relabeling it supported. Prove that the same binary follows the distinct acceptance policies. This is a boundary test, not a recommendation that the production Docs product adopt a permissive policy.

## TS-02: Judgment And Extraction Prompts Contain Compiled Theory

Classification: demonstrated source-level theory placement; model accuracy remains a separate question.

[assess_claim_batch][judgment] embeds the definitions of support, contradiction, insufficient evidence, permitted evidence partitions, restrictions on inference, and clause-level citation instructions in Rust strings. [extract_provider_claims][source-extraction] separately instructs extraction to cover interfaces, behavior, configuration, data, and constraints. Changing those instructions changes which evidence and claims the system obtains and accepts; these are not only serialization instructions.

The [deterministic guard path][judgment] also mixes integrity enforcement with semantic decisions. Rejecting unknown identifiers, duplicate responses, malformed confidence, or a quotation absent from its claimed input protects the contract. Unconditionally requiring a command line to appear literally in direct source, selecting evidence classes, and converting lexical-coverage failures into an `Unsupported` verdict impose an epistemic policy.

Disposition required: install the product-specific judgment, extraction, evidence-admissibility, and verdict policies as exact theory resources or references to admitted reusable operators. Keep transport shape and provenance validation in runtime. Separate invalid evidence from a theory's decision about what valid evidence establishes.

Moving the prompt text alone is not sufficient while compiled guards retain conflicting product policy. Conversely, externalizing theory must not make forged citations, unknown owner identities, or unauthorized effects acceptable through a policy switch. Mandatory contract invariants remain enforced independently.

## TS-03: Rust Selects The Product's Comparison Scope

Classification: demonstrated source-level scope selection.

In [correspondence.rs][correspondence], `captured_readmes` constructs expected managed paths as `README.md` or `<directory>/README.md`. `selected_sources` strips that suffix and selects every source file whose path begins with the remaining directory prefix. This fixes directory-subtree correspondence, including descendant claims, independently of an installed comparison-scope policy.

The source correctly says this does not make every selected assertion mandatory. Nevertheless, deciding which material to compare is already product theory. A prefix-matching operation can be a reusable runtime mechanism; selecting it as the universal Docs relationship cannot be an implicit runtime default.

Disposition required: make target naming, governed source scope, comparison relationships, and materiality choices explicit in admitted theory. Runtime resolves and executes those selections against exact owner revisions. Follow the capture and traversal callers far enough to remove any earlier compiled selector that silently narrows the theory's requested scope.

Do not turn the review's `start` and `stop` example into a universal requirement to document every public function. Whether an omitted claim matters is a theory decision. Represented, omitted, required, supported, and correct remain distinct meanings.

## TS-04: Validation Selects A Compiled Repair Strategy

Classification: demonstrated source-level product-policy selection.

[validate_patch_set][repair] selects an assess-revise-prune sequence. Before its final allowed attempt, it invokes `prune_rejected_claims`; the [revision prompt][judgment] directs removal of unsupported claims rather than another response to uncertainty. The attempt count is configurable, but the response strategy is compiled.

Another admitted product policy could request additional evidence, preserve explicitly unresolved assertions, escalate, or refuse publication. The mechanisms for revision and deletion can be implemented in Rust. The product's choice among those mechanisms must come from theory under the owning authorization and planning contracts.

Disposition required: expose the actual repair policy or recipe through admitted theory and invoke only authorized work. Do not disguise a product workflow as generic validation, and do not move it into a new PDS scheduler. Strategy, Agent, Curation, and Execution retain their existing responsibilities; the report does not require every internal string operation to become an independent Task.

## TS-05: A Score Threshold Is Not The Missing Correctness Theory

Classification: inspected configuration and acknowledged incomplete product semantics.

The [maintained condition][condition] declares `confidence > 0.7`. The [epistemic template][epistemic] selects `workspace_fs`, an expected generic `assessment`, and `curation_assesses`. The [outcome mapping][mapping] still consumes a `docs_freshness_assessment` artifact carried by `execution.task.succeeded`. The active audit explicitly says the generic assessment, Task-success mapping, and receipt-byte assessment do not establish the desired coverage and no-action outcome.

These configured resources do not supply the missing definition of required content. Provider confidence, supported included assertions, observed correspondence, and byte-currentness are different evidence claims. None should be renamed correctness merely to connect the remaining loop.

Disposition required: admit an owner-defined theory of required source content, materiality, evidence, correspondence or coverage, and satisfaction. Correctness must be relative to that explicit scope and policy. The reviewer does not prescribe a universal coverage rule. A correctly maintained README must be evaluable from independent owner evidence without first running a repair Task, under the exact theory that defines the maintained condition.

## TS-06: Effective Theory Identity Is Wider Than The Numeric Policy Hash

Classification: demonstrated identity boundary with a source-derived replay concern; no cache-corruption reproducer was executed.

[DocsClaimPolicy.content_identity][policy-type] hashes its serialized numeric and weight configuration. Judgment prompts live outside that value. [Correspondence input identity][correspondence] cites a static contract identifier, that policy identity, the observation revision, and source report. [Source extraction][source-extraction] similarly uses a compiled contract and policy identity while constructing its prompt in code.

Those identifiers trace portions of the decision but do not externalize its complete theory. Changing a compiled semantic instruction can change decisions without changing the numeric policy. Naming compiled behavior `docs.claim-correspondence.v1` makes it addressable; it does not make product meaning package-selected. This observation does not claim that every runtime or provider lineage field is absent throughout the repository.

Disposition required: bind each product to the exact effective theory resources and admitted evaluator contracts. Record the provider and model execution provenance relevant to judgment. Changed theory must invalidate dependent currentness and reuse according to owner rules while preserving old results under their original identities. Restart must resolve the originally prepared revision, not an unrelated current registry head. Do not promise deterministic LLM output merely because the policy and prompt are frozen.

## Required Separation

| Concern | Declarative theory or admitted product selection | Runtime mechanism and non-waivable contract |
| --- | --- | --- |
| Scope | Governed entities, target naming, inclusion, exclusions, materiality | Exact capture, path resolution, bounded reads, completeness reporting |
| Knowledge | Extraction instructions, evidence classes, support rules, comparison relationships | Provider invocation, parsing, identity checks, quotation verification, immutable results |
| Judgment | Acceptance predicates, comparator selection and parameters, required coverage | Evaluate the selected contract; preserve unresolved, false, and incomplete distinctions |
| Work | Available recipes, response to failed judgment, desired outcomes | Strategy constructs; Agent authorizes; Curation authors epistemics; Execution realizes Tasks |
| Satisfaction | Maintained conditions and required evidence or milestones | Owner admission, exact currentness, separate durable Agent disposition |
| Safety and durability | Requested authority and bindings within owner limits | Authority ceilings, provenance integrity, generation fences, idempotency, replay, confined effects |

The exact package schema and implementation decomposition remain executive implementation decisions. A new interpreter, crate, service, database, universal ontology, or arbitrary executable configuration is not implied. Reuse existing package installation, native owner admission, and runtime mechanisms where they can express the actual semantics.

Do not solve the violation by moving hardcoded policy to another Rust directory, embedding it with `include_str!`, adding more optional numeric fields while retaining mandatory overrides, or placing a product-name dispatch switch behind a generic interface. If a reusable operator has fixed semantics, it must be explicit in the admitted theory selection; a Docs-specific compiled workflow cannot remain the implicit authority.

## Completion Evidence Requested From The Executive Agent

The user direction should be reflected in the active outcome audit with concrete source and test evidence, not only a new package version or passing fixture count.

| Proof | Required observable result |
| --- | --- |
| Same binary, different theory | Two valid installed theories change scope, acceptance, or repair decisions through native product entrypoints without recompilation or a product-specific runtime override. Use deterministic judge proposals to isolate policy evaluation from model variation. |
| Missing or unsupported theory | Missing required semantic resources or unsupported operator contracts cause an explicit unresolved or rejected position. No hidden compiled product default supplies success. |
| Exact lineage and invalidation | Changed effective theory produces distinct judged lineage and invalidates dependent reuse. Older products remain readable under their original theory after close and reopen. |
| No-action and work-required cases | The installed theory establishes a maintained-condition disposition for sufficient current documentation without repair, while a theory-defined missing requirement produces justified work or a named unresolved condition. |
| Invariant preservation | Foreign evidence, forged identities, invalid citations, stale authority, and out-of-scope writes remain rejected under every installed theory. |
| Canonical cutover | Real callers use the theory-driven successor, and superseded hardcoded selectors, judges, repair policies, defaults, and exclusive fixtures no longer provide another semantic authority. Retain only evidenced read compatibility. |

The last obligation follows [Runtime Invariants][invariants] and [Contribution Policy][contribution]. A dormant generalized evaluator beside the old Docs path is not a correction. Neither is accepting current compiled theory with a promise to externalize it in a later cleanup.

## Executive Handoff And Evidence Limits

Treat TS-01 through TS-04 as concrete source findings under the user-confirmed separation rule. TS-05 records the missing theory obligation without inventing the user's documentation standard. TS-06 identifies the effective-theory lineage work needed to make the separation verifiable.

Disposition should name each migrated selection, the installed semantic resource, its consuming owner, removed authority, and executable proof. Keep those decisions in the active outcome audit rather than a new delivery ledger. Review adjacent product-specific semantics reached through installation, runtime assembly, and other PDS products; this Docs inspection is not evidence that every other product is already compliant.

Existing Startup completion and Docs observation tests retain their behavioral evidence. They do not certify this boundary and should not be discarded wholesale. Preserve mechanisms that still satisfy owner contracts while removing the compiled product theory.

This report re-read the listed files at the pinned snapshot. It did not run Rust tests, reproduce model accuracy results, amend governance, or change runtime source or the active outcome audit. The submission is documentation only. Earlier implementation-account test totals are not independent verification by this review. Later code changes must be dispositioned against their actual source rather than silently rewriting what this snapshot established.

**Closure requires PDS theory to be the explicit source of product-specific meaning, with native Rust runtime evaluating and enforcing admitted contracts instead of supplying competing product theory.**

[pds]: https://github.com/JerkyTreats/meld/blob/360b8120a5e015a0d6832046c53387d9d7787da0/design/cognitive_architecture/persistent_domain_stewardship.md
[audit]: https://github.com/JerkyTreats/meld/blob/360b8120a5e015a0d6832046c53387d9d7787da0/design/plan/architecture_overhauls/world_model_knowledge_traversal/reconciliation_outcome_audit.md#L390-L410
[policy]: https://github.com/JerkyTreats/meld/blob/360b8120a5e015a0d6832046c53387d9d7787da0/theory/docs_freshness/claim_policy.docs-claims-strict-v1.json
[policy-type]: https://github.com/JerkyTreats/meld/blob/360b8120a5e015a0d6832046c53387d9d7787da0/src/docs/claim_validation.rs#L28-L101
[judgment]: https://github.com/JerkyTreats/meld/blob/360b8120a5e015a0d6832046c53387d9d7787da0/src/docs/claim_validation.rs#L550-L880
[repair]: https://github.com/JerkyTreats/meld/blob/360b8120a5e015a0d6832046c53387d9d7787da0/src/docs/claim_validation.rs#L385-L525
[source-extraction]: https://github.com/JerkyTreats/meld/blob/360b8120a5e015a0d6832046c53387d9d7787da0/src/docs/source_claims.rs#L327-L380
[correspondence]: https://github.com/JerkyTreats/meld/blob/360b8120a5e015a0d6832046c53387d9d7787da0/src/docs/correspondence.rs#L1-L285
[condition]: https://github.com/JerkyTreats/meld/blob/360b8120a5e015a0d6832046c53387d9d7787da0/theory/docs_freshness/maintained_condition.docs_freshness.json
[epistemic]: https://github.com/JerkyTreats/meld/blob/360b8120a5e015a0d6832046c53387d9d7787da0/theory/docs_freshness/epistemic_rule.docs_freshness.json
[mapping]: https://github.com/JerkyTreats/meld/blob/360b8120a5e015a0d6832046c53387d9d7787da0/theory/docs_freshness/outcome_interpretation.docs_freshness.json
[invariants]: https://github.com/JerkyTreats/meld/blob/360b8120a5e015a0d6832046c53387d9d7787da0/governance/runtime_invariants.md
[contribution]: https://github.com/JerkyTreats/meld/blob/360b8120a5e015a0d6832046c53387d9d7787da0/governance/contribution_policy.md
