//! Synthetic interaction-owner characterization over native dispatch, registry, and Event seams.

use meld_execution::capability::{
    BoundCapabilityInstance, CapabilityExecutionContext, CapabilityInvocationPayload,
    InputValueSource, SuppliedInputValue, SuppliedValueRef,
};
use meld_execution::task::{
    ArtifactProducerRef, ArtifactRecord, CompiledTaskRecord, TaskInitSlotSpec,
    TaskInitializationPayload, TaskRunContext,
};
use meld_execution::task_network::command::{Command, Request as CommandRequest, Response};
use meld_execution::task_network::dispatch::{Claim, OutcomeStatus};
use meld_execution::task_network::dispatch_actor::{
    ClaimedInvocationOutcome, ClaimedTaskInvoker, DispatchPortError, DispatchRuntimeActor,
    DispatchTickRequest, TaskNetworkCommandPort,
};
use meld_execution::task_network::mutation::{Inject, Mutation, Set};
use meld_execution::task_network::state::{
    StaticSeedInitSource, TaskInitSource, TaskLineage, TaskNode, TaskStatus,
};
use meld_execution::task_network::SledTaskNetworkStore;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

const INPUT: &str = "interaction_request_binding_v1";
const RECEIPT: &str = "interaction_qualified_receipt_v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct InteractionBinding {
    request_ref: String,
    owner_revision: String,
    owner_digest: String,
    operation_id: String,
}

impl InteractionBinding {
    fn for_request(request_ref: String) -> Self {
        let owner_revision = "synthetic-owner-revision-7".to_string();
        let owner_digest = blake3::hash(b"synthetic interaction request body")
            .to_hex()
            .to_string();
        let operation_id = format!(
            "synthetic-interaction::{}",
            blake3::hash(
                serde_json::to_vec(&(&request_ref, &owner_revision, &owner_digest))
                    .unwrap()
                    .as_slice()
            )
            .to_hex()
        );
        Self {
            request_ref,
            owner_revision,
            owner_digest,
            operation_id,
        }
    }
}

fn node(id: &str, binding: &InteractionBinding) -> TaskNode {
    TaskNode {
        task_instance_id: id.into(),
        lifecycle_epoch: 1,
        compiled_task: CompiledTaskRecord {
            task_id: id.into(),
            task_version: 1,
            init_slots: vec![TaskInitSlotSpec {
                init_slot_id: INPUT.into(),
                artifact_type_id: INPUT.into(),
                schema_version: 1,
                required: true,
            }],
            capability_instances: Vec::new(),
            dependency_edges: Vec::new(),
        },
        init_sources: vec![TaskInitSource::StaticSeed(StaticSeedInitSource {
            init_slot_id: INPUT.into(),
            artifact_type_id: INPUT.into(),
            schema_version: 1,
            content: serde_json::to_value(binding).unwrap(),
        })],
        task_run_context: TaskRunContext {
            task_run_id: format!("run::{id}"),
            session_id: Some("synthetic-interaction".into()),
            trigger: "characterization".into(),
        },
        lineage: TaskLineage::unattributed(id.into(), id.into(), "synthetic.interaction".into(), 1),
    }
}

fn submit_nodes(store: &mut SledTaskNetworkStore, nodes: Vec<TaskNode>) {
    let state = store.state();
    let set = Set::new(
        state.network_id.clone(),
        "synthetic-interaction-task",
        "synthetic-interaction-set",
        nodes
            .into_iter()
            .map(|task_node| Mutation::Inject(Inject::new(task_node, Vec::new())))
            .collect(),
    );
    let response = store
        .submit(CommandRequest {
            command_id: "inject-synthetic-interaction".into(),
            network_id: state.network_id.clone(),
            base_revision: state.revision,
            base_state_hash: state.state_hash.clone(),
            read_preconditions: Vec::new(),
            command: Command::ApplyMutationSet(set),
        })
        .unwrap();
    assert!(matches!(response, Response::Accepted { .. }));
}

fn binding_from(payload: &TaskInitializationPayload) -> InteractionBinding {
    serde_json::from_value(
        payload
            .init_artifacts
            .iter()
            .find(|input| input.init_slot_id == INPUT)
            .unwrap()
            .content
            .clone(),
    )
    .unwrap()
}

fn artifact(claim: &Claim, binding: &InteractionBinding, drift: bool) -> ArtifactRecord {
    ArtifactRecord {
        artifact_id: format!("{}::{RECEIPT}", claim.claim_id),
        artifact_type_id: RECEIPT.into(),
        schema_version: 1,
        content: serde_json::json!({
            "operation_id": binding.operation_id,
            "owner_revision": binding.owner_revision,
            "owner_digest": if drift { "drift" } else { &binding.owner_digest },
            "provenance_class": "synthetic"
        }),
        producer: ArtifactProducerRef {
            task_id: claim.task_instance_id.clone(),
            capability_instance_id: "synthetic-interaction-owner".into(),
            invocation_id: Some(claim.claim_id.clone()),
            output_slot_id: Some(RECEIPT.into()),
        },
    }
}

#[derive(Debug, Clone, Copy)]
enum OwnerBehavior {
    Pending,
    Qualified,
    Terminal,
    DriftAfterFirst,
}

#[derive(Default)]
struct OwnerObservations {
    // This retained fixture state measures repeat calls. It does not prove
    // durable product-owner effect deduplication across a process restart.
    attempts: Vec<(String, String, String)>,
    admitted_creation_effects: BTreeSet<String>,
}

struct SyntheticClaimInvoker {
    behavior: BTreeMap<String, OwnerBehavior>,
    observations: Arc<Mutex<OwnerObservations>>,
}

#[async_trait::async_trait]
impl ClaimedTaskInvoker for SyntheticClaimInvoker {
    async fn invoke_claimed_task(
        &self,
        node: &TaskNode,
        claim: &Claim,
        payload: &TaskInitializationPayload,
    ) -> Result<ClaimedInvocationOutcome, DispatchPortError> {
        let binding = binding_from(payload);
        let attempt = {
            let mut observations = self.observations.lock().unwrap();
            observations
                .admitted_creation_effects
                .insert(binding.operation_id.clone());
            observations.attempts.push((
                node.task_instance_id.clone(),
                claim.claim_id.clone(),
                binding.operation_id.clone(),
            ));
            observations
                .attempts
                .iter()
                .filter(|seen| seen.0 == node.task_instance_id)
                .count()
        };
        match self.behavior[&node.task_instance_id] {
            OwnerBehavior::Pending => Ok(ClaimedInvocationOutcome::Pending {
                detail: "synthetic owner evidence pending".into(),
            }),
            OwnerBehavior::Qualified => Ok(ClaimedInvocationOutcome::Completed(vec![artifact(
                claim, &binding, false,
            )])),
            OwnerBehavior::Terminal => Ok(ClaimedInvocationOutcome::Failed {
                error: "synthetic owner terminal failure".into(),
            }),
            OwnerBehavior::DriftAfterFirst => {
                Ok(ClaimedInvocationOutcome::Completed(vec![artifact(
                    claim,
                    &binding,
                    attempt > 1,
                )]))
            }
        }
    }
}

fn actor(
    db: &sled::Db,
    behavior: BTreeMap<String, OwnerBehavior>,
    observations: Arc<Mutex<OwnerObservations>>,
) -> DispatchRuntimeActor<SyntheticClaimInvoker> {
    DispatchRuntimeActor::new(
        "synthetic-interaction-worker",
        db.clone(),
        SyntheticClaimInvoker {
            behavior,
            observations,
        },
    )
    .unwrap()
}

fn tick(
    actor: &DispatchRuntimeActor<SyntheticClaimInvoker>,
    store: &mut impl TaskNetworkCommandPort,
    sequence: u64,
    max_items: usize,
) -> meld_execution::task_network::dispatch_actor::DispatchTickReport {
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(actor.tick(
            store,
            DispatchTickRequest {
                sequence,
                max_items,
            },
        ))
        .unwrap()
}

#[test]
fn pending_interaction_budget_one_rotates_after_a0_starvation_counterexample() {
    let root = tempfile::tempdir().unwrap();
    let db = sled::open(root.path().join("execution")).unwrap();
    let binding = InteractionBinding::for_request("request::pending".into());
    let unrelated = InteractionBinding::for_request("request::unrelated".into());
    let mut store = SledTaskNetworkStore::open(db.clone(), "pending-network").unwrap();
    submit_nodes(
        &mut store,
        vec![
            node("a-interaction", &binding),
            node("z-unrelated", &unrelated),
        ],
    );
    let behavior = BTreeMap::from([
        ("a-interaction".into(), OwnerBehavior::Pending),
        ("z-unrelated".into(), OwnerBehavior::Qualified),
    ]);
    let observations = Arc::new(Mutex::new(OwnerObservations::default()));
    let first_actor = actor(&db, behavior.clone(), observations.clone());
    let pending = tick(&first_actor, &mut store, 1, 1);
    assert_eq!(pending.retryable_errors.len(), 1, "{pending:?}");
    assert_eq!(
        pending.retryable_errors[0].code,
        "claimed_invocation_pending"
    );
    assert_eq!(store.state().statuses["z-unrelated"], TaskStatus::Pending);
    drop(first_actor);
    drop(store);

    let mut reopened = SledTaskNetworkStore::open(db.clone(), "pending-network").unwrap();
    let reopened_actor = actor(&db, behavior, observations.clone());
    // The accepted A0 commit proved this budget-one peer starved before the
    // canonical dispatch cursor was added. Reopen must retain the new order.
    let progressed = tick(&reopened_actor, &mut reopened, 2, 1);
    assert!(progressed.retryable_errors.is_empty(), "{progressed:?}");
    assert!(matches!(
        reopened.state().statuses["z-unrelated"],
        TaskStatus::Succeeded { .. }
    ));
    let retried = tick(&reopened_actor, &mut reopened, 3, 1);
    assert_eq!(retried.retryable_errors.len(), 1, "{retried:?}");
    assert_eq!(
        retried.retryable_errors[0].code,
        "claimed_invocation_pending"
    );
    let observed = observations.lock().unwrap();
    let interaction: Vec<_> = observed
        .attempts
        .iter()
        .filter(|attempt| attempt.0 == "a-interaction")
        .collect();
    assert_eq!(interaction.len(), 2);
    assert!(interaction.windows(2).all(|pair| pair[0].1 == pair[1].1));
    assert!(interaction.windows(2).all(|pair| pair[0].2 == pair[1].2));
    assert_eq!(observed.admitted_creation_effects.len(), 2);
}

struct LoseFirstOutcome<'a> {
    store: &'a mut SledTaskNetworkStore,
    lost: bool,
}

impl TaskNetworkCommandPort for LoseFirstOutcome<'_> {
    fn network_state(&self) -> &meld_execution::task_network::NetworkState {
        self.store.state()
    }

    fn submit_command(&mut self, request: CommandRequest) -> Result<Response, DispatchPortError> {
        if !self.lost && matches!(&request.command, Command::RecordTaskOutcome(_)) {
            self.lost = true;
            return Err(DispatchPortError::retryable(
                "synthetic lost outcome callback",
            ));
        }
        self.store
            .submit(request)
            .map_err(|error| DispatchPortError::retryable(error.to_string()))
    }
}

#[test]
fn artifact_to_outcome_replay_accepts_identity_and_rejects_drift() {
    for (behavior, expected) in [
        (OwnerBehavior::Qualified, OutcomeStatus::Succeeded),
        (OwnerBehavior::DriftAfterFirst, OutcomeStatus::Failed),
    ] {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let binding = InteractionBinding::for_request(format!("request::{expected:?}"));
        let mut store = SledTaskNetworkStore::open(db.clone(), "artifact-network").unwrap();
        submit_nodes(&mut store, vec![node("interaction", &binding)]);
        let observations = Arc::new(Mutex::new(OwnerObservations::default()));
        let runner = actor(
            &db,
            BTreeMap::from([("interaction".into(), behavior)]),
            observations.clone(),
        );
        {
            let mut lossy = LoseFirstOutcome {
                store: &mut store,
                lost: false,
            };
            let lost = tick(&runner, &mut lossy, 1, 1);
            assert_eq!(lost.retryable_errors.len(), 1, "{lost:?}");
            assert!(matches!(
                lossy.store.state().statuses["interaction"],
                TaskStatus::Running { .. }
            ));
        }
        store.flush().unwrap();
        drop(runner);
        drop(store);
        let mut reopened = SledTaskNetworkStore::open(db.clone(), "artifact-network").unwrap();
        let reopened_runner = actor(
            &db,
            BTreeMap::from([("interaction".into(), behavior)]),
            observations,
        );
        let replayed = tick(&reopened_runner, &mut reopened, 2, 1);
        assert!(replayed.retryable_errors.is_empty(), "{replayed:?}");
        let outcome = reopened.state().outcomes.values().next().unwrap();
        assert_eq!(outcome.status, expected);
        if expected == OutcomeStatus::Failed {
            assert!(outcome
                .error
                .as_deref()
                .unwrap()
                .contains("drifted content"));
        }
    }
}

#[test]
fn terminal_owner_failure_is_not_pending() {
    let db = sled::Config::new().temporary(true).open().unwrap();
    let binding = InteractionBinding::for_request("request::failure".into());
    let mut store = SledTaskNetworkStore::open(db.clone(), "terminal-network").unwrap();
    submit_nodes(
        &mut store,
        vec![node("a-pending", &binding), node("b-terminal", &binding)],
    );
    let runner = actor(
        &db,
        BTreeMap::from([
            ("a-pending".into(), OwnerBehavior::Pending),
            ("b-terminal".into(), OwnerBehavior::Terminal),
        ]),
        Arc::new(Mutex::new(OwnerObservations::default())),
    );
    let report = tick(&runner, &mut store, 1, 2);
    assert_eq!(report.retryable_errors.len(), 1, "{report:?}");
    assert!(matches!(
        store.state().statuses["a-pending"],
        TaskStatus::Running { .. }
    ));
    assert!(matches!(
        store.state().statuses["b-terminal"],
        TaskStatus::Failed { .. }
    ));
}

#[tokio::test]
async fn qualified_registry_recovers_only_from_exact_owner_evidence() {
    use crate::capability::{
        ExactCapabilityActivationRequest, OwnerBindingView, ProductCapabilityInventory,
    };
    use meld_events::{DomainObjectRef, EventAuthority, EventAuthorityOpenOptions};

    let root = tempfile::tempdir().unwrap();
    let authority = EventAuthority::open(
        sled::open(root.path().join("events")).unwrap(),
        EventAuthorityOpenOptions::default(),
    )
    .unwrap();
    let api = crate::api::ContextApi::new(
        Arc::new(crate::store::SledNodeRecordStore::new(root.path().join("nodes")).unwrap()),
        Arc::new(crate::context::frame::FrameStorage::new(root.path().join("frames")).unwrap()),
        crate::heads::HeadIndex::new(),
        Arc::new(
            crate::prompt_context::PromptContextArtifactStorage::new(root.path().join("prompts"))
                .unwrap(),
        ),
        Arc::new(parking_lot::RwLock::new(crate::agent::AgentRegistry::new())),
        Arc::new(parking_lot::RwLock::new(
            crate::provider::ProviderRegistry::new(),
        )),
        Arc::new(crate::concurrency::NodeLockManager::new()),
    );
    api.bind_event_append(authority.append_capability())
        .unwrap();
    let inventory = ProductCapabilityInventory::assemble(vec![Arc::new(
        crate::nonce::capability::NonceCapabilityContributor,
    )])
    .unwrap();
    let revision = inventory.contracts().next().unwrap().revision_ref();
    let implementation = inventory.unique_implementation_ref(&revision).unwrap();
    let prepared = inventory
        .prepare(
            ExactCapabilityActivationRequest {
                assignment_id: "synthetic-interaction-assignment".into(),
                activation_id: "synthetic-interaction-activation".into(),
                selected_contracts: vec![revision.clone()],
                selected_implementations: BTreeMap::from([(
                    revision.clone(),
                    implementation.clone(),
                )]),
                compatibility_policy_revision: "synthetic-policy-v1".into(),
            },
            &OwnerBindingView::default(),
        )
        .unwrap();
    let request = crate::nonce::NonceRequest::new(
        "synthetic-owner".into(),
        DomainObjectRef::new("synthetic", "interaction", "one").unwrap(),
        vec!["synthetic-evidence".into()],
        "synthetic-fence".into(),
    )
    .unwrap();
    let instance = BoundCapabilityInstance {
        capability_instance_id: "synthetic-nonce-instance".into(),
        capability_type_id: crate::nonce::capability::EMIT.into(),
        capability_version: 1,
        scope_ref: request.subject_ref.object_id.clone(),
        scope_kind: "domain_object".into(),
        binding_values: Vec::new(),
        input_wiring: Vec::new(),
    };
    let runtime = prepared.invokers.runtime_init_for(&instance).unwrap();
    let payload = CapabilityInvocationPayload {
        invocation_id: "synthetic-qualified-invocation".into(),
        capability_instance_id: instance.capability_instance_id,
        supplied_inputs: vec![SuppliedInputValue {
            slot_id: crate::nonce::capability::REQUEST.into(),
            source: InputValueSource::InitPayload,
            value: SuppliedValueRef::StructuredValue(serde_json::to_value(&request).unwrap()),
        }],
        upstream_lineage: None,
        execution_context: CapabilityExecutionContext::default(),
    };
    let context = crate::execution::ExecutionEventContext {
        session_id: "synthetic-registry".into(),
        effect_authority: Some(meld_execution::ExecutionEffectAuthority {
            request_ref: None,
            issuer_ref: request.issuer_ref.clone(),
            principal_id: request.issuer_ref.clone(),
            subject: request.subject_ref.clone(),
            fence_ref: request.fence_ref.clone(),
        }),
    };
    let invoker = prepared
        .invokers
        .get(crate::nonce::capability::EMIT, 1)
        .unwrap();
    assert!(invoker
        .recover(
            Some(&authority.replay_capability()),
            &runtime,
            &payload,
            Some(&context)
        )
        .await
        .unwrap()
        .is_none());
    let before = authority.watermark_capability().snapshot().unwrap().tip_seq;
    let qualified = invoker
        .invoke(&api, &runtime, &payload, Some(&context))
        .await
        .unwrap();
    let reopened = inventory
        .prepare(
            ExactCapabilityActivationRequest {
                assignment_id: "synthetic-interaction-assignment".into(),
                activation_id: "synthetic-interaction-activation".into(),
                selected_contracts: vec![revision.clone()],
                selected_implementations: BTreeMap::from([(revision, implementation)]),
                compatibility_policy_revision: "synthetic-policy-v1".into(),
            },
            &OwnerBindingView::default(),
        )
        .unwrap();
    let reopened_invoker = reopened
        .invokers
        .get(crate::nonce::capability::EMIT, 1)
        .unwrap();
    let recovered = reopened_invoker
        .recover(
            Some(&authority.replay_capability()),
            &runtime,
            &payload,
            Some(&context),
        )
        .await
        .unwrap()
        .unwrap();
    assert_eq!(recovered.emitted_artifacts, qualified.emitted_artifacts);
    assert!(reopened_invoker
        .recover(None, &runtime, &payload, Some(&context))
        .await
        .unwrap()
        .is_none());
    assert_eq!(
        authority.watermark_capability().snapshot().unwrap().tip_seq,
        before + 1
    );
}
