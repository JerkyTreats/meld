//! World-initialization pipeline: theory installation, identity genesis,
//! and epistemic seeding.
//!
//! Owner: root init. This is the command path behind the frozen shapes in
//! [`crate::init::world`], executing the four world-initialization stages
//! of the staged contract in
//! `design/plan/integration/runtime_initialization.md`: theory installs
//! through the world-model registry command, genesis identities are created
//! through the Agent-owned genesis operation, and the epistemic genesis fact
//! seeds through the canonical event append. The pipeline never writes a domain
//! store directly — every durable effect crosses a domain command or the
//! append capability, so each stage stays idempotent by the content
//! identity its owning domain defines.
//!
//! Stage selection is normalized to pipeline order without duplicates: a
//! request only chooses the subset, never the order. Re-running any subset
//! against unchanged content reports `Unchanged` with the same record ids.

use meld_events::{AppendDisposition, AppendMode, EventAppendCapability, EventEnvelope};
use meld_world_model::agent::{
    AgentGenesis, AgentGenesisIntentV1, AgentMaintainedConditionRegistryStore, AgentStore,
    AgentSubscriptionRequestV1, SeedAgentRegistration,
};
use meld_world_model::belief::genesis::{
    UnobservedScopeDeclaration, EPISTEMIC_GENESIS_STREAM_ID, UNOBSERVED_SCOPE_EVENT_TYPE,
};
use meld_world_model::belief::{
    configured_belief_key, BeliefFamilyRegistry, BeliefStore, BeliefSubscriptionAuthority,
    BranchScope,
};
use meld_world_model::PerspectiveKey;
use thiserror::Error;

use crate::capability::{
    ExactCapabilityActivationRequest, OwnerBindingView, ProductCapabilityInventory,
};
use crate::config::{StewardshipActivationV1, StewardshipAssignmentV1};
use crate::init::world::{
    StageDisposition, WorldInitReport, WorldInitRequest, WorldInitStage, WorldInitStageReport,
};
use crate::theory::{
    product_owner_preparation_refs, EffectiveAuthorityInputRefs, OwnerBindingRevisionRef,
    PdsPackageInstallationReceiptV1, PdsProductStore, PreparedActivationClosureV1,
    ProductAgentTopologyReceiptV1, ProductCompilationReceiptV1, ProductDeclarationV1,
};

/// Domain that owns the epistemic genesis fact appended by stage 5.
const GENESIS_DOMAIN_ID: &str = "world_model";

/// Canonical stage order of the world-initialization pipeline.
const PIPELINE_ORDER: [WorldInitStage; 4] = [
    WorldInitStage::InstallTheory,
    WorldInitStage::GenesisIdentities,
    WorldInitStage::PrepareActivation,
    WorldInitStage::SeedEpistemicFacts,
];

/// Failure of one world-initialization stage.
///
/// Every variant names the stage that failed so a partial run stays
/// diagnosable; stages already completed hold their durable effects, and
/// re-running the pipeline after the cause is fixed is always safe.
#[derive(Debug, Error)]
pub(crate) enum WorldInitError {
    /// Stage 2 could not install or resolve theory.
    #[error("install theory stage failed: {0}")]
    Theory(String),
    /// Stage 3 could not create or resolve genesis identities.
    #[error("genesis identities stage failed: {0}")]
    Identity(String),
    /// A later stage requires an installed theory revision that is missing.
    #[error(
        "belief family '{family_id}' has no installed revision; run the install-theory stage first"
    )]
    TheoryNotInstalled {
        /// Family the failing stage tried to resolve.
        family_id: String,
    },
    /// Stage 4 could not append the epistemic genesis fact.
    #[error("seed epistemic facts stage failed: {0}")]
    Genesis(String),
    /// Stage 4 could not prepare an inert product activation closure.
    #[error("prepare activation stage failed: {0}")]
    Activation(String),
}

/// Operational metadata for one world-initialization run.
///
/// Semantic identities come exclusively from the installed product,
/// compilation, and assignment records. This input carries only append and
/// provenance metadata that does not select product meaning.
#[derive(Debug, Clone)]
pub(crate) struct WorldInitRunMetadata {
    /// Provenance recorded on Agent registrations and the genesis fact.
    pub(crate) provenance: String,
    /// Session partition for the stage 5 ledger append.
    pub(crate) session_id: String,
    /// Ledger sequence observed when this run started; recorded on new
    /// registrations and revisions, ignored by unchanged re-runs.
    pub(crate) observed_seq: u64,
}

/// Canonical package installation result consumed by world initialization.
pub(crate) struct CompleteTheoryInstall {
    pub(crate) routed: RoutedTheoryInstall,
}

#[derive(Debug, Clone)]
pub(crate) struct RoutedTheoryInstall {
    pub(crate) receipt: PdsPackageInstallationReceiptV1,
    pub(crate) changed: bool,
}

/// Exact product inputs supplied to the root composition adapter.
pub(crate) struct CompleteProductInitialization<'a> {
    pub(crate) product_store: &'a PdsProductStore,
    pub(crate) maintained_conditions: &'a AgentMaintainedConditionRegistryStore,
    pub(crate) declaration: ProductDeclarationV1,
    pub(crate) compilation: ProductCompilationReceiptV1,
    pub(crate) assignment: StewardshipAssignmentV1,
    pub(crate) activation: StewardshipActivationV1,
    pub(crate) capability_inventory: ProductCapabilityInventory,
    pub(crate) capability_bindings: OwnerBindingView,
}

/// Normalize a stage selection to pipeline order without duplicates.
///
/// The request only selects the subset: duplicates collapse and request
/// order is ignored, so `[seed, install, seed]` runs install first and
/// seeds once.
pub fn normalized_stages(request: &WorldInitRequest) -> Vec<WorldInitStage> {
    PIPELINE_ORDER
        .into_iter()
        .filter(|stage| request.stages.contains(stage))
        .collect()
}

/// The stage 2 through 5 command pipeline over injected domain capabilities.
///
/// Holds no storage of its own: the belief-family registry, the agent
/// store, and the append capability are the only write surfaces, and all
/// three are domain-owned.
pub(crate) struct WorldInitPipeline<'a> {
    registry: &'a mut dyn BeliefFamilyRegistry,
    belief_store: &'a BeliefStore,
    agent_store: &'a AgentStore,
    append: &'a EventAppendCapability,
    complete_theory: Option<CompleteTheoryInstall>,
    complete_product: Option<CompleteProductInitialization<'a>>,
    topology_receipt: Option<ProductAgentTopologyReceiptV1>,
}

impl<'a> WorldInitPipeline<'a> {
    /// Bind the pipeline to its domain command surfaces.
    pub(crate) fn new(
        registry: &'a mut dyn BeliefFamilyRegistry,
        belief_store: &'a BeliefStore,
        agent_store: &'a AgentStore,
        append: &'a EventAppendCapability,
    ) -> Self {
        Self {
            registry,
            belief_store,
            agent_store,
            append,
            complete_theory: None,
            complete_product: None,
            topology_receipt: None,
        }
    }

    /// Bind exact PDS, Agent, Belief, and Capability owner inputs.
    pub(crate) fn with_complete_product(
        mut self,
        product: CompleteProductInitialization<'a>,
    ) -> Self {
        self.complete_product = Some(product);
        self
    }

    /// Bind the complete image and all owner stores for the elevated path.
    pub(crate) fn with_complete_theory(mut self, install: CompleteTheoryInstall) -> Self {
        self.complete_theory = Some(install);
        self
    }

    /// Run the selected stages in pipeline order and report per stage.
    ///
    /// Stops at the first failing stage; completed stages keep their
    /// durable effects and the run is safe to repeat.
    pub(crate) fn run(
        &mut self,
        request: &WorldInitRequest,
        metadata: &WorldInitRunMetadata,
    ) -> Result<WorldInitReport, WorldInitError> {
        let mut stage_reports = Vec::new();
        for stage in normalized_stages(request) {
            let mut report = match stage {
                WorldInitStage::InstallTheory => self.install_theory()?,
                WorldInitStage::GenesisIdentities => self.genesis_identities(metadata)?,
                WorldInitStage::PrepareActivation => self.prepare_activation()?,
                WorldInitStage::SeedEpistemicFacts => self.seed_epistemic_facts(metadata)?,
            };
            if stage == WorldInitStage::InstallTheory {
                let (changed, mut ids) = self.compile_product()?;
                if changed {
                    report.disposition = StageDisposition::Applied;
                }
                report.record_ids.append(&mut ids);
            }
            stage_reports.push(report);
        }
        Ok(WorldInitReport { stage_reports })
    }

    fn compile_product(&self) -> Result<(bool, Vec<String>), WorldInitError> {
        let product = self.complete_product.as_ref().ok_or_else(|| {
            WorldInitError::Theory(
                "complete product inputs are required for product compilation".to_string(),
            )
        })?;
        let prior = product
            .product_store
            .head(&product.declaration.product_id)
            .map_err(|error| WorldInitError::Theory(error.to_string()))?;
        let changed = prior
            .as_ref()
            .map(|head| head.compilation_receipt_id.as_str())
            != Some(product.compilation.compilation_receipt_id.as_str());
        product
            .product_store
            .install(&product.declaration, &product.compilation, prior.as_ref())
            .map_err(|error| WorldInitError::Theory(error.to_string()))?;
        Ok((
            changed,
            vec![
                format!(
                    "pds_product_revision::{}",
                    product.declaration.product_revision_id
                ),
                format!(
                    "pds_product_compilation::{}",
                    product.compilation.compilation_receipt_id
                ),
            ],
        ))
    }

    /// Stage 2 reports the canonical PDS package installation already
    /// performed through owner routes. No alternate installer exists here.
    fn install_theory(&self) -> Result<WorldInitStageReport, WorldInitError> {
        let routed = &self
            .complete_theory
            .as_ref()
            .ok_or_else(|| {
                WorldInitError::Theory(
                    "the canonical PDS package installation receipt is required".to_string(),
                )
            })?
            .routed;
        routed
            .receipt
            .verify_identity()
            .map_err(|error| WorldInitError::Theory(error.to_string()))?;
        let product = self.complete_product.as_ref().ok_or_else(|| {
            WorldInitError::Theory(
                "complete product inputs are required for package installation".to_string(),
            )
        })?;
        if !product
            .compilation
            .package_receipt_ids
            .contains(&routed.receipt.receipt_id)
        {
            return Err(WorldInitError::Theory(
                "package installation receipt is absent from the product compilation".to_string(),
            ));
        }
        let mut record_ids: Vec<String> = routed
            .receipt
            .components
            .iter()
            .map(|component| {
                format!(
                    "{}::{}::{}",
                    component.owner_revision.registry,
                    component.owner_revision.id,
                    component.owner_revision.content_hash
                )
            })
            .collect();
        record_ids.push(format!(
            "pds_package_receipt::{}",
            routed.receipt.receipt_id
        ));
        Ok(WorldInitStageReport {
            stage: WorldInitStage::InstallTheory,
            disposition: if routed.changed {
                StageDisposition::Applied
            } else {
                StageDisposition::Unchanged
            },
            record_ids,
        })
    }

    /// Stage 3: create every Agent declared by the complete product image.
    ///
    /// Product lineage, topology, source contracts, and canonical Event
    /// publication are mandatory inputs. A caller that did not bind the
    /// complete product image fails closed rather than manufacturing
    /// compatibility identities.
    fn genesis_identities(
        &mut self,
        metadata: &WorldInitRunMetadata,
    ) -> Result<WorldInitStageReport, WorldInitError> {
        if self.complete_product.is_none() {
            return Err(WorldInitError::Identity(
                "Agent genesis requires complete product initialization inputs".to_string(),
            ));
        }
        self.genesis_product(metadata)
    }

    fn genesis_product(
        &mut self,
        metadata: &WorldInitRunMetadata,
    ) -> Result<WorldInitStageReport, WorldInitError> {
        let product = self
            .complete_product
            .as_ref()
            .expect("complete product checked");
        verify_current_product(product)?;
        let curation_ref =
            compilation_world_ref(&product.compilation, "world-model", "agent-curation-rule")?;
        let maintained_ref = compilation_world_ref(
            &product.compilation,
            "world-model",
            "agent-maintained-condition",
        )?;
        let maintained = product
            .maintained_conditions
            .resolve(&maintained_ref.id, &maintained_ref.content_hash)
            .map_err(|error| WorldInitError::Identity(error.to_string()))?
            .ok_or_else(|| {
                WorldInitError::Identity(
                    "installed maintained condition revision is missing".to_string(),
                )
            })?
            .binding()
            .map_err(|error| WorldInitError::Identity(error.to_string()))?;
        let perspective = assignment_perspective(&product.assignment)?;
        let branch_scope = BranchScope::new(product.assignment.branch_id.clone())
            .map_err(|error| WorldInitError::Identity(error.to_string()))?;
        let installed_owner_revisions = product
            .compilation
            .installed_owner_revisions
            .iter()
            .map(|component| meld_world_model::belief::TheoryRevisionRef {
                registry: component.owner_revision.registry.clone(),
                id: component.owner_revision.id.clone(),
                content_hash: component.owner_revision.content_hash.clone(),
            })
            .collect::<Vec<_>>();
        let mut receipts = Vec::new();
        let mut changed = false;
        for assigned in &product.assignment.agent_positions {
            let position = product
                .declaration
                .agent_topology
                .iter()
                .find(|position| position.position_id == assigned.position_id)
                .ok_or_else(|| {
                    WorldInitError::Identity(
                        "assignment contains an Agent position absent from the product".to_string(),
                    )
                })?;
            let observation_ref = compilation_component_ref(
                &product.compilation,
                &position.observation_scope_component_id,
            )?;
            observation_ref
                .validate_for_registry("belief_family")
                .map_err(|error| WorldInitError::Identity(error.to_string()))?;
            let family = self
                .registry
                .resolve(&observation_ref.id, &observation_ref.content_hash)
                .map_err(|error| WorldInitError::Identity(error.to_string()))?
                .ok_or_else(|| WorldInitError::TheoryNotInstalled {
                    family_id: observation_ref.id.clone(),
                })?;
            let belief_key = configured_belief_key(
                &family,
                &product.assignment.subject,
                &perspective,
                &branch_scope,
            );
            let requests = position
                .required_subscriptions
                .iter()
                .map(|subscription| {
                    if subscription.source_owner != "belief" {
                        return Err(WorldInitError::Identity(format!(
                            "Agent genesis has no source-owner adapter for '{}'",
                            subscription.source_owner
                        )));
                    }
                    let source_revision = compilation_component_ref(
                        &product.compilation,
                        &subscription.source_contract_component_id,
                    )?;
                    if source_revision != observation_ref {
                        return Err(WorldInitError::Identity(
                            "Agent subscription differs from the installed Belief family"
                                .to_string(),
                        ));
                    }
                    AgentSubscriptionRequestV1::new(
                        assigned.agent_id.clone(),
                        subscription.source_owner.clone(),
                        source_revision,
                        belief_key.clone(),
                        subscription.initial_cursor_policy.clone(),
                    )
                    .map_err(|error| WorldInitError::Identity(error.to_string()))
                })
                .collect::<Result<Vec<_>, _>>()?;
            let created_at_seq = self
                .agent_store
                .get_agent(&assigned.agent_id)
                .map_err(|error| WorldInitError::Identity(error.to_string()))?
                .map_or(metadata.observed_seq, |record| record.created_at_seq);
            let intent = AgentGenesisIntentV1::new(
                product.assignment.assignment_id.clone(),
                product.compilation.compilation_receipt_id.clone(),
                assigned.position_id.clone(),
                installed_owner_revisions.clone(),
                SeedAgentRegistration {
                    agent_id: assigned.agent_id.clone(),
                    perspective_key: perspective.clone(),
                    subject: product.assignment.subject.clone(),
                    branch_scope: branch_scope.clone(),
                    observation_scope: family.config.dimension_id.clone(),
                    directive: position.directive.clone(),
                    seed_provenance: metadata.provenance.clone(),
                    curation_rule: None,
                    curation_rule_revision: Some(curation_ref.clone()),
                    maintained_condition: Some(maintained.clone()),
                    maintained_condition_revision: Some(maintained_ref.clone()),
                    created_at_seq,
                },
                requests.clone(),
            )
            .map_err(|error| WorldInitError::Identity(error.to_string()))?;
            let genesis = AgentGenesis::new(self.agent_store, self.append);
            let pending = genesis
                .prepare(intent)
                .map_err(|error| WorldInitError::Identity(error.to_string()))?;
            let acceptances = requests
                .iter()
                .map(|request| {
                    BeliefSubscriptionAuthority::new(self.belief_store)
                        .accept(request, &family)
                        .map_err(|error| WorldInitError::Identity(error.to_string()))
                })
                .collect::<Result<Vec<_>, _>>()?;
            let (receipt, receipt_changed) = genesis
                .complete(pending, acceptances, &metadata.session_id)
                .map_err(|error| WorldInitError::Identity(error.to_string()))?;
            changed |= receipt_changed;
            receipts.push(receipt);
        }
        let topology = ProductAgentTopologyReceiptV1::new(
            &product.assignment,
            &product.declaration,
            receipts.clone(),
        )
        .map_err(|error| WorldInitError::Identity(error.to_string()))?;
        self.topology_receipt = Some(topology.clone());
        let mut record_ids = receipts
            .iter()
            .flat_map(|receipt| {
                [
                    format!("agent::{}", receipt.agent_id),
                    format!("agent_genesis_receipt::{}", receipt.genesis_receipt_id),
                    format!("agent_genesis_publication::{}", receipt.publication_id),
                    format!("agent_genesis_event::{}", receipt.event_record_id),
                ]
            })
            .collect::<Vec<_>>();
        record_ids.push(format!(
            "agent_topology_receipt::{}",
            topology.topology_receipt_id
        ));
        Ok(WorldInitStageReport {
            stage: WorldInitStage::GenesisIdentities,
            disposition: if changed {
                StageDisposition::Applied
            } else {
                StageDisposition::Unchanged
            },
            record_ids,
        })
    }

    /// Prepare and persist one inert closure over every product owner.
    fn prepare_activation(&mut self) -> Result<WorldInitStageReport, WorldInitError> {
        let product = self.complete_product.as_ref().ok_or_else(|| {
            WorldInitError::Activation(
                "complete product inputs are required for activation preparation".to_string(),
            )
        })?;
        verify_current_product(product)?;
        if self.topology_receipt.is_none() {
            let receipts = self
                .agent_store
                .genesis_receipts_for_assignment(&product.assignment.assignment_id)
                .map_err(|error| WorldInitError::Activation(error.to_string()))?;
            if !receipts.is_empty() {
                self.topology_receipt = Some(
                    ProductAgentTopologyReceiptV1::new(
                        &product.assignment,
                        &product.declaration,
                        receipts,
                    )
                    .map_err(|error| WorldInitError::Activation(error.to_string()))?,
                );
            }
        }
        let topology = self.topology_receipt.as_ref().ok_or_else(|| {
            WorldInitError::Activation(
                "Agent genesis must complete before activation preparation".to_string(),
            )
        })?;
        let selected_contracts = product
            .activation
            .selected_implementations
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        let prepared_capabilities = product
            .capability_inventory
            .prepare(
                ExactCapabilityActivationRequest {
                    assignment_id: product.assignment.assignment_id.clone(),
                    activation_id: product.activation.activation_id.clone(),
                    selected_contracts,
                    selected_implementations: product.activation.selected_implementations.clone(),
                    compatibility_policy_revision: "capability-compatibility.v1".to_string(),
                },
                &product.capability_bindings,
            )
            .map_err(|error| WorldInitError::Activation(error.to_string()))?;
        let owner_receipts = product_owner_preparation_refs(&product.compilation)
            .map_err(|error| WorldInitError::Activation(error.to_string()))?;
        let binding_revision_refs = prepared_capabilities
            .preparation_receipt
            .binding_revision_refs
            .iter()
            .map(|(binding_id, revision_ref)| OwnerBindingRevisionRef {
                binding_id: binding_id.clone(),
                revision_ref: revision_ref.clone(),
            })
            .collect();
        let prior = product
            .product_store
            .prepared_head(&product.declaration.product_id)
            .map_err(|error| WorldInitError::Activation(error.to_string()))?;
        let effective_authority_inputs = EffectiveAuthorityInputRefs {
            requested_authority_ref: product.assignment.requested_authority_ref.clone(),
            principal_grant_ref: product.assignment.principal_grant_ref.clone(),
            current_judgment_ref: format!(
                "activation-authority::{}",
                product.activation.activation_id
            ),
        };
        if let Some(head) = &prior {
            let current = product
                .product_store
                .prepared_closure(&head.prepared_id)
                .map_err(|error| WorldInitError::Activation(error.to_string()))?
                .ok_or_else(|| {
                    WorldInitError::Activation(
                        "prepared product head cites a missing closure".to_string(),
                    )
                })?;
            if current.assignment == product.assignment
                && current.activation == product.activation
                && current.agent_topology_receipt_id == topology.topology_receipt_id
                && current.owner_receipts == owner_receipts
                && current.capability_preparation_receipt_id
                    == prepared_capabilities
                        .preparation_receipt
                        .preparation_receipt_id
                && current.participant_plan == product.declaration.participant_plan
                && current.binding_revision_refs == binding_revision_refs
                && current.effective_authority_inputs == effective_authority_inputs
            {
                return Ok(preparation_report(
                    StageDisposition::Unchanged,
                    &product.assignment,
                    &prepared_capabilities
                        .preparation_receipt
                        .preparation_receipt_id,
                    &current.prepared_id,
                ));
            }
        }
        let closure = PreparedActivationClosureV1::new(
            product.assignment.clone(),
            product.activation.clone(),
            topology.topology_receipt_id.clone(),
            owner_receipts,
            prepared_capabilities
                .preparation_receipt
                .preparation_receipt_id
                .clone(),
            product.declaration.participant_plan.clone(),
            binding_revision_refs,
            effective_authority_inputs,
            prior.as_ref().map(|head| head.prepared_id.clone()),
        )
        .map_err(|error| WorldInitError::Activation(error.to_string()))?;
        product
            .product_store
            .install_prepared(
                &product.assignment,
                self.agent_store,
                &prepared_capabilities.preparation_receipt,
                &closure,
                prior.as_ref(),
            )
            .map_err(|error| WorldInitError::Activation(error.to_string()))?;
        Ok(preparation_report(
            StageDisposition::Applied,
            &product.assignment,
            &prepared_capabilities
                .preparation_receipt
                .preparation_receipt_id,
            &closure.prepared_id,
        ))
    }

    /// Stage 5: append the unobserved-scope declaration for the selected
    /// subtree through the canonical append capability.
    ///
    /// The frozen record identity is per scope and carries no time, so the
    /// events constructor's wall-clock timestamp does not affect
    /// idempotency: a re-run dedupes on the record id and the original
    /// recorded time stands. An injected-time constructor variant for
    /// isolate boots is a recorded events-domain request.
    fn seed_epistemic_facts(
        &self,
        metadata: &WorldInitRunMetadata,
    ) -> Result<WorldInitStageReport, WorldInitError> {
        let product = self.complete_product.as_ref().ok_or_else(|| {
            WorldInitError::Genesis(
                "epistemic genesis requires complete product initialization inputs".to_string(),
            )
        })?;
        verify_current_product(product)?;
        let declaration = UnobservedScopeDeclaration {
            subject: product.assignment.subject.clone(),
            declared_by: metadata.provenance.clone(),
        };
        let payload = serde_json::to_value(&declaration)
            .map_err(|error| WorldInitError::Genesis(error.to_string()))?;
        let envelope = EventEnvelope::epistemic_genesis(
            metadata.session_id.clone(),
            GENESIS_DOMAIN_ID,
            EPISTEMIC_GENESIS_STREAM_ID,
            &declaration.scope_key(),
            UNOBSERVED_SCOPE_EVENT_TYPE,
            payload,
        )
        .with_graph(vec![declaration.subject.clone()], Vec::new());
        // The constructor and the declaration derive the same frozen id;
        // the assertion keeps the joint events/world-model contract honest
        // if either side ever drifts.
        debug_assert_eq!(
            envelope.record_id.as_deref(),
            Some(declaration.record_id()).as_deref()
        );
        let receipt = self
            .append
            .append_durable(envelope, AppendMode::Idempotent)
            .map_err(|error| WorldInitError::Genesis(error.to_string()))?;
        Ok(WorldInitStageReport {
            stage: WorldInitStage::SeedEpistemicFacts,
            disposition: match receipt.disposition {
                AppendDisposition::Inserted => StageDisposition::Applied,
                AppendDisposition::Duplicate => StageDisposition::Unchanged,
            },
            record_ids: vec![declaration.record_id()],
        })
    }
}

fn verify_current_product(
    product: &CompleteProductInitialization<'_>,
) -> Result<(), WorldInitError> {
    product
        .declaration
        .verify_identity()
        .map_err(|error| WorldInitError::Theory(error.to_string()))?;
    product
        .compilation
        .verify_identity()
        .map_err(|error| WorldInitError::Theory(error.to_string()))?;
    let compilation_head = product
        .product_store
        .head(&product.declaration.product_id)
        .map_err(|error| WorldInitError::Theory(error.to_string()))?
        .ok_or_else(|| {
            WorldInitError::Theory(
                "world initialization requires an installed current compilation for the product"
                    .to_string(),
            )
        })?;
    let stored_declaration = product
        .product_store
        .declaration(&product.declaration.product_revision_id)
        .map_err(|error| WorldInitError::Theory(error.to_string()))?;
    let stored_compilation = product
        .product_store
        .compilation(&product.compilation.compilation_receipt_id)
        .map_err(|error| WorldInitError::Theory(error.to_string()))?;
    if compilation_head.product_revision_id != product.declaration.product_revision_id
        || compilation_head.compilation_receipt_id != product.compilation.compilation_receipt_id
        || stored_declaration.as_ref() != Some(&product.declaration)
        || stored_compilation.is_none()
    {
        return Err(WorldInitError::Theory(
            "world initialization inputs differ from the installed current compilation".to_string(),
        ));
    }
    Ok(())
}

fn assignment_perspective(
    assignment: &StewardshipAssignmentV1,
) -> Result<PerspectiveKey, WorldInitError> {
    let (kind, id) = assignment.perspective_id.split_once("::").ok_or_else(|| {
        WorldInitError::Identity(
            "assignment perspective must be a canonical perspective index key".to_string(),
        )
    })?;
    PerspectiveKey::new(kind, id).map_err(|error| WorldInitError::Identity(error.to_string()))
}

fn compilation_world_ref(
    receipt: &ProductCompilationReceiptV1,
    owner: &str,
    kind: &str,
) -> Result<meld_world_model::belief::TheoryRevisionRef, WorldInitError> {
    let route = crate::theory::TheoryRouteId::new(owner, kind, 1);
    let mut matches = receipt
        .installed_owner_revisions
        .iter()
        .filter(|component| component.route == route);
    let component = matches.next().ok_or_else(|| {
        WorldInitError::Identity(format!(
            "product compilation is missing route '{}'",
            route.key()
        ))
    })?;
    if matches.next().is_some() {
        return Err(WorldInitError::Identity(format!(
            "product compilation route '{}' is ambiguous",
            route.key()
        )));
    }
    Ok(meld_world_model::belief::TheoryRevisionRef {
        registry: component.owner_revision.registry.clone(),
        id: component.owner_revision.id.clone(),
        content_hash: component.owner_revision.content_hash.clone(),
    })
}

fn compilation_component_ref(
    receipt: &ProductCompilationReceiptV1,
    component_id: &str,
) -> Result<meld_world_model::belief::TheoryRevisionRef, WorldInitError> {
    let mut matches = receipt
        .installed_owner_revisions
        .iter()
        .filter(|component| component.component_id == component_id);
    let component = matches.next().ok_or_else(|| {
        WorldInitError::Identity(format!(
            "product compilation is missing component '{component_id}'"
        ))
    })?;
    if matches.next().is_some() {
        return Err(WorldInitError::Identity(format!(
            "product compilation component '{component_id}' is ambiguous"
        )));
    }
    Ok(meld_world_model::belief::TheoryRevisionRef {
        registry: component.owner_revision.registry.clone(),
        id: component.owner_revision.id.clone(),
        content_hash: component.owner_revision.content_hash.clone(),
    })
}

fn preparation_report(
    disposition: StageDisposition,
    assignment: &StewardshipAssignmentV1,
    capability_preparation_receipt_id: &str,
    prepared_id: &str,
) -> WorldInitStageReport {
    WorldInitStageReport {
        stage: WorldInitStage::PrepareActivation,
        disposition,
        record_ids: vec![
            format!("stewardship_assignment::{}", assignment.assignment_id),
            format!("capability_preparation::{capability_preparation_receipt_id}"),
            format!("prepared_product::{prepared_id}"),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalization_orders_and_dedupes_stage_selection() {
        let request = WorldInitRequest {
            stages: vec![
                WorldInitStage::SeedEpistemicFacts,
                WorldInitStage::InstallTheory,
                WorldInitStage::SeedEpistemicFacts,
                WorldInitStage::GenesisIdentities,
                WorldInitStage::InstallTheory,
            ],
        };
        assert_eq!(
            normalized_stages(&request),
            vec![
                WorldInitStage::InstallTheory,
                WorldInitStage::GenesisIdentities,
                WorldInitStage::SeedEpistemicFacts,
            ]
        );
    }

    #[test]
    fn normalization_keeps_a_partial_selection_partial() {
        let request = WorldInitRequest {
            stages: vec![WorldInitStage::SeedEpistemicFacts],
        };
        assert_eq!(
            normalized_stages(&request),
            vec![WorldInitStage::SeedEpistemicFacts]
        );
    }

    #[test]
    fn normalization_of_an_empty_selection_runs_nothing() {
        let request = WorldInitRequest { stages: Vec::new() };
        assert!(normalized_stages(&request).is_empty());
    }
}
