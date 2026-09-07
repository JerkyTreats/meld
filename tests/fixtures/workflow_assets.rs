//! Historical, explicitly installed inputs for generic Workflow compatibility tests.

pub const FILES: &[(&str, &str)] = &[
    (
        "docs_writer_thread_v1.yaml",
        include_str!("legacy_workflow/docs_writer_thread_v1.yaml"),
    ),
    (
        "prompts/docs_writer/evidence_gather.md",
        include_str!("legacy_workflow/prompts/docs_writer/evidence_gather.md"),
    ),
    (
        "prompts/docs_writer/readme_struct.md",
        include_str!("legacy_workflow/prompts/docs_writer/readme_struct.md"),
    ),
    (
        "prompts/docs_writer/style_refine.md",
        include_str!("legacy_workflow/prompts/docs_writer/style_refine.md"),
    ),
    (
        "prompts/docs_writer/verification.md",
        include_str!("legacy_workflow/prompts/docs_writer/verification.md"),
    ),
];
