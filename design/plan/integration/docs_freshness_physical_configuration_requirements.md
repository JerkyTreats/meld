# Docs Freshness Physical Configuration Requirements

Date: 2026-06-18
Status: proposed
Scope: physical configuration requirements for the first `docs_freshness` durable flywheel example

## Purpose

This document answers where the physical `docs_freshness` example gets its configuration and where the freshness directive is seeded.

The short answer is:

- the product-visible `docs_freshness` example should be described by one activation document
- freshness belief family configuration already exists as world model belief runtime configuration
- a directive has independent identity as a thin durable record, and the seed agent references it by `directive_id`
- the current physical example seeds the directive from the integration fixture
- the supervisor must not seed the directive
- product assembly loads and validates the activation document, then decomposes it into domain runtime inputs
- world model agent bootstrap owns idempotent registration of the directive record and the seed agent that references it

## Configuration Planes

The physical source should be one `docs_freshness` activation document.

That document may be TOML, YAML, or JSON. The first implementation should choose one concrete format and parse it into the same typed DTO. Additional formats must be loaders for the same DTO, not new runtime contracts.

The activation document is a transport and validation input for root product assembly. It is not semantic authority. Product assembly splits the validated DTO into owner-scoped runtime inputs. The downstream runtimes receive typed inputs and do not know whether those inputs came from TOML, YAML, JSON, fixture code, or CLI overrides.

The physical example still needs separate configuration planes because each value has a different owner.

| Plane | Owner | Purpose |
| --- | --- | --- |
| activation document source | root assembly | parse one external `docs_freshness` file into a validated DTO |
| product root config | root assembly | choose product storage root and enabled runtime ids |
| belief family config | `meld-world-model` belief runtime | define `docs_freshness` evidence, comparator, projection, and threshold |
| seed directive config | `meld-world-model` agent runtime | define the trusted directive identity and text, plus the scope the referencing seed agent serves |
| curation rule config | `meld-world-model` agent runtime | define the first threshold rule that turns low belief confidence into a goal |
| execution method config | `meld-execution` planning runtime | define the docs refresh method and capability requirements |
| task package config | `meld-execution` task runtime | define task init seeds, capability wiring, and artifact outputs |
| publication event config | `meld-execution` publication runtime plus `meld-events` | define event type and stable record id derivation |
| proof fixture constants | integration proof only | pin first vertical proof identities and checkpoint expectations |

No single supervisor record should combine these into semantic authority.

No domain runtime should load or parse the activation document directly.

## Directive Identity And Cardinality

The directive is the agent meta-layer noun. See [World Model Agent](../../cognitive_architecture/world_model/agent/README.md#the-agent-meta-layer).

For this example the durable shell is minimal:

- the directive is a thin record `{ directive_id, text }` with identity independent of the agent
- the seed agent references the directive by `directive_id` and never embeds it
- the reference is not constrained to one-to-one; the first slice uses one directive served by one agent, and the shape reserves one directive served by many agents

Deferred for this example: intent-to-agent-set translation, agent-set decomposition records, directive revision, and multi-agent coordination. None are required to seed one agent, and each is additive over the shell above.

## Existing Configuration Evidence

The belief family config is already concrete in the physical fixture.

[docs freshness fixture](/home/jerkytreats/meld/tests/integration/docs_freshness_fixture.rs:164) carries a runtime JSON config for:

- family id `docs_freshness`
- dimension id `docs_freshness`
- predicate id `confidence`
- evidence policy id `default_policy`
- graph anchor evidence schema
- content written evidence schema
- content review evidence schema
- source mappings
- weighted Bayesian comparator
- default prior
- planner projection threshold `0.7`
- posterior meaning `stale_probability`

[belief plan](/home/jerkytreats/meld/design/plan/world_model/belief/PLAN.md:86) says this is intentional: the family is runtime configuration, not Rust family code.

[fixture contract](/home/jerkytreats/meld/tests/integration/docs_freshness_fixture_contract.rs:43) asserts that belief config, curation threshold, source mapping, and promoted evidence align.

The seed directive exists today only in fixture code.

[docs freshness fixture](/home/jerkytreats/meld/tests/integration/docs_freshness_fixture.rs:95) builds a `SeedAgentRegistration` with:

- agent id `seed.docs_freshness`
- perspective `default`
- subject `workspace_fs` node `node-a`
- branch scope `main`
- observation scope `docs_freshness`
- directive `curate docs freshness goals`
- seed provenance `trusted init`
- created sequence `0`

[reopen contract](/home/jerkytreats/meld/tests/integration/docs_freshness_reopen_contract.rs:340) currently registers that seed agent manually in the integration proof setup.

That is proof fixture seeding, not product supervisor seeding.

## Required Activation Document Shape

The first product config surface should be one activation document that makes the physical example explicit without moving ownership to root.

Suggested logical shape:

```text
flywheel.docs_freshness:
  subject:
    domain_id: workspace_fs
    object_kind: node
    object_id: node-a
  branch_scope:
    branch_id: main
  perspective:
    kind: default
    id: default
  belief_family:
    config_ref: docs_freshness
  directive:
    directive_id: directive.docs_freshness
    text: curate docs freshness goals
  seed_agent:
    agent_id: seed.docs_freshness
    directive_id: directive.docs_freshness
    observation_scope: docs_freshness
    seed_provenance: trusted init
  curation_rule:
    dimension_id: docs_freshness
    threshold: 0.7
    priority_urgency: 50
    desired_summary: confidence above 0.7
    source_kind: belief_divergence
  execution:
    method_id: refresh_docs_v1
    task_network_id: network-docs
    required_artifact_type_id: docs_patch
  publication:
    success_event_type: execution.task.succeeded
    failure_event_type: execution.task.failed
    content_source_kind: content_written
```

The product-visible implementation should not rely on fixture constants or embedded proof config for these values. It should load one activation document, validate it in product assembly, and build owner-scoped runtime input structs.

The important point is ownership:

- root assembly loads the activation document and validates the DTO shape
- root assembly resolves file paths, provider bindings, runtime ids, and references without querying semantic stores
- root assembly builds owner-scoped runtime input packages
- world model belief runtime receives its family config input or config ref from assembly and owns config snapshot persistence
- world model agent runtime receives directive, seed agent, subscription, and curation rule inputs from assembly and owns durable registration
- execution receives method, task package, artifact, task network, and publication mapping inputs from assembly and owns operational use
- event runtime owns append and sequence only
- supervisor owns lifecycle only

## Where The Directive Is Seeded

The supervisor does not seed the freshness directive.

Correct ownership:

```text
product assembly
-> load docs freshness activation document
-> validate activation DTO
-> split DTO into domain runtime inputs
-> build world model agent bootstrap runtime factory
-> hand factory to supervisor
-> supervisor acquires lease and starts runtime
-> world model agent bootstrap registers the directive record
-> world model agent bootstrap registers the seed agent referencing directive_id
-> world model agent runtime binds subscription
-> world model agent runtime performs curation on belief delivery
```

The directive and the seed agent that references it enter durable state at the world model agent boundary.

The durable writes are:

```text
register the directive record { directive_id, text }
register the seed agent referencing directive_id
```

The owner is:

```text
meld-world-model agent store
```

The supervisor may observe that `world_model.agent.seed.docs_freshness` started and heartbeated. It must not create the directive record or `SeedAgentRegistration`, choose the directive, choose the threshold, bind the subscription, or decide what the directive means.

## Bootstrap Runtime Requirement

The product runtime needs an explicit world model bootstrap actor for seed directives.

Suggested runtime id:

```text
world_model.agent.bootstrap.docs_freshness
```

Responsibilities:

- read validated seed directive and agent config supplied by product assembly
- register or confirm the directive record `{ directive_id, text }` in the world model agent store
- register or confirm the seed agent referencing `directive_id`
- bind the configured subscription to the `docs_freshness` belief key
- store bootstrap result idempotently
- expose a bounded diagnostic report
- stop after bootstrap work is complete or remain idle

Forbidden responsibilities:

- evaluate belief confidence
- decide whether a goal should exist
- call execution goal API
- mutate execution goal lifecycle
- publish events
- own supervisor leases

Idempotency rules:

- re-running bootstrap with the same config returns the existing directive, agent, and subscription
- the directive record is keyed by `directive_id`; re-running with the same id and divergent text fails with a world model config conflict
- re-running bootstrap with a conflicting `directive_id` for the same seed agent id fails with a world model config conflict
- subscription id is deterministic from agent id and belief key
- bootstrap success is read from the world model agent store, not supervisor state

## Supervisor Role

The supervisor role is lifecycle only.

Supervisor may:

- start the bootstrap runtime after lease acquisition
- record heartbeat and health
- restart the bootstrap runtime on retryable failure
- report bootstrap diagnostics

Supervisor must not:

- construct the directive text
- choose the seed agent id
- choose the observation scope
- choose the curation threshold
- write to the agent store directly
- write to belief config
- write to execution goal state
- treat bootstrap completion as semantic progress for the flywheel

## Product Assembly Role

Product assembly loads and validates the activation document.

The assembly boundary owns source format parsing. Runtimes receive typed values only.

Allowed validation:

- required field presence
- id syntax
- threshold numeric range
- runtime id syntax
- storage paths
- method id presence
- task network id syntax
- event type syntax
- activation document schema version
- unsupported source format
- unknown top level activation section

Forbidden validation:

- deciding if the directive is useful
- deciding if docs freshness should be monitored
- deciding if low confidence should create a goal
- deciding which evidence supports `docs_freshness`
- querying active goals
- querying belief views
- querying pending publications

Assembly passes validated input structs into owning runtime factories. It does not register the seed agent itself unless it is executing a world model bootstrap API whose owner is explicit.

## Vertical Proof Requirement

The physical proof may keep fixture seeding while the activation loader is not implemented, but it must name fixture seeding as a stand in for the single activation document.

The proof should assert:

- one activation document can produce the same domain input packages as the fixture
- fixture seed config can produce the same directive record and `SeedAgentRegistration`
- the seed agent references the fixture `directive_id`
- bootstrap registration is idempotent
- subscription id matches the deterministic fixture value
- curation rule threshold matches belief planner projection threshold
- supervisor reports never contain the directive as owned state
- checkpoint reopen reloads the directive, seed agent, and subscription from world model agent storage

## Acceptance

The physical example configuration is sufficiently designed when:

- one activation document is the product source for the `docs_freshness` slice
- root product assembly parses and validates that document before stores open
- root product assembly distributes owner-scoped runtime inputs instead of sharing the raw document
- downstream runtimes never parse the activation file and never depend on its source format
- `docs_freshness` belief family config is loaded as world model runtime config
- the directive has independent identity as a thin durable record and the seed agent references it by `directive_id`
- seed directive config is represented as product input, not supervisor state
- world model bootstrap runtime owns directive and seed agent registration
- supervisor only starts and monitors bootstrap
- the first proof can replace manual fixture registration with bootstrap runtime behavior
- curation threshold and belief projection threshold are pinned to the same product config value
- no root or supervisor component decides what the freshness directive means
