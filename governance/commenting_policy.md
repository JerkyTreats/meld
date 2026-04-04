# Commenting Policy

Date: 2026-04-04
Status: active
Scope: Rust code comments for public contracts, domain boundaries, orchestration logic, and compatibility adapters

## Purpose

Define the repository expectation for code comments.

The goal is not to maximize comment volume.
The goal is to make public contracts, ownership boundaries, invariants, and non-obvious control flow easy to read and maintain.

## Core Rule

Comments should explain meaning, ownership, invariants, and intent.
They should not narrate obvious syntax or restate code line by line.

## Rust Idiomatic Posture

Use the Rust comment form that matches the purpose:

- use Rustdoc comments for public modules, public types, public traits, public functions, and public fields when the semantic role is not obvious from the type alone
- use line comments for local invariants, tricky branches, sequencing rules, compatibility seams, and safety or persistence reasoning
- use module-level Rustdoc when a domain file introduces a boundary or execution model that readers need before scanning the code

## Where Comments Are Required

Comments are required for:

- public task, capability, control, provider, context, and workflow contracts
- public traits that define domain boundaries
- orchestrators and executors with non-obvious sequencing or readiness rules
- artifact repo and graph code where invariants are easy to violate
- compatibility adapters that still exist for migration reasons
- code that intentionally preserves legacy behavior that would otherwise look redundant or strange

## Where Comments Are Usually Not Needed

Comments are usually unnecessary for:

- direct assignments
- straightforward field mapping
- short helper functions with obvious names
- obvious assertions that are already clear from the code
- trivial tests where the fixture and assertion already tell the story

## Public Rustdoc Expectations

Public Rustdoc should help a reader answer:

- what this thing is for
- who owns it
- what assumptions or invariants it relies on
- what it does not own when boundary confusion is likely

Public Rustdoc does not need to become a design document.
Keep it short, specific, and domain-accurate.

## Inline Comment Expectations

Inline comments should be brief and placed only where they reduce reader confusion.

Prefer comments that explain:

- why this branch exists
- why ordering must be preserved
- why a compatibility shim is still necessary
- why data is persisted or not persisted at a certain boundary
- why an invariant is enforced at one layer instead of another

Avoid comments that merely translate Rust into English.

## Comment Maintenance Rule

When behavior changes, update or remove stale comments in the same change.
Incorrect comments are worse than missing comments.

## Capability And Task Guidance

For the capability and task buildout, apply this policy with extra care in these areas:

- capability contract publication
- sig adapter input resolution
- task compiler edge derivation
- task executor readiness and release logic
- artifact repo lineage and supersession
- workflow compatibility adapters
- task and capability event emission boundaries

## Review Guidance

During review, treat missing comments as a real issue when:

- a public contract would be hard to understand without Rustdoc
- orchestration logic relies on an invariant that is not obvious from the code shape
- compatibility code exists without any note on why it remains
- ownership boundaries could be misunderstood by a future editor

Do not require comments just to satisfy a quota.
Require comments where they materially improve correctness and maintainability.
