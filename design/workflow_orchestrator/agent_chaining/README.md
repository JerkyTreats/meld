# Agent Chaining

Date: 2026-03-09
Status: active

## Intent

Define how workflows coordinate agent usage across multiple primitive capabilities.

## HTN Position

- workflow should bind agent roles at planning time rather than relying only on direct step level ids
- capabilities may provide safe defaults, but workflow remains the authoritative owner of cross task agent coordination
- role binding should stay explicit enough for authorization, debugging, and repair
- telemetry must preserve both declared role and resolved agent identity

## Provisional Answers

### Direct Names Versus Roles

- workflow definitions should prefer named roles such as generator, reviewer, verifier, and publisher
- direct agent ids should remain available as explicit overrides when a workflow truly depends on a specific agent
- role resolution should happen before runtime starts so task networks compile with stable bindings

### Capability Profiles

- capabilities may bind agents internally through profiles only for low value defaults
- workflow level bindings should always override capability defaults when both are present
- capability defaults should remain visible in compile output so hidden agent choice does not surprise operators

### Mixed Capability Workflows

- workflows that mix ordering, generation, review, and publish steps should resolve roles centrally in workflow planning
- primitive tasks that do not need an agent should say so explicitly rather than participating in agent binding by convention
- authorization checks should use resolved role and resolved agent identity together where that improves clarity

### Configuration And Telemetry

- configuration should expose declared role bindings and optional direct overrides
- telemetry should record declared role, resolved agent id, source of the binding, and any override reason
- repair and resume records should preserve the resolved bindings used when the original task network was compiled

## Initial Requirements

- support workflows that chain multiple capabilities with one or more agents
- keep agent selection explicit enough for debugging and policy checks
- avoid forcing every workflow config to repeat low value agent wiring
- allow capability profiles to provide safe defaults where that improves simplicity
- keep role based authorization clear across chained steps

## Residual Questions

- how many first phase roles should be standardized before the role set becomes too rigid
- whether role bindings should be versioned separately from capability profiles when migration changes defaults

## Related Areas

- [HTN Glossary](../htn_glossary.md)
- [Capability Contract](../capability_contract/README.md)
- [Telemetry Model](../telemetry_model/README.md)
- [Migration Plan](../migration_plan/README.md)
