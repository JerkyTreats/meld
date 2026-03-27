# Plan Model

Date: 2026-03-27
Status: active

## Intent

Define plan as the durable orchestration product.

## Definition

A plan is a validated, locked DAG of bound capability instances.

Plan has three lifecycle concerns:

- compilation
- execution
- record

Current design work is centered on compilation.

## Plan Rules

- a plan is not a goal
- a plan is not a capability catalog
- a plan is not an execution trace
- a plan is the locked result of compiler validation
- a plan is ready for execution when all graph and artifact contracts validate
- a plan can contain parallel-ready branches

## Core Records

- compiled plan
- capability instance
- dependency edge
- artifact handoff
- binding digest
- scope digest

## Graph Rules

- plan structure is a DAG
- dependency edges express execution preconditions
- artifact handoffs express producer-consumer data flow
- dependency edges and artifact handoffs are related but not interchangeable
- absence of an edge means no execution precondition was declared

## First Slice

The first slice plan model must support:

- current docs writer compatibility paths
- current `context generate`
- future capability families without changing the graph substrate
