# Theory Elevation Steps 1 Through 4 Fresh Review

Date: 2026-08-13
Status: reviewed and reconciled
Review base: `0aa4dfad`
Scope: delivered Theory Elevation Steps 1 through 4

> Historical review of the delivered compatibility aggregate. Its package capability and authority-request findings remain evidence, but [Canonical Persistent Domain Stewardship](../../cognitive_architecture/persistent_domain_stewardship.md) now owns the target split.

## Review Method

This pass treated the committed code, persistence shapes, call paths, and tests as primary evidence. Prior design conclusions were checked rather than assumed. The review followed each semantic product from authored source through installation, exact resolution, Agent judgment, execution admission, planning, task lineage, dispatch, and compatibility decoding.

The review focused on three questions:

1. Did the delivery invent machinery that should have reused or extended an existing product
2. Does the implementation preserve the architectural spirit of the design
3. Is the result appropriate for the current single-expression product maturity

## Verdict

The delivery is coherent and ready to initiate Step 5 after the reconciliations recorded below. No unresolved blocker remains in Steps 1 through 4.

The implementation consistently extends existing semantic products. New contracts exist only where the system previously had no owner product: complete installation receipts, missing owner registries, standing maintained conditions, and effective-authority policy values. The root remains an installer, resolver, and router. It does not become a second owner of belief, Strategy, maintained-condition, capability, claim-policy, or authority meaning.

## Findings And Reconciliation

| Severity | Finding | Evidence | Reconciliation |
| --- | --- | --- | --- |
| high | Exact authority resolution checked policy identity and principal but not the addressed physical subject | `src/runtime/theory.rs` resolved a receipt from `SelectedStewardshipPackage`, which carries no physical subject | exact activation now requires the expected subject and rejects a policy scope mismatch before actor hydration |
| medium | Dispatch duplicated part of the authority intersection and could drift from the shared decision contract | `dispatch_actor.rs` manually checked policy, grant, allowance, restriction, and authorized actions but omitted the package request | one execution-owned action revalidation helper now checks retained request, authorization, active policy, grant, allowance, restriction, principal, and scope |
| medium | A newly ready unauthorized task was claimed before dispatch denied invocation | claim submission preceded authority validation, leaving a denied task in running state | dispatch now validates authority before claim mutation and retains the second check immediately before invocation |
| medium | Agent abstention cleared the Goal command but retained its command identity in the durable decision | authority denial changed the decision kind and removed the command without clearing `goal_command_id` | both authority denial and Strategy failure now clear command and authorization lineage before persistence |
| low | Deserialized authority subjects could bypass constructor validation | policy and decision validation checked identifiers and action sets but not every subject coordinate | shared authority validation now rejects empty domain, kind, or object identity |

Focused tests cover each reconciliation. Legacy task-network hashes still round-trip because absent authority remains omitted from serialization.

## Duplicate Machinery Assessment

### Reuse that is correct

Strategy theory remains the package body and now states requested actions. Existing Strategy authorization remains the one Agent judgment record. Existing Composition operators supply required capability type identities. Existing guarded Goal admission remains the persistence boundary. Existing planning and dispatch actors remain enforcement points. Existing execution composition and task lineage carry the compact decision. Existing complete receipts remain the activation barrier.

This is the strongest evidence that the design spirit was followed. Authority did not become a parallel role engine, workflow, approval service, authorization ledger, capability wrapper, or event-authority extension.

### New structures that are justified

`AuthorityPolicy` and `AuthorityDecision` are new because no prior contract represented the difference between availability and permission. `AuthorityPolicyRegistryStore` is new because exact historical policy resolution is execution-owned behavior that no existing catalog provides. `AuthorityPolicyRevisionRef` remains owner-specific because the shared language crate must not depend on execution durability, just as exact capability and docs claim-policy references remain with their owners.

The complete receipt is not a second theory catalog. It is the cross-owner activation barrier and carries owner references intact. The resolved runtime image is not another authority store. It freezes one receipt-selected image for one assembled process.

### Similarity that should remain for now

The owner registries repeat append-only storage mechanics. A generic registry would currently erase owner validation and dependency direction while saving little code. Repeated registry mechanics are therefore an implementation pattern, not duplicated semantic authority.

The authority decision is copied into each task lineage. For the current five-action docs chain this is bounded, inspectable, and avoids copying the policy body. A separate decision store or reference resolution path would add more machinery than it removes.

Package requested actions overlap the package capability list by identity but not meaning. One states what construction can use. The other states what the package asks permission to exercise. Collapsing them would recreate the defect Step 4 exists to remove.

## Design Spirit Assessment

Step 1 achieved durability symmetry without one central theory owner. World-model theory shares `TheoryRevisionRef`, while execution capability and docs claim-policy references remain owner-specific where dependency direction requires them.

Step 2 removed expression dispatch from root assembly. Named declarations select identities, owner bodies activate through exact receipts, and docs-specific Rust remains a test antecedent rather than the extension model.

Step 3 made the maintained condition causal. Agent-owned condition bodies cause zero or more transient Goals and preserve exact condition lineage. Execution stores the result without interpreting standing responsibility.

Step 4 keeps capability availability distinct from permission. Agent judgment computes effective authority once, then execution independently revalidates at admission, planning, and dispatch without minting a second Agent decision.

Across all four steps, compatibility is explicit. Older records decode through defaults and compatibility lowering. Exact elevated activation fails closed instead of silently manufacturing missing theory or authority.

## Current Maturity Assessment

The chosen authority model is appropriately small for the current product. One principal, one exact immutable policy, exact subject matching, and capability type identities as provisional action classes are enough to prove the missing seam. Approvals, expiring grants, delegation, budgets, organization hierarchy, hot revocation, and a generic action taxonomy would be speculative today.

The solution also avoids pretending to support dynamic revocation. One policy binding is frozen per runtime assembly. Policy changes produce new exact revisions and take effect on a later composition. That is honest for the current process model.

Two limitations are intentional and should remain visible:

- package-route plans do not yet carry exact authority lineage and are rejected under an active policy
- capability type identity remains the action class until the CVE expression supplies contrary evidence

Step 5 must test both assumptions. It should extend existing package-plan lineage only if the CVE expression actually needs that route. It should introduce a broader action taxonomy only if capability type identity fails the dissimilar-expression proof.

## Readiness

Steps 1 through 4 form one coherent implementation base after reconciliation. Exact owner revisions, declaration-selected activation, standing responsibility, and effective authority now meet at the existing Strategy authorization and execution path without hidden root dispatch or duplicated semantic owners.

The next program work remains:

1. Step 5, implement the dissimilar CVE freshness expression as the generality proof
2. Step 6, reuse settled recorded judgment without candidate regeneration

No additional infrastructure work should precede Step 5 unless its concrete expression exposes a missing seam.
