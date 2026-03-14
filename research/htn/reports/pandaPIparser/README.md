# `pandaPIparser`

- Upstream: <https://github.com/panda-planner-dev/pandaPIparser>
- Research index: [`HTN Codebase Structure Report`](../../README.md)
- Snapshot: C plus plus, branch `master`, commit `88c0995`

## Role

- `pandaPIparser` is a front-end compiler for hierarchical planning models.
- It parses HDDL into internal data structures and can emit several target formats for downstream planners and analysis tools.

## Layout

- Parser and transformation code lives in `src/`.
- Domain and intermediate representation types live in `src/domain.hpp`, `src/parsetree.hpp`, and related implementation files.
- Static analysis and normalization helpers live in files such as `src/cwa.*`, `src/properties.*`, `src/sortexpansion.*`, and `src/typeof.*`.
- Output backends live in writers such as `src/hddlWriter.*`, `src/hpdlWriter.*`, `src/shopWriter.*`, and `src/htn2stripsWriter.*`.
- Verification-specific output logic lives in `src/verification_encoding.*` and `src/verify.*`.

## HTN Structure

- The internal organization looks like a compiler pipeline built around a reusable IR.
- Parsing is only one phase. The repo also normalizes sorts, flattens tasks, compiles away some constructs, and emits alternate surface languages.
- Output targets include HDDL, HPDL, SHOP style output, and STRIPS-oriented encodings, which makes the parser a bridge among HTN ecosystems rather than a parser for one solver only.
- Verification support in the same repo shows that the front end is also responsible for building analysis-friendly encodings.

## Design Considerations

- This codebase treats input language work as a standalone concern. The parser is not embedded inside the search engine.
- The many writer modules suggest that the core abstraction is the internal model, not the original HDDL text.
- Several downstream systems can share one front end, which reduces duplication in syntax and preprocessing logic.
- The structure is well suited for research workflows where a single domain model must be converted into several execution or validation substrates.

## Cross Repo Takeaways

- `pandaPIparser` is one of the clearest parser-first repos in the set.
- Its file layout strongly resembles the parser half of `thtn`, which suggests a repeatable HTN compiler architecture of parse tree, domain IR, static analysis, then writers.
- It also helps explain why other PANDA tools consume grounded or normalized models rather than raw HDDL text.
