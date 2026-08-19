# Capability And Task Substrate

## Capability

A Capability is Meld's universal atomic executable contract. It may wrap functionality implemented anywhere in the runtime or bind an external implementation through activation.

The contract declares operational truth required for composition and execution:

- exact identity and version
- typed inputs and outputs
- required bindings
- supported scope
- operational effects
- execution behavior

A Capability carries no PDS maintained-condition, belief, settlement, or action-class meaning. PDS may co-ship a normal Capability contributor, but availability always enters through ordinary runtime registration and activation.

## Active Catalog

Runtime contribution and activation produce the complete exact Capability catalog available to an assignment generation. Strategy receives one immutable snapshot of that catalog. PDS does not select, filter, wrap, or duplicate the catalog.

Authority remains separate from availability. A Capability may be present while use of it is denied for the current Agent, Goal, or subject.

## Composition And Lowering

The shared action substrate is:

```text
Capability
→ bound occurrence in a candidate Composition
→ compiled into a Task region
→ committed into the Task Network
→ dispatched as Capability work
```

A Task is a composite graph of bound Capability instances. A Task Network is a graph of Tasks. Strategy and Execution use one graph language across this boundary while retaining separate authority.

Strategy selects exact Capability identities and constructs the candidate Composition. Agent authorization preserves that exact candidate. Execution validates live feasibility, owns Task identity and compilation, and lowers the authorized graph without semantic replanning.

## Operator

An Operator may represent one episode-local grounded Capability use inside a Composition. It may contain situated bindings, preconditions, projected effects, and cost estimates for that occurrence.

An Operator must reference the exact Capability contract. It must not repeat or override authoritative Capability inputs, outputs, scope, operational effects, or execution behavior.

## Semantic Unit Invariant

The canonical Capability contract crosses publication, Strategy, authorization, Task compilation, and dispatch as one semantic unit. Receiver-specific projections may narrow visibility but may not become parallel contracts.
