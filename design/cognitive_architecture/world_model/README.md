# World Model

The world model is Meld's situated epistemic system. It turns durable observations and owner-issued semantic products into an Agent-scoped understanding of the world, then uses that understanding to construct and reconcile Plans against directives.

The world model is not an execution engine. It does not dispatch Capabilities, schedule Tasks, or own the Task Network.

## Domain Structure

Graph preserves addressable objects, relation occurrences, provenance, currentness, and owner-issued assertions.

Traversal reads graph knowledge through one federated capability. Domain entities opt into traversal by publishing owner-shaped graph material that satisfies the substrate contract. Traversal does not require one universal store or ontology and does not imply that every record is a node.

Belief selectively admits evidence and settles Agent-scoped propositions. Graph reachability is not evidence by itself.

Causation represents mechanisms and expected effects. Regime represents changing conditions that alter which mechanisms or evidence remain relevant.

Planner materializes immutable, relation-rich cuts for reasoning. A cut binds exact object and relation occurrences, provenance, temporal scope, branch scope, authority, and owner revisions.

Curation performs bounded epistemic operations under Agent authority. It authors expected entities, assertions, and relations without performing external work or settling belief.

Strategy constructs immutable heterogeneous Plans. It selects and chains complete Tasks and bounded Epistemic Operations against frozen world-model context.

Agent owns the directive, Goal authority, Plan authorization, durable Plan progression, and reconciliation when admitted knowledge or outcomes change.

## Reconciliation Loop

An Agent detects a meaningful mismatch between directive and admitted world state. It asks Strategy for a Plan that explains what conditions must become true and why. Strategy may include epistemic operations needed to establish or connect knowledge, executable Tasks needed to change the world, or both.

Agent authorizes the exact Plan. It routes each eligible Epistemic Operation to Curation and each eligible complete Task to Execution with Goal attribution. Results return through Events, become eligible for graph and belief admission, and may cause the Agent to reconcile the remaining Plan.

Reconciliation preserves completed causal history. It does not require restarting the entire Plan and it does not make Execution aware of Plan semantics.

## README Freshness Example

A product directive states that every folder has a correct README. Curation can author the expected README entity for a folder and connect source claims to claims required in that README. Observation can then establish that the expected file is absent. Strategy receives a precise mismatch and can construct a complete Task to create or update the file. Later observation and epistemic admission close the Goal when the file exists and its required claims are correct.

The expected entity, the observed entity, the correctness claim, and the executable Task are different semantic units. Traversal connects them. Curation authors epistemic structure. Strategy constructs the causal Plan. Execution realizes only the Task.

## Domain Documents

- [Public Interface](public_interface.md)
- [Graph And Traversal](graph/README.md)
- [Belief](belief/README.md)
- [Causation](causation/README.md)
- [Regime](regime/README.md)
- [Planner](planner/README.md)
- [Curation](curation/README.md)
- [Strategy](strategy/README.md)
- [Agent](agent/README.md)
