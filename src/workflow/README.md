# Legacy Workflow inspection

Workflow turn execution is retired. Native Agent and Strategy own reconciliation planning and authorization. Execution realizes their complete Tasks, and native product owners return observations for Curation, Belief and Agent judgment.

`meld workflow list`, `validate` and `inspect` retain access to existing profile documents. `meld workflow execute` reports retirement. Context generation with an explicit Workflow override or a Workflow-bound Agent also reports retirement. Control and the generation queue reject retained Workflow program addresses before submission. Watch reports a retired binding without substituting metadata or provider-generated frames for its requested Workflow.

Native CLI boot and watch no longer load or validate Workflow profiles. The profile registry is loaded only for explicit profile inspection or a historical Context read that needs the profile's final frame type. A malformed retired profile or an Agent binding to an absent profile cannot prevent native Startup.

The former root facade, capability-registry builder, direct turn executor, retry and resume loop, and Workflow thread/turn store are removed. The state writer cannot be constructed through `meld` or `meld-execution`. Existing profile files, thread files, frames and ledger Events are not rewritten or deleted. In particular, a rejected force-generation request leaves historical frame heads intact.

Use an installed native product for executable reconciliation. [Docs maintenance](../../theory/docs_freshness/README.md), [Startup](../../theory/startup/README.md), [declared Security mitigation](../../theory/dependency_security_mitigation/README.md) and [declared code changes](../../theory/code_change/README.md) describe configuration, preparation and activation. A supported `runtime request` asks the installed Agent to reconcile its own intent; it does not translate an arbitrary Workflow profile or authorize its turns.

Historical Workflow Event and metadata contracts remain readable. Profile schemas and normalization helpers also remain referenced by the older Task-package compiler and its characterization fixtures. That compiler and the old frame-head publication capability are a remaining source-retirement obligation, not another supported Workflow command route. Their removal must preserve the generic complete-Task machinery used by native Execution. See the [outcome audit](../../design/plan/architecture_overhauls/world_model_knowledge_traversal/reconciliation_outcome_audit.md) for current evidence and remaining work.
