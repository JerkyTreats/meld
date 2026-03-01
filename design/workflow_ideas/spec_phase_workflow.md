# Repeatable Spec Workflow

Date: 2026-03-01
Status: draft

## Intent

Capture a reusable process for taking a large feature from early discovery to a complete technical phase specification.

## Suggested Location

This document lives in `design/workflow_ideas` as a drafting area for reusable workflow patterns.

## High Level Flow

1. define mission and scope boundaries
2. gather entry sources from roadmap docs and active workload docs
3. run code seam discovery and produce evidence findings
4. classify findings by domain ownership and blast radius
5. order work by enablement so cleanup lands before dependent feature work
6. split work into workload folders with one workload `README.md` per folder
7. write focused specs for each concern with done criteria and verification
8. synthesize roadmap and findings into one phase technical specification
9. align reading order references and labels across all docs
10. define acceptance gates test strategy and completion criteria
11. run refinement loops and keep docs declarative to current phase assumptions

## Inputs

- roadmap entry document for the workstream
- workload README documents
- code path findings document
- boundary cleanup specs when applicable

## Outputs

- workload index with clear reading order
- focused concern specs
- synthesized phase technical specification
- explicit gate checklist and completion definition

## Refinement Prompts

- what assumptions became stale after the latest code changes
- where ownership boundaries are still ambiguous
- whether operation order still minimizes blast radius
- what tests are still missing for deterministic verification

## Evolution Path

When meld workflow execution components are ready, this process can be encoded as a first class workflow profile with checkpoints for each phase gate.
