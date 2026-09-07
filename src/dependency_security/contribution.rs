//! Exact Dependency Security resources selected during inert preparation.

use super::capability::*;
use super::contracts::*;
use super::inventory::cargo::CargoInventoryLimits;
use super::theory::{DependencySecurityPolicyRegistry, DependencySecurityPolicyRevision};
use crate::capability::*;
use meld_events::DomainObjectRef;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::Arc;

const POLICY: &str = "dependency-security.policy";
pub const CARGO: &str = "dependency-security.cargo";
pub const ADVISORY_SOURCE: &str = "dependency-security.advisories";
pub const LIMITS: &str = "dependency-security.limits";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PolicyBinding {
    revision: DependencySecurityPolicyRevision,
    subject: DomainObjectRef,
}

/// Resolve only the immutable policy selected by this complete product.
pub fn bind_selected_policy(
    bindings: OwnerBindingView,
    registry: &DependencySecurityPolicyRegistry,
    references: &[crate::theory::TheoryRevisionRef],
    subject: &DomainObjectRef,
) -> Result<OwnerBindingView, String> {
    if bindings.contains(POLICY) {
        return Err("Security policy must be issued from the selected owner revision, not supplied as a physical binding".into());
    }
    let selected: Vec<_> = references
        .iter()
        .filter(|reference| reference.registry == "dependency_security_policy")
        .collect();
    let reference = match selected.as_slice() {
        [] => return Ok(bindings),
        [reference] => *reference,
        _ => return Err("Dependency Security requires an unambiguous selected policy".into()),
    };
    let revision = registry
        .resolve(reference)?
        .ok_or("selected Security policy is absent")?;
    if revision.reference != revision.policy.revision_ref()? {
        return Err("selected Security policy is corrupt".into());
    }
    subject.validate().map_err(|error| error.to_string())?;
    let body = serde_json::to_string(&PolicyBinding {
        revision,
        subject: subject.clone(),
    })
    .map_err(|error| error.to_string())?;
    Ok(bindings.with_value(POLICY, body))
}

pub struct DependencySecurityCapabilityContributor;
impl ProductCapabilityContributor for DependencySecurityCapabilityContributor {
    fn owner_domain(&self) -> &str {
        "dependency-security"
    }
    fn published_contracts(&self) -> Vec<meld_execution::capability::CapabilityContractRevision> {
        super::capability::published_contracts()
            .into_iter()
            .map(
                |contract| meld_execution::capability::CapabilityContractRevision {
                    content_identity: contract.content_identity(),
                    contract,
                    installed_at_seq: 0,
                },
            )
            .collect()
    }
    fn implementation_offers(&self) -> Vec<CapabilityImplementationOffer> {
        self.published_contracts()
            .into_iter()
            .map(|revision| {
                let id = revision.contract.capability_type_id.clone();
                let mut required_binding_ids = BTreeSet::from([POLICY.into()]);
                if id == OBSERVE_INVENTORY {
                    required_binding_ids.extend(["workspace".into(), CARGO.into()]);
                }
                if id == ACQUIRE_ADVISORIES {
                    required_binding_ids.insert(ADVISORY_SOURCE.into());
                }
                CapabilityImplementationOffer {
                    contract_ref: revision.revision_ref(),
                    implementation_ref: format!("dependency-security.local-source.v1::{id}"),
                    required_binding_ids,
                    execution_class: ExecutionClass::Inline,
                    factory: Arc::new(Factory { id }),
                }
            })
            .collect()
    }
}

struct Factory {
    id: String,
}
impl CapabilityInvokerFactory for Factory {
    fn prepare(
        &self,
        _request: &CapabilityFactoryRequest<'_>,
        bindings: &OwnerBindingView,
    ) -> Result<PreparedCapabilityInvoker, CapabilityContributionDiagnostic> {
        let policy: PolicyBinding = serde_json::from_str(
            bindings
                .get(POLICY)
                .ok_or_else(|| diagnostic("selected Security policy is absent"))?,
        )
        .map_err(|error| diagnostic(error.to_string()))?;
        if policy.revision.reference != policy.revision.policy.revision_ref().map_err(diagnostic)? {
            return Err(diagnostic(
                "Security policy content differs from its exact reference",
            ));
        }
        policy
            .subject
            .validate()
            .map_err(|error| diagnostic(error.to_string()))?;
        let limits: CargoInventoryLimits = bindings
            .get(LIMITS)
            .map(serde_json::from_str)
            .transpose()
            .map_err(|error| diagnostic(error.to_string()))?
            .unwrap_or_default();
        limits.validate().map_err(diagnostic)?;
        let workspace = if self.id == OBSERVE_INVENTORY {
            Some(path(bindings, "workspace")?)
        } else {
            None
        };
        let cargo = if self.id == OBSERVE_INVENTORY {
            Some(path(bindings, CARGO)?)
        } else {
            None
        };
        let advisories = if self.id == ACQUIRE_ADVISORIES {
            Some(path(bindings, ADVISORY_SOURCE)?)
        } else {
            None
        };
        // These identify Security-owned captures, not fabricated workspace objects.
        let subject_id = content_hash(&policy.subject).map_err(diagnostic)?;
        let subject = DependencySecuritySubjectV1 {
            subject: policy.subject.clone(),
            ecosystem: PackageEcosystem::Cargo,
            inventory_scope: InventoryScopeV1 {
                manifest_ref: DomainObjectRef::new(
                    "dependency-security",
                    "manifest_source",
                    format!("{subject_id}::Cargo.toml"),
                )
                .map_err(|error| diagnostic(error.to_string()))?,
                lockfile_ref: DomainObjectRef::new(
                    "dependency-security",
                    "lockfile_source",
                    format!("{subject_id}::Cargo.lock"),
                )
                .map_err(|error| diagnostic(error.to_string()))?,
                include_transitive: true,
            },
        };
        Ok(Arc::new(SecurityCapability {
            id: self.id.clone(),
            subject,
            policy: policy.revision.policy,
            workspace,
            cargo,
            advisories,
            limits,
        }))
    }
}

fn path(
    bindings: &OwnerBindingView,
    id: &str,
) -> Result<PathBuf, CapabilityContributionDiagnostic> {
    let path = PathBuf::from(
        bindings
            .get(id)
            .ok_or_else(|| diagnostic(format!("missing resource '{id}'")))?,
    );
    if !path.is_absolute() {
        return Err(diagnostic(format!(
            "resource '{id}' requires an absolute path"
        )));
    }
    Ok(path)
}
fn diagnostic(message: impl Into<String>) -> CapabilityContributionDiagnostic {
    CapabilityContributionDiagnostic::new("security_binding_invalid", message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::ArtifactRecord;
    use std::collections::BTreeMap;

    fn api(root: &std::path::Path) -> crate::api::ContextApi {
        crate::api::ContextApi::new(
            Arc::new(crate::store::SledNodeRecordStore::new(root.join("nodes")).unwrap()),
            Arc::new(crate::context::frame::FrameStorage::new(root.join("frames")).unwrap()),
            Arc::new(parking_lot::RwLock::new(crate::heads::HeadIndex::new())),
            Arc::new(
                crate::prompt_context::PromptContextArtifactStorage::new(root.join("prompts"))
                    .unwrap(),
            ),
            Arc::new(parking_lot::RwLock::new(crate::agent::AgentRegistry::new())),
            Arc::new(parking_lot::RwLock::new(
                crate::provider::ProviderRegistry::new(),
            )),
            Arc::new(crate::concurrency::NodeLockManager::new()),
        )
    }

    async fn invoke(
        closure: &PreparedCapabilityClosure,
        api: &crate::api::ContextApi,
        id: &str,
        artifacts: &BTreeMap<String, ArtifactRecord>,
    ) -> ArtifactRecord {
        let invoker = closure.invokers.get(id, 1).unwrap();
        let contract = invoker.contract();
        let payload = CapabilityInvocationPayload {
            invocation_id: {
                static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
                format!(
                    "{id}::{}",
                    NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
                )
            },
            capability_instance_id: id.into(),
            supplied_inputs: contract
                .input_contract
                .iter()
                .map(|slot| SuppliedInputValue {
                    slot_id: slot.slot_id.clone(),
                    source: InputValueSource::ArtifactHandoff,
                    value: SuppliedValueRef::Artifact(ArtifactValueRef {
                        artifact_id: artifacts[&slot.slot_id].artifact_id.clone(),
                        artifact_type_id: artifacts[&slot.slot_id].artifact_type_id.clone(),
                        schema_version: artifacts[&slot.slot_id].schema_version,
                        content: artifacts[&slot.slot_id].content.clone(),
                    }),
                })
                .collect(),
            upstream_lineage: None,
            execution_context: CapabilityExecutionContext::default(),
        };
        let runtime = CapabilityRuntimeInit {
            capability_instance_id: id.into(),
            capability_type_id: id.into(),
            capability_version: 1,
            scope_ref: "repo".into(),
            scope_kind: contract.scope_contract.scope_kind,
            binding_values: Vec::new(),
            input_contract: contract.input_contract,
            output_contract: contract.output_contract,
            effect_contract: contract.effect_contract,
            execution_contract: contract.execution_contract,
        };
        let result = invoker.invoke(api, &runtime, &payload, None).await.unwrap();
        assert_eq!(result.emitted_artifacts.len(), 1);
        result.emitted_artifacts.into_iter().next().unwrap()
    }

    #[tokio::test]
    async fn prepared_actions_return_real_inventory_assessment_and_separate_verification() {
        let workspace = tempfile::tempdir().unwrap();
        std::fs::create_dir(workspace.path().join("src")).unwrap();
        std::fs::write(workspace.path().join("src/lib.rs"), "pub fn value() {}\n").unwrap();
        std::fs::write(
            workspace.path().join("Cargo.toml"),
            "[package]\nname = \"security-proof\"\nversion = \"1.0.0\"\nedition = \"2021\"\n",
        )
        .unwrap();
        let output = std::process::Command::new(env!("CARGO"))
            .args(["generate-lockfile", "--offline"])
            .current_dir(workspace.path())
            .output()
            .unwrap();
        assert!(output.status.success());
        let external = tempfile::tempdir().unwrap();
        let advisory_path = external.path().join("advisories.json");
        let registry = DependencySecurityPolicyRegistry::new(
            sled::Config::new().temporary(true).open().unwrap(),
        )
        .unwrap();
        let policy = serde_json::from_str(include_str!(
            "../../theory/dependency_security/policy.cargo_fixture.json"
        ))
        .unwrap();
        let reference = registry.install(policy, 1).unwrap();
        let bindings = OwnerBindingView::new(BTreeMap::from([
            ("workspace".into(), workspace.path().display().to_string()),
            (CARGO.into(), env!("CARGO").into()),
            (ADVISORY_SOURCE.into(), advisory_path.display().to_string()),
        ]));
        let inventory = ProductCapabilityInventory::assemble(vec![Arc::new(
            DependencySecurityCapabilityContributor,
        )])
        .unwrap();
        let contracts: Vec<_> = inventory
            .contracts()
            .map(|revision| revision.revision_ref())
            .collect();
        let request = ExactCapabilityActivationRequest {
            assignment_id: "security-assignment".into(),
            activation_id: "security-activation".into(),
            selected_implementations: contracts
                .iter()
                .map(|reference| {
                    (
                        reference.clone(),
                        inventory.unique_implementation_ref(reference).unwrap(),
                    )
                })
                .collect(),
            selected_contracts: contracts,
            compatibility_policy_revision: "capability-compatibility.v1".into(),
        };
        assert!(inventory.prepare(request.clone(), &bindings).is_err());
        assert!(bind_selected_policy(
            bindings.clone().with_value(POLICY, "unselected-policy"),
            &registry,
            std::slice::from_ref(&reference),
            &DomainObjectRef::new("workspace_fs", "node", "repo").unwrap(),
        )
        .is_err());
        let bindings = bind_selected_policy(
            bindings,
            &registry,
            &[reference],
            &DomainObjectRef::new("workspace_fs", "node", "repo").unwrap(),
        )
        .unwrap();
        let closure = inventory.prepare(request, &bindings).unwrap();
        assert!(
            !advisory_path.exists(),
            "preparation must not manufacture source evidence"
        );
        let api = api(external.path());
        let mut artifacts = BTreeMap::new();
        let artifact = invoke(&closure, &api, OBSERVE_INVENTORY, &artifacts).await;
        let observed: DependencyInventorySnapshotV1 =
            serde_json::from_value(artifact.content.clone()).unwrap();
        assert_eq!(observed.components.len(), 1);
        assert_eq!(
            observed.completeness,
            InventoryCompleteness::CompleteTransitive
        );
        artifacts.insert(artifact.artifact_type_id.clone(), artifact);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let coverage = observed
            .components
            .iter()
            .map(|component| ComponentCoverageV1 {
                package_name: component.package_name.clone(),
                source_identity: component.source_identity.clone(),
            })
            .collect::<Vec<_>>();
        let advisory = NormalizedAdvisoryV1 {
            source_advisory_id: "proof-advisory".into(),
            aliases: Vec::new(),
            package_name: "security-proof".into(),
            affected_versions: vec!["1.0.0".into()],
            severity: SeverityV1::High,
        };
        for (revision, covered, findings, expected) in [
            (
                "violated",
                coverage.clone(),
                vec![advisory],
                DependencySecurityPosture::Violated,
            ),
            (
                "incomplete",
                Vec::new(),
                Vec::new(),
                DependencySecurityPosture::Insufficient,
            ),
            (
                "clean",
                coverage,
                Vec::new(),
                DependencySecurityPosture::CleanWithinCoverage,
            ),
        ] {
            let source = AdvisoryKnowledgeSnapshotV1::canonical(
                "fixture".into(),
                revision.into(),
                PackageEcosystem::Cargo,
                covered,
                findings,
                Vec::new(),
                AdvisoryCompleteness::CompleteForDeclaredCoverage,
                now,
            )
            .unwrap();
            std::fs::write(
                &advisory_path,
                serde_json::to_vec(&super::super::advisory::AdvisorySourceDocumentV1::from(
                    source,
                ))
                .unwrap(),
            )
            .unwrap();
            let artifact = invoke(&closure, &api, ACQUIRE_ADVISORIES, &artifacts).await;
            artifacts.insert(artifact.artifact_type_id.clone(), artifact);
            let assessment = invoke(&closure, &api, ASSESS, &artifacts).await;
            let product: DependencySecurityAssessmentV1 =
                serde_json::from_value(assessment.content.clone()).unwrap();
            assert_eq!(product.posture, expected);
            artifacts.insert(assessment.artifact_type_id.clone(), assessment);
            let verification = invoke(&closure, &api, VERIFY, &artifacts).await;
            let verified: DependencySecurityVerificationV1 =
                serde_json::from_value(verification.content).unwrap();
            assert!(verified.verified);
            assert_eq!(verified.independently_computed_posture, expected);
            assert_eq!(verified.assessment_ref, product.assessment_id);
            if !product.findings.is_empty() {
                artifacts.get_mut(ASSESSMENT).unwrap().content["findings"][0]["severity"] =
                    "low".into();
                let verification = invoke(&closure, &api, VERIFY, &artifacts).await;
                let verified: DependencySecurityVerificationV1 =
                    serde_json::from_value(verification.content).unwrap();
                assert!(
                    !verified.verified,
                    "retained finding identities cannot authenticate altered finding content"
                );
            }
        }
        artifacts.get_mut(ASSESSMENT).unwrap().content["assessment_id"] =
            "forged-assessment".into();
        let verification = invoke(&closure, &api, VERIFY, &artifacts).await;
        let verified: DependencySecurityVerificationV1 =
            serde_json::from_value(verification.content).unwrap();
        assert!(
            !verified.verified,
            "a valid calculation cannot authenticate a forged product identity"
        );
        assert!(!workspace.path().join("target").exists());
    }
}
