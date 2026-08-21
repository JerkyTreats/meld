# Agent Genesis And Activation

Agent genesis realizes one PDS assignment as one Agent identity inside one activation generation.

Genesis receives exact package receipts, principal declarations, directives, perspective, owner routes, semantic resources, participant identity, and activation fence. It publishes one durable genesis receipt.

An Agent becomes operational only after every required participant and owner route for the same generation has published readiness. A belief subscription alone is not sufficient readiness.

Generation changes never mutate an existing Agent incarnation. A successor incarnation is created with explicit predecessor lineage. Retirement fences new publications, drains owned work, records safe points, and closes the incarnation durably.
