# World Model Vision

Date: 2026-05-01
Status: active
Scope: long-range research vision for the world model beyond the current grounded design baseline
NOTE: User generated. Do not edit without explicit authorization.

## To Each Domain Be True

The operating axiom for domain architecture in this project is "To Each Domain Be True." We will be exploring 6 distinct "Cognition Layers," and will need to model performant code for each domain.

Each layer should ideally have a single, unified substrate for interaction with other layers.

Cognition Layers:

1. boundary and event spine
2. bitemporal state graph
3. belief and generative inference
4. causal layer
5. regime layer
6. planner surface

## En Masse and At Will: Independent Agents Requirement

In shaping the technical architecture, one clear requirement must be satisfied: Horizontally scalable "Agents".

An Agent in the current meld context is a set of prompts and workflows owning specific `DomainObjectRef` values.

There are many Agents, and most importantly, Agents can be created by customers as a runtime concern.

The technical architecture shall be extended to Belief. Not only will Agents have an internally consistent belief set, but considerations should be taken for 1 -> 12 -> 64 -> 128 Agents operating as independent entities.

All Agents will share the underlying substrate of belief manifestation, but the facts they observe, the beliefs they generate, the regime they understand to be in, will be wholly owned by each Agent.

### The Architecture of Agent Swarms 

The rejected entity-component-system design note is no longer an active architecture source. Two requirements remain:

* meld-world-model has been broken out into its own crate
* The principle of En Masse and At Will has been defined as core vision 

Given this requirement, each world model subdomain should define one consolidated spec with domain types, data model, and pipelines.

This consistency in design language leads to consistency of software architecture, allowing the concept of Agents to scale horizontally, en masse and at will.

Note: Other crates are not expected to follow world model implementation vocabulary. Public APIs are the only consumption method, and each crate owns its own architecture. 
