# Docs Freshness Strategy

Date: 2026-07-22
Status: active worked example
Scope: derive correct README work from domain theory and current world state

## Objective

The docs freshness Directive is:

```text
Every folder in the activated scope has a current README
whose correctness meets the configured threshold.
```

The target preserves the useful output shape of the existing docs freshness workflow: README files are produced per folder, work may fan out across independent folders, and semantic context may flow from descendants toward ancestors. Strategy adds stronger correctness and convergence semantics: exact published bytes are evaluated and bounded work may repeat until the active Goal is satisfied or no useful action is currently available.

The Strategy proof must obtain that behavior without placing bottom-up traversal, repeated turns, prompt order, child wiring, or a fixed task network in the Directive or stewardship package.

## What the PDS must supply

The docs correctness operational domain theory must supply meaning, not procedure.

### Vocabulary and relations

The theory identifies workspace, folder, source file, README, correctness evaluation, and semantic coverage artifact.

It defines relations such as contains, direct child, documents, located at path, derived from evidence, and covers.

It declares the canonical README location as `README.md` beneath each folder.

### Maintained condition

The theory declares that each material folder in activated scope requires one current README and that the exact published README bytes must meet the configured correctness threshold.

### Correctness theory

Correctness is not the existence of a file.

For a folder README to be correct, it must:

- faithfully represent material direct contents
- represent every material direct child subtree
- support its claims with current admitted evidence
- omit no required folder concern
- satisfy required structure and style dimensions
- pass an evaluator against the exact published bytes

The theory owns threshold, dimensions, materiality, exclusions, evaluation route, evaluator independence, and accepted outcome shape.

### Evidence semantics

The theory distinguishes relevance, admission, projected sufficiency, and settled correctness.

A file or child subtree may be relevant to a README without its raw bytes being admitted or sufficient generation evidence.

The theory distinguishes two artifacts.

A `SourceCoverageSummary` is pre-generation evidence derived from admitted direct material. Under this docs correctness theory it covers only that direct material, may support generation for the same folder, and cannot discharge a transitive child-subtree obligation. It does not prove that the folder README is correct.

A `SettledFolderCoverage` artifact is admitted only after the exact published README bytes meet the relevant correctness and obligation-coverage thresholds and the result is reconciled. A below-threshold result remains outcome evidence, blocks the prospective edge, and does not release parent work. An admitted artifact may summarize one subtree for an ancestor while retaining:

```text
covered subjects
transitive source references
graph revision
obligation coverage
admitted semantic representation reference
semantic representation schema and content digest
freshness
semantic loss
size
producer lineage
```

PDS declares the evidence policy. Belief owns the admission verdict that decides whether a `SettledFolderCoverage` artifact may discharge a subtree-coverage obligation. The Agent selects decision context but cannot override that verdict.

Plain Markdown, a prior README, or a generic `frame_ref` is not automatically a semantic coverage artifact. Exact published README bytes, accepted evaluation results, and provenance may support admission of a new typed coverage artifact. They are not interchangeable with that artifact.

### Semantic action affordances

The theory exposes unordered action meaning for:

```text
observe folder topology
inspect material file content
produce source coverage summary
assemble bounded generation context
generate a README candidate
publish README bytes
evaluate exact published bytes
```

Each affordance declares semantic preconditions, predicted effects, artifact meaning, capacity constraints, authority class, and outcome contract.

The PDS may also import known reusable Methods. It must not require one generated episode to use them.

### Capacity and authority

The theory and activation expose configured context limits, quotas, publication authority, evaluator authority, and independence constraints. Live availability and remaining quota are runtime state supplied through an operational projection.

It does not select a traversal strategy.

## What the PDS must not encode

The Strategy derivation is falsified if its PDS source must declare:

```text
bottom-up traversal
leaf-first order
repeated regions
turn sequence
prompt sequence
child frame wiring
retry count
timeout policy
workflow gates
provider instance
```

Those are candidate realization details or operational policy. They are not the meaning of docs correctness.

## Grounding the Directive

Belief Reconciliation and graph projection first settle the current folder graph and the relevant beliefs that apply to its members.

The Directive Agent authorizes grounding the universal maintained condition over one exact trusted scope.

For each material folder, Strategy derives obligations such as:

```text
README exists at the canonical path
README covers material direct files
README covers every material direct child subtree
README claims use current admitted evidence
exact published bytes have a current correctness evaluation
correctness meets the threshold
```

These are internal Strategy obligations. They are not separately inserted Execution Goals.

## Discovering action paths

Strategy works backward from each unsettled obligation through semantic affordances and authoritative verdicts.

For a missing README, candidate paths may include:

```text
direct generation from admitted raw evidence
generation from admitted semantic coverage artifacts
hybrid generation from raw direct evidence and child coverage artifacts
reuse of a current README followed by exact-byte evaluation
observation before generation when evidence is missing
```

Artifact compatibility alone does not make an evidentiary path valid. Every existing input used as evidence or obligation discharge must have an admission verdict and projected sufficiency. Ordinary operational artifacts such as candidate bytes use capability and artifact contracts without becoming epistemic evidence.

A future evidence artifact does not yet have an admission verdict. Strategy may use it for obligation discharge only through a prospective artifact contract that names the expected producer, content identity, outcome contract, owning admission authority, and downstream obligation. The evidentiary edge must be gated on a later authoritative admission verdict. Execution waits for that verdict and never decides admission itself.

## Why bottom-up fan-out becomes viable

Bottom-up fan-out follows from the correctness obligations and capacity facts rather than a named traversal rule.

### Leaf case

A leaf folder has no child-subtree obligations.

When its admitted direct evidence fits the context bound, Strategy can construct:

```text
inspect direct material
→ produce source coverage summary
→ generate README
→ publish exact bytes
→ evaluate exact bytes
→ await Belief reconciliation and admission verdict
```

The exact-byte evaluation binds output path, published content digest, folder graph revision, admitted source-evidence set, evaluator revision, score dimensions, threshold, independence posture, and provenance. Any later byte or material source revision invalidates that result.

### Parent case

A parent README has one coverage obligation for each material direct child subtree.

If raw descendant evidence exceeds the context bound, direct generation is ineligible or projects below the correctness threshold. An admitted child `SettledFolderCoverage` artifact can discharge the corresponding child obligation within the bound.

The child README generation, publication, exact-byte evaluation, reconciliation, and coverage admission must therefore precede the parent generation step.

Sibling child producers do not depend on one another. Strategy preserves them as independent branches. Execution may run those branches concurrently. Parent work becomes eligible after the required child artifacts are admitted.

```text
child A README → evaluate → admit coverage ─┐
                                            ├→ parent bounded context
child B README → evaluate → admit coverage ─┤  → parent README
                                            │  → exact-byte evaluation
admitted direct evidence ───────────────────┘
```

This is a bottom-up fan-out candidate derived from:

```text
recursive folder obligations
+ bounded generation context
+ prospective admission gates for settled folder coverage
+ causal input requirements
```

No bottom-up declaration is needed.

## Why bottom-up is not prescribed

The same theory can yield another valid Composition when the world changes.

If all admitted evidence fits one context and direct generation projects above threshold, Strategy may choose a direct path.

If some child artifacts are current and others are stale, Strategy may choose a hybrid path.

If observation is cheaper than regenerating uncertain branches, Strategy may acquire evidence first.

Correctness semantics establish which alternatives are viable. Dynamic efficacy, critical-path time, resource cost, uncertainty, risk, and the Agent value posture rank viable alternatives.

Cost does not create the dependency topology. It helps choose among topologies that already have semantic support.

## Concrete Strategy walk

The initial world-model frame may establish:

```text
folder topology is stale
README coverage is not yet groundable
```

Strategy can propose a bounded observation Composition. After Agent authorization, Execution scans the scope and publishes topology evidence. Belief Reconciliation settles a new trusted graph revision.

The next Strategy attempt grounds the Directive over the discovered folders. Missing README and unknown correctness beliefs yield concrete obligations.

For one folder, constrained context and recursive coverage may make settled child coverage necessary. Strategy constructs a concrete Composition with parallel child README branches, exact-byte evaluation, world-model admission waits, and child-to-parent edges gated by the resulting authoritative verdicts.

Execution realizes the authorized candidate and commits the task network. Completed work publishes artifacts and outcomes. Reconciliation revises existence, coverage, freshness, and correctness beliefs.

If correctness remains below threshold, the Goal remains active. A later Strategy attempt uses the revised evidence and may refine only the deficient concern, choose another provider path, acquire more evidence, or abstain.

When reconciled correctness crosses the threshold for every grounded folder, Agent satisfaction curation closes the Goal.

## Bounded convergence

The example proves a loop rather than one workflow run.

```text
observe topology
→ reconcile scope
→ ground README obligations
→ construct bounded Strategy
→ execute bounded work
→ evaluate exact published bytes
→ reconcile outcome
→ re-strategy when still below threshold
```

When no useful action is available, Strategy records abstention and the Strategy association becomes quiescent while the Goal remains `Active`. New evidence, authority, capacity, or capability availability may wake it.

The acceptance trace must prove all of the following:

1. The first published attempt evaluates below threshold.
2. Exact outcome evidence and revised beliefs persist across restart.
3. A second Strategy decision cites that revised state and materially changes or refines the candidate.
4. The second attempt produces distinct outcome evidence.
5. Reconciled correctness crosses threshold and Agent satisfaction curation closes the Goal.
6. A no-useful-action case leaves the active Goal with a quiescent Strategy association and no repeated equivalent work.
7. New relevant evidence wakes that association and permits another bounded attempt.
8. Later source drift reopens maintenance work after prior satisfaction.

The runtime remains active after satisfaction. Later filetree drift can reopen docs freshness work through new evidence and Agent curation.

## Relationship to the existing workflow

The existing docs writer is a valuable compatibility Method and implementation asset. It already demonstrates scanning, traversal, fan-out, context preparation, provider calls, verification stages, publication, and repeated work.

Its strategy is authored explicitly in package and workflow configuration. The current package selects bottom-up traversal, fixed stages, repeated turns, and child artifact wiring. The current workflow fixes turn order, gates, retry posture, and persistence behavior.

### Current YAML classification

The current YAML mixes domain meaning, one chosen theory of action, and physical execution policy. Migration classifies each concern rather than copying the file into PDS.

| Current configuration | Target owner | Strategy meaning |
|---|---|---|
| accepted target fields | assignment scope and Execution trigger | identifies the requested workspace subject |
| target Agent identity | stewardship assignment and Agent perspective | identifies responsibility and judgment authority |
| provider and frame bindings | activation | supplies physical provider and context bindings |
| force posture | Agent and Strategy episode posture | changes reuse and intervention preference without changing domain theory |
| canonical `README.md` filename | docs correctness domain theory | identifies the maintained artifact |
| `directories_bottom_up` | compatibility Method only | candidate topology must be derived from obligations and constraints |
| overwrite on new head | publication affordance plus governance | declares allowed publication effect and conflict posture |
| repeated node region | compatibility Method and lowering machinery | Strategy grounds concrete folder obligations and independent branches |
| fixed prepare, execute, finalize stages | capability implementation and compatibility Method | semantic affordances remain implementation-neutral |
| four named turns and prompt refs | compatibility Method assets | Strategy may select actions that use these assets but PDS does not order them |
| child `frame_ref` wiring | compatibility dataflow | cannot discharge subtree coverage without typed admission |
| schema and no-drift gates | capability and artifact validation | do not establish exact-byte correctness |
| retries, timeouts, fail-fast, and resume | Execution policy | bounded operational realization rather than domain meaning |
| artifact persistence | Execution durability policy | preserves replay inputs and outputs |

The Strategy target does not discard that machinery. It separates reusable capabilities and proven operational mechanics from the hardcoded theory of action.

During migration, the existing workflow remains a characterized compatibility Method until Strategy-derived execution demonstrates parity for:

- accepted targets and start conditions
- traversal and node coverage
- sibling concurrency
- child artifact propagation
- force and reuse semantics
- provider and prompt asset selection
- retry and timeout precedence
- nonblocking gates
- persistence and replay
- publication and failure behavior

No current precedence rule should be inferred or silently changed during that migration.

## Falsification tests

The worked example is credible only if tests can disprove accidental hardcoding.

Required tests include:

- deep constrained trees derive child-to-parent dependencies while shallow unconstrained trees permit direct generation
- increasing context capacity can remove semantic-compression edges
- decreasing capacity makes the direct path ineligible unless another admitted path exists
- removing an affordance removes every edge that depends on its effect
- revoking authority rejects the candidate despite installed capability
- changing folder containment changes grounded obligations and candidate topology
- stale or denied child evidence invalidates the exact parent edge that cites it
- one changed descendant invalidates its branch and provenance-dependent ancestors while unrelated siblings remain reusable
- plain Markdown and generic frame references fail subtree-coverage admission
- publication and pre-generation checks never satisfy exact-byte correctness
- exact-byte evaluation identity includes digest, path, graph revision, evidence set, evaluator revision, dimensions, threshold, independence, and provenance
- the first below-threshold attempt persists revised state and a materially refined second Strategy produces new evidence that crosses threshold
- no useful action leaves the Goal active with a quiescent Strategy association and no duplicate work
- new evidence wakes quiescent Strategy and later drift reopens satisfied maintenance work
- the authored docs-writer Method and named traversal strategy may be unavailable while the same domain theory and affordances still derive the candidate topology
- when an alternate domain theory admits a transitive source summary, the child README dependency disappears
- a non-documentation domain derives the same dependency pattern from recursive obligations and bounded evidence

## Current implementation gap

Meld does not yet implement this derivation.

Current planner selection chooses among authored Methods and current docs freshness configuration contains the detailed workflow shape. The graph, belief, traversal, context, provider, task-network, and event foundations are useful primitives, but the following are still missing:

- Strategy decision persistence and invalidation
- universal Directive grounding over trusted scope
- semantic action affordance discovery
- typed evidence admission and projected sufficiency for coverage
- alternative Composition construction
- bound candidate efficacy and critical-path projection
- generic many-branch artifact fan-in
- exact published-byte correctness evaluation and Directive aggregation
- authoritative post-production evidence gates inside the task network

The example defines the required architectural outcome. It does not claim that the current runtime already produces it.

## Read with

- [Strategy Overview](README.md)
- [Strategy Requirements](requirements.md)
- [Strategy Contracts](contracts.md)
- [Belief Reconciliation Network](../belief/README.md)
- [Meld Lang Compositions](../../meld-lang/compositions.md)
- [Execution Planning](../../execution/planning/README.md)
- [Persistent Domain Stewardship](../../../persistent_domain_stewardship/README.md)
