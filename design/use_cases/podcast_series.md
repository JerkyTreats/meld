# Use Case: Podcast Series

Date: 2026-07-24
Status: frontier
Scope: research a supplied topic and maintain a coherent, teachable, multi-episode audio series for it

## Use case

A user supplies a topic. The system researches it, breaks it into teachable concepts, arranges those concepts into a sequence of episodes, writes a script for each episode of roughly an hour, keeps a narrative throughline intact across the whole series, synthesizes each script to speech, and saves the audio files. The user judges the result by whether the series teaches: concepts build on one another, each episode stands alone well enough to follow, and the series as a whole tells one story rather than ten disconnected lectures.

Unlike the maintenance use cases, almost nothing here exists before the work begins. There is no folder tree or dependency manifest to observe: the concept breakdown, the episode structure, and the scripts are all products of the work, and each product becomes the ground the next stage stands on. Quality is dominated by a global property — the throughline — that belongs to the sequence, not to any single episode: editing episode two can break the coherence of episodes one and three. And the request is a delivered creation rather than a standing condition, though a maintained reading exists: the series should remain current and coherent if the topic develops or the user's requirements change.

There is no meaningful deterministic baseline. Single-prompt generation cannot hold an hour-per-episode series coherent, and a fixed pipeline cannot decide how many episodes a topic needs or when research suffices.

## Meld semantics conversion

This conversion is fluid, has not been worked at canonical depth, and is expected to force schema findings. That is its catalog role.

- The user request enters as user-directed desired state: a complete, coherent, teachable series for topic T exists at quality thresholds, satisfied on delivery and reopened by topic drift or requirement change.
- Subjects are the topic, research sources, concepts, episodes, scripts, and audio artifacts — but concepts and episodes do not exist at grounding time. Grounding is staged: first ground that an admitted concept decomposition exists; once the decomposition is admitted, its content materializes concept and episode subjects with sequence relations, and per-episode questions ground over them.
- The throughline is a sequence-scoped obligation. Its natural derivation is a series outline produced and admitted first as a global constraint artifact; each script is evaluated for coherence against the admitted outline. This is the settled-coverage pattern run top-down: a globally produced artifact gates children, where documentation runs child-produced artifacts up to parents.
- Bounded context reappears without a filesystem: the research corpus exceeds any script-writing context, forcing admitted research summaries as compression artifacts feeding the outline and scripts.
- Dimensions mix: teachability and coherence are observational, settled by evaluators; audio existence and duration are directly assertable effects, exercising the settlement transform's pass-through branch in the same Goal.

## Required semantics

For this use case to be usable on paper, the following must hold. Standing is given per entry.

1. **Artifact-to-graph projection.** The domain theory must be able to declare that an admitted artifact of a given type materializes subjects and relations in the graph, so grounding can quantify over structure the work itself produced. No such declaration exists anywhere in the schema; this is the largest known missing surface and the primary reason this use case is in the catalog.
2. **Staged grounding.** Grounding must run in stages as produced structure is admitted, with later-stage questions unaskable until earlier admissions land. The trusted-scope gate supports the sequencing; grounding over produced rather than observed structure depends on entry 1.
3. **Sequence-scoped obligations.** Obligations must bind tuples or sequences of subjects, and evidence for a sequence property must invalidate per the affected members. The obligation vocabulary currently binds single subjects; this is an open amendment.
4. **Top-down constraint artifacts.** A globally admitted artifact must be able to gate many downstream obligations through prospective contracts. The existing contract shape appears to support the direction inversion; the worked walk must confirm no new record kind is needed.
5. **User-directed creation Goals.** A user request with no prior divergence must enter as high-weight value evidence through ordinary curation. Canonical in Goal curation.
6. **Mixed dimension targets.** One Goal target must combine observational and directly assertable propositions, with settlement transforming only the former. Canonical as the transform's pass-through branch.
7. **Provider-heavy capacity.** Research, generation, and synthesis quotas and costs must flow as capacity facts and cost ceilings. Canonical pattern; scale is the only novelty.

## Read with

- [Use Case Catalog](README.md)
- [World Model Strategy](../cognitive_architecture/world_model/strategy/README.md)
- [Directive Grounding](../cognitive_architecture/world_model/agent/directive_grounding.md)
