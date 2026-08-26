# Use Case: Commenting Style

Date: 2026-07-24
Status: proposed control
Scope: keep all code files compliant with the project's commenting policy

## Use case

A project maintains a commenting policy: comments should state constraints the code cannot show, avoid narrating what the next line does, and stay idiomatic to the codebase. As code changes, comments drift out of compliance — stale explanations survive refactors, new code arrives over-commented or under-commented, and review attention is inconsistent. A maintainer wants every file held to the policy continuously, with judgment applied where the policy is a matter of idiom rather than syntax, and with the whole scope re-examined when the policy itself changes.

The mechanical part of this is a solved problem: linters enforce syntactic comment rules cheaply and deterministically, and a linter plus a codemod is the mandatory baseline. The judgmental part is not lintable: whether a comment is high-signal, whether it narrates rather than explains, whether the density matches the surrounding idiom. The use case is justified only for that judgment layer and for the standing-mandate property: compliance is re-established after every drift, indefinitely.

This use case is deliberately close to documentation freshness. Its catalog role is a control: it shares the evaluator-over-bytes pattern while stripping away hierarchy, coverage, and compression. A correct derivation must be trivially flat.

## Meld semantics conversion

This conversion is fluid and has not been worked at canonical depth.

- Subjects are source files; relations are containment for scope walking only, carrying no obligations.
- The maintained condition holds each file's commenting compliance at or above threshold, evaluated against exact file bytes under an exact policy revision.
- Compliance is an observational dimension settled by a judgment evaluator whose identity binds file digest, policy revision, evaluator revision, and dimensions.
- Affordances are inspect file, evaluate compliance, rewrite comments, publish revision. Ordering is entailed: evaluation requires bytes, publication requires a produced revision, settlement requires evaluation of the published bytes.
- The derived topology is flat per-file chains with no cross-subject edges. If coverage artifacts, hierarchy obligations, or compression steps appear in a derivation for this domain, the constructor or the schema is over-indexed on documentation and the control has caught it.

## Required semantics

For this use case to be usable on paper, the following must hold. Standing is given per entry.

1. **Policy revision inside evaluation identity.** A compliance verdict must be bound to the exact policy revision it applied, so a policy edit invalidates rather than contradicts prior verdicts. Canonical pattern exists in exact-byte evaluation identity.
2. **Mass invalidation damping.** A one-line policy change invalidates every settled compliance belief in scope at once. Curation must be able to tolerate, defer, or lazily re-establish rather than eagerly re-evaluating the world; materiality and cost beliefs are the mechanism. Canonical cost-benefit curation exists in design; the damping behavior is unproven anywhere.
3. **Judgment evaluators alongside mechanical ones.** The theory must compose a deterministic linter verdict and a model-backed idiom verdict as distinct dimensions without letting either substitute for the other. Canonical evidence typing supports this; evaluator routes are unbuilt.
4. **Trivial-topology derivation.** The constructor must produce flat independent candidates when no constraint couples subjects, activating none of the structural machinery. This is the control assertion; it is testable only once the constructor exists.
5. **Baseline subtraction.** The mechanical rule subset must remain delegable to the linter inside the theory, so Meld work concentrates on judgment dimensions. This is a domain-theory authoring concern rather than a runtime capability.

## Read with

- [Use Case Catalog](README.md)
- [World Model Strategy](../cognitive_architecture/world_model/strategy/README.md)
- [Contribution Policy](../../governance/contribution_policy.md)
