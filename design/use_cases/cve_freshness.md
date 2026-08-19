# Use Case: CVE Freshness

Date: 2026-07-24
Status: worked contrasting example
Scope: keep a project's dependencies free of known vulnerabilities and within currency policy

## Use case

A software project declares dependencies whose exact versions are pinned in a lockfile. Security advisories against those versions are published continuously by external databases, on no schedule and regardless of whether anyone is watching. A maintainer wants the project to stay free of known vulnerabilities above a chosen severity, and reasonably current, without hand-tracking advisories — and wants remediation to be safe: an upgrade must actually resolve, must not break the build or tests, and must not silently pull in a newly vulnerable transitive version.

The work has its own structure, different from documentation. Dependencies constrain one another: upgrading one may be impossible alone, or may force others to move. All remediations funnel through one shared pair of artifacts, the manifest and the lockfile. And the central fact a maintainer cares about — no known vulnerability — is a claim about the current state of external knowledge, which can only ever be as good as the advisory sources consulted and the moment they were consulted. Sometimes no fixed version exists yet, and the only correct move is to wait, alert, and act the moment one appears.

The deterministic baseline is strong: automated dependency bots open upgrade pull requests today. What they do not do is weigh upgrade risk against verification evidence, choose remediation scope when constraints couple dependencies, distinguish unknown from known-clean, or treat waiting on an unfixed advisory as a first-class state with a wake condition. The use case is justified only where that judgment layer earns its cost.

## Meld semantics conversion

The conversion is a contrasting design exploration alongside [CVE Freshness Strategy](../cognitive_architecture/world_model/strategy/cve_freshness.md). It is not implementation authority. In summary:

- Subjects are dependency declarations, resolved versions, advisories, advisory sources, the manifest, and the lockfile; relations are declares, constrains, resolves to, affects, and fixed in.
- The maintained condition is a scoped negative: no advisory admitted from the declared source set at its admitted revision affects any resolved version at or above the severity threshold, within currency policy, under passing verification.
- Advisory status is an observational dimension settled by acquisition: observation of external sources, not evaluation of produced bytes.
- Candidate scope may need to reflect dependency coupling. The minimal Strategy slice does not establish this behavior.

## Required semantics

For this use case to be usable on paper, the following must hold. Standing is given per entry.

1. **Settlement by acquisition.** Observationality must cover dimensions settled by observing an authoritative external source, not only by evaluating produced artifacts. This is a use-case requirement, not a current Strategy contract.
2. **Scoped negative obligations.** The system must express and evaluate claims of the form no admitted evidence of kind K affects subject S, with unassessed distinct from settled clean, and must not treat closed-world absence as truth. The shared-language negation semantics currently collapse indeterminate cases; this is named delta work in the ground map.
3. **External evaluator authority for feasibility.** A resolver must be declarable as the authority whose satisfiability verdicts are feasibility grounds, consumed as typed propositions. New declaration surface; fits the fit-verdict pattern.
4. **Candidate scoping over a shared artifact.** Curation and Strategy must be able to aggregate coupled divergences into one Goal or one candidate, and to keep uncoupled ones separate with serialization left to Execution. Aggregation is currently under-specified anywhere in the schema; this is an open finding.
5. **External evidence revision.** An unfixed advisory motivates later reconsideration when advisory evidence changes. The minimal Strategy slice establishes no durable mechanism for this behavior.
6. **Evidence non-substitution.** Verification outcomes, resolver verdicts, and advisory observations must remain typed and mutually non-substitutable. Required by domain ownership; implementation ground is not assessed here.

## Read with

- [Use Case Catalog](README.md)
- [CVE Freshness Strategy](../cognitive_architecture/world_model/strategy/cve_freshness.md)
- [Strategy Ground Map](../completed/world_model/strategy/ground_map.md)
