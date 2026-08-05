# Strategy Boundary Contracts

## Purpose

Strategy is the semantic boundary between an Agent-curated proposed Goal and an authorized course of action. It grounds a candidate in exact world-model context while preserving the authority of the Agent, Execution, evidence, belief, and satisfaction domains.

These contracts define the smallest boundary needed to carry authorized meaning from deliberation into realization and back through observation:

```text
belief divergence
→ Agent-curated Goal
→ Strategy candidate
→ Agent judgment
→ Goal admission
→ Execution realization
→ substantive outcome
→ evidence and belief reconciliation
→ Agent satisfaction
```

## Candidate contract

Strategy may construct a concrete candidate for a proposed Goal. Construction does not authorize the candidate or admit the Goal.

A candidate is eligible only when all of the following meaning is established:

- the proposed Goal and its target are ground
- the planner context has exact identity and matches the Goal scope
- the Method applies to the Goal target
- every Method precondition is satisfied
- every binding is ground
- the resulting Composition is structurally valid
- the Method has one unambiguous available action realization
- the action outcome meaning is governed by a valid outcome contract
- an observational Goal has a valid prospective evidence route for its subject and dimension

Strategy must use the authoritative world-model, belief, theory, and shared-language semantics for these judgments. It must not create alternate evidence admission, comparison, causal, or satisfaction semantics.

If any required meaning is missing, indeterminate, stale, invalid, unavailable, ambiguous, or unauthorized, Strategy produces no eligible candidate.

## Agent authorization contract

The Agent alone authorizes a Strategy candidate. Successful construction, retention, admission, or realization does not confer Agent authority.

Authorization binds one Agent judgment to the exact Goal target, planner context, Method meaning, ground bindings, concrete Composition, action outcome meaning, prospective evidence route, and judgment policy used for the choice. This binding preserves what the Agent chose and the grounds on which it chose.

Authorization does not include an inferred outcome, a claim of Goal satisfaction, an execution schedule, a task identity, a provider choice, learned preference, or reusable catalog knowledge. Those meanings either belong to other domains or require evidence not established by the choice.

## Settlement and replay contract

Candidate consideration and Agent choice may involve epistemic uncertainty or nondeterminism. Once the Agent judgment is settled, operational replay is no longer a new epistemic choice.

The exact authorization and selected candidate must be settled before the Goal crosses into admission. Every replay after settlement must reuse that judgment and candidate without reconstructing the candidate or asking the Agent to choose again.

Operational work derived from a settled authorization must be reproducible and idempotent under Execution contracts. Repeated handling must not create divergent authorized meaning.

## Goal admission contract

An Agent-curated Goal may enter Execution only with a nonempty Agent authorization for an eligible candidate.

Admission must establish that the authorization belongs to the Agent judgment and matches the Goal, target, planner context, Method, bindings, Composition, action, outcome meaning, and prospective evidence route being admitted. Missing, empty, mismatched, or content-invalid authorization is rejected.

A repeated admission of the same settled authorization must preserve the same accepted Goal and authorized meaning.

## Authorized realization contract

Execution owns revalidation and realization of authorized meaning. Before mutating the task network, it must establish that the authorized Method content, ground bindings, Composition, action availability, realization, and Goal lifecycle remain valid.

Execution must realize the admitted candidate exactly. It must not select a different Method, add semantic work, change the claimed outcome meaning, or substitute a different Composition or action. If current conditions no longer support exact realization, Execution fails without task-network mutation.

Authorization permits realization. It does not permit Execution to satisfy the Goal.

## Observational evidence contract

For an observational Goal, the candidate must identify a prospective route from the chosen Method and action through governed outcome meaning to admissible evidence for the Goal subject and dimension.

The route is a prediction that realization can produce relevant evidence. It is not evidence, a belief update, an assertion that the Goal condition became true, or a guarantee that the Goal threshold will be crossed.

Projected effects, planning commitment, task completion, and artifact creation do not satisfy an observational Goal. A substantive outcome must enter the authoritative evidence path, beliefs must reconcile, and the Agent must judge satisfaction over the reconciled belief.

## Domain ownership contract

Strategy owns candidate construction only. The Agent owns candidate authorization and Goal satisfaction judgment. Execution owns admission revalidation and exact realization. The task network owns operational work. Outcome, evidence, and belief domains own interpretation, evidence admission, and reconciliation according to their contracts.

An authorization identity must preserve continuity from the Agent judgment through Goal admission and authorized realization. This lineage demonstrates authority provenance without transferring ownership among domains.

No domain may treat another domain's intermediate success as satisfaction. In particular, Strategy construction, Goal admission, planning, dispatch, task completion, publication, and outcome projection remain semantically distinct from evidence-backed Agent satisfaction.

## Failure closure contract

Failure to establish any required candidate, authorization, admission, or realization meaning closes the attempt without an admitted Goal or task-network mutation.

This closure preserves the prior semantic state. Uncertainty may prevent progress, but it must not be converted into authority or operational work.
