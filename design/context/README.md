# Context design (new)

Design for the current context and context-generation work. Original context specs live under `completed/design/context/`.

## Documents

- **llm_payload_spec.md** — Specification for what is sent to the LLM when generating context: current node content (file bytes or that agent's child context), prompt, optional response template; full response each time (no diff). Includes binary-file handling and response-template placement in agent metadata.

- **context_generate_by_path_spec.md** — Updates to `merkle context generate` for generation by path: path (file or directory), descendant missing-context check for directories, `--force` and `--recursive`, level-ordered recursive batches (deepest first), enqueued-only generation, subtree collection and level grouping, and error messaging.
