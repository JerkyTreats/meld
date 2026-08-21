# Capabilities And Tasks

A Capability is Meld's atomic executable contract. It declares exact identity and revision, typed inputs and outputs, required bindings, supported scope, operational effects, resource requirements, runner identity, and execution behavior.

Capability availability is published through runtime activation. Authority to use a Capability remains separate from availability.

A Task is a complete executable graph of bound Capability occurrences. It contains exact artifact flow, dependencies, protected effects, expected outputs, and producer lineage. Strategy constructs Task meaning. Agent authorizes the exact Task. Execution accepts and compiles it without reconstructing the Strategy proof.

Execution may reject admission when the Task is malformed, unauthorized, unbound, incompatible with live Capability contracts, or unsafe under Execution-owned invariants. It may not substitute a semantically different Capability or invent a missing step.

One Capability contract remains one semantic unit across catalog publication, Task construction, authorization, compilation, dispatch, and outcome attribution. Consumer views may narrow fields but cannot create parallel meanings.
