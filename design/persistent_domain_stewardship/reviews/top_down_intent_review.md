# Top-Down Review: From User Intent Toward PDS Lowering

Date: 2026-08-08
Status: independent architecture review
Scope: interrogate the PDS layering question starting from the principal/user surface and moving downward, without using runtime implementation convenience as the authoring model

> This review predates the accepted PDS cognition boundary. Where it treats exact actions or Methods as compiled PDS output, use [Canonical Persistent Domain Stewardship](../../cognitive_architecture/persistent_domain_stewardship.md) instead.

## Question

Are the current detailed PDS examples better understood as compiler-facing material than as the eventual end-user expression of a persistent domain steward?

## Starting Constraint

The customer-facing surface exists to express intent and policy. It should not require understanding belief leases, task networks, event projections, comparator weights, runtime actors, evidence mapping ids, or method realization mechanics.

The current profile hypothesis already provides the intended user vocabulary:

```text
steward type
scope
desired conditions
sensitivity
autonomy
budget
escalation
verification
```

This review treats those concepts as the starting point and asks what representations are required below them.

## Finding 1 — The Detailed Examples Are Too Low-Level To Be The Principal Authoring Surface

The current examples directly discuss:

- belief families;
- evidence schemas and mappings;
- comparator semantics;
- curation rules;
- methods and actions;
- task packages;
- outcome mapping;
- runtime provenance;
- authority enforcement seams.

Those concepts are necessary to make stewardship executable, but they are not normally choices a repository owner, service owner, teacher, game designer, or narrative user should have to author.

**Conclusion:** the examples are not candidate user syntax. They are evidence for expert package semantics and compiler output requirements.

## Finding 2 — There Must Be A Durable Approval Representation Between UI And IR

The user surface may be conversational, visual, forms-based, YAML, an SDK, or another interface. That surface is not sufficient as the durable authority boundary because different interfaces may express the same steward.

The system therefore needs a canonical declaration that records what the principal actually approved:

```text
selected steward family/profile
concrete scope
selected objective presets
policy choices
requested autonomy/authority
budget selection
escalation policy
verification policy
principal/assignment identity
package/profile revision constraints
activation requirements or references
```

This declaration should be:

- stable;
- diffable;
- inspectable;
- approvable;
- portable across authoring interfaces;
- independent of process-local runtime topology.

The principal should be able to understand its effect without reading the compiled theory image.

## Finding 3 — Package-Defined Presets Are The Main Compression Mechanism

The user should not set raw curation thresholds, evidence reliability values, or comparator factors in routine use. Instead, a domain package exposes named semantic choices.

For example:

```text
sensitivity: strict
verification: source_backed
autonomy: draft
```

may compile to domain-specific:

```text
breach threshold
restore threshold
freshness windows
observation budget
curation posture
required outcome evidence
permitted action classes
```

The named choice must remain package-defined. PDS should not assign one universal meaning to `strict`, `balanced`, or `release_grade`.

## Finding 4 — Defaults Are Semantic And Must Not Be Invisible Compiler Guesswork

A compiler cannot fill materially meaningful omissions using arbitrary internal defaults.

A default is acceptable only when it is part of the selected package/profile revision and therefore:

- inspectable;
- versioned;
- deterministic;
- represented in semantic diff;
- reproducible during replay.

If a required policy choice has no declared default, compilation should remain unresolved rather than silently manufacture one.

## Finding 5 — Assignment And Activation Must Remain Distinct From Steward Intent

The user can mean the same steward while deployment changes.

For example:

```text
keep docs current for /services/payments
```

is conceptually distinct from:

```text
use provider X
credential Y
sensor adapter Z
runtime placement Q
quota R
```

The first belongs to steward intent/declaration. The latter belongs to activation or physical binding.

A canonical declaration may reference activation requirements, but it should not collapse the steward's normative meaning into one machine-specific deployment record.

## Finding 6 — The Declaration Must Be Semantically Diffable At The User's Level

A useful approval diff should say things such as:

```text
scope expanded from service A to services A+B
autonomy changed from recommend to draft
verification changed from standard to release_grade
critical-risk exception approval changed from manual-only to delegated
```

It should not primarily say:

```text
belief family hash changed
mapping set id changed
method realization changed
```

Those lower-level changes still matter for lineage, but they belong to the compiled semantic diff. The product needs both levels.

## Finding 7 — User Intent Must Be Complete Enough To Prevent Authority Inflation During Lowering

The compiler may resolve implementation detail, but it may not create authority that the principal did not approve.

The declaration therefore needs an explicit authority posture that constrains all compiled actions. A package may declare available actions; the declaration selects the permitted subset/posture; assignment and external governance may further restrict it.

The effective authority must be no greater than the approved declaration and assignment grant.

## Finding 8 — User Vocabulary Should Select Domain Meaning, Not Runtime Mechanisms

A user may select:

```text
maintain documentation freshness
maintain critical CVE exposure below policy
maintain character continuity
maintain source-backed canon definitions
```

The compiler may lower those choices into:

```text
belief families
evidence mappings
maintained propositions
curation rules
methods
actions
outcome contracts
```

The user should not need to know which runtime domain owns those lowerings.

## Finding 9 — The Current Examples Are A Hybrid, Not Pure IR

The detailed examples contain two different compiler-side layers:

### Expert package/facet source

Examples:

```text
domain vocabulary
belief-family definitions
source authority semantics
supported actions
verification profiles
governance requirements
named user-facing presets
```

This material may remain human-authored by package/domain experts.

### Lowered semantic IR

Examples:

```text
resolved belief-family revision
resolved evidence mapping revision
meld-lang propositions and methods
available-action bindings
method realizations
task-package references
content hashes and linked symbols
```

This material should be compiler/linker output.

The existing examples often describe both because they were intentionally developed from the runtime upward.

## Finding 10 — Round-Trip Explanation Is A Product Requirement

Given a compiled stewardship image, the system should be able to explain it in declaration vocabulary:

```text
why is this observation being acquired?
which approved objective requires this belief?
why is this action available?
which authority selection permits it?
which verification policy requires this evidence?
```

This does not require reproducing the original conversational wording. It requires traceability from compiled semantics back to the canonical declaration and package/profile symbols.

## Top-Down Acceptance Test

A proposed PDS authoring design passes this review when a principal can define and approve a steward without seeing runtime mechanics, while the declaration remains sufficiently explicit to make compilation deterministic and authority-preserving.

For each example, ask:

1. Can the user state the desired stewardship outcome in package vocabulary?
2. Can scope and authority be understood without runtime knowledge?
3. Are sensitivity, budget, escalation, and verification selectable without editing low-level theory?
4. Can a semantic declaration diff be reviewed by the principal?
5. Can every compiled semantic choice be traced back to either the approved declaration or the selected package/profile revision?
6. Can the compiler refuse unresolved choices instead of inventing semantics?

## Verdict

**Validated.** The current detailed PDS examples should not be promoted into end-user syntax.

More precisely, they span:

```text
expert-authored package/facet semantics
+
hand-authored compatibility forms of compiled semantic IR
```

The eventual user expression belongs above both, with a canonical, approvable declaration between the authoring surface and the compiled stewardship image.
