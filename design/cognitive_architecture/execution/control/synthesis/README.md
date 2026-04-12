# Capability Synthesis

Date: 2026-04-09
Status: proposed
Scope: runtime synthesis of capabilities from external processes, enabling the task network to extend its own capability set without compile-time anticipation

## The Problem

Every compiled capability is an implicit assertion: this is a problem worth solving, and this
is the correct way to solve it. The accumulation of those assertions defines a ceiling.
The system can do exactly what was anticipated and nothing else.

For a documentation agent this is tolerable. For an HTN planner intended to operate across
arbitrary goal domains, it is a fundamental constraint. The planner is only as capable as its
capability catalog at compile time.

Runtime synthesis removes that ceiling. The system can discover and create new capabilities
in response to planning needs, without requiring a developer to anticipate the requirement
and write Rust code.

## The Mechanism

The classical objection to wrapping arbitrary CLI output as a capability is that the output
is unstructured and the contract cannot be guaranteed. This objection does not hold in an
environment with LLM access.

The pattern is:

```
subprocess output (unstructured)
    → provider_execute_chat (LLM conversion prompt)
    → typed artifact (ChangeSummary, AstImpact, or any target schema)
```

The capability contract remains fully typed. Input slots and output slots are defined at
synthesis time and stored with the capability definition. The implementation path to a typed
artifact is: run subprocess, convert output via LLM. The contract is stable. The
implementation is flexible.

This is a generalization of a pattern already present in the system. `ContextGenerateFinalize`
takes raw provider text and shapes it into typed artifacts. The synthesis mechanism extends
that same pattern to arbitrary subprocess output.

## Compiled vs Synthesized Capabilities

Compiled capabilities and synthesized capabilities have the same contract shape. Both declare
typed input slots, output slots, and execution contracts. The task compiler treats them
identically for dependency validation and slot wiring.

They differ in two ways:

| Property | Compiled | Synthesized |
|---|---|---|
| Source | Rust code, build-time | Runtime synthesis task |
| Trust | Full, statically verified | Validated at synthesis time |
| Execution class | Inline / queued / session | ExternalProcess |
| Failure response | Retry or repair | Retry, or re-synthesis if schema drift detected |

Compiled capabilities are authoritative for the artifact schemas they define. When a
synthesized capability produces a `ChangeSummary`, that capability targets the schema defined
by the compiled `git_diff_summary` reference implementation. Compiled capabilities are
interfaces; synthesized capabilities are additional implementations.

## Planning Integration: Online Capability Acquisition

With synthesis available, the HTN planner gains a new response to planning failure:

```
goal: produce ChangeSummary for node X
  → query capability catalog: any capability producing ChangeSummary?
  → [found: compiled git_diff_summary]    → use it, proceed
  → [not found]                           → instantiate CapabilitySynthesisTask
                                          → seed: GoalContext { artifact_type: ChangeSummary }
                                          → await synthesis completion
                                          → requery catalog
                                          → [found: synthesized git_diff_summary_v1]
                                          → use it, proceed
```

This is online planning with capability acquisition. Planning failure on "no capability
available" triggers synthesis, then replanning. The HTN planner structure does not change.
The catalog lookup gains a second resolution path.

The capability catalog query is by **artifact type produced**, not by capability identity.
"I need something that produces a `ChangeSummary`" is the query. The response is any
registered capability whose output contract includes a `ChangeSummary` slot — compiled or
synthesized, whichever is available.

## The LLM Agent Harness Option

The synthesis task can use a full LLM agent harness for the scripting and discovery steps.
Frontier model agents with tool access (shell, file read/write, web search) make the
"find a program that measures X and wrap its output" request almost trivial compared to
implementing it in Rust.

The harness invocation is just another provider execution. What changes is the execution
mode: instead of a single chat turn, the synthesis step may involve multiple tool calls,
file writes, and test runs orchestrated by the agent. The task structure handles this: the
agent harness invocation is one capability in the synthesis chain, and its output is the
`SynthesisPlan` artifact that subsequent capabilities validate and register.

## Documents

- [External Process Capability](external_process_capability.md)
  the ExternalProcess execution class and LLM output adapter
- [Synthesis Task](synthesis_task.md)
  the CapabilitySynthesisTask definition and chain
- [Runtime Catalog](runtime_catalog.md)
  the sled-backed catalog for synthesized capabilities and trust model

## Read With

- [Control Graph Model](../program/control_graph.md)
- [Execution Planning](../planning/README.md)
- [Bayesian Evaluation Example](../../examples/bayesian_evaluation.md)
- [Event Spine](../events/multi_domain_spine.md)
