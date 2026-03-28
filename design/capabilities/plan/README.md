# Plan Model

Date: 2026-03-27
Status: active

## Intent

Define plan as the durable orchestration product.

## Definition

A plan is a validated, locked DAG of bound capability instances.
Plan has three lifecycle concerns: compilation, execution, and record.
Current design work is centered on compilation.

## Plan Rules

A plan is not a goal, a capability catalog, or an execution trace.
It is the locked result of compiler validation.
A plan is ready for execution when all graph and artifact contracts validate, and it may contain parallel-ready branches.

## Core Records

The core records are compiled plan, capability instance, dependency edge, artifact handoff, binding digest, and scope digest.

## Graph Rules

Plan structure is a DAG.
Dependency edges express execution preconditions.
Artifact handoffs express producer-consumer data flow.
Those two structures are related but not interchangeable.
Absence of an edge means no execution precondition was declared.

## First Slice

The first slice plan model must support current docs writer compatibility paths, current `context generate`, and future capability families without changing the graph substrate.
