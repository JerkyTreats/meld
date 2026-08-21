# Task Network

The Task Network is Execution's single durable operational graph. It contains compiled Task regions, shared action nodes, dependencies, resources, claims, outcomes, and Goal admission attribution.

The Task Network is not a Strategy Plan. It contains no Epistemic Operations, beliefs, directive reasoning, or product-domain satisfaction rules.

## Identity

Task identity, Goal admission identity, and operational node identity are distinct. One Task may compile into several nodes. Compatible admissions may share a node. One admission may gain evidence from several nodes.

## Commands

All writes use explicit commands with expected revision, activation generation, authority, and idempotency key. Mutation sets commit atomically. Readers observe committed revisions only.

Commands may insert compiled regions, attach admission attribution, add Execution-owned ordering or resource edges, claim ready nodes, record outcomes, release claims, suspend work, or retire completed regions.

## Ready Set

A node is ready only when dependencies, guards, artifacts, authority, resources, effect compatibility, lifecycle epoch, and activation fence all permit dispatch.

## Shared Work

A shared node preserves every contributing admission and result attribution. Reuse is legal only when Capability contract, inputs, source revision, observable effects, authority, validity window, and result schema are compatible.

## Dispatch And Recovery

Dispatch records a durable claim before invoking a runner. Outcomes name the exact node, lifecycle epoch, claim, attempt, Capability revision, and activation generation.

Restart reconstructs the ready set and claims from durable network state. Bounded retry and operational recovery remain inside Execution. A new semantic course of action requires a new producer admission.

## Events

Task Network mutations and outcomes are appended through Events. Consumers can attribute every external effect to Task, Goal admission, producer, and activation lineage.
