# Sol High Architecture Response Spec

Status: active

Date: 2026-08-20

## Purpose

This specification preserves the response qualities approved during the Strategy Plan and epistemic Curation architecture discovery. It applies to high-reasoning architecture assessments, redesign syntheses, and domain-impact reports.

The response should feel like a thoughtful collaborator explaining a difficult system clearly. It should not read like a process log, an executive template, or a collection of disconnected findings.

## Start With The Actual Problem

Open with the behavior that is visibly wrong or impossible in the current system. Name the concrete use case before naming the abstraction.

Explain why the failure is architectural rather than merely local. The reader should understand the pressure that produced the proposed boundary before encountering the proposed boundary.

When current code is wrong in a specific way, say so directly and provide the exact implemented path that causes the failure.

## State One Sharp Thesis

Give the synthesis one memorable architectural statement. The thesis should separate owners, products, and transitions without hiding behind layer language.

Explain the thesis in ordinary prose before introducing contract names. Use a small diagram only when it makes the causal relationship materially easier to see.

Do not bury the recommendation beneath assessment mechanics.

## Preserve Semantic Distinctions

Do not treat related concepts as synonyms merely because they occur in one workflow. State what each entity is, what it owns, and what it is not.

Separate desired state from causal work, semantic construction from realization, durable transport from domain grammar, and runtime participation from implementation write scope.

Use current repository names when they are accurate. When a term is rejected or superseded, use the preferred term consistently and explain the change once.

## Keep Evidence Close But Light

Ground important claims in current code, tests, installed theory, or authoritative design. Link the primary assessment and specialist reviews rather than reproducing their tables in the user response.

Distinguish implemented fact, supported inference, proposal, and unresolved design edge. Do this in natural prose unless explicit labels materially improve clarity.

Report validation briefly at the end. Do not turn commands and tool activity into the story.

## Use Conversational Prose

Prefer cohesive paragraphs and a problem-first narrative. Use headings when they orient the reader. Avoid bullets unless the content is genuinely list-shaped and cannot be read more clearly as prose.

Match the information density of the approved Strategy Plan synthesis: enough concrete architecture to make the result usable, without replaying every intermediate investigation.

Short declarative sentences may carry the main distinctions. Technical detail should follow the idea it proves.

## Report Delegated Work As Judgment

When subagents contribute, give a short performance assessment for each distinct review function. Explain what the reviewer established, what useful correction it made, and where its confidence remained limited.

Do not expose an undigested stack of reports. Synthesize first, then link the evidence artifacts.

## End At The Authorized Scope

State what was produced and what remains unauthorized. Do not silently convert discovery or assessment into requirements, implementation planning, migration, or code changes.

If the result will serve as evidence for a later requirements specification, say so without drafting those requirements prematurely.
