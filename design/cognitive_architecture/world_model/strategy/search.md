# Strategy Plan Construction

Plan construction is a pure search over immutable world-model context. Its search state is not a runtime control graph and has no authority after construction returns.

## Search Position

A search position records the remaining desired conditions, selected Tasks, selected Epistemic Operations, dependency edges, assumptions, unresolved information needs, estimated cost, and explanation lineage.

## Expansion

Strategy expands a desired condition by evaluating owner-admitted mechanisms, relevant graph relations, current beliefs, regime constraints, available Capability contracts, and possible epistemic authorship.

Expansion may prove the condition already satisfied, bind a complete Task, bind a bounded Epistemic Operation, introduce prerequisite conditions, or retain an unresolved information branch with an explicit observation path.

## Ranking

Ranking may consider causal confidence, information value, Capability cost, reversibility, latency, resource pressure, risk, and the amount of already authorized work preserved by a successor Plan.

Ranking never substitutes for closure. A high-scoring product that lacks exact bindings or authority requirements is not eligible.

## Terminality

A search position is terminal when every desired condition is satisfied or connected to a closed product and every dependency needed for authorization is explicit. Terminality means the Plan is ready for Agent judgment. It does not mean the Goal is satisfied.

## Determinism

Equal requests over equal frozen cuts produce stable semantic products and stable ordering. Exploration metadata may vary, but it cannot alter authorized meaning without changing Plan identity.
