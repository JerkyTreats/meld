//! Passive product activation loading and typed package construction.

mod contracts;
mod loader;
mod normalization;
mod packages;
mod preflight;
mod semantic;
mod service;

pub use contracts::{
    ActivationDiagnostic, ActivationDiagnostics, ActivationHash, ActivationLoadError,
    ActivationSource, ActivationTargetSelector, DocsFreshnessActivationConfig,
    DocsFreshnessActivationDocument, DocsFreshnessBeliefFamilyConfig,
    DocsFreshnessBranchScopeConfig, DocsFreshnessCurationRuleConfig, DocsFreshnessDirectiveConfig,
    DocsFreshnessExecutionConfig, DocsFreshnessPerspectiveConfig, DocsFreshnessPublicationConfig,
    DocsFreshnessRuntimeConfig, DocsFreshnessSeedAgentConfig, DocsFreshnessSubjectConfig,
    ExecutionActivationPreflight, PassiveActivationDescription, PreparedProductActivation,
    StrictComparatorConfig, StrictComparatorFactorConfig, StrictEvidenceSchemaConfig,
    StrictEvidenceSourceMapping, StrictPlannerProjectionConfig, ValidatedDocsFreshnessActivation,
    ValidatedProductActivationPreflight, DOCS_FRESHNESS_ACTIVATION_SCHEMA_VERSION,
    MAX_ACTIVATION_SOURCE_BYTES,
};
pub use meld_execution::activation::{
    ExecutionActivationSelection, ExecutionForcePolicy, ExecutionTargetKind,
    ExecutionTargetSelector,
};
pub use packages::{ProductActivationRuntimeInputs, RuntimeActivationInput};
pub use preflight::{
    preflight_execution_activation, preflight_validated_activation, ActivationPreflightError,
};
pub use semantic::{
    SemanticRuntimeSelection, SemanticRuntimeSelectionError,
    SEMANTIC_RUNTIME_SELECTION_SCHEMA_VERSION,
};
pub use service::{
    load_and_preflight_activation, load_and_prepare_activation, load_and_validate_activation,
};

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use super::*;

    const FIXTURE: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/runtime/docs_freshness_activation.toml"
    ));

    struct TestSource {
        root: tempfile::TempDir,
        relative_path: PathBuf,
    }

    impl TestSource {
        fn new(contents: &str) -> Self {
            let root = tempfile::tempdir().unwrap();
            let relative_path = PathBuf::from("activation.toml");
            fs::write(root.path().join(&relative_path), contents).unwrap();
            Self {
                root,
                relative_path,
            }
        }

        fn load(&self) -> Result<ValidatedDocsFreshnessActivation, ActivationLoadError> {
            load_and_validate_activation(self.root.path(), &self.relative_path)
        }

        fn absolute_path(&self) -> PathBuf {
            self.root.path().join(&self.relative_path)
        }
    }

    fn load_pair(
        first: &str,
        second: &str,
    ) -> (
        ValidatedDocsFreshnessActivation,
        ValidatedDocsFreshnessActivation,
    ) {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("first.toml"), first).unwrap();
        fs::write(root.path().join("second.toml"), second).unwrap();
        (
            load_and_validate_activation(root.path(), "first.toml").unwrap(),
            load_and_validate_activation(root.path(), "second.toml").unwrap(),
        )
    }

    #[test]
    fn checked_in_fixture_loads_into_owner_packages() {
        let activation = TestSource::new(FIXTURE).load().unwrap();

        assert_eq!(activation.document.schema_version, 1);
        assert_eq!(
            activation.runtime_inputs.runtime.activation_id,
            "docs_freshness"
        );
        assert_eq!(
            activation.runtime_inputs.world_model.directive.directive_id,
            "directive.docs_freshness"
        );
        assert_eq!(
            activation.runtime_inputs.execution.method_id,
            "refresh_docs_v1"
        );
        assert_eq!(activation.activation_hash.as_str().len(), 64);
    }

    #[test]
    fn strict_source_rejects_unknown_top_level_and_nested_fields() {
        let top_level = TestSource::new(&FIXTURE.replacen(
            "schema_version = 1",
            "schema_version = 1\nunknown = true",
            1,
        ));
        assert!(matches!(
            top_level.load(),
            Err(ActivationLoadError::Parse(_))
        ));

        let nested = TestSource::new(&FIXTURE.replace(
            "bootstrap_runtime_id = \"world_model.agent.bootstrap.docs_freshness\"",
            "bootstrap_runtime_id = \"world_model.agent.bootstrap.docs_freshness\"\nunknown = true",
        ));
        assert!(matches!(nested.load(), Err(ActivationLoadError::Parse(_))));
    }

    #[test]
    fn source_byte_limit_reads_exactly_one_extra_byte() {
        let exact_padding = MAX_ACTIVATION_SOURCE_BYTES - FIXTURE.len();
        let exact = format!("{FIXTURE}{}", " ".repeat(exact_padding));
        assert_eq!(exact.len(), MAX_ACTIVATION_SOURCE_BYTES);
        let source = TestSource::new(&exact);
        assert_eq!(
            source.load().unwrap().source.byte_count,
            MAX_ACTIVATION_SOURCE_BYTES
        );

        let oversized = TestSource::new(&format!("{exact} "));
        assert!(matches!(
            oversized.load(),
            Err(ActivationLoadError::SourceTooLarge)
        ));
    }

    #[test]
    fn formatting_and_toml_key_order_do_not_change_hash() {
        let reordered = FIXTURE
            .replace(
                "directive_id = \"directive.docs_freshness\"\ntext = \"curate docs freshness goals\"",
                "text = \"curate docs freshness goals\"\ndirective_id = \"directive.docs_freshness\"",
            )
            .replace("schema_version = 1", "# source formatting\nschema_version = 1");
        let (first, second) = load_pair(FIXTURE, &reordered);

        assert_ne!(first.source.byte_count, second.source.byte_count);
        assert_eq!(first.activation_hash, second.activation_hash);
        assert_eq!(first.runtime_inputs, second.runtime_inputs);
    }

    #[test]
    fn semantic_change_changes_hash() {
        let changed = FIXTURE.replace(
            "curate docs freshness goals",
            "curate current docs freshness goals",
        );
        let (first, second) = load_pair(FIXTURE, &changed);

        assert_ne!(first.activation_hash, second.activation_hash);
    }

    #[test]
    fn deployment_root_is_part_of_activation_identity() {
        let first = TestSource::new(FIXTURE).load().unwrap();
        let second = TestSource::new(FIXTURE).load().unwrap();

        assert_ne!(
            first.canonical_workspace_root,
            second.canonical_workspace_root
        );
        assert_ne!(first.activation_hash, second.activation_hash);
    }

    #[test]
    fn relative_and_absolute_source_paths_have_hash_parity() {
        let source = TestSource::new(FIXTURE);
        let relative = source.load().unwrap();
        let absolute =
            load_and_validate_activation(source.root.path(), source.absolute_path()).unwrap();

        assert_eq!(relative.activation_hash, absolute.activation_hash);
        assert_eq!(relative.runtime_inputs, absolute.runtime_inputs);
    }

    #[test]
    fn equivalent_target_selectors_have_hash_parity() {
        let equivalent = FIXTURE.replace("value = \".\"", "value = \"./\"");
        let (first, second) = load_pair(FIXTURE, &equivalent);

        assert_eq!(first.resolved_target, second.resolved_target);
        assert_eq!(first.activation_hash, second.activation_hash);
        assert_eq!(first.runtime_inputs, second.runtime_inputs);
    }

    #[test]
    fn filesystem_target_preserves_distinct_nfc_and_nfd_names() {
        let root = tempfile::tempdir().unwrap();
        let nfc = "caf\u{e9}";
        let nfd = "cafe\u{301}";
        fs::create_dir(root.path().join(nfc)).unwrap();
        fs::create_dir(root.path().join(nfd)).unwrap();
        fs::write(
            root.path().join("nfc.toml"),
            FIXTURE.replace("value = \".\"", &format!("value = \"{nfc}\"")),
        )
        .unwrap();
        fs::write(
            root.path().join("nfd.toml"),
            FIXTURE.replace("value = \".\"", &format!("value = \"{nfd}\"")),
        )
        .unwrap();

        let nfc_activation = load_and_validate_activation(root.path(), "nfc.toml").unwrap();
        let nfd_activation = load_and_validate_activation(root.path(), "nfd.toml").unwrap();

        assert_eq!(nfc_activation.resolved_target, root.path().join(nfc));
        assert_eq!(nfd_activation.resolved_target, root.path().join(nfd));
        assert_ne!(
            nfc_activation.activation_hash,
            nfd_activation.activation_hash
        );
    }

    #[test]
    fn schema_one_rejects_node_id_target_kind() {
        let source = TestSource::new(&FIXTURE.replace("kind = \"path\"", "kind = \"node_id\""));
        assert!(matches!(
            source.load(),
            Err(ActivationLoadError::Validation { field, .. })
                if field == "execution.target.kind"
        ));
    }

    #[test]
    fn runtime_id_order_is_canonical_and_duplicates_fail() {
        let reversed = FIXTURE.replace(
            "  \"world_model.graph_replay\",\n  \"world_model.agent.bootstrap.docs_freshness\",",
            "  \"world_model.agent.bootstrap.docs_freshness\",\n  \"world_model.graph_replay\",",
        );
        let (first, second) = load_pair(FIXTURE, &reversed);
        assert_eq!(first.activation_hash, second.activation_hash);
        assert_eq!(
            first.runtime_inputs.runtime.enabled_runtime_ids,
            vec![
                "world_model.agent.bootstrap.docs_freshness".to_string(),
                "world_model.graph_replay".to_string(),
            ]
        );

        let duplicate = FIXTURE.replace(
            "  \"world_model.graph_replay\",\n  \"world_model.agent.bootstrap.docs_freshness\",",
            "  \"world_model.graph_replay\",\n  \"world_model.graph_replay\",",
        );
        assert!(matches!(
            TestSource::new(&duplicate).load(),
            Err(ActivationLoadError::Validation { field, .. })
                if field == "runtime.enabled_runtime_ids"
        ));
    }

    #[test]
    fn source_metadata_never_enters_owner_packages() {
        let root = tempfile::tempdir().unwrap();
        let first_path = Path::new("first.toml");
        let second_path = Path::new("second.toml");
        fs::write(root.path().join(first_path), FIXTURE).unwrap();
        fs::write(
            root.path().join(second_path),
            format!("# different byte count\n{FIXTURE}"),
        )
        .unwrap();

        let first = load_and_validate_activation(root.path(), first_path).unwrap();
        let second = load_and_validate_activation(root.path(), second_path).unwrap();
        assert_ne!(first.source.canonical_path, second.source.canonical_path);
        assert_ne!(first.source.byte_count, second.source.byte_count);
        assert_eq!(first.runtime_inputs, second.runtime_inputs);

        let package_json = serde_json::to_string(&first.runtime_inputs).unwrap();
        assert!(!package_json.contains("first.toml"));
        assert!(!package_json.contains("source_bytes"));
    }

    #[test]
    fn unsupported_schema_and_non_docs_flywheel_fail_closed() {
        let unsupported =
            TestSource::new(&FIXTURE.replacen("schema_version = 1", "schema_version = 2", 1));
        assert!(matches!(
            unsupported.load(),
            Err(ActivationLoadError::Validation { field, .. }) if field == "schema_version"
        ));

        let wrong_key =
            TestSource::new(&FIXTURE.replace("flywheel.docs_freshness", "flywheel.other"));
        assert!(matches!(
            wrong_key.load(),
            Err(ActivationLoadError::Validation { field, .. }) if field == "flywheel"
        ));
    }
}
