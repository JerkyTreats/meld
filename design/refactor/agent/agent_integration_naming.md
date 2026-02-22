# Agent–context boundary module naming

## Current name: `agent/integration`

**What the module does:** Provides the agent domain with the ability to read context, write frames, and generate frames via the context engine. It holds the contract (trait) and the ContextApi-backed implementation; no other agent code talks to context directly for these operations.

---

## Pros of "integration"

- Single, short word; no nesting of "context" inside agent path.
- Captures the idea that this code "integrates" the agent with context.
- One place for all agent–context interaction (read, write, generate), which avoids scattering the boundary.

## Cons of "integration"

- **Vague:** "Integration" does not say what is integrated or what behavior is performed. New readers see `agent/integration` and do not immediately infer "agent’s use of the context engine."
- **Relationship, not behavior:** The name describes the relationship (agent–context integration) rather than what the module does (read/write/generate against context). PLAN.md and AGENTS.md prescribe naming by *behavior* (e.g. `query`, `mutation`, `queue`, `storage`), not by pattern or relationship.
- **Pattern-y:** In many codebases "integration" is used as a technical layer name (integration layer, integration tests), which conflicts with "behavior over layer" and "behavior over pattern."

---

## Behavior-driven alternatives

The actual behaviors are: **read context**, **write frame**, **generate frame**. So the module is "how the agent domain uses the context engine."

| Name | Pros | Cons |
|------|------|------|
| **context_access** | Describes behavior: access to context (read, write, generate). Clear for newcomers. Aligns with "name by behavior." | Repeats "context" in path (`agent/context_access`). |
| **context_ops** | Short; "ops" = operations against context. Behavioral. | Slightly informal; "ops" can be overused. |
| **context_use** | "How we use context." Behavioral. | "Use" is generic. |
| **read_write_generate** | Explicit behaviors. | Long; reads like a list, not a single concern; generate is different in kind (async, queue). |
| **context_boundary** | Accurate as the boundary to context. | Describes architecture, not behavior. |

---

## Recommendation

Rename **`agent/integration`** to **`agent/context_access`**. **Done:** the codebase uses `agent/context_access` as of this decision.

- **Behavior:** The module provides *access to context* (read, write, generate). That is the behavior the rest of the agent domain sees.
- **Consistency:** Matches PLAN.md ("behavior over layer", "behavior over pattern") and AGENTS.md examples (`query`, `mutation`, `queue`): name by what the module does, not by "integration" or "adapter."
- **Clarity:** `agent/context_access` makes it obvious that this is the agent’s path to the context engine. The submodules `contract` and `context_api` remain; they can stay as-is or be renamed later (e.g. contract → trait or capability) if desired.

No change to public API surface: re-exports stay on `crate::agent` (e.g. `AgentAdapter`, `ContextApiAdapter`). Only the internal module path and doc comments change.
