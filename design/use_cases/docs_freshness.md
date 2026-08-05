# Use Case: Docs Freshness

Date: 2026-07-24
Status: worked canonical example with historical role
Scope: keep folder-level documentation correct as the code it describes changes

## Use case

A software workspace contains folders of source code. Each folder should carry a README that faithfully describes what the folder contains, why it exists, and how its parts relate — and that description should stay true as the code changes. Documentation of this kind decays silently: files are added, renamed, and rewritten while the README says what used to be true. A maintainer wants the documentation to stay correct without hand-auditing every folder after every change, wants generated text to be checked against the actual code rather than trusted, and wants the checking to survive indefinitely rather than run once.

The work has a natural structure. A folder's README depends on the folder's own files and on an accurate picture of its subfolders. Deep trees contain more material than any single writing pass can hold in view at once, so summaries of subtrees become inputs to their parents. Quality is judgmental rather than mechanical: a README can be well-formed and still wrong, incomplete, or stale.

## History

Docs freshness is the original proof scenario for the cognitive runtime, and its residue is deliberately widespread in the codebase.

The original vertical slice threaded the thinnest possible path through every layer using this scenario: the `docs_freshness` belief family proved the belief layer, the confidence-threshold curation rule proved the Agent, the planner projection proved the world-model-to-execution bridge, and the docs writer proved task execution. Each layer's first slice was validated against this one domain, which is why the name appears in belief fixtures, the `refresh_docs_v1` method fixture, integration tests, the docs writer workflow packages and prompts, and one product constant in belief-context assembly.

Before the sense-model-act runtime existed, the use case ran as an authored workflow: the docs writer package selected bottom-up traversal, fixed stages, ordered turns, and child artifact wiring explicitly in YAML. That workflow is the prior architecture, retained as a characterized compatibility Method while composition-path parity is proven under the runtime completion authority. The Strategy design work of 2026-07 exists to show the same behavior can be derived from declared meaning instead of authored procedure.

The consequence for readers: `docs_freshness` appearing throughout tests and runtime code is evidence of its role as the ladder every layer climbed, not evidence that the runtime is docs-specific. The structural audit of what is genuinely generic versus docs-bound lives in [Strategy Ground Map](../plan/world_model/strategy/ground_map.md).

## Meld semantics conversion

The conversion is worked at full canonical depth in [Docs Freshness Strategy](../cognitive_architecture/world_model/strategy/docs_freshness.md), which is authoritative for this use case. In summary:

- Subjects are workspace, folders, source files, READMEs, and coverage artifacts; relations are containment, documents, and derivation.
- The maintained condition requires each material folder's README to meet a correctness threshold evaluated over exact published bytes.
- Correctness is an observational dimension settled by an evaluator; settlement, not the score, is what candidates may project.
- Two-tier evidence distinguishes pre-generation source summaries from settled folder coverage admitted only after evaluation.
- Bottom-up fan-out is derived from recursive coverage obligations plus bounded context plus admission gates, never declared.

## Required semantics

For this use case to be usable on paper, the following must hold. Standing is given per entry.

1. **Containment relations are projected into planning frames.** Grounding and construction may eventually need typed containment propositions. This is outside the minimal slice.
2. **Capacity is a typed fit verdict.** Per-folder context fit is a design hypothesis with no current code ground.
3. **Evaluator settlement over exact bytes.** Exact-byte evaluation is a use-case requirement whose route is not established by Strategy.
4. **Two-tier evidence with admission authority.** Summary production must remain distinct from settled coverage. A dedicated admission-verdict mechanism is unproven.
5. **Prospective evidence gating.** The minimal contract requires a prospective evidence route. Obligation-discharge machinery is not canonical.
6. **Settlement regression.** The minimal contract requires evidence separation. Regression against a settlement question remains an unproven mechanism rather than canonical design.

## Read with

- [Use Case Catalog](README.md)
- [Docs Freshness Strategy](../cognitive_architecture/world_model/strategy/docs_freshness.md)
- [Strategy Ground Map](../plan/world_model/strategy/ground_map.md)
