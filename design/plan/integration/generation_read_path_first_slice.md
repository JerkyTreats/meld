# Generation Read Path First Slice

Date: 2026-07-07
Status: implemented and verified per the evidence log; the `belief_context` flag ships off by default
Scope: ground content-producing capabilities in current belief through two mechanisms the existing contracts already sanction

## Purpose

Close the seam where accumulated belief never conditions generated content. No new domain, no new projection surface, no schema changes to goals or the task network. Two mechanistically small changes: a hydrated belief brief entering the task package as a seed artifact, and belief-governed frame selection replacing recency where a belief family covers the subject.

The sanctioning design text is [Belief Microarchitecture](../../cognitive_architecture/world_model/belief/microarchitecture.md): during task construction the agent may hydrate source facts, graph anchors, and evidence records referenced by the chosen belief view, as execution preparation.

## Current Truth

- Prompt assembly selects prior frames by recency: `src/context/generation/prompt_collection.rs` uses `OrderingPolicy::Recency` with `max_frames: 1` for directory children and `max_frames: 10` for node scope. The policy enum in `src/context/query/view_policy.rs` offers `Recency`, `Type`, `Agent`. No selection path consults belief.
- The `docs_writer_v2` task package assigns capability inputs at two moments: seed artifacts bound to init slots at trigger with a `source.kind` resolver, and expansion-time prerequisite edges wiring producer output slots to consumer input slots. Child `frame_ref` outputs already chain bottom-up into parent `upstream_artifact` slots.
- Goal curation records link each emitted goal to the belief revision and decision that caused it.
- `docs_freshness` is the only loaded belief family.
- The prompt lineage store persists rendered prompts and context payloads as digest-addressed artifacts.

## Change 1: Belief Context Bundle

A new artifact type `belief_context_bundle` carrying the trigger subject's current belief view: the current belief revision for the subject, its confidence and status, hydrated evidence references, any contradicted claims on the subject scope, and one current assertion per covered subject in the target subtree. The first slice hydrates the trigger subject's current view because the docs_writer trigger boundary carries no goal reference yet; goal-decision-linked hydration is deferred.

- Produced at trigger time by hydrating the trigger subject's current belief view through the belief read port, walking the target subtree so every covered descendant carries a seeded assertion.
- Enters the task package as a new seed `source.kind`, `goal_belief_hydration`, bound to an init slot like every existing seed artifact.
- Consumed by `context_generate_prepare` through a declared input slot. The seeded bundle is the only belief source at generation time.
- Lowering the bundle into prompt text is owned by the capability adapter, alongside the provider and rendering concerns that already live there. The world model never sees a prompt.
- The bundle digest is recorded in the prompt lineage record, so every artifact is attributable to the beliefs it was conditioned on.
- Bundle coverage is budgeted, an explicit deviation from full-subtree coverage: at most `MAX_BELIEF_CONTEXT_SUBJECTS` assertions are seeded, retained contradicted-first, then stale, then endorsed, and a byte-aware trim keeps the serialized bundle under the `MAX_CONTEXT_ARTIFACT_BYTES` artifact cap. The trim never evicts the trigger subject's own assertion, so a bundle whose trigger assertion alone exceeds the budget is passed through oversized and rejected at artifact write time with `PromptContextArtifactBudgetExceeded`. Omitted subjects are counted in `omitted_subject_count`, are absent from the assertion map, and consumers treat them as uncovered, falling back to recency selection.

## Change 2: Belief-Governed Frame Selection

A new selection policy, `BeliefEndorsed`, for frame collection during context assembly.

- For each candidate subject, consult the subject's seeded bundle assertion where a family covers it: exclude and explicitly flag frames whose subject is contradicted, annotate confidence, prefer endorsed frames.
- Fall back to `Recency` where no belief family covers the subject. With one loaded family this initially upgrades exactly one judgment: whether a child frame is semantically current per `docs_freshness`.
- Selection reads only the seeded bundle, so it is a deterministic function of the per-subject belief views captured in the bundle, each current at its own as-of sequence, with the bundle recording the high-water sequence across them. Replay is preserved across retries even when live belief state moves.

## Flag

Both changes gate behind `belief_context`, off by default, settable per workflow profile with environment override. Flag off is byte-identical to current behavior.

## Acceptance Criteria

- A/B contract test: same fixture workspace, same seeded beliefs, one run flag off and one flag on. Rendered prompts differ, the on-variant contains the belief brief content, and the on-variant lineage record links the bundle digest. Flag off changes no existing fixture or parity test.
- Determinism test: repeated hydration and selection over the same store state yield byte-identical bundle and selection results across reopen.
- Selection test: seeding a contradicted `docs_freshness` belief on one child excludes that child's frame content from `BeliefEndorsed` assembly, and a stale belief annotates the child and demotes its section ordering, while `Recency` assembly is unchanged.

## Deferred

- Goal-decision-linked hydration: producing the bundle from the belief view linked through a goal's curation decision record, once the trigger boundary carries a goal reference.
- Prompts as first-class artifacts with their own slots. Today turn `prompt_ref` entries are file paths resolved by `prompt_map`.
- Claim re-entry: extracting claims from produced artifacts as evidence.
- Belief families beyond `docs_freshness` and any salience ranking beyond status and confidence.
- A capability-side world model read for settled beliefs embodied in no frame. The bundle covers the trigger subject's subtree; the general read waits for evidence of need.

## Dependencies

No blocking dependency. Proceeds in parallel with the runtime activation program.

## Evidence Log

- 2026-07-07: first slice implemented and verified. The A/B contract test proves flag-off byte-identity and flag-on brief injection with lineage-linked bundle digests. The determinism test proves byte-identical bundles and selection across store reopen, and proves rendered prompts and lineage digests are unchanged when live belief state mutates after hydration, confirming the seeded bundle is the only belief source at generation time. The selection test pins uncovered before stale before contradicted payload ordering with contradicted content excluded while recency assembly is unchanged. Property tests cover canonical serialization stability, subject budget invariants including trigger retention and class-first eviction, and byte budget invariants. Integration suite passes 405 tests with one known unrelated pre-existing failure in the prompt cache test. The change survived a four-round adversarial review burn-down of forty-five confirmed findings, converging with only doc-precision and polish residue.
