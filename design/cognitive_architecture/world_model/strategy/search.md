# Strategy Plan Construction

Plan construction is a pure search over immutable world-model context. Its search state is not a runtime control graph and has no authority after construction returns.

## Search Position

A search position records the remaining desired conditions, selected Tasks, selected Epistemic Operations, dependency edges, assumptions, unresolved information needs, estimated cost, and explanation lineage.

## Expansion

Strategy expands a desired condition by evaluating owner-admitted mechanisms, relevant graph relations, current beliefs, regime constraints, available Capability contracts, and possible epistemic authorship.

Expansion may prove the condition already satisfied, bind a complete Task, bind a bounded Epistemic Operation, introduce prerequisite conditions, or retain an unresolved information branch with an explicit observation path.

## Hierarchical refinement

Domain methods declare alternative ways to establish a desired condition. Each method has a trigger, guarded applicability and a composition of abstract conditions or primitive operations. Strategy selects a method against its frozen cut and recursively reduces abstract occurrences. A method is knowledge about available means; it does not authorize work or prescribe the one runtime procedure.

Search applies primitive effects only to private hypothetical state. A later precondition may depend on those effects when the executable graph retains the causal ordering. Unrelated work cannot silently supply a prerequisite. Already-established abstract conditions require no operation. Predicted effects never enter Events or replace independently observed Evidence.

A selected Plan retains the method hierarchy, occurrence paths, local variable bindings and expansion order. The executable Tasks contain primitive operations and explicit artifact and ordering dependencies. Independent verification replays the selected derivation against the same cut, re-resolves primitives against canonical capability contracts, and checks product closure. Abstract conclusions alone do not establish the Agent's desired condition.

A successor is constructed from a new relevant cut and retained completed history. It preserves prior meaning and attributes new work to a new immutable Plan revision. Search depth and expansion limits bound recursive methods; reaching a limit does not prove that no remedy exists.

## Ranking

Ranking may consider causal confidence, information value, Capability cost, reversibility, latency, resource pressure, risk, and the amount of already authorized work preserved by a successor Plan.

Ranking never substitutes for closure. A high-scoring product that lacks exact bindings or authority requirements is not eligible.

## Terminality

A search position is terminal when every desired condition is satisfied or connected to a closed product and every dependency needed for authorization is explicit. Terminality means the Plan is ready for Agent judgment. It does not mean the Goal is satisfied.

## Determinism

Equal requests over equal frozen cuts produce stable semantic products and stable ordering. Exploration metadata may vary, but it cannot alter authorized meaning without changing Plan identity.
