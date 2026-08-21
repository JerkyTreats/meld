# `HDDL-Parser`

- Upstream: <https://github.com/koala-planner/HDDL-Parser>
- Research index: [`HTN Codebase Structure Report`](../../README.md)
- Snapshot: Rust, branch `master`, commit `cf0095b`

## Role

- `HDDL-Parser` is a tooling-first implementation for HDDL parsing, semantic checking, metadata extraction, JSON export, and language-server diagnostics.
- It is a modern parser and validator stack rather than a planner.

## Layout

- The main library root is `src/lib.rs`.
- Lexical analysis lives in `src/lexical_analyzer/`.
- Syntactic analysis and AST building live in `src/syntactic_analyzer/`.
- Semantic analysis, type checking, and task graph analysis live in `src/semantic_analyzer/`.
- Error and warning output types live in `src/output/`.
- Editor integration lives in `src/language_server/`.
- Two binaries live under `src/bin/`, one for the CLI analyzer and one for the language server.

## HTN Structure

- The architecture follows a compiler pipeline very explicitly.
- `HDDLAnalyzer` in `src/lib.rs` chains lexer, parser, semantic analyzer, and metadata extraction into a small public API.
- The semantic layer goes beyond syntax. It includes symbol table work, undefined element checks, type checking, recursion metadata, and task dependency graph analysis.
- The language server is not bolted on from the outside. It reuses the same analysis pipeline and output types.

## Design Considerations

- This repo treats editor feedback and diagnostics as part of the main product.
- The layered directory split makes each compiler phase easy to inspect in isolation.
- Tests are organized around both flawed inputs and IPC domains, which reinforces the idea that validation is a core workflow in HTN engineering.
- JSON export and metadata extraction show that parsed HTN models are intended for reuse by other tools, not only for immediate validation.

## Cross Repo Takeaways

- `HDDL-Parser` is the cleanest example of a modern validation and language tooling architecture in this set.
- It shares the parser-first philosophy of `pandaPIparser`, but with a stronger focus on diagnostics, editor integration, and explicit compiler phase boundaries.
- It underscores a major consensus pattern in newer HTN tooling: parsing and semantic validation are treated as standalone infrastructure.
