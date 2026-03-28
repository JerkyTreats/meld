# Migration Plan

Date: 2026-03-27
Status: active

## Intent

Define how current behavior moves into the capability and plan model without breaking command behavior.

## Migration Rules

Preserve command behavior first and preserve output quality first.
Move mixed concerns out of domains before broad compiler cutover.
Add compatibility lowering only as an input path into compiler.
Do not preserve `workflow` as the long-term abstraction.

## First Slice Migration Targets

The first slice migration targets are current `context generate` and the current docs writer flow.

## Required Migration Outcomes

Current behavior lowers into candidate capability graphs.
Compiler emits locked plans for those graphs.
Domain code executes behind explicit capability contracts.
Hidden workflow-shaped sequencing stops being the durable model.
