use super::*;
use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

use meld_execution::task::{ArtifactProducerRef, ArtifactRecord, TaskInitializationPayload};
use meld_execution::task_network::dispatch::Claim;
use meld_execution::task_network::dispatch_actor::{
    AdmissionGenerationObserver, ClaimedInvocationOutcome, ClaimedTaskInvoker, DispatchPortError,
    DispatchRuntimeActor, DispatchTickRequest,
};
use meld_execution::task_network::{sharing::admission_discharge_account, TaskNode};
use meld_lang::{AuthorityPolicy, AuthorityPolicyBinding};

struct AgentGenerations(Arc<Mutex<BTreeSet<String>>>);

impl AdmissionGenerationObserver for AgentGenerations {
    fn active_generation(&self, agent_id: &str) -> Result<Option<String>, String> {
        Ok(self
            .0
            .lock()
            .unwrap()
            .contains(agent_id)
            .then(|| "generation-v1".into()))
    }
}

struct MetadataInvoker(Arc<Mutex<usize>>);

#[async_trait::async_trait]
impl ClaimedTaskInvoker for MetadataInvoker {
    async fn invoke_claimed_task(
        &self,
        node: &TaskNode,
        claim: &Claim,
        _inputs: &TaskInitializationPayload,
    ) -> Result<ClaimedInvocationOutcome, DispatchPortError> {
        *self.0.lock().unwrap() += 1;
        Ok(ClaimedInvocationOutcome::Completed(vec![ArtifactRecord {
            artifact_id: format!("{}::summary", claim.claim_id),
            artifact_type_id: "summary_doc".into(),
            schema_version: 1,
            content: serde_json::json!({"summary": "shared metadata"}),
            producer: ArtifactProducerRef {
                task_id: node.task_instance_id.clone(),
                capability_instance_id: "write_metadata".into(),
                invocation_id: Some(format!("{}::invoke", claim.claim_id)),
                output_slot_id: Some("summary_doc".into()),
            },
        }]))
    }
}

#[test]
fn dispatch_rechecks_each_agents_authority_after_sharing_and_before_invocation() {
    let db = sled::Config::new().temporary(true).open().unwrap();
    let (catalog, first) = shared_input_request("first", true);
    let (_, second) = shared_input_request("second", true);
    let policy = AuthorityPolicy {
        policy_id: "shared-metadata".into(),
        principal_id: "workspace-owner".into(),
        subject: first
            .lineage
            .authority_decision
            .as_ref()
            .unwrap()
            .subject
            .clone(),
        principal_granted_action_ids: first.task.authority_requirements.clone(),
        runtime_allowed_action_ids: first.task.authority_requirements.clone(),
        restricted_action_ids: Vec::new(),
    };
    let policy =
        AuthorityPolicyBinding::new(policy.clone(), policy.content_hash().unwrap()).unwrap();
    let agents = Arc::new(Mutex::new(BTreeSet::from([
        "agent-a".into(),
        "agent-b".into(),
    ])));
    let mut store = SledTaskNetworkStore::open(db.clone(), "network-docs").unwrap();
    let mut admissions = Vec::new();
    for (mut request, agent_id) in [first, second].into_iter().zip(["agent-a", "agent-b"]) {
        request.lineage.agent_id = agent_id.into();
        request.lineage.authority_scope_id = policy.policy.policy_id.clone();
        request.lineage.authority_policy_content_hash = policy.content_hash.clone();
        let grant = request.lineage.authority_decision.as_mut().unwrap();
        grant.policy_id = policy.policy.policy_id.clone();
        grant.policy_content_hash = policy.content_hash.clone();
        grant.principal_id = policy.policy.principal_id.clone();
        let admission =
            TaskAdmissionApi::new(&mut store, &catalog, "generation-v1", &policy.content_hash)
                .admit(request)
                .unwrap();
        assert_eq!(admission.decision, TaskAdmissionDecision::Admitted);
        admissions.push(admission);
    }
    TaskAdmissionRuntimeActor::new(TaskAdmissionLowerer::new(
        TaskCompiler::new(),
        catalog.clone(),
    ))
    .run_once(
        &mut store,
        TaskAdmissionRuntimeRequest {
            network_id: "network-docs".into(),
            max_items: 2,
        },
    )
    .unwrap();
    let calls = Arc::new(Mutex::new(0));
    let actor =
        DispatchRuntimeActor::new("shared-worker", db.clone(), MetadataInvoker(calls.clone()))
            .unwrap()
            .with_capability_catalog(catalog)
            .with_authority_policy(policy)
            .with_admission_generation_observer(Arc::new(AgentGenerations(agents.clone())));
    let tick = |sequence| DispatchTickRequest {
        sequence,
        max_items: 1,
    };
    let sharing = futures::executor::block_on(actor.tick(&mut store, tick(1))).unwrap();
    assert!(sharing.fatal_errors.is_empty(), "{sharing:?}");
    assert_eq!(store.state().shared_steps.len(), 1);
    assert!(store.state().claims.is_empty());
    let contributor = store
        .state()
        .shared_steps
        .values()
        .next()
        .unwrap()
        .task_node
        .lineage
        .admission
        .as_ref()
        .unwrap()
        .agent_id
        .clone();
    agents.lock().unwrap().remove(&contributor);
    let denied = futures::executor::block_on(actor.tick(&mut store, tick(2))).unwrap();
    assert!(
        denied
            .fatal_errors
            .iter()
            .any(|issue| issue.code == "effective_authority_denied"),
        "{denied:?}"
    );
    assert!(store.state().claims.is_empty());
    assert_eq!(*calls.lock().unwrap(), 0);

    agents.lock().unwrap().insert(contributor);
    let completed = futures::executor::block_on(actor.tick(&mut store, tick(3))).unwrap();
    assert!(completed.fatal_errors.is_empty(), "{completed:?}");
    assert!(completed.retryable_errors.is_empty(), "{completed:?}");
    assert_eq!(*calls.lock().unwrap(), 1);
    assert_eq!(store.state().claims.len(), 1);
    assert_eq!(store.state().outcomes.len(), 1);
    let accounts: Vec<_> = admissions
        .iter()
        .map(|admission| {
            let account =
                admission_discharge_account(store.state(), &admission.admission_id).unwrap();
            assert_eq!(
                account.admission,
                meld_execution::task_network::TaskAdmissionAttribution::from_record(admission)
            );
            assert!(store
                .state()
                .publications
                .values()
                .any(|publication| publication.shared_discharge_accounts.contains(&account)));
            account
        })
        .collect();
    assert_ne!(
        accounts[0].admission.agent_id,
        accounts[1].admission.agent_id
    );
    assert_ne!(accounts[0].admission.goal_id, accounts[1].admission.goal_id);
    assert_eq!(accounts[0].outcome_id, accounts[1].outcome_id);
    store.flush().unwrap();
    drop(store);
    let mut reopened = SledTaskNetworkStore::open(db, "network-docs").unwrap();
    for account in accounts {
        assert_eq!(
            admission_discharge_account(reopened.state(), &account.admission.admission_id),
            Some(account)
        );
    }
    let replay = futures::executor::block_on(actor.tick(&mut reopened, tick(4))).unwrap();
    assert!(replay.fatal_errors.is_empty(), "{replay:?}");
    assert_eq!(replay.items_attempted, 0);
    assert_eq!(*calls.lock().unwrap(), 1);
}
