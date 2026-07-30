# Reflexive Eval Requirements

Date: 2026-07-30
Status: proposed
Scope: loose requirements and scaffolding for eval as a cross-cutting quality substrate — what it is, what it must never become, the record it needs now, and the gates before any eval system is chartered

## Thesis

Eval is reflexive belief: the world model pointed at the system's own products and processes. Quality as a cross-org substrate does not need a new substrate — it needs the existing causal record to carry cost and attribution, and belief families whose subjects are the system's own durable identities. Method efficacy, theory-revision efficacy, cost fidelity, and output quality are belief dimensions like any other: evidence-mediated, comparator-assessed, revision-disciplined, provenance-stamped, and explainable by the same walks the harness already serves.

The system is early by exactly one layer. The eval record is overdue — costs never recorded cannot be retrofitted — while the eval system waits for its consumers per the layering discipline in [Persistent Domain Stewardship](../../cognitive_architecture/persistent_domain_stewardship.md).

## Anti-Goals

These name the failure modes this document exists to prevent.

- No second epistemics. An eval score that is not a belief is an unaccountable judgment: no evidence chain, no revision discipline, no walk that can explain it. The moment an eval score can outrank a belief, quality has two sources of truth.
- No ungoverned self-reinforcement. Eval signals act only through the evidence to belief to goal chain, under the same authority boundaries, epoch fences, and non-absorbing satisfaction as every other signal. A closed fast-path loop from score to selection pressure is an ungoverned optimizer.
- No authored quality catalogs. Encoding anticipated failure modes into gates and specifications is the road to a thousandfold workflow pile. Integrity checks stay in workflows; quality judgments ride the epistemic loop, where being wrong is recoverable.
- No eval application. Eval surfaces are projections over the served substrate, consumed externally, per the recorded substrate boundary and freeze principle.

## Requirements

Loose register; each is a direction with its first concrete step, not a specification.

- EVAL-R1 eval record, near term and primitive-shaped: every durable outcome carries its actual cost — provider spend, tokens, wall time, provider and model identity — alongside the attribution the lineage already stamps. Estimates exist in the shared language; actuals have no durable home. Every flywheel turn before this lands is lost eval evidence.
- EVAL-R2 attribution completeness: any outcome must be walkable to the theory revisions, method, plan, stewardship expression, and agent that produced it. Largely built through frame identity and TheoryRevisionRef; gaps found during eval work land here as findings.
- EVAL-R3 first contradicting evidence source: outcome payloads carry their semantic yield and one mapping rule reads it, so belief can learn an output was empty. Without evidence that can contradict, no eval judgment has anything to stand on. Chartered context: the docs writer cold-context diagnosis.
- EVAL-R4 quality dimensions are theory: family-declared, selected by identity, installed by revision, exactly like every operational dimension. Judged dimensions declare `Derived` observationality — the first real customer of that field.
- EVAL-R5 reflexive subject vocabulary: system identities — method, theory revision, provider binding, expression — become belief subjects with the same identity discipline as workspace subjects. The subject vocabulary is the seam where eval becomes cross-org: one record shape, many stewarded domains.
- EVAL-R6 authority: action from eval flows only through goal curation. Eval families propose; agents authorize; execution acts. An eval revision is a theory revision — attributable through lineage and scoreable by its downstream outcomes, which is also the anti-Goodhart discipline: evals of evals are just more theory under the same accountability.
- EVAL-R7 consumers before machinery: the Strategy first slice ranks on static cost and preference; reflexive families later replace the statics through the same request seams. The multiplier program is the first eval experiment — same tasks, different providers, quality per cost, measured through the substrate.

## Relationship To Strategy

Strategy achieves the majority of this structure by weight, and that is the design: its deferred normative tier — learned ranking, the learned efficacy model — is the learning half of eval, and this document's gates make Strategy's first slice the consumer eval replaces statics for. The boundary is the existing one. Strategy's contract records a prospective viability judgment and explicitly does not record that the theory succeeded or that the desired state was restored; eval is the retrospective half — outcomes becoming evidence, efficacy becoming belief — and it lives in the world model, which Strategy consumes as it consumes any belief. The standing decision this section exists to record: when the normative tier arrives, efficacy judgments are reflexive belief families, never a Strategy-private score store. Eval scope outside Strategy entirely: output quality of produced artifacts, provider and model comparison, belief-family calibration, and pipeline cost fidelity. The eval record is upstream of Strategy, not downstream: the learned efficacy model is unbuildable without accumulated actuals.

## Gates Before An Eval System Is Chartered

- EVAL-R1 actual-cost recording landed and accumulating.
- EVAL-R3 contradicting evidence proven live: a goal that truthfully fails to satisfy on empty yield.
- Strategy first slice consuming static ranking, so eval has a real ranking consumer to replace.

## Not In Scope

Learned ranking implementation, eval scoring engines, catalog governance, any authored gate taxonomy, and any eval-specific presentation surface.

## Related Documentation

- [Persistent Domain Stewardship](../../cognitive_architecture/persistent_domain_stewardship.md) — the layering discipline this proposal sequences under
- [PDS Theory Runtime Layer Map](pds_theory_runtime_layer.md) — the theory seams eval families reuse
- [Agent-Native Debugger Requirements](agent_native_debugger_requirements.md) — the causal record and served substrate eval projects over
- [Multiplier Harness Program](multiplier_harness_program.md) — the first eval-shaped experiment
- [Runtime Completion Implementation Workstreams](runtime_completion_implementation_workstreams.md) — the docs writer diagnosis motivating EVAL-R3
