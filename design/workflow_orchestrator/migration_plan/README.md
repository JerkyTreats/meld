# Migration Plan

Date: 2026-03-27
Status: active

## Intent

Define how current behavior moves into the capability and plan model without breaking command behavior.

## Migration Rules

- preserve command behavior first
- preserve output quality first
- move mixed concerns out of domains before broad compiler cutover
- add compatibility lowering only as an input path into compiler
- do not preserve `workflow` as the long-term abstraction

## First Slice Migration Targets

- current `context generate`
- current docs writer flow

## Required Migration Outcomes

- current behavior lowers into candidate capability graphs
- compiler emits locked plans for those graphs
- domain code executes behind explicit capability contracts
- hidden workflow-shaped sequencing stops being the durable model
