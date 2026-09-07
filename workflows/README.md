# Workflow compatibility

Maintained Docs behavior is supplied by the [native Docs package](../theory/docs_freshness/README.md). `meld init` initializes context-generation agents and prompts. It no longer installs a Docs Workflow application, and Execution no longer selects an embedded task package by Workflow name.

User-supplied explicit Workflow profiles remain supported. A task package must be present in the profile's `packages` directory, or the configured package directory for a profile without a source path. Existing profiles and files are not deleted or overwritten by this migration, including with `meld init --force`.

The historical bundled Docs profile, package and prompt assets now live only in test fixtures. They characterize generic Workflow compatibility and are not a shipped application or a native reconciliation proof. Workflow gates, frame generation and frame-head publication remain separate from native Agent authorization and maintained-condition satisfaction.

Rust callers must remove uses of `initialize_workflows`, `DEFAULT_WORKFLOW_FILES`, `load_builtin_task_package_spec`, `load_builtin_task_package_spec_for_workflow`, and the `workflows` fields on `InitSummary` and `InitPreview`. Explicit package loading continues through `load_task_package_spec_for_workflow`.
