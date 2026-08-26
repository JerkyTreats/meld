# Runtime Invariants

Status: active

## Scope

These invariants govern source and runtime meaning across Meld. Domain contracts may be stricter but may not weaken them. Skills, plans, reviews, and delivery procedures explain how work is performed and cannot waive these rules.

The [Contribution Policy](contribution_policy.md) governs how changes are authored and delivered. Neither policy overrides the other.

## Canonical Runtime Authority

One semantic responsibility has one canonical runtime authority in completed source.

When a change replaces or moves existing runtime behavior, it must identify the current entrypoint, the intended successor, the affected callers, and the behavior that becomes superseded. The completed change must route real callers through the successor and remove the superseded writer, planner, actor, selector, or decision authority.

A dormant successor is not a canonical cutover. Retirement cannot be deferred merely because it is assigned to later work.

New behavior and in-place extensions do not require migration ceremony. They must still avoid duplicating an existing runtime authority.

## Compatibility

Domain clarity and explicit ownership take priority over backward compatibility. Breaking user-facing changes must be disclosed before commit and described in the commit message.

A compatibility reader, stored-format decoder, or thin forwarding adapter may remain when current evidence requires it. It must route into the canonical authority, reject new callers onto the old contract, and retain characterization or parity evidence. A temporary seam carries a local removal condition. A long-lived reader is named and owned as a stable contract.

Compatibility does not authorize a second writer, planner, actor, selector, runtime coordinator, or decision authority. Temporary parallel authority requires explicit user approval with a named noncanonical route and removal condition.

## Semantic Products

A domain-owned product travels intact through wrappers, queues, Event envelopes, stores, and handoffs until a receiving domain explicitly accepts ownership and translates it.

Wrappers may add operational metadata such as routing, sequence, retry, status, cursor, or storage keys. They must not repeat product fields as a second source of truth.

Primitive domains remain free of consumer-specific meaning. Consumers may validate and translate only at an explicit ownership boundary.

## Storage Purity

Runtime state must never be persisted under the target workspace path. Runtime state uses an XDG data root or another external root, including all fallback paths.

New storage behavior must prove that its write paths remain outside the target workspace. Product work intentionally performed on workspace files is not runtime-state storage.

## Evidence

A replacement is complete only when the successor is exercised through the real runtime entrypoint and source evidence shows that in-scope callers no longer reach the superseded authority.

Persistent changes also require restart, replay, and compatibility evidence proportionate to the affected stored state.

Passing local tests or preserving old code for speculative rollback does not substitute for canonical runtime ownership.
