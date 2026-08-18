# Visual Concept Expansion For Render Hydration

Date: 2026-08-15
Status: scoped discovery case
Scope: first-class visual concepts that expand narrative selections into render-ready image specifications

## Purpose

This case extends the freeform narrative database example with one bounded image-product need.

A user should be able to request an image using compact narrative language such as:

```text
Generate an image of Kelsey at the Moonbeam Gala
```

The external visual product resolves the selected character and event, hydrates their current narrative state, selects applicable visual concepts, and expands those concepts into the larger prompt and workflow representation needed for visual lock.

This case does not generalize concept expansion into a Meld-wide reasoning mechanism. It records the smaller product feature independently so it can be evaluated without the broader lateral-lowering idea.

## Working Model

A visual concept is a first-class, revisioned product. It may be selected by a short tag or natural-language reference while carrying a much richer expansion.

```text
visual.atmosphere.moonbeam_gala
→ moonlit grand ballroom
→ celestial ornamentation
→ silver and midnight-blue palette
→ crystal chandeliers
→ formal eveningwear
→ dense gathering of elegantly dressed guests
→ layered background figures
→ luminous editorial composition
→ controlled visual clutter
```

The concept is not equivalent to the flattened words that currently realize it.

```text
visual concept identity
!= current concept revision
!= model-specific render expansion
```

The same visual concept may have distinct render expansions for different model families, workflows, aspect ratios, media, or quality targets while preserving one intended visual identity.

## Concept Composition

A concept may refer to other concepts. The Moonbeam Gala atmosphere may compose a celestial lighting concept, an elegant social setting, a crowded formal gathering, and event-specific material that belongs only to their combination.

```text
Moonbeam Gala
→ Moonbeam Gala atmosphere
  → celestial ballroom lighting
  → silver-blue luminous palette
  → crowded formal gathering
  → editorial fantasy treatment
```

The combined atmosphere remains a distinct product. It is not merely the union of its child expansions. It may contribute its own ordering, weights, exclusions, regional instructions, and visual references.

## Render Hydration

The bounded product flow is:

```text
natural-language render request
→ narrative entity and scene resolution
→ current accepted character and event state
→ applicable visual concept selection
→ bounded transitive concept expansion
→ render-ready specification
→ external image workflow
```

A render-ready specification may contain:

- positive prompt material and weights
- negative prompt material and exclusions
- model and checkpoint selection hints
- LoRA, embedding, and reference-image bindings
- regional prompting and composition constraints
- ControlNet or equivalent guidance bindings
- sampler, scheduler, resolution, and workflow parameters
- exact narrative and visual-concept revision references

These fields are owned and interpreted by the external visual product. Meld need not understand ComfyUI node graphs or model-specific prompt syntax to preserve the selected concept identities and lineage.

## Ownership Boundary

The external narrative product owns narrative entities, claims, editorial acceptance, and render requests.

The external visual product owns visual concept declarations, render expansions, model bindings, visual references, prompt compilation, workflow construction, render history, and visual-lock evaluation.

Meld may preserve and query admitted concept identities, revisions, relations, source references, and derived-specification lineage through served domain contracts. Generic graph proximity does not establish that a visual concept applies to a render request.

PDS is optional. It becomes relevant only if a standing mandate must maintain concept identity, dependency freshness, affected-only refresh, and verified visual lock as narrative state, visual assets, or model profiles change.

## Bounded Expansion

Concept composition may be recursive, but one render request expands under an explicit render context and finite limits. The context selects a concept revision, target model profile, media, narrative revision, and requested scene. The visual product determines which dependencies apply and stops expansion at its declared boundaries.

This scoped feature does not require learned attention, semantic similarity search, reinforcement, autonomous corpus growth, or Regime integration. Those possibilities belong to the separate lateral-lowering discovery record.

## Success Shape

The feature is useful when a user can invoke a stable high-level concept while the visual product can produce and explain the complete model-specific expansion used for one render.

For the representative request, the resulting specification should answer:

- which Kelsey identity and character-state revision were selected
- which Moonbeam Gala event and scene state were selected
- which visual concepts were applied
- how those concepts expanded for the selected model profile
- which negative constraints and asset bindings were introduced
- which exact revisions can reproduce or explain the render request

## Explicit Non Commitments

This case does not commit Meld to a tag ontology, prompt language, vector database, embedding provider, image store, ComfyUI schema, visual model registry, recursive inference engine, or generalized lateral-thinking mechanism.

It does not treat generated images as narrative authority. It does not require PDS for ordinary concept storage, lookup, expansion, or rendering.

## Related Discovery

- [Freeform Narrative Database Stewardship](README.md)
- [Activation-Bounded Lateral Lowering](../../../ideas/activation_bounded_lateral_lowering.md)
- [Freeform Narrative Database Constraint Case](../freeform_narrative_database.md)
