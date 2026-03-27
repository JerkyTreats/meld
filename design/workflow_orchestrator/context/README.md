# Context Capability Readiness

Date: 2026-03-27
Status: active

## Intent

Define the refactor work required to make current `context generate` behavior capability-ready and plan-ready.

## Current Problem

Current `context generate` behavior still mixes several concerns:

- target expansion
- ordering
- generation execution
- retry and resume assumptions
- workflow-shaped sequencing
- output shaping

That mixed ownership prevents clean capability contracts and clean plan compilation.

## Required End State

The `context` domain should own only domain behavior behind explicit capability contracts.

For the first slice, `context` must expose:

- a `context_generate` capability contract
- typed input artifact contracts
- typed output artifact contracts
- execution-facing implementation behind the contract

The following concerns must move out of `context`:

- plan graph assembly
- dependency edge construction
- artifact handoff validation
- execution ordering policy
- workflow-shaped retry policy

## First Slice Refactor Work

- extract target expansion outputs into typed artifacts
- extract ordering inputs and outputs into a plan-consumable contract
- isolate generation execution behind a capability-facing adapter
- remove hidden coupling between generation execution and ordering policy
- remove hidden coupling between generation execution and workflow-wide retry policy
- make result artifacts explicit enough for downstream file materialization

## Required Inputs

The first slice `context_generate` capability must accept:

- scope reference
- ordered target artifact
- generation policy binding
- provider binding
- agent binding when required

## Required Outputs

The first slice `context_generate` capability must emit:

- generation result artifact
- frame reference artifact when materialized frames exist
- structured observation summary
- structured effect summary

## Non Goals

- moving plan compilation into `context`
- preserving hidden ordering behavior inside `context`
- preserving workflow-specific sequencing inside `context`
