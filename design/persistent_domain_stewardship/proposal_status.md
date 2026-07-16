# Proposal Status And Decision Semantics

Date: 2026-07-16  
Status: proposed  
Scope: interpretation rules for Persistent Domain Stewardship design documents

## Purpose

Persistent Domain Stewardship is a design proposal.

It is not an authoritative implementation contract, crate boundary, package schema, source language, migration commitment, or accepted runtime architecture.

The proposal exists to:

- define the problem PDS may solve
- identify reusable abstractions
- compare architectural options
- expose assumptions and unresolved ownership questions
- map candidate concepts to existing Meld runtime anchors
- produce falsifiable implementation hypotheses

The proposal should become more precise without implying that every explored structure has been accepted.

## Status Vocabulary

PDS documents use the following decision labels.

| Label | Meaning |
|---|---|
| Constraint | Existing Meld architecture or governance rule that PDS must respect |
| Hypothesis | A model believed useful enough to test |
| Option | One possible design with stated advantages and costs |
| Recommendation | Current preferred option based on available evidence |
| Open | A decision that remains unresolved |
| Illustrative | An example used to test an abstraction; not a frozen schema |
| Deferred | Deliberately outside the current proposal |
| Rejected | An option considered inconsistent with current constraints or goals |

## Normative Language

Normative wording is interpreted as follows.

- **must** and **must not** are reserved for established Meld constraints or safety invariants.
- **candidate requirement** identifies a proposed implementation requirement.
- **recommended** identifies the current preferred option.
- **could** and **may** identify alternatives.
- Code and schema blocks are illustrative unless explicitly labelled as a candidate contract.

## Current Constraints

The proposal currently treats these as established constraints:

1. Existing domains remain authoritative for their own source truth, state, and behavior.
2. PDS must not create a second event ledger, belief engine, planner, task executor, or capability runtime.
3. Capability availability does not imply authority to invoke it.
4. Planner effects are predictions rather than verified environmental outcomes.
5. Source systems remain authoritative where Meld is not the system of record.
6. Runtime vocabulary may remain open-ended while package authoring may add static validation.
7. A simpler controller, workflow, rule engine, or optimizer remains the mandatory baseline for each proposed steward.

## Current Hypotheses

The proposal is testing these hypotheses:

1. Persistent Domain Stewardship is a useful application model above the cognitive runtime.
2. Dissimilar stewards share a common profile and package shape.
3. Standing objectives and recurring episodes are distinct from transient goals and tasks.
4. PDS can be expressed without moving domain authority into a central super-domain.
5. Customer-facing steward profiles can remain materially simpler than the full runtime representation.
6. Existing workflows can become reusable methods or compatibility mechanisms beneath stewardship without discarding durable execution mechanics.

## Current Recommendations

The current recommendations are not final decisions:

- Treat PDS as a control-plane meta-domain rather than the cognitive data plane.
- Explore federated domain-owned facets rather than one PDS-owned universal package schema.
- Separate package semantics, user profile, assignment, and physical activation.
- Use a small structured profile language with generated visual and conversational editing surfaces.
- Treat the package compiler primarily as a linker and coordinator of domain-owned compilation.
- Bind runtime history to exact package, profile, assignment, and activation identity.

## Explicitly Open

The following remain open:

- whether PDS becomes an extracted crate or remains root-product composition
- central package schema versus federated facets
- YAML, HCL, custom profile DSL, or another source representation
- authoritative versus projected ownership of stewardship episodes
- Agent-owned versus PDS-owned standing-objective state
- assignment and activation as separate or initially unified records
- location of context-projection declarations
- package scenario execution ownership
- workflow migration endpoint
- package upgrade and rollback semantics

See [Open Decisions](open_decisions.md).

## Reading Rule

When another PDS document uses concrete Rust types, YAML, state machines, commands, or crate names, read them as proposed models unless the document explicitly identifies an existing Meld contract.

The proposal should preserve competing options long enough to compare them against:

- domain isolation
- user simplicity
- runtime implementation evidence
- package portability
- upgrade safety
- operational inspectability
- cross-domain reuse

## Decision Promotion

A recommendation should be promoted to an accepted design only after:

1. its alternatives are documented;
2. affected domains are assessed independently;
3. runtime anchors and ownership are identified;
4. at least one concrete package proves the path;
5. a dissimilar package tests generality;
6. migration and upgrade behavior are demonstrated;
7. the design record is moved or restated in an authoritative implementation plan.

Until then, PDS documents remain a proposal corpus.