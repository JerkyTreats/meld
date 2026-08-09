# Bottom-Up Review: From Runtime Seams Toward PDS Declaration

Date: 2026-08-08
Status: independent architecture review
Scope: interrogate the PDS layering question starting from the implemented runtime seams and moving upward, without assuming the current hand-authored theory/package forms are appropriate user syntax

## Question

Given the current Meld runtime, what must a declaration/compiler supply so that new PDS expressions can execute through existing domain contracts without adding application-specific runtime branches?

## Starting Constraint

The settled PDS theory-to-runtime layer defines the runtime consumer seams. The next layer up must canonicalize against those seams rather than inventing a declaration that requires runtime capabilities that do not exist.

The current theory kinds are:

```text
belief families
evidence mapping sets
curation rules
planning methods
available actions
method realizations
task packages
maintained scope and conditions
authority and governance
```

The current docs-freshness proof hand-authors or injects these forms. That makes it a compatibility antecedent of a compiled stewardship image.

## Finding 1 — The Runtime Does Not Need A User-Facing PDS Object

There is no single PDS runtime object today, and none is required by the current authority model.

Runtime ownership is already distributed:

```text
events -> event authority
graph -> world model graph
belief theory/state -> world model belief
curation/agent identity -> Agent domain
goals/meld-lang -> language + execution
methods/actions/realization -> planning/execution
task state -> execution
action effects -> capabilities
outcomes -> publishing domain + evidence interpretation
```

A PDS compiler should link declarations into these domain-owned contracts, not create a parallel PDS engine.

## Finding 2 — The Current Hand-Authored Theory Bundle Is Structurally A Compiler Target

The current runtime consumes identities and theory bodies in forms such as:

```text
BeliefFamilyConfig
OutcomeMappingSetConfig
AgentCurationRuleBinding
Method
AvailableActionSet
MethodRealizationBinding
TaskPackageSpec
```

These are mechanically close to what a compiler/linker must emit or select. They are mechanically far from a normal user declaration.

**Conclusion:** the existing docs/CVE-style theory definitions are correctly interpreted as compiler-target material.

## Finding 3 — The Compiler Target Is Federated IR, Not Necessarily One Monolithic AST

The runtime seams are owned by different domains and have different durability models.

Therefore the compiled stewardship image can be understood as a linked manifest over domain-owned semantic fragments:

```text
compiled image
├── belief registry installations
├── evidence mapping installation
├── agent/curation binding
├── meld-lang/planning theory
├── available actions and realizations
├── task package selection
├── maintained-condition lowering
├── governance requirements
└── activation requirements
```

A single giant PDS-owned Rust enum would duplicate domain authority and create pressure for every new application vocabulary to modify the kernel.

## Finding 4 — The Declaration Must Resolve Every Runtime-Relevant Semantic Choice Without Encoding Runtime Topology

A declaration need not name actor ids, store paths, task-network internals, or event-consumer roles.

But compilation must still deterministically resolve:

- which belief semantics apply;
- which observation/evidence routes are admissible;
- which maintained conditions exist;
- which curation posture applies;
- which actions are afforded;
- which methods realize those actions;
- which outcome verification semantics determine restoration;
- which authority requirements constrain dispatch.

This implies a declaration can remain high-level only if its selected package/profile revisions close those semantic choices deterministically.

## Finding 5 — A Compiler May Resolve Symbols; It May Not Invent Missing Domain Meaning

The declaration can say:

```text
verification: release_grade
```

and the selected package may resolve that symbol into detailed outcome requirements.

The compiler may perform that resolution.

It may not invent a new verification interpretation that is absent from both the approved declaration and selected package/profile revision.

This is the same truthfulness rule already present at runtime: missing theory yields an unresolved binding rather than manufactured meaning.

## Finding 6 — Exact Revision Lineage Is Required At The Compiler Boundary

Replay requires knowing not merely that a steward used `docs_freshness`, but exactly which semantics were installed.

The current belief-family registry demonstrates the target pattern:

```text
identity
+ content hash
+ append-only revisions
+ current head
+ historical resolution
```

The compiler/linker should produce a content-addressed image whose domain fragments can be resolved historically.

Current gaps remain for evidence mappings, planning theory, action sets/realizations, and task-package selection. These are runtime implementation gaps, not reasons to push their details into the user declaration.

## Finding 7 — New Expressions Must Not Extend Through Root `match expression` Branches

The current docs proof still contains compatibility logic mapping `docs_freshness` to `docs_writer` and first-proof conventions.

That is acceptable for one vertical proof but cannot be the PDS extension model.

The required generic path is:

```text
new expression
    -> package/profile/declaration data
    -> compiler/linker output
    -> existing runtime domain seams
```

not:

```text
new expression
    -> add application-specific branch in root runtime code
```

This is a direct bottom-up requirement for the declaration/compiler architecture.

## Finding 8 — Strongly Typed Runtime Substrate Should Not Leak Into The Declaration

Several implemented types are important for execution but are not PDS semantics:

- Merkle structures;
- Context Frames;
- event consumer cursors;
- leases;
- task-network records;
- capability scheduling classes;
- supervisor registrations.

The compiler may depend on those systems indirectly through domain contracts, but the PDS declaration should not expose them merely because they are strongly typed.

This is the main protection against designing the user surface by mirroring implementation structure.

## Finding 9 — Some Current Example Concepts Expose Real Missing Runtime Seams

The compiler cannot lower a concept into a nonexistent runtime contract and pretend the feature is implemented.

Examples include:

- standing maintained-condition authority;
- principal grants and generic governance;
- durable evidence-mapping revisions;
- durable planning-theory revisions;
- generic outcome-contract installation;
- assignment and activation records;
- domain-level claim valid-time/merge/split semantics where needed.

For these, the correct sequence is:

```text
identify required semantic seam
-> establish domain ownership
-> implement/freeze runtime contract
-> then allow upper PDS layers to depend on it
```

This preserves the PDS layering discipline.

## Finding 10 — The Compiler Must Be Deterministic And Auditable Across The Approval Boundary

Given:

```text
canonical declaration revision
+ selected package/profile/facet revisions
+ assignment
+ activation inputs
```

compilation should yield one inspectable semantic image identity.

The runtime must be able to answer:

```text
which declaration authorized this steward?
which package/profile revisions supplied its semantics?
which compiled image revision was installed?
which exact theory revision produced this belief/decision/plan?
```

The declaration identity and compiled image identity therefore serve different audit questions and should both exist.

## Finding 11 — The Current Examples Are A Hybrid Of Package Source And Lowered IR

From the runtime side, not everything in the examples should be compiler-generated.

Package/domain experts may legitimately author:

```text
belief semantics
evidence admissibility
named sensitivity presets
action meanings
verification profiles
governance requirements
```

The compiler then resolves and lowers that expert source into domain-owned runtime contracts.

Therefore a useful architecture has at least:

```text
principal-facing declaration
+
expert-authored package/facet source
-> linker/compiler
-> compiled stewardship image
-> domain runtime contracts
```

This is more precise than calling every detailed example field "IR."

## Bottom-Up Acceptance Test

A proposed PDS declaration/compiler design passes this review when:

1. Every required runtime theory seam can be produced or selected deterministically.
2. New expressions require no application-specific runtime branch.
3. Domain-owned contracts remain authoritative for their own semantics and state.
4. Missing runtime seams fail truthfully rather than being hidden in prompts or compiler conventions.
5. Compiled semantics have exact revision lineage.
6. Runtime substrate details that are not stewardship meaning remain below the declaration boundary.
7. The compiled image can be traced back to the declaration and package/profile revisions that produced it.

## Verdict

**Validated.** The current detailed docs-freshness/CVE-style material is predominantly below the eventual principal-facing PDS declaration.

The runtime-side refinement is that the compiler target should be viewed as a **linked set of domain-owned semantic IR fragments**, not necessarily one PDS-owned monolithic intermediate language.

The current hand-authored packages are useful because they expose exactly what the compiler must eventually generate, select, install, and version.
