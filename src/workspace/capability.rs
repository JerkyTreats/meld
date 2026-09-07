//! Workspace capability publication and invocation.

use crate::capability::{
    ArtifactSchemaVersionRange, BindingSpec, BindingValueKind, CapabilityInvocationPayload,
    CapabilityInvocationResult, CapabilityInvoker, CapabilityTypeContract, EffectKind, EffectSpec,
    ExecutionClass, ExecutionContract, InputCardinality, InputSlotSpec, OutputSlotSpec,
    ScopeContract, SuppliedValueRef,
};
use crate::error::ApiError;
use crate::events::EventEnvelope;
use crate::execution::ExecutionRuntimeContext;
use crate::task::{ArtifactProducerRef, ArtifactRecord};
use crate::workspace::commands::resolve_workspace_node_id;
use crate::workspace::scan::{WorkspaceScanPolicy, WorkspaceScanRequest};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

const RESOLVE_CAPABILITY_TYPE_ID: &str = "workspace_resolve_node_id";
const SCAN_CAPABILITY_TYPE_ID: &str = "workspace_scan";
const CAPABILITY_VERSION: u32 = 1;
const ARTIFACT_SCHEMA_VERSION: u32 = 1;

/// Publishes and invokes the workspace target resolution capability.
#[derive(Debug, Clone, Default)]
pub struct WorkspaceResolveNodeIdCapability;

impl WorkspaceResolveNodeIdCapability {
    fn extract_selector(
        payload: &CapabilityInvocationPayload,
    ) -> Result<(Option<PathBuf>, Option<String>, String), ApiError> {
        let supplied = payload
            .supplied_inputs
            .iter()
            .find(|input| input.slot_id == "target_selector")
            .ok_or_else(|| {
                ApiError::ConfigError(format!(
                    "Capability invocation '{}' is missing 'target_selector'",
                    payload.invocation_id
                ))
            })?;
        let value = match &supplied.value {
            SuppliedValueRef::Artifact(artifact) => &artifact.content,
            SuppliedValueRef::StructuredValue(value) => value,
        };

        if let Some(path) = value.get("path").and_then(Value::as_str) {
            return Ok((Some(PathBuf::from(path)), None, "path".to_string()));
        }
        if let Some(node_id) = value.get("node_id").and_then(Value::as_str) {
            return Ok((None, Some(node_id.to_string()), "node_id".to_string()));
        }

        Err(ApiError::ConfigError(format!(
            "Capability invocation '{}' target_selector must contain 'path' or 'node_id'",
            payload.invocation_id
        )))
    }

    fn include_tombstoned(runtime_init: &crate::capability::CapabilityRuntimeInit) -> bool {
        runtime_init
            .binding_values
            .iter()
            .find(|binding| binding.binding_id == "include_tombstoned")
            .and_then(|binding| binding.value.as_bool())
            .unwrap_or(false)
    }

    fn artifact_id(invocation_id: &str, output_slot_id: &str) -> String {
        format!("{invocation_id}::{output_slot_id}")
    }
}

#[async_trait]
impl CapabilityInvoker for WorkspaceResolveNodeIdCapability {
    type Error = ApiError;
    type ExecutionApi = dyn ExecutionRuntimeContext;

    fn contract(&self) -> CapabilityTypeContract {
        CapabilityTypeContract {
            capability_type_id: RESOLVE_CAPABILITY_TYPE_ID.to_string(),
            capability_version: CAPABILITY_VERSION,
            owning_domain: "workspace".to_string(),
            scope_contract: ScopeContract {
                scope_kind: "workspace".to_string(),
                scope_ref_kind: "workspace_root".to_string(),
                allow_fan_out: false,
            },
            binding_contract: vec![BindingSpec {
                binding_id: "include_tombstoned".to_string(),
                value_kind: BindingValueKind::Literal,
                required: false,
                affects_deterministic_identity: true,
            }],
            input_contract: vec![InputSlotSpec {
                slot_id: "target_selector".to_string(),
                accepted_artifact_type_ids: vec!["target_selector".to_string()],
                schema_versions: ArtifactSchemaVersionRange {
                    min: ARTIFACT_SCHEMA_VERSION,
                    max: ARTIFACT_SCHEMA_VERSION,
                },
                required: true,
                cardinality: InputCardinality::One,
            }],
            output_contract: vec![
                OutputSlotSpec {
                    slot_id: "resolved_node_ref".to_string(),
                    artifact_type_id: "resolved_node_ref".to_string(),
                    schema_version: ARTIFACT_SCHEMA_VERSION,
                    guaranteed: true,
                },
                OutputSlotSpec {
                    slot_id: "target_resolution_summary".to_string(),
                    artifact_type_id: "target_resolution_summary".to_string(),
                    schema_version: ARTIFACT_SCHEMA_VERSION,
                    guaranteed: true,
                },
            ],
            effect_contract: vec![EffectSpec {
                effect_id: "read_workspace_tree".to_string(),
                kind: EffectKind::Read,
                target: "workspace_tree".to_string(),
                exclusive: false,
            }],
            execution_contract: ExecutionContract {
                execution_class: ExecutionClass::Inline,
                completion_semantics: "artifacts".to_string(),
                retry_class: "none".to_string(),
                cancellation_supported: false,
            },
        }
    }

    async fn invoke(
        &self,
        api: &dyn ExecutionRuntimeContext,
        runtime_init: &crate::capability::CapabilityRuntimeInit,
        payload: &CapabilityInvocationPayload,
        _event_context: Option<&crate::execution::ExecutionEventContext>,
    ) -> Result<CapabilityInvocationResult, ApiError> {
        payload.validate_against(runtime_init)?;

        let workspace_root = api.workspace_root().ok_or_else(|| {
            ApiError::ConfigError(
                "Workspace resolve capability requires workspace root context".to_string(),
            )
        })?;
        let include_tombstoned = Self::include_tombstoned(runtime_init);
        let (path, node_hex, selector_kind) = Self::extract_selector(payload)?;
        let node_id = resolve_workspace_node_id(
            api,
            workspace_root,
            path.as_deref(),
            node_hex.as_deref(),
            include_tombstoned,
        )?;
        let record = api
            .read_node_record(&node_id)?
            .ok_or(ApiError::NodeNotFound(node_id))?;

        let producer = ArtifactProducerRef {
            task_id: payload
                .upstream_lineage
                .as_ref()
                .map(|lineage| lineage.task_id.clone())
                .unwrap_or_default(),
            capability_instance_id: runtime_init.capability_instance_id.clone(),
            invocation_id: Some(payload.invocation_id.clone()),
            output_slot_id: None,
        };

        Ok(CapabilityInvocationResult {
            emitted_artifacts: vec![
                ArtifactRecord {
                    artifact_id: Self::artifact_id(&payload.invocation_id, "resolved_node_ref"),
                    artifact_type_id: "resolved_node_ref".to_string(),
                    schema_version: ARTIFACT_SCHEMA_VERSION,
                    content: json!({
                        "node_id": hex::encode(node_id),
                        "path": record.path.to_string_lossy(),
                        "include_tombstoned": include_tombstoned,
                    }),
                    producer: ArtifactProducerRef {
                        output_slot_id: Some("resolved_node_ref".to_string()),
                        ..producer.clone()
                    },
                },
                ArtifactRecord {
                    artifact_id: Self::artifact_id(
                        &payload.invocation_id,
                        "target_resolution_summary",
                    ),
                    artifact_type_id: "target_resolution_summary".to_string(),
                    schema_version: ARTIFACT_SCHEMA_VERSION,
                    content: json!({
                        "selector_kind": selector_kind,
                        "lookup_mode": "workspace_path_then_canonical_fallback",
                        "resolved": true,
                    }),
                    producer: ArtifactProducerRef {
                        output_slot_id: Some("target_resolution_summary".to_string()),
                        ..producer
                    },
                },
            ],
        })
    }
}

/// Runs one workspace scan as a typed task step.
///
/// The capability updates the workspace tree through the workspace scan port
/// and reports scan facts as artifacts. It never appends canonical events:
/// publication candidates travel in the outcome for the owning publication
/// runtime to append.
#[derive(Debug, Clone, Default)]
pub struct WorkspaceScanCapability;

/// Selector parsed from the optional `target_selector` input slot.
enum ScanTargetSelector {
    /// Workspace path selector.
    Path(PathBuf),
    /// Hex-encoded node id selector.
    NodeId(String),
}

impl WorkspaceScanCapability {
    fn artifact_id(invocation_id: &str, output_slot_id: &str) -> String {
        format!("{invocation_id}::{output_slot_id}")
    }

    fn scan_policy(
        runtime_init: &crate::capability::CapabilityRuntimeInit,
    ) -> Result<WorkspaceScanPolicy, ApiError> {
        let Some(binding) = runtime_init
            .binding_values
            .iter()
            .find(|binding| binding.binding_id == "scan_policy")
        else {
            return Ok(WorkspaceScanPolicy::default());
        };
        serde_json::from_value(binding.value.clone()).map_err(|err| {
            ApiError::ConfigError(format!(
                "Capability '{}' received invalid scan_policy binding: {}",
                SCAN_CAPABILITY_TYPE_ID, err
            ))
        })
    }

    /// Session shaping publication candidates. Sourced only from the durable
    /// authored `session_id` binding: the ambient event context must never
    /// gate candidate production, because the binding is declared
    /// `affects_deterministic_identity: false` and the artifact set has to
    /// stay a pure function of the durable inputs.
    fn session_id(runtime_init: &crate::capability::CapabilityRuntimeInit) -> Option<String> {
        runtime_init
            .binding_values
            .iter()
            .find(|binding| binding.binding_id == "session_id")
            .and_then(|binding| binding.value.as_str())
            .map(ToString::to_string)
    }

    fn parse_target_selector(
        payload: &CapabilityInvocationPayload,
    ) -> Result<Option<ScanTargetSelector>, ApiError> {
        let Some(input) = payload
            .supplied_inputs
            .iter()
            .find(|input| input.slot_id == "target_selector")
        else {
            return Ok(None);
        };
        let value = match &input.value {
            SuppliedValueRef::Artifact(artifact) => &artifact.content,
            SuppliedValueRef::StructuredValue(value) => value,
        };
        if let Some(path) = value.get("path").and_then(Value::as_str) {
            return Ok(Some(ScanTargetSelector::Path(PathBuf::from(path))));
        }
        if let Some(node_id) = value.get("node_id").and_then(Value::as_str) {
            return Ok(Some(ScanTargetSelector::NodeId(node_id.to_string())));
        }

        Err(ApiError::ConfigError(format!(
            "Capability invocation '{}' target_selector must contain 'path' or 'node_id'",
            payload.invocation_id
        )))
    }

    /// Projects one publication-candidate envelope to its time-free fields.
    /// The artifact content must be deterministic for a fixed tree, so
    /// append-time fields (`ts`, `recorded_at`, `session`) are left for the
    /// owning publication runtime to stamp when it appends the canonical
    /// event.
    fn candidate_descriptor(envelope: &EventEnvelope) -> Result<Value, serde_json::Error> {
        Ok(json!({
            "type": envelope.event_type,
            "domain_id": envelope.domain_id,
            "stream_id": envelope.stream_id,
            "objects": serde_json::to_value(&envelope.objects)?,
            "relations": serde_json::to_value(&envelope.relations)?,
            "data": envelope.data,
        }))
    }
}

#[async_trait]
impl CapabilityInvoker for WorkspaceScanCapability {
    type Error = ApiError;
    type ExecutionApi = dyn ExecutionRuntimeContext;

    fn contract(&self) -> CapabilityTypeContract {
        CapabilityTypeContract {
            capability_type_id: SCAN_CAPABILITY_TYPE_ID.to_string(),
            capability_version: CAPABILITY_VERSION,
            owning_domain: "workspace".to_string(),
            scope_contract: ScopeContract {
                scope_kind: "workspace".to_string(),
                scope_ref_kind: "workspace_root".to_string(),
                allow_fan_out: false,
            },
            binding_contract: vec![
                // The policy binds as an inline structured value decoded by
                // `scan_policy`, not a reference resolved through a policy
                // registry, so the published kind is Literal.
                BindingSpec {
                    binding_id: "scan_policy".to_string(),
                    value_kind: BindingValueKind::Literal,
                    required: false,
                    affects_deterministic_identity: true,
                },
                BindingSpec {
                    binding_id: "session_id".to_string(),
                    value_kind: BindingValueKind::Literal,
                    required: false,
                    affects_deterministic_identity: false,
                },
            ],
            input_contract: vec![InputSlotSpec {
                slot_id: "target_selector".to_string(),
                accepted_artifact_type_ids: vec!["target_selector".to_string()],
                schema_versions: ArtifactSchemaVersionRange {
                    min: ARTIFACT_SCHEMA_VERSION,
                    max: ARTIFACT_SCHEMA_VERSION,
                },
                required: false,
                cardinality: InputCardinality::One,
            }],
            output_contract: vec![
                OutputSlotSpec {
                    slot_id: "workspace_scan_summary".to_string(),
                    artifact_type_id: "workspace_scan_summary".to_string(),
                    schema_version: ARTIFACT_SCHEMA_VERSION,
                    guaranteed: true,
                },
                OutputSlotSpec {
                    slot_id: "workspace_snapshot_ref".to_string(),
                    artifact_type_id: "workspace_snapshot_ref".to_string(),
                    schema_version: ARTIFACT_SCHEMA_VERSION,
                    guaranteed: true,
                },
                OutputSlotSpec {
                    slot_id: "root_node_ref".to_string(),
                    artifact_type_id: "resolved_node_ref".to_string(),
                    schema_version: ARTIFACT_SCHEMA_VERSION,
                    guaranteed: true,
                },
                OutputSlotSpec {
                    slot_id: "observed_node_refs".to_string(),
                    artifact_type_id: "workspace_observed_node_refs".to_string(),
                    schema_version: ARTIFACT_SCHEMA_VERSION,
                    guaranteed: true,
                },
                OutputSlotSpec {
                    slot_id: "publication_candidates".to_string(),
                    artifact_type_id: "workspace_event_candidates".to_string(),
                    schema_version: ARTIFACT_SCHEMA_VERSION,
                    guaranteed: false,
                },
            ],
            effect_contract: vec![
                EffectSpec {
                    effect_id: "read_filesystem".to_string(),
                    kind: EffectKind::Read,
                    target: "workspace_filesystem".to_string(),
                    exclusive: false,
                },
                EffectSpec {
                    effect_id: "update_workspace_tree".to_string(),
                    kind: EffectKind::Write,
                    target: "workspace_tree".to_string(),
                    exclusive: true,
                },
                // Scan execution conditionally rewrites the workspace ignore
                // policy when the tree's .gitignore hash changed.
                EffectSpec {
                    effect_id: "sync_ignore_policy".to_string(),
                    kind: EffectKind::Write,
                    target: "workspace_ignore_policy".to_string(),
                    exclusive: false,
                },
                EffectSpec {
                    effect_id: "produce_publication_candidates".to_string(),
                    kind: EffectKind::Emit,
                    target: "event_publication".to_string(),
                    exclusive: false,
                },
            ],
            execution_contract: ExecutionContract {
                execution_class: ExecutionClass::Inline,
                completion_semantics: "artifacts".to_string(),
                retry_class: "workspace_io".to_string(),
                cancellation_supported: false,
            },
        }
    }

    async fn invoke(
        &self,
        api: &dyn ExecutionRuntimeContext,
        runtime_init: &crate::capability::CapabilityRuntimeInit,
        payload: &CapabilityInvocationPayload,
        _event_context: Option<&crate::execution::ExecutionEventContext>,
    ) -> Result<CapabilityInvocationResult, ApiError> {
        payload.validate_against(runtime_init)?;

        let workspace_root = api.workspace_root().ok_or_else(|| {
            ApiError::ConfigError(
                "Workspace scan capability requires workspace root context".to_string(),
            )
        })?;
        let request = WorkspaceScanRequest {
            workspace_root: workspace_root.to_path_buf(),
            policy: Self::scan_policy(runtime_init)?,
            session_id: Self::session_id(runtime_init),
            collect_observed: true,
        };
        let selector = Self::parse_target_selector(payload)?;
        let mut outcome = api.scan_workspace(&request)?;

        // Selector resolution runs after the scan so a first scan can target
        // nodes it just observed.
        let target_path = match &selector {
            Some(selector) => {
                let (path, node_hex) = match selector {
                    ScanTargetSelector::Path(path) => (Some(path.as_path()), None),
                    ScanTargetSelector::NodeId(node_id) => (None, Some(node_id.as_str())),
                };
                let node_id =
                    resolve_workspace_node_id(api, workspace_root, path, node_hex, false)?;
                let record = api
                    .read_node_record(&node_id)?
                    .ok_or(ApiError::NodeNotFound(node_id))?;
                Some(record.path.to_string_lossy().to_string())
            }
            None => None,
        };
        let observed_nodes: Vec<_> = match &target_path {
            Some(target_path) => std::mem::take(&mut outcome.observed_nodes)
                .into_iter()
                .filter(|node| Path::new(&node.path).starts_with(target_path))
                .collect(),
            None => std::mem::take(&mut outcome.observed_nodes),
        };

        let encode_error = |context: &str, err: serde_json::Error| {
            ApiError::ConfigError(format!(
                "Capability invocation '{}' failed to encode {}: {}",
                payload.invocation_id, context, err
            ))
        };
        let producer = ArtifactProducerRef {
            task_id: payload
                .upstream_lineage
                .as_ref()
                .map(|lineage| lineage.task_id.clone())
                .unwrap_or_default(),
            capability_instance_id: runtime_init.capability_instance_id.clone(),
            invocation_id: Some(payload.invocation_id.clone()),
            output_slot_id: None,
        };

        let mut emitted_artifacts = vec![
            ArtifactRecord {
                artifact_id: Self::artifact_id(&payload.invocation_id, "workspace_scan_summary"),
                artifact_type_id: "workspace_scan_summary".to_string(),
                schema_version: ARTIFACT_SCHEMA_VERSION,
                content: serde_json::to_value(&outcome.summary)
                    .map_err(|err| encode_error("scan summary", err))?,
                producer: ArtifactProducerRef {
                    output_slot_id: Some("workspace_scan_summary".to_string()),
                    ..producer.clone()
                },
            },
            ArtifactRecord {
                artifact_id: Self::artifact_id(&payload.invocation_id, "workspace_snapshot_ref"),
                artifact_type_id: "workspace_snapshot_ref".to_string(),
                schema_version: ARTIFACT_SCHEMA_VERSION,
                content: serde_json::to_value(&outcome.snapshot_ref)
                    .map_err(|err| encode_error("snapshot ref", err))?,
                producer: ArtifactProducerRef {
                    output_slot_id: Some("workspace_snapshot_ref".to_string()),
                    ..producer.clone()
                },
            },
            ArtifactRecord {
                artifact_id: Self::artifact_id(&payload.invocation_id, "root_node_ref"),
                artifact_type_id: "resolved_node_ref".to_string(),
                schema_version: ARTIFACT_SCHEMA_VERSION,
                content: json!({
                    "node_id": outcome.root_node_ref.node_id,
                    "path": outcome.root_node_ref.path,
                }),
                producer: ArtifactProducerRef {
                    output_slot_id: Some("root_node_ref".to_string()),
                    ..producer.clone()
                },
            },
            ArtifactRecord {
                artifact_id: Self::artifact_id(&payload.invocation_id, "observed_node_refs"),
                artifact_type_id: "workspace_observed_node_refs".to_string(),
                schema_version: ARTIFACT_SCHEMA_VERSION,
                content: json!({
                    "target_path": target_path,
                    "node_count": observed_nodes.len(),
                    "nodes": observed_nodes,
                }),
                producer: ArtifactProducerRef {
                    output_slot_id: Some("observed_node_refs".to_string()),
                    ..producer.clone()
                },
            },
        ];

        if !outcome.publication_candidates.is_empty() {
            let candidates: Vec<Value> = outcome
                .publication_candidates
                .iter()
                .map(Self::candidate_descriptor)
                .collect::<Result<_, _>>()
                .map_err(|err| encode_error("publication candidates", err))?;
            emitted_artifacts.push(ArtifactRecord {
                artifact_id: Self::artifact_id(&payload.invocation_id, "publication_candidates"),
                artifact_type_id: "workspace_event_candidates".to_string(),
                schema_version: ARTIFACT_SCHEMA_VERSION,
                content: json!({
                    "candidate_count": candidates.len(),
                    "candidates": candidates,
                }),
                producer: ArtifactProducerRef {
                    output_slot_id: Some("publication_candidates".to_string()),
                    ..producer
                },
            });
        }

        Ok(CapabilityInvocationResult { emitted_artifacts })
    }
}
