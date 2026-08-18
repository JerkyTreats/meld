# Docs Freshness PDS Router Refactor Detailed Design Specification

Date: 2026-08-18
Status: proposed migration design
Scope: migrate the existing docs freshness expression from fixed root theory and direct activation into the PDS Router and domain adapter pattern

## Objective

Move docs freshness behind the [PDS Router](pds_router_design_spec.md) and the [canonical runtime lifecycle and quiescence design](../../cognitive_architecture/runtime_lifecycle_and_quiescence.md) without changing its domain meaning or successful flywheel behavior. The refactor must make docs freshness an ordinary consumer of domain-owned theory routes and capability contribution, establish parity with the current exact-revision path, and remove docs-shaped fields and calls from root configuration, installation, receipt, resolution, and runtime assembly.

This is an ownership refactor. It is not a redesign of docs claim semantics, the five-capability chain, Agent behavior, Strategy search, task execution, or evidence revision.

The [canonical PDS architecture](../../cognitive_architecture/persistent_domain_stewardship.md) now settles the semantic redesign target. Every package-carried exact capability, authority policy, Method, search bound, and final scalar in this document is compatibility material preserved for parity. This refactor must not promote that shape as the canonical PDS contract.

## Use-Case Trace

The normalized [documentation freshness use case](../../persistent_domain_stewardship/examples/documentation_freshness/README.md) and its [router specification](../../persistent_domain_stewardship/examples/documentation_freshness/router_spec.md) are the use-case evidence for this workstream.

They place the following router behavior in the next iteration:

- one package routes exact docs, world-model, and execution theory to their owners
- deterministic inspection may activate without a provider
- model-backed drafting and validation add physical bindings without changing package identity
- publication remains a bounded repository effect under independent authority
- only an admitted freshness assessment may support restoration
- late provider results retain exact generation and attempt lineage
- legacy and routed activation must preserve flywheel and lifecycle parity

Wider examples do not expand this refactor. Perspective projection, narrative disclosure, generic graph hydration, learner privacy, portfolio approval, and physical interlock contracts remain outside docs parity. The docs package must use open route and activation boundaries so those future packages do not require docs-shaped or application-shaped root changes.

## Current Ground

Current docs freshness already has strong domain ownership:

- `docs` owns five capability contracts and invokers
- `docs` owns claim validation and exact claim-policy revisions
- `meld-world-model` owns belief family, outcome mapping, curation, maintained condition, and Strategy registries
- `meld-execution` owns exact capability contract and authority policy registries
- root initialization installs exact owner revisions
- root runtime freezes one complete exact receipt

The remaining coupling is at attachment edges:

- `TheorySelection` contains fixed component ids including `claim_policy_id`
- `SelectedStewardshipPackage` repeats the fixed shape
- `WorldInitTheoryBundle` contains every fixed body and a docs claim policy
- `TheoryInstallationReceipt` contains fixed fields and a docs-specific claim-policy ref
- `ResolvedStewardshipTheory` resolves docs claim policy directly
- `published_product_contracts` returns docs contracts only
- runtime assembly calls `crate::docs::capability::register_exact_contracts` directly
- provider binding is mandatory because docs activation is the only path
- the direct docs PDS composer remains as a test-only regression fixture

## Target Shape

```text
docs freshness package entry
→ theory router
→ world-model route handlers
→ execution route handlers
→ docs claim-policy route handler
→ generic package receipt
→ docs assignment and activation
→ docs capability contributor
→ existing five-capability flywheel
```

Root sees one package receipt and one assignment. During compatibility migration it does not know that the receipt refers to a docs claim policy or five docs capabilities. The destination design moves those exact capabilities into activation-owned input.

## Package Layout

The existing authored files may remain in `theory/docs_freshness` during migration. One manifest becomes the package entry.

```text
theory/docs_freshness/
├── pds-package.json
├── belief_family.docs_freshness.json
├── outcome_interpretation.docs_freshness.json
├── curation_rule.docs_freshness.json
├── maintained_condition.docs_freshness.json
├── strategy_theory.docs_freshness.json
├── authority_policy.docs_workspace_local.json
└── claim_policy.docs-claims-strict-v1.json
```

Illustrative manifest:

```json
{
  "schema_version": 1,
  "package_id": "meld.docs-freshness",
  "package_version": "1.0.0",
  "imports": [],
  "components": [
    {
      "component_id": "docs-belief-family",
      "route": "world-model.belief-family.v1",
      "component_schema_version": 1,
      "content": {
        "RelativeFile": {
          "path": "belief_family.docs_freshness.json",
          "content_hash": "exact-source-hash"
        }
      },
      "requires": []
    },
    {
      "component_id": "docs-claim-policy",
      "route": "docs.claim-policy.v1",
      "component_schema_version": 1,
      "content": {
        "RelativeFile": {
          "path": "claim_policy.docs-claims-strict-v1.json",
          "content_hash": "exact-source-hash"
        }
      },
      "requires": []
    }
  ]
}
```

The real manifest includes all seven current theory bodies. The abbreviated example does not weaken package closure.

## Compatibility Component Route Map

| Current body | Target route | Installing owner |
| --- | --- | --- |
| `belief_family.docs_freshness.json` | `world-model.belief-family.v1` | belief domain |
| `outcome_interpretation.docs_freshness.json` | `world-model.outcome-mapping.v1` | belief domain |
| `curation_rule.docs_freshness.json` | `world-model.agent-curation-rule.v1` | Agent domain |
| `maintained_condition.docs_freshness.json` | `world-model.agent-maintained-condition.v1` | Agent domain |
| `strategy_theory.docs_freshness.json` | `world-model.strategy-theory.v1` | Strategy domain |
| each exact capability contract carried for parity | `execution.capability-contract.v1` | capability domain |
| `authority_policy.docs_workspace_local.json` carried for parity | `execution.authority-policy.v1` | authority domain |
| `claim_policy.docs-claims-strict-v1.json` | `docs.claim-policy.v1` | docs domain |

The compatibility manifest explicitly lists five `execution.capability-contract.v1` components using exact published refs. The compatibility Strategy body structurally requires those component ids and continues to cite their exact content identities. The execution route handler resolves each contract from product capability inventory, installs it through the execution registry, and returns its exact revision ref. The destination design removes these refs from PDS theory and builds the exact catalog from activation contributions.

## Docs Claim-Policy Route

Docs publishes one theory route handler for `docs.claim-policy.v1`.

```rust
struct DocsClaimPolicyTheoryHandler {
    registry: DocsClaimPolicyRegistryCommand,
    resolver: DocsClaimPolicyRegistryQuery,
}
```

The handler:

1. Decodes only its routed `DocsClaimPolicy` body.
2. Validates current policy invariants through `DocsClaimPolicy::validate`.
3. Confirms the component id and body policy id relationship under docs rules.
4. Installs through `DocsClaimPolicyRegistryStore` or its extracted public command contract.
5. Returns a generic route-addressed `TheoryRevisionRef`.
6. Verifies historical exact resolution through the docs registry.

The router never imports `DocsClaimPolicy`, `DocsClaimPolicyRevisionRef`, or docs validation code.

The docs registry remains the canonical store. Migration may add conversion between its current ref and generic `TheoryRevisionRef`, but must not duplicate policy bodies in a theory-owned store.

## Docs Capability Contributor

Docs publishes one product capability contributor covering the existing five exact contracts:

```text
docs.inspect_scope
docs.draft_patch_set
docs.validate_patch_set
docs.publish_patch_set
docs.assess_published_scope
```

The contracts, artifact schemas, effect declarations, and current content identities remain unchanged unless a separately reviewed semantic change requires a version increment.

The contributor owns:

- publication of the five contract definitions
- exact selector matching
- docs physical binding validation
- exact claim-policy resolution from the package refs
- construction of docs invokers
- registration into the assignment capability catalog and executor registry
- rejection of contract identity drift

Root capability aggregation calls the common contributor contract. Root runtime never calls a docs registration function directly.

## Docs Physical Activation

Docs activation needs these logical bindings:

```text
workspace
subject
agent
claim policy exact ref
provider for selected model-backed capabilities
```

Workspace, subject, and Agent come from assignment and activation. Claim policy comes from the exact package receipt. Provider is required only when `docs.draft_patch_set` or `docs.validate_patch_set` is selected.

The current `DocsCapabilityConfig` may remain an internal docs adapter value, but root must stop constructing it. The docs contributor translates neutral assignment and activation refs into this owner value.

If a future docs package selects only deterministic inspection and assessment capabilities, activation must not require a provider.

Docs provides the trusted local end of the placement spectrum. Existing path validation and narrow workspace contracts preserve semantic and effect boundaries, but in-process execution is not described as a hostile-code filesystem sandbox. Model-backed invokers add a provider binding and external result-admission path without changing package meaning.

Two docs assignments over one workspace receive distinct catalogs, executor registries, binding namespaces, authority contexts, and activation generations. The same exact capability contracts may bind different provider or workspace implementation instances without registry collision.

The docs activation contributor returns an exact participant plan. Deterministic local capabilities may run as bounded owner participants. Model-backed provider calls use an operation adapter that persists the durable operation and attempt before bounded dispatch, then admits completion through a later poll, delivery, or reconciliation observation. Each participant names readiness dependencies, structural wake refs, an owner safe-point ref, a stop ref, and required status.

Readiness proves the exact workspace, subject, claim policy, selected contracts, invokers, and provider binding when selected. A `no_work` result is only an owner wait reason. It proves active idle only when its workspace, event, timer, provider-completion, or pending-operation wake path is durable and resolvable.

## Lifecycle Parity

The router migration preserves more than successful capability output. Legacy and routed activation must converge on the same participant readiness, work admission, justified waiting, wake behavior, safe-point ownership, durable external-operation handling, stop behavior, and recovery result.

Quiescence is assignment-wide. It requires owner receipts over the exact participant plan, not one empty docs queue or one `no_work` tick. Graceful retirement closes admission, reaches owner safe points for accepted claims and publication outboxes, records the required event and projection watermarks, then stops participant incarnations. Recovery keeps admission closed until those durable states and wake registrations reconcile.

## Existing Flywheel Preservation

The successful semantic chain remains:

```text
inspect docs scope
→ evidence bundle
→ draft patch set
→ validate claims and patch set
→ publish validated patch set
→ publication receipt
→ assess published scope
→ docs freshness assessment
→ outcome mapping
→ belief revision
→ maintained-condition satisfaction review
```

Router installation and activation do not add workflow steps. They determine the exact theory and implementation image under which the existing chain runs.

Task success remains insufficient freshness evidence. Only the terminal docs freshness assessment admitted through the exact outcome mapping may support the freshness belief.

## Root Installation Refactor

`WorldInitTheoryBundle` is replaced on the canonical path by one loaded PDS package source plus the route catalog.

Current root behavior:

```text
load each fixed body by selected id
→ build fixed bundle
→ call every owner registry
→ build fixed receipt
```

Target root behavior:

```text
load one package entry
→ call theory router install
→ receive generic package receipt
```

`init` retains stage ordering and presentation. It stops importing docs policy loaders and owner body types. The router commits the generic receipt after all owner handlers install and verify exact revisions.

The existing authored-theory provisioning command becomes a package-source adapter during migration. It may copy or validate the package directory under the XDG theory source root, but canonical installation always begins at `pds-package.json`.

## Root Receipt And Resolution Refactor

The current fixed receipt fields become generic routed component refs.

| Current field | Generic target |
| --- | --- |
| `belief_family` | component ref on belief-family route |
| `curation_rule` | component ref on Agent curation route |
| `maintained_condition` | component ref on Agent maintained-condition route |
| `outcome_mapping` | component ref on outcome-mapping route |
| `strategy_theory` | component ref on Strategy route |
| `executable_contracts` | component refs on execution capability route |
| `authority_policy` | component ref on execution authority route |
| `claim_policy` | component ref on docs claim-policy route |

`ResolvedStewardshipTheory` becomes a generic `ResolvedPdsPackage` at root. Each runtime domain resolves its own bodies from route-addressed refs.

The docs contributor queries the resolved package for `docs.claim-policy.v1`, requires exactly one ref for the current package contract, and resolves it through docs. Root never stores a decoded claim policy on the generic resolved package.

## Root Runtime Assembly Refactor

Current direct path:

```text
hydrate_stewardship_theory
→ activate_exact_capabilities
→ docs register_exact_contracts
```

Target path:

```text
resolve package receipt
→ resolve assignment and activation bindings
→ collect activation-selected exact capability refs
→ invoke product capability contributors
→ verify exact catalog and invoker closure
→ build existing domain runtimes
```

Root assembly may know the contributor trait and neutral package refs. It must not import docs claim policy, docs capability config, or docs capability constants.

Static boundary scan:

```text
rg -n "crate::docs|DocsClaimPolicy|DocsCapabilityConfig|docs_freshness" src/runtime src/init/world src/capability.rs
```

Expected canonical result is no semantic docs dependency. Compatibility modules may remain temporarily and must be explicitly allowlisted with removal notes.

## Configuration Refactor

Canonical configuration moves from fixed theory fields to package, assignment, and activation selection.

Conceptual target:

```toml
[stewardship.assignments.docs_freshness]
package_receipt = "exact-package-receipt"
principal_id = "local-user"
agent_id = "docs-freshness-agent"
subject_domain = "workspace_fs"
subject_kind = "node"
subject_id = "workspace-root"
perspective_id = "default"
branch_id = "main"

[stewardship.activations.docs_freshness]
assignment_id = "docs_freshness"
workspace = "/absolute/workspace"
provider = "configured-provider"
```

New configuration does not repeat belief family, mapping, curation, maintained condition, Strategy, authority, or claim-policy ids. Those are package component truth.

Several assignments may target the same workspace. Target lookup therefore returns a collection. Existing singular resolution remains only as a temporary compatibility function for callers that prove exactly one assignment.

## Compatibility Lowering

Two current input forms need one migration seam:

```text
legacy stewardship.docs_freshness
canonical fixed stewardship.declarations entry
```

Both lower into the same synthetic docs package entry, assignment, and activation.

The synthetic package manifest is deterministic from the selected fixed identities and the exact current authored bodies. It routes the bodies exactly as the real docs package manifest does. It does not call the old fixed installation path.

The shim must carry a local note stating:

- which current configuration forms it accepts
- that it preserves fixed selection behavior through canonical routing
- that new configuration must use package assignments
- that removal requires configuration parity tests and supported deployment migration

No dependency-security code may use the compatibility shape.

## Historical Receipt Compatibility

Historical fixed docs receipts must remain resolvable for their supported data window.

A versioned compatibility decoder converts one fixed receipt into a read-only generic package view:

```text
fixed receipt
→ synthetic docs package identity
→ route-addressed exact owner refs
→ historical generic view
```

The decoder carries existing exact refs intact. It does not reinstall owner bodies or recompute them from current files.

New installations write only generic package receipts after cutover. The compatibility decoder is removed only when old receipt support is intentionally ended under compatibility policy.

## Test Fixture Refactor

`src/docs/pds.rs` currently proves direct docs composition in tests. During migration it becomes characterization evidence only.

Required transition:

1. Freeze current five-contract, Strategy, artifact chain, policy, and exact-binding behavior in characterization tests.
2. Add equivalent tests that install the docs package through the router.
3. Run both paths over identical workspace and provider stubs.
4. Compare semantic outputs and exact selected contract identities.
5. Delete direct composition after parity and retain router-path tests as enduring coverage.

New tests must not call `compose_with_theory` or direct docs registration except inside the characterized legacy fixture.

## Docs Domain Requirements

| Requirement | Contract |
| --- | --- |
| `DF-R01` | publish `docs.claim-policy.v1` through a docs-owned route handler |
| `DF-R02` | preserve `DocsClaimPolicy` validation and exact revision identity |
| `DF-R03` | expose generic owner revision refs without copying policy bodies |
| `DF-R04` | publish all five docs capability contracts through one domain contributor |
| `DF-R05` | bind exact selected docs invokers through the contributor |
| `DF-R06` | resolve claim policy from the exact package receipt during docs activation |
| `DF-R07` | preserve current artifact schemas and evidence non-substitution |
| `DF-R08` | keep current docs filesystem, claim validation, publication, and assessment semantics |
| `DF-R09` | require provider only for selected model-backed docs capabilities |
| `DF-R10` | emit no package, belief, Agent, Goal, or task state from docs theory installation |

## Theory, Config, Init, And Runtime Requirements

| Requirement | Contract |
| --- | --- |
| `DF-R11` | add one docs package manifest routing every current body |
| `DF-R12` | install docs through the generic router and package receipt |
| `DF-R13` | remove fixed docs slots from canonical package selection |
| `DF-R14` | separate docs assignment from physical activation |
| `DF-R15` | let init call the router rather than docs and owner loaders |
| `DF-R16` | let runtime freeze a generic package view rather than decoded docs bodies |
| `DF-R17` | remove direct docs calls and imports from root assembly |
| `DF-R18` | verify all selected docs contracts have exact invokers after contributor assembly |
| `DF-R19` | support historical exact docs package resolution |
| `DF-R20` | preserve external storage and workspace purity |

## World Model And Execution Requirements

| Requirement | Contract |
| --- | --- |
| `DF-R21` | publish route handlers over current belief, mapping, Agent, Strategy, capability, and authority registries |
| `DF-R22` | preserve exact Strategy selectors and capability content identities |
| `DF-R23` | preserve curation and maintained-condition revision lineage |
| `DF-R24` | preserve outcome mapping as the only task-outcome to docs evidence interpretation path |
| `DF-R25` | preserve current Agent, Goal, planning, task, dispatch, and satisfaction behavior |
| `DF-R26` | add no docs variants or expression branches to generic crates |

## Compatibility Requirements

| Requirement | Contract |
| --- | --- |
| `DF-R27` | characterize current direct docs package composition before refactor |
| `DF-R28` | lower both current config forms through one synthetic package adapter |
| `DF-R29` | decode historical fixed receipts into read-only generic package views |
| `DF-R30` | require parity before removing direct installation or activation paths |
| `DF-R31` | mark every temporary shim with a concrete removal condition |
| `DF-R32` | ensure new docs tests and configuration target only the router path |

## Lifecycle And Recovery Requirements

| Requirement | Contract |
| --- | --- |
| `DF-R33` | publish an exact expected participant plan with readiness, wake, safe-point, stop, dependency, and required-status refs |
| `DF-R34` | distinguish active idle, quiescent, stalled, fenced quiescent, stopped, and interrupted docs states |
| `DF-R35` | treat `no_work` only as an owner wait reason with a durable structural wake path |
| `DF-R36` | persist provider operation and attempt lineage before bounded dispatch |
| `DF-R37` | admit provider completion only under exact activation generation and participant incarnation lineage |
| `DF-R38` | keep recovery admission closed until claims, artifacts, outboxes, operations, projections, and wake registrations reconcile |
| `DF-R39` | permit equivalent participant restart under one activation generation only through a new incarnation and readiness proof |
| `DF-R40` | prove assignment quiescence from the complete participant plan and owner receipts without a cross-store transaction |

## Requirement Trace To Assessment

| Assessment requirement | Docs requirements |
| --- | --- |
| `AR-07` | `DF-R04`, `DF-R05`, `DF-R18` |
| `AR-08` | `DF-R01`, `DF-R06`, `DF-R17` |
| `AR-09` | `DF-R12`, `DF-R21` |
| `AR-14` | `DF-R27` through `DF-R32` |
| `AR-15` | `DF-R12`, `DF-R19` |
| `AR-16` | `DF-R03`, `DF-R19`, `DF-R21` |
| `AR-17` | `DF-R15`, `DF-R17`, `DF-R21` |
| `AR-18` | `DF-R10`, `DF-R14`, `DF-R20`, `DF-R25` |
| canonical lifecycle and quiescence | `DF-R33` through `DF-R40` |

## Migration Design

### Gate A Characterize current semantics

Capture current behavior before moving ownership:

- fixed theory selection validation
- fixed receipt identity and exact resolution
- five published capability contracts and content identities
- exact invoker registration
- claim-policy revision identity
- Strategy closure over five capabilities
- terminal assessment evidence mapping
- reopen and historical resolution behavior

No canonical contract changes in this gate.

### Gate B Introduce router consumers beside current path

Add route handlers over existing owner registries, the docs package manifest, generic package receipt, and docs capability contributor. Install the same current bodies through the router in tests while production remains on the characterized path.

No owner body is duplicated in a router registry.

### Gate C Cut canonical installation and activation

Make new docs package installation use the router and make runtime assembly use capability contributors. Canonical configuration selects package receipt, assignment, and activation.

Compatibility input lowers into this canonical path. It does not invoke the fixed path.

### Gate D Prove parity and historical resolution

Run both paths against identical fixtures. Verify semantic artifact parity, exact selected capability parity, policy identity parity, event and evidence parity, Goal satisfaction parity, reopen behavior, and historical fixed receipt resolution.

Any intentional difference is a separately approved breaking change.

### Gate E Retire direct docs seams

After parity:

- remove direct docs package composition
- remove root docs claim-policy loading
- remove root docs capability registration
- remove fixed canonical theory selection fields
- stop writing fixed receipts
- retain only explicitly supported historical receipt decoding
- remove configuration shims when their stated deployment condition is satisfied

## Expected Domain-First Code Shape

```text
theory/docs_freshness/pds-package.json

src/docs.rs
src/docs/theory.rs
src/docs/theory/claim_policy.rs
src/docs/capability.rs
src/docs/claim_validation.rs

src/theory.rs
src/theory/router.rs
src/theory/receipt.rs

src/config/stewardship/assignment.rs
src/config/stewardship/activation.rs
src/config/stewardship/compat.rs

src/init/world/pipeline.rs
src/runtime/assembly.rs

crates/meld-world-model/src/belief/theory.rs
crates/meld-world-model/src/agent/theory.rs
crates/meld-world-model/src/strategy/theory.rs
crates/meld-execution/src/capability/theory.rs
crates/meld-execution/src/authority/theory.rs
```

Exact file extraction follows implementation evidence. No `mod.rs` files are introduced.

## Verification And Acceptance Criteria

### Package installation

- `DF-AC01` one docs package entry routes every current theory body to its owner
- `DF-AC02` package receipt contains exact refs for every current body and selected capability contract
- `DF-AC03` missing docs claim policy prevents package receipt commit
- `DF-AC04` malformed docs claim policy is rejected by docs, not by root
- `DF-AC05` exact reinstall reuses owner revisions and package receipt
- `DF-AC06` historical docs package receipt resolves after a newer package installs

### Capability parity

- `DF-AC07` router activation selects the same five capability type and version pairs
- `DF-AC08` unchanged contracts retain current content identities
- `DF-AC09` each selected contract binds exactly one docs invoker
- `DF-AC10` contract identity drift fails activation
- `DF-AC11` unselected docs capability remains absent from the assignment catalog
- `DF-AC12` deterministic docs subset activates without provider when no selected contract requires it

### Flywheel parity

- `DF-AC13` inspection produces semantically equivalent evidence bundle
- `DF-AC14` drafting produces semantically equivalent patch-set contract
- `DF-AC15` claim validation applies the same exact policy revision
- `DF-AC16` publication enforces the same validation fingerprint and path safety
- `DF-AC17` assessment produces semantically equivalent freshness assessment
- `DF-AC18` task success alone remains insufficient belief evidence
- `DF-AC19` terminal assessment maps through the same exact outcome mapping
- `DF-AC20` belief revision and maintained-condition satisfaction retain theory lineage

### Root isolation

- `DF-AC21` root theory contracts contain no docs claim-policy type
- `DF-AC22` root install code contains no docs body loader
- `DF-AC23` root assembly contains no direct docs registration call
- `DF-AC24` root capability inventory does not return docs-only publication
- `DF-AC25` root dispatch contains no docs expression or route branch
- `DF-AC26` generic crates contain no new docs variants

### Configuration and compatibility

- `DF-AC27` canonical assignment config names package receipt rather than fixed body ids
- `DF-AC28` legacy docs table lowers into the canonical router path
- `DF-AC29` current fixed declaration lowers into the same canonical router path
- `DF-AC30` legacy and canonical inputs produce equivalent assignment and activation semantics
- `DF-AC31` old fixed receipts remain historically resolvable through the compatibility decoder
- `DF-AC32` compatibility shims have removal notes and no new callers

### Durability and workspace purity

- `DF-AC33` package install, assignment, and activation survive reopen
- `DF-AC34` owner partial installation without package receipt remains inactive and retry converges
- `DF-AC35` no theory, receipt, assignment, activation, belief, task, or adapter state is written beneath the workspace
- `DF-AC36` current event, evidence, Goal, task, and outcome reopen proofs remain green

### Second-consumer validation

- `DF-AC37` docs and dependency-security packages install through the same manifest, route, and receipt contracts
- `DF-AC38` neither package causes owner-specific fields to be added to the common receipt
- `DF-AC39` docs and dependency-security capability contributors coexist in one product inventory
- `DF-AC40` runtime assembly activates each through exact contracts without expression dispatch
- `DF-AC41` deterministic and model-backed docs assignments over one workspace retain separate selected catalogs, executor instances, grants, and binding requirements
- `DF-AC42` a missing provider fails only the model-backed activation and leaves deterministic docs and dependency-security assignments unaffected
- `DF-AC43` docs and security assignments consuming overlapping workspace facts retain independent package, assignment, perspective, activation, and authority lineage
- `DF-AC44` the same exact docs capability contract binds two physical implementations without changing contract identity or colliding in a global executor registry
- `DF-AC45` provider retry after credential rotation preserves a durable operation key while recording a new attempt and activation generation
- `DF-AC46` prepared docs closure exposes no invoker until required local and provider-backed objects prove readiness and the generation becomes current
- `DF-AC47` legacy and routed docs activation converge on equivalent wake, readiness, admission, stop, and recovery behavior
- `DF-AC48` a docs `no_work` tick remains active idle and does not by itself prove admission closure or shutdown

### Lifecycle and recovery

- `DF-AC49` a clean zero-work result cannot prove quiescence while a required participant or structural wake path is absent
- `DF-AC50` a fully idle docs activation proves each expected participant has a durable wait reason and resolvable wake path
- `DF-AC51` crash after preparation but before current publication resumes or retires the inert generation without exposing invokers
- `DF-AC52` an equivalent docs participant restart creates a new incarnation under the same activation generation only after readiness closes
- `DF-AC53` provider work is durable before dispatch and ambiguous completion reconciles before retry or canonical admission
- `DF-AC54` a result from an older participant incarnation cannot alter current docs products without explicit historical admission policy
- `DF-AC55` workspace or canonical event advance wakes the eligible docs participant without a manual workflow edge
- `DF-AC56` recovery keeps admission closed until claims, artifacts, publication outboxes, provider operations, projection watermarks, and wake registrations reconcile
- `DF-AC57` fenced quiescence preserves owner safe points for artifacts and outboxes while the activation service records only aggregate receipts

## Static Boundary Checks

```text
rg -n "DocsClaimPolicy|DocsClaimPolicyRevisionRef" src/runtime src/init/world src/config
rg -n "crate::docs::capability::register_exact_contracts" src/runtime src/init src/capability.rs
rg -n "claim_policy_id" src/runtime src/config/stewardship src/init/world
rg -n "docs_freshness" crates/meld-events crates/meld-execution/src crates/meld-lang/src
rg --files src crates | rg '/mod\.rs$'
```

Expected outcome after canonical cutover:

- docs policy types appear only in docs and explicit compatibility code
- root contains no direct docs activation call
- canonical package selection contains no claim-policy field
- generic crates contain no application dispatch branches
- no new `mod.rs` files exist

## Intentional Non-Changes

- The docs domain remains inside the current repository.
- The five capabilities remain domain-owned.
- Claim validation remains docs-owned.
- Strategy remains world-model-owned.
- Capability contracts and authority remain execution-owned.
- Provider behavior remains provider-owned.
- Workspace reads and writes remain behind existing domain contracts.
- Event, belief, Agent, Goal, task, and satisfaction mechanisms remain unchanged unless the second consumer falsifies a generic contract.
- No Synthesis dependency is introduced.

## Rejection Criteria

Reject the refactor if it:

- copies `DocsClaimPolicy` into theory contracts
- adds a docs component field to the generic receipt
- keeps direct docs registration as the canonical runtime path
- selects capability contributors by `docs_freshness` expression name
- requires every PDS package to have a docs claim policy
- stores decoded docs bodies in root runtime resolution
- changes capability content identity without a contract version decision
- treats package installation as flywheel success
- deletes the old path without characterization and parity proof
- adds new callers to a compatibility shim

## Completion Condition

The refactor is complete when docs freshness is installed from one PDS package entry, every component is validated and resolved by its owning domain, the generic package receipt contains no docs-shaped slot, exact docs invokers bind through the common contributor pattern, the full existing flywheel retains semantic and durability parity, and root contains no direct docs installation or activation knowledge.
