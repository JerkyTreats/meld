# Docs Style Policy

Date: 2026-03-01
Status: active

## Architecture And Workflow Diagrams

- Prefer Mermaid style diagrams for architecture and workflow visuals when a diagram improves clarity.
- Keep diagrams near the related plan or spec section so design intent and ownership boundaries remain explicit.
- Use concise labels that match domain terms used in code and design docs.

## Markdown Parentheses Rule

- Do not use literal `(` or `)` in Markdown prose.
- Parentheses are allowed only when required by Markdown formatting syntax such as `[label](/path)`.
- Parentheses are allowed inside inline code and fenced code blocks.

## Cognitive Architecture Canonicality Rule

- Files under `design/cognitive_architecture` describe authoritative design intent.
- Do not include implementation status, evidence dates, gap verdicts, readiness verdicts, migration state, first slice acceptance criteria, or open implementation questions in cognitive architecture files.
- State target ownership, boundaries, contracts, and invariants directly.
- Put gap analysis, implementation sequencing, readiness assessment, migration notes, and evidence under `design/plan` or `design/completed`.
- When compatibility matters, describe the canonical contract and compatibility rule without anchoring it to present implementation state.

## Code Comment Policy Reference

- Rust code commenting is governed by [Commenting Policy](commenting_policy.md).
- Keep Markdown doc style and Rust code comment style aligned on concise, high-signal explanation.
