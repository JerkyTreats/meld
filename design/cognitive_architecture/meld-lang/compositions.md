# Compositions

A Composition is a directed graph of typed steps and edges. Within a Task, every executable leaf is an exact bound Operator and every required artifact flow and dependency is closed.

Step kinds may express executable Operators and structurally nested composition regions. Epistemic Operations are not added as a step kind because they belong to a different consumer and authority domain.

Edges express ordering, data flow, and explicit executable guards. They preserve stable identity and producer justification.

Pure validation checks identity uniqueness, endpoint existence, binding closure, artifact compatibility, graph constraints, and deterministic hashing. Strategy separately proves that the Composition advances a Goal. Execution separately proves that the accepted Task can enter its operational network safely.

A Composition is language data. A Task is the producer-owned closed executable aggregate that contains it. A Task Network is Execution-owned operational state produced by compilation.
