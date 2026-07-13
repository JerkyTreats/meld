use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use meld_lang::{Condition, GoalLifecycle, Proposition, Term, WorldState};
use meld_world_model::activation::{
    AgentCurationRuleRecord, DirectiveRecord, SeedAgentActivation, WorldModelActivationInput,
};
use meld_world_model::agent::{
    AgentBootstrapRuntime, AgentCurationRuleConfig, AgentHydrationActor, AgentHydrationCheckpoint,
    AgentHydrationTickReport, AgentHydrationTickRequest, AgentProcessHydrationStatus,
    AgentReadinessProof, AgentStatus, AgentStore,
};
use meld_world_model::belief::{
    AssessmentLease, BeliefFamilyConfig, BeliefKey, BeliefProvenanceSummary,
    BeliefReadinessAttestation, BeliefReadinessSnapshot, BeliefRevision, BeliefStore, BranchScope,
    ContradictionState, FreshnessState, LeaseStatus, PlannerProjectionSummary, PosteriorSummary,
};
use meld_world_model::events::DomainObjectRef;
use meld_world_model::planner::{
    PlannerHydrationRefs, PlannerProjectionActor, PlannerProjectionFrame,
    PlannerProjectionFrameIdentity, PlannerProjectionOutput, PlannerProjectionRequest,
    PlannerProjectionRequestRecord, PlannerProjectionRequestStatus, PlannerProjectionStore,
    PlannerProjectionTickRequest, PlannerSourceRef, PLANNER_PROJECTION_VERSION,
};
use meld_world_model::world_state::graph::store::TraversalStore;
use meld_world_model::{BeliefStatus, PerspectiveKey};
use serde::Serialize;

fn reopen(path: &Path) -> sled::Db {
    for attempt in 0..50 {
        match sled::open(path) {
            Ok(db) => return db,
            Err(error) if error.to_string().contains("could not acquire lock") && attempt < 49 => {
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(error) => panic!("cannot reopen fixture: {error}"),
        }
    }
    unreachable!()
}

fn activation(suffix: &str) -> WorldModelActivationInput {
    let dimension = format!("family_{suffix}");
    let subject = DomainObjectRef::new("workspace_fs", "node", format!("node-{suffix}")).unwrap();
    let perspective = PerspectiveKey::new("agent", suffix).unwrap();
    let branch_scope = BranchScope::main();
    let belief_family: BeliefFamilyConfig = serde_json::from_value(serde_json::json!({
        "family_id": dimension,
        "dimension_id": dimension,
        "predicate_id": "confidence",
        "evidence_policy_id": format!("policy-{suffix}"),
        "evidence_schemas": [{
            "schema_id": "content",
            "required": true,
            "role": "Support",
            "reliability": 1.0,
            "precision": 1.0
        }],
        "source_mappings": [{
            "mapping_id": "content",
            "source_kind": "content_written",
            "evidence_schema_id": "content",
            "subject_from": "record.subject",
            "value_field": "stale_probability",
            "factor_id": "freshness"
        }],
        "comparator": {
            "engine_id": "weighted_bayesian",
            "engine_version": "1",
            "factors": [{
                "factor_id": "freshness",
                "evidence_schema_id": "content",
                "weight": 1.0,
                "polarity": "Supports"
            }],
            "missing_evidence_uncertainty": 0.5
        },
        "default_prior": 0.8,
        "planner_projection": {
            "confidence_field": "confidence",
            "threshold": 0.7,
            "posterior_meaning": "stale_probability"
        },
        "config_version": "1"
    }))
    .unwrap();
    WorldModelActivationInput {
        activation_hash: suffix.repeat(64).chars().take(64).collect(),
        activation_id: format!("activation-{suffix}"),
        bootstrap_id: format!("bootstrap-{suffix}"),
        belief_family,
        directive: DirectiveRecord {
            directive_id: format!("directive-{suffix}"),
            text: format!("curate {dimension}"),
        },
        seed_agent: SeedAgentActivation {
            agent_id: format!("seed-{suffix}"),
            perspective_key: perspective.clone(),
            subject: subject.clone(),
            branch_scope: branch_scope.clone(),
            observation_scope: dimension.clone(),
            directive_id: format!("directive-{suffix}"),
            seed_provenance: "trusted init".to_string(),
        },
        curation_rule: AgentCurationRuleRecord {
            rule_id: format!("rule-{suffix}"),
            agent_id: format!("seed-{suffix}"),
            config: AgentCurationRuleConfig {
                dimension_id: dimension.clone(),
                threshold: 0.7,
                priority_urgency: 10,
                desired_summary: format!("{dimension} confidence above threshold"),
                source_kind: "belief_divergence".to_string(),
            },
        },
        belief_key: BeliefKey {
            subject,
            dimension_id: dimension,
            predicate_id: "confidence".to_string(),
            perspective,
            branch_scope,
            evidence_policy_id: format!("policy-{suffix}"),
        },
    }
}

fn seed_revision(
    db: &sled::Db,
    input: &WorldModelActivationInput,
    config_hash: &str,
    revision_id: &str,
    prior_revision_id: Option<String>,
    seq: u64,
    epoch: u64,
) {
    let store = BeliefStore::new(db.clone()).unwrap();
    seed_revision_in_store(
        &store,
        input,
        config_hash,
        revision_id,
        prior_revision_id,
        seq,
        epoch,
    );
}

fn seed_revision_in_store(
    store: &BeliefStore,
    input: &WorldModelActivationInput,
    config_hash: &str,
    revision_id: &str,
    prior_revision_id: Option<String>,
    seq: u64,
    _epoch: u64,
) {
    store.mark_dirty(&input.belief_key, seq).unwrap();
    let dirty = store
        .dirty_state(&input.belief_key)
        .unwrap()
        .expect("dirty belief fixture");
    let lease = store
        .acquire_lease(AssessmentLease {
            lease_id: format!("belief-lease-{revision_id}"),
            belief_key: input.belief_key.clone(),
            epoch: dirty.mutation_generation,
            owner_id: "test-assessment".to_string(),
            input_cursor_start: seq,
            input_cursor_end: seq,
            assignment_cursor_start: dirty.assessment_cursor.clone(),
            assignment_cursor_end: dirty.assessment_cursor,
            assignment_window_complete: true,
            started_at_seq: seq,
            expires_at_seq: seq + 20,
            comparator_engine_id: "weighted_bayesian".to_string(),
            config_snapshot_hash: config_hash.to_string(),
            status: LeaseStatus::Queued,
        })
        .unwrap();
    store
        .commit_revision(
            &lease,
            &BeliefRevision {
                revision_id: revision_id.to_string(),
                belief_key: input.belief_key.clone(),
                prior_revision_id,
                comparator_engine_id: "weighted_bayesian".to_string(),
                comparator_engine_version: "1".to_string(),
                config_snapshot_hash: config_hash.to_string(),
                evidence_ids: Vec::new(),
                supporting_evidence_ids: Vec::new(),
                contradicted_evidence_ids: Vec::new(),
                source_cursor_start: seq,
                source_cursor_end: seq,
                posterior: PosteriorSummary {
                    probability: 0.8,
                    meaning: "probability".to_string(),
                },
                planner_projection: PlannerProjectionSummary {
                    confidence_field: "confidence".to_string(),
                    confidence: 0.8,
                    threshold: 0.7,
                },
                uncertainty: 0.2,
                precision: 1.0,
                freshness: FreshnessState {
                    stale: false,
                    reasons: Vec::new(),
                    high_water_seq: seq,
                },
                contradiction: ContradictionState {
                    contradicted: false,
                    reasons: Vec::new(),
                    supporting_evidence_ids: Vec::new(),
                    contradicted_evidence_ids: Vec::new(),
                },
                status: BeliefStatus::Settled,
                observation: None,
                provenance: BeliefProvenanceSummary::empty(),
            },
        )
        .unwrap();
}

fn initialize(path: &Path, suffixes: &[&str]) {
    let db = reopen(path);
    let store = AgentStore::new(db.clone()).unwrap();
    let bootstrap = AgentBootstrapRuntime::from_agent_store(&store).unwrap();
    for suffix in suffixes {
        let input = activation(suffix);
        let report = bootstrap.bootstrap(&input).unwrap();
        seed_revision(
            &db,
            &input,
            &report.receipt.belief.config_snapshot_hash,
            &format!("revision-{suffix}-1"),
            None,
            20,
            1,
        );
    }
    store.flush().unwrap();
}

struct OpenRuntime {
    db: sled::Db,
    agent: Arc<AgentStore>,
    belief: Arc<BeliefStore>,
    planner: Arc<PlannerProjectionStore>,
}

impl OpenRuntime {
    fn open(path: &Path) -> Self {
        let db = reopen(path);
        Self {
            agent: AgentStore::shared(db.clone()).unwrap(),
            belief: BeliefStore::shared(db.clone()).unwrap(),
            planner: Arc::new(PlannerProjectionStore::new(db.clone()).unwrap()),
            db,
        }
    }

    fn hydration_tick(&self, lease_id: &str, max_items: usize) -> AgentHydrationTickReport {
        AgentHydrationActor::new(
            Arc::clone(&self.agent),
            Arc::clone(&self.belief),
            Arc::clone(&self.planner),
        )
        .tick(AgentHydrationTickRequest {
            lease_id: lease_id.to_string(),
            max_items,
        })
    }

    fn planner_tick(&self, max_items: usize) {
        let traversal = Arc::new(TraversalStore::new(self.db.clone()).unwrap());
        let report = PlannerProjectionActor::new(
            Arc::clone(&self.planner),
            Arc::clone(&self.belief),
            traversal,
        )
        .tick(PlannerProjectionTickRequest { max_items });
        assert!(report.retryable_errors.is_empty(), "{report:?}");
        assert!(report.fatal_errors.is_empty(), "{report:?}");
    }
}

fn only_hydration(
    runtime: &OpenRuntime,
    agent_id: &str,
) -> meld_world_model::AgentProcessHydrationRecord {
    let records = runtime
        .agent
        .process_hydrations_for_agent(agent_id)
        .unwrap();
    assert_eq!(records.len(), 1);
    records.into_iter().next().unwrap()
}

#[derive(Serialize)]
struct LegacyReadinessIdentity<'a> {
    agent_id: &'a str,
    subscription_id: &'a str,
    belief_key: &'a BeliefKey,
    belief_revision_id: &'a str,
    belief_view_id: &'a str,
    belief_view_hash: &'a str,
    attested_at_seq: u64,
}

#[derive(Serialize)]
struct LegacyReadinessWire<'a> {
    attestation_id: &'a str,
    agent_id: &'a str,
    subscription_id: &'a str,
    belief_key: &'a BeliefKey,
    belief_revision_id: &'a str,
    belief_view_id: &'a str,
    belief_view_hash: &'a str,
    attested_at_seq: u64,
}

fn compatibility_hash(domain: &[u8], value: &impl Serialize) -> String {
    let encoded = serde_json::to_vec(value).unwrap();
    let mut hasher = blake3::Hasher::new();
    hasher.update(domain);
    hasher.update(&encoded);
    hasher.finalize().to_hex().to_string()
}

fn replace_checkpoint_attestation_with_legacy(
    runtime: &OpenRuntime,
    checkpoint: &AgentHydrationCheckpoint,
) -> String {
    let current_id = checkpoint.attestation_id.as_deref().unwrap();
    let legacy_id = replace_attestation_with_legacy(runtime, current_id);
    let mut legacy_checkpoint = checkpoint.clone();
    legacy_checkpoint.attestation_id = Some(legacy_id.clone());
    runtime
        .db
        .open_tree("agent_hydration_checkpoints")
        .unwrap()
        .insert(
            checkpoint.hydration_id.as_bytes(),
            serde_json::to_vec(&legacy_checkpoint).unwrap(),
        )
        .unwrap();
    runtime.db.flush().unwrap();
    legacy_id
}

fn replace_attestation_with_legacy(runtime: &OpenRuntime, current_id: &str) -> String {
    let current = runtime
        .belief
        .get_readiness_attestation(current_id)
        .unwrap()
        .unwrap();
    let legacy_id = compatibility_hash(
        b"meld.belief-readiness-attestation.v1",
        &LegacyReadinessIdentity {
            agent_id: &current.agent_id,
            subscription_id: &current.subscription_id,
            belief_key: &current.belief_key,
            belief_revision_id: &current.belief_revision_id,
            belief_view_id: &current.belief_view_id,
            belief_view_hash: &current.belief_view_hash,
            attested_at_seq: current.attested_at_seq,
        },
    );
    let legacy = LegacyReadinessWire {
        attestation_id: &legacy_id,
        agent_id: &current.agent_id,
        subscription_id: &current.subscription_id,
        belief_key: &current.belief_key,
        belief_revision_id: &current.belief_revision_id,
        belief_view_id: &current.belief_view_id,
        belief_view_hash: &current.belief_view_hash,
        attested_at_seq: current.attested_at_seq,
    };
    let legacy_bytes = serde_json::to_vec(&legacy).unwrap();
    for tree_name in [
        "belief_readiness_attestation_intents",
        "belief_readiness_attestation_owner_fences",
        "belief_readiness_attestation_snapshots",
        "belief_readiness_attestations",
    ] {
        runtime
            .db
            .open_tree(tree_name)
            .unwrap()
            .remove(current_id.as_bytes())
            .unwrap();
    }
    runtime
        .db
        .open_tree("belief_readiness_attestations")
        .unwrap()
        .insert(legacy_id.as_bytes(), legacy_bytes)
        .unwrap();
    runtime.db.flush().unwrap();
    legacy_id
}

#[derive(Serialize)]
struct LegacySignalWire<'a> {
    signal_id: &'a str,
    attestation: LegacyReadinessWire<'a>,
}

#[derive(Serialize)]
struct LegacyProofIdentity<'a> {
    expected_agent_updated_at_seq: u64,
    signal: &'a LegacySignalWire<'a>,
    planner_request: &'a PlannerProjectionRequestRecord,
    planner_frame: &'a PlannerProjectionFrame,
    ground_goal: &'a meld_lang::Goal,
}

#[derive(Serialize)]
struct LegacyProofWire<'a> {
    proof_id: &'a str,
    expected_agent_updated_at_seq: u64,
    signal: LegacySignalWire<'a>,
    planner_request: &'a PlannerProjectionRequestRecord,
    planner_frame: &'a PlannerProjectionFrame,
    ground_goal: &'a meld_lang::Goal,
}

fn replace_readiness_proof_with_legacy(
    runtime: &OpenRuntime,
    hydration: &meld_world_model::AgentProcessHydrationRecord,
) -> String {
    let current_id = hydration.readiness_proof_id.as_deref().unwrap();
    let current = runtime
        .agent
        .get_readiness_proof(current_id)
        .unwrap()
        .unwrap();
    let current_attestation = current.signal.attestation.clone();
    let legacy_attestation_id =
        replace_attestation_with_legacy(runtime, &current_attestation.attestation_id);
    let legacy_attestation = LegacyReadinessWire {
        attestation_id: &legacy_attestation_id,
        agent_id: &current_attestation.agent_id,
        subscription_id: &current_attestation.subscription_id,
        belief_key: &current_attestation.belief_key,
        belief_revision_id: &current_attestation.belief_revision_id,
        belief_view_id: &current_attestation.belief_view_id,
        belief_view_hash: &current_attestation.belief_view_hash,
        attested_at_seq: current_attestation.attested_at_seq,
    };
    let signal_id = compatibility_hash(b"meld.agent-readiness-signal.v2", &legacy_attestation);
    let legacy_signal = LegacySignalWire {
        signal_id: &signal_id,
        attestation: legacy_attestation,
    };
    let request = &current.planner_request.request;
    let legacy_request = PlannerProjectionRequest::identified(
        request.source_request_hash.clone(),
        request.agent_id.clone(),
        request.subject.clone(),
        request.perspective.clone(),
        request.branch_scope.clone(),
        request.requested_dimensions.clone(),
        request.required_preconditions.clone(),
    )
    .unwrap();
    let mut planner_frame = current.planner_frame.clone();
    planner_frame.identity =
        PlannerProjectionFrameIdentity::identified(&legacy_request, &planner_frame.output).unwrap();
    let mut planner_request = current.planner_request.clone();
    planner_request.request = legacy_request;
    planner_request.frame_id = Some(planner_frame.identity.frame_id.clone());
    planner_request.validate().unwrap();
    planner_frame.validate().unwrap();
    let proof_id = compatibility_hash(
        b"meld.agent-readiness-proof.v2",
        &LegacyProofIdentity {
            expected_agent_updated_at_seq: current.expected_agent_updated_at_seq,
            signal: &legacy_signal,
            planner_request: &planner_request,
            planner_frame: &planner_frame,
            ground_goal: &current.ground_goal,
        },
    );
    let legacy_proof = LegacyProofWire {
        proof_id: &proof_id,
        expected_agent_updated_at_seq: current.expected_agent_updated_at_seq,
        signal: legacy_signal,
        planner_request: &planner_request,
        planner_frame: &planner_frame,
        ground_goal: &current.ground_goal,
    };
    let proofs = runtime.db.open_tree("agent_readiness_proofs").unwrap();
    proofs.remove(current_id.as_bytes()).unwrap();
    proofs
        .insert(
            proof_id.as_bytes(),
            serde_json::to_vec(&legacy_proof).unwrap(),
        )
        .unwrap();
    let mut legacy_hydration = hydration.clone();
    legacy_hydration.readiness_proof_id = Some(proof_id.clone());
    runtime
        .db
        .open_tree("agent_process_hydrations")
        .unwrap()
        .insert(
            hydration.hydration_id.as_bytes(),
            serde_json::to_vec(&legacy_hydration).unwrap(),
        )
        .unwrap();
    runtime.db.flush().unwrap();
    proof_id
}

fn forged_attested_frame(record: &PlannerProjectionRequestRecord) -> PlannerProjectionFrame {
    let revision_id = record
        .request
        .attested_belief
        .as_ref()
        .unwrap()
        .revision_id
        .clone();
    let output = PlannerProjectionOutput {
        world_state: WorldState::new(Vec::new()).unwrap(),
        projection_version: PLANNER_PROJECTION_VERSION.to_string(),
        source_refs: vec![PlannerSourceRef::BeliefRevision {
            revision_id: revision_id.clone(),
        }],
        hydration_refs: PlannerHydrationRefs {
            revision_ids: vec![revision_id],
            ..PlannerHydrationRefs::default()
        },
        warnings: Vec::new(),
    };
    PlannerProjectionFrame {
        identity: PlannerProjectionFrameIdentity::identified(&record.request, &output).unwrap(),
        output,
        completed_at_seq: record.updated_at_seq + 1,
    }
}

#[test]
fn hydration_starts_waits_completes_and_replays_exactly() {
    let temp = tempfile::tempdir().unwrap();
    initialize(temp.path(), &["a"]);
    let runtime = OpenRuntime::open(temp.path());

    let start = runtime.hydration_tick("lease-a", 1);
    assert_eq!(start.started_count, 1);
    assert!(start.work_performed());
    let attested = runtime.hydration_tick("lease-a", 1);
    assert_eq!(attested.attested_count, 1);
    let requested = runtime.hydration_tick("lease-a", 1);
    assert_eq!(requested.projection_requested_count, 1);
    let pending = runtime.hydration_tick("lease-a", 1);
    assert_eq!(pending.projection_pending_count, 1);
    assert!(!pending.work_performed());
    assert_eq!(
        pending.retryable_errors[0].code,
        "planner_projection_pending"
    );

    runtime.planner_tick(1);
    let completed = runtime.hydration_tick("lease-a", 1);
    assert_eq!(completed.operational_count, 1);
    let replay = runtime.hydration_tick("lease-a", 1);
    assert_eq!(replay.selected_count, 0);
    assert!(!replay.work_performed());

    let agent = runtime.agent.get_agent("seed-a").unwrap().unwrap();
    assert_eq!(agent.status, AgentStatus::Operational);
    let hydration = only_hydration(&runtime, "seed-a");
    assert_eq!(hydration.status, AgentProcessHydrationStatus::Ready);
    let proof = runtime
        .agent
        .get_readiness_proof(hydration.readiness_proof_id.as_deref().unwrap())
        .unwrap()
        .unwrap();
    assert!(proof.ground_goal.target.is_ground());
    assert_eq!(proof.ground_goal.lifecycle, GoalLifecycle::Proposed);
    assert!(matches!(
        proof.ground_goal.target,
        Proposition::Holds {
            dimension: Term::Dimension(ref dimension),
            condition: Condition::Above(_),
            ..
        } if dimension == "family_a"
    ));
}

#[test]
fn public_planner_terminal_methods_cannot_resolve_attested_hydration() {
    let temp = tempfile::tempdir().unwrap();
    initialize(temp.path(), &["a"]);
    let runtime = OpenRuntime::open(temp.path());
    for _ in 0..3 {
        runtime.hydration_tick("lease-public-terminal", 1);
    }
    let hydration = only_hydration(&runtime, "seed-a");
    let checkpoint = runtime
        .agent
        .hydration_checkpoint(&hydration.hydration_id)
        .unwrap()
        .unwrap();
    let request_id = checkpoint.planner_request_id.unwrap();
    let pending = runtime.planner.get_request(&request_id).unwrap().unwrap();
    let forged = forged_attested_frame(&pending);
    let forged_frame_id = forged.identity.frame_id.clone();

    assert!(matches!(
        runtime.planner.complete(
            &request_id,
            pending.updated_at_seq,
            forged,
            pending.updated_at_seq + 1,
        ),
        Err(meld_world_model::error::StorageError::InvalidPath(message))
            if message.contains("actor terminal authority")
    ));
    assert!(matches!(
        runtime.planner.fail(
            &request_id,
            pending.updated_at_seq,
            "forged public failure",
            pending.updated_at_seq + 1,
        ),
        Err(meld_world_model::error::StorageError::InvalidPath(message))
            if message.contains("actor terminal authority")
    ));
    assert_eq!(
        runtime.planner.get_request(&request_id).unwrap(),
        Some(pending)
    );
    assert!(runtime
        .planner
        .get_frame(&forged_frame_id)
        .unwrap()
        .is_none());
    assert_eq!(
        runtime
            .planner
            .pending_requests_bounded(1)
            .unwrap()
            .records
            .len(),
        1
    );

    runtime.planner_tick(1);
    let completed = runtime.hydration_tick("lease-public-terminal", 1);
    assert_eq!(completed.operational_count, 1, "{completed:?}");
}

#[test]
fn every_durable_boundary_reopens_and_converges() {
    let temp = tempfile::tempdir().unwrap();
    initialize(temp.path(), &["a"]);

    for expected in ["started", "attested", "requested"] {
        let runtime = OpenRuntime::open(temp.path());
        let report = runtime.hydration_tick("lease-reopen", 1);
        match expected {
            "started" => assert_eq!(report.started_count, 1),
            "attested" => assert_eq!(report.attested_count, 1),
            "requested" => assert_eq!(report.projection_requested_count, 1),
            _ => unreachable!(),
        }
    }
    {
        let runtime = OpenRuntime::open(temp.path());
        let pending = runtime.hydration_tick("lease-reopen", 1);
        assert_eq!(pending.projection_pending_count, 1);
        runtime.planner_tick(1);
    }
    {
        let runtime = OpenRuntime::open(temp.path());
        let completed = runtime.hydration_tick("lease-reopen", 1);
        assert_eq!(completed.operational_count, 1);
        assert_eq!(
            runtime.agent.get_agent("seed-a").unwrap().unwrap().status,
            AgentStatus::Operational
        );
    }
}

#[test]
fn accepted_w3a_attestation_reopens_migrates_and_resumes_hydration() {
    let temp = tempfile::tempdir().unwrap();
    initialize(temp.path(), &["a"]);
    let legacy_id;
    {
        let runtime = OpenRuntime::open(temp.path());
        assert_eq!(runtime.hydration_tick("lease-legacy", 1).started_count, 1);
        assert_eq!(runtime.hydration_tick("lease-legacy", 1).attested_count, 1);
        let hydration = only_hydration(&runtime, "seed-a");
        let checkpoint = runtime
            .agent
            .hydration_checkpoint(&hydration.hydration_id)
            .unwrap()
            .unwrap();
        legacy_id = replace_checkpoint_attestation_with_legacy(&runtime, &checkpoint);
    }

    {
        let runtime = OpenRuntime::open(temp.path());
        let migrated: BeliefReadinessAttestation = runtime
            .belief
            .get_readiness_attestation(&legacy_id)
            .unwrap()
            .unwrap();
        assert_eq!(migrated.attestation_id, legacy_id);
        assert_eq!(migrated.schema_version, 2);
        assert_eq!(migrated.identity_version, 1);
        runtime
            .belief
            .readiness_snapshot(&legacy_id)
            .unwrap()
            .unwrap()
            .validate()
            .unwrap();
        let owner_fences = runtime
            .db
            .open_tree("belief_readiness_attestation_owner_fences")
            .unwrap();
        assert_eq!(
            owner_fences.get(legacy_id.as_bytes()).unwrap().as_deref(),
            Some(b"legacy-unfenced".as_slice())
        );
        let requested = runtime.hydration_tick("lease-legacy", 1);
        assert_eq!(requested.projection_requested_count, 1, "{requested:?}");
        let claimed = owner_fences.get(legacy_id.as_bytes()).unwrap().unwrap();
        assert_eq!(claimed.len(), 64);
        assert_ne!(claimed.as_ref(), b"legacy-unfenced");
        runtime.planner_tick(1);
        let completed = runtime.hydration_tick("lease-legacy", 1);
        assert_eq!(completed.operational_count, 1, "{completed:?}");
        let hydration = only_hydration(&runtime, "seed-a");
        assert_eq!(hydration.status, AgentProcessHydrationStatus::Ready);
    }

    let reopened = OpenRuntime::open(temp.path());
    assert_eq!(
        reopened.agent.get_agent("seed-a").unwrap().unwrap().status,
        AgentStatus::Operational
    );
    assert_eq!(
        reopened
            .belief
            .get_readiness_attestation(&legacy_id)
            .unwrap()
            .unwrap()
            .attestation_id,
        legacy_id
    );
}

#[test]
fn accepted_w3a_embedded_readiness_proof_reopens_with_stable_identity() {
    let temp = tempfile::tempdir().unwrap();
    initialize(temp.path(), &["a"]);
    let legacy_proof_id;
    {
        let runtime = OpenRuntime::open(temp.path());
        for _ in 0..3 {
            runtime.hydration_tick("lease-proof-legacy", 1);
        }
        runtime.planner_tick(1);
        let completed = runtime.hydration_tick("lease-proof-legacy", 1);
        assert_eq!(completed.operational_count, 1, "{completed:?}");
        let hydration = only_hydration(&runtime, "seed-a");
        legacy_proof_id = replace_readiness_proof_with_legacy(&runtime, &hydration);
    }

    {
        let runtime = OpenRuntime::open(temp.path());
        let hydration = only_hydration(&runtime, "seed-a");
        assert_eq!(
            hydration.readiness_proof_id.as_deref(),
            Some(legacy_proof_id.as_str())
        );
        let proof = runtime
            .agent
            .get_readiness_proof(&legacy_proof_id)
            .unwrap()
            .unwrap();
        assert_eq!(proof.proof_id, legacy_proof_id);
        assert_eq!(proof.schema_version, 2);
        assert_eq!(proof.identity_version, 1);
        assert_eq!(proof.signal.schema_version, 2);
        assert_eq!(proof.signal.identity_version, 1);
        assert_eq!(proof.signal.attestation.schema_version, 2);
        assert_eq!(proof.signal.attestation.identity_version, 1);
        proof.validate().unwrap();
    }

    let reopened = OpenRuntime::open(temp.path());
    assert_eq!(
        reopened
            .agent
            .get_readiness_proof(&legacy_proof_id)
            .unwrap()
            .unwrap()
            .proof_id,
        legacy_proof_id
    );
}

#[test]
fn current_attested_checkpoint_reopen_rejects_missing_or_divergent_products() {
    for corruption in ["missing-owner", "divergent-snapshot"] {
        let temp = tempfile::tempdir().unwrap();
        initialize(temp.path(), &["a"]);
        let attestation_id = {
            let runtime = OpenRuntime::open(temp.path());
            assert_eq!(runtime.hydration_tick("lease-corrupt", 1).started_count, 1);
            assert_eq!(runtime.hydration_tick("lease-corrupt", 1).attested_count, 1);
            let hydration = only_hydration(&runtime, "seed-a");
            runtime
                .agent
                .hydration_checkpoint(&hydration.hydration_id)
                .unwrap()
                .unwrap()
                .attestation_id
                .unwrap()
        };
        let db = reopen(temp.path());
        match corruption {
            "missing-owner" => {
                db.open_tree("belief_readiness_attestation_owner_fences")
                    .unwrap()
                    .remove(attestation_id.as_bytes())
                    .unwrap();
            }
            "divergent-snapshot" => {
                let snapshots = db
                    .open_tree("belief_readiness_attestation_snapshots")
                    .unwrap();
                let raw = snapshots.get(attestation_id.as_bytes()).unwrap().unwrap();
                let mut snapshot: BeliefReadinessSnapshot = serde_json::from_slice(&raw).unwrap();
                snapshot.view.view_id.push_str("-drift");
                snapshots
                    .insert(
                        attestation_id.as_bytes(),
                        serde_json::to_vec(&snapshot).unwrap(),
                    )
                    .unwrap();
            }
            _ => unreachable!(),
        }
        db.flush().unwrap();
        drop(db);

        let runtime = OpenRuntime::open(temp.path());
        let report = runtime.hydration_tick("lease-corrupt", 1);
        assert_eq!(report.fatal_errors.len(), 1, "{corruption}: {report:?}");
        assert_eq!(
            runtime.agent.get_agent("seed-a").unwrap().unwrap().status,
            AgentStatus::Registered
        );
        if corruption == "missing-owner" {
            assert!(runtime
                .db
                .open_tree("belief_readiness_attestation_owner_fences")
                .unwrap()
                .get(attestation_id.as_bytes())
                .unwrap()
                .is_none());
        }
    }
}

#[test]
fn current_operational_proof_reopen_rejects_missing_or_divergent_belief_products() {
    for corruption in ["missing-attestation", "divergent-snapshot"] {
        let temp = tempfile::tempdir().unwrap();
        initialize(temp.path(), &["a"]);
        let (proof_id, attestation_id) = {
            let runtime = OpenRuntime::open(temp.path());
            for _ in 0..3 {
                runtime.hydration_tick("lease-proof-corrupt", 1);
            }
            runtime.planner_tick(1);
            assert_eq!(
                runtime
                    .hydration_tick("lease-proof-corrupt", 1)
                    .operational_count,
                1
            );
            let hydration = only_hydration(&runtime, "seed-a");
            let proof_id = hydration.readiness_proof_id.unwrap();
            let proof = runtime
                .agent
                .get_readiness_proof(&proof_id)
                .unwrap()
                .unwrap();
            (proof_id, proof.signal.attestation.attestation_id)
        };
        let db = reopen(temp.path());
        match corruption {
            "missing-attestation" => {
                db.open_tree("belief_readiness_attestations")
                    .unwrap()
                    .remove(attestation_id.as_bytes())
                    .unwrap();
            }
            "divergent-snapshot" => {
                let snapshots = db
                    .open_tree("belief_readiness_attestation_snapshots")
                    .unwrap();
                let raw = snapshots.get(attestation_id.as_bytes()).unwrap().unwrap();
                let mut snapshot: BeliefReadinessSnapshot = serde_json::from_slice(&raw).unwrap();
                snapshot.revision.revision_id.push_str("-drift");
                snapshots
                    .insert(
                        attestation_id.as_bytes(),
                        serde_json::to_vec(&snapshot).unwrap(),
                    )
                    .unwrap();
            }
            _ => unreachable!(),
        }
        db.flush().unwrap();
        drop(db);

        let runtime = OpenRuntime::open(temp.path());
        assert!(runtime.agent.get_readiness_proof(&proof_id).is_err());
    }
}

#[test]
fn agent_readiness_schema_migration_spans_multiple_batches_and_reopens_complete() {
    let temp = tempfile::tempdir().unwrap();
    initialize(temp.path(), &["a"]);
    let proof_ids = {
        let runtime = OpenRuntime::open(temp.path());
        for _ in 0..3 {
            runtime.hydration_tick("lease-proof-migration", 1);
        }
        runtime.planner_tick(1);
        assert_eq!(
            runtime
                .hydration_tick("lease-proof-migration", 1)
                .operational_count,
            1
        );
        let hydration = only_hydration(&runtime, "seed-a");
        let base = runtime
            .agent
            .get_readiness_proof(hydration.readiness_proof_id.as_deref().unwrap())
            .unwrap()
            .unwrap();
        let proofs = runtime.db.open_tree("agent_readiness_proofs").unwrap();
        let mut proof_ids = Vec::with_capacity(300);
        for index in 1..=300_u64 {
            let proof = AgentReadinessProof::identified(
                base.expected_agent_updated_at_seq + index,
                base.signal.clone(),
                base.planner_request.clone(),
                base.planner_frame.clone(),
                base.ground_goal.clone(),
            )
            .unwrap();
            proofs
                .insert(
                    proof.proof_id.as_bytes(),
                    serde_json::to_vec(&proof).unwrap(),
                )
                .unwrap();
            proof_ids.push(proof.proof_id);
        }
        runtime
            .db
            .open_tree("agent_readiness_schema")
            .unwrap()
            .remove(b"state")
            .unwrap();
        runtime.db.flush().unwrap();
        proof_ids
    };

    let runtime = OpenRuntime::open(temp.path());
    let state: serde_json::Value = serde_json::from_slice(
        &runtime
            .db
            .open_tree("agent_readiness_schema")
            .unwrap()
            .get(b"state")
            .unwrap()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(state["schema_version"], 2);
    assert_eq!(state["complete"], true);
    assert!(state["cursor"].is_null());
    for index in [0, 127, 128, 255, 299] {
        assert!(runtime
            .agent
            .get_readiness_proof(&proof_ids[index])
            .unwrap()
            .is_some());
    }
}

#[test]
fn operational_proof_reuse_uses_narrow_belief_reader_and_rejects_alias_key() {
    let temp = tempfile::tempdir().unwrap();
    initialize(temp.path(), &["a"]);
    let proof_id = {
        let runtime = OpenRuntime::open(temp.path());
        for _ in 0..3 {
            runtime.hydration_tick("lease-proof-reader", 1);
        }
        runtime.planner_tick(1);
        assert_eq!(
            runtime
                .hydration_tick("lease-proof-reader", 1)
                .operational_count,
            1
        );
        let hydration = only_hydration(&runtime, "seed-a");
        let proof_id = hydration.readiness_proof_id.unwrap();
        let proofs = runtime.db.open_tree("agent_readiness_proofs").unwrap();
        let encoded = proofs.get(proof_id.as_bytes()).unwrap().unwrap();
        proofs.insert(b"proof-alias", encoded).unwrap();
        runtime
            .db
            .open_tree("belief_commit_intents")
            .unwrap()
            .insert(b"unrelated-malformed-intent", b"{")
            .unwrap();
        runtime.db.flush().unwrap();
        proof_id
    };

    let db = reopen(temp.path());
    let agent = AgentStore::new(db).unwrap();
    assert!(agent.get_readiness_proof(&proof_id).unwrap().is_some());
    assert!(agent.get_readiness_proof("proof-alias").is_err());
}

#[test]
fn replacement_lease_supersedes_once_and_stale_writer_fails_closed() {
    let temp = tempfile::tempdir().unwrap();
    initialize(temp.path(), &["a"]);
    let runtime = OpenRuntime::open(temp.path());
    assert_eq!(runtime.hydration_tick("lease-old", 1).started_count, 1);
    assert_eq!(runtime.hydration_tick("lease-new", 1).started_count, 1);

    let stale = runtime.hydration_tick("lease-old", 1);
    assert_eq!(stale.fatal_errors[0].code, "stale_lease");
    let current = runtime
        .agent
        .current_process_hydration("seed-a")
        .unwrap()
        .unwrap();
    assert_eq!(current.lease_id, "lease-new");
    assert_eq!(current.attempt_epoch, 2);
}

#[test]
fn changed_view_and_failed_projection_fail_the_exact_epoch() {
    let temp = tempfile::tempdir().unwrap();
    initialize(temp.path(), &["a"]);
    let input = activation("a");
    let runtime = OpenRuntime::open(temp.path());
    assert_eq!(runtime.hydration_tick("lease-view", 1).started_count, 1);
    let config_hash = runtime
        .agent
        .bootstrap_receipt_for_agent("seed-a")
        .unwrap()
        .unwrap()
        .belief
        .config_snapshot_hash
        .clone();
    seed_revision(
        &runtime.db,
        &input,
        &config_hash,
        "revision-a-2",
        Some("revision-a-1".to_string()),
        30,
        2,
    );
    let changed = runtime.hydration_tick("lease-view", 1);
    assert_eq!(changed.failed_count, 1);
    assert_eq!(changed.fatal_errors[0].code, "belief_view_changed");
    assert!(changed.input_sequence >= 30);

    let temp = tempfile::tempdir().unwrap();
    initialize(temp.path(), &["a"]);
    let runtime = OpenRuntime::open(temp.path());
    let input = activation("a");
    let mut view = runtime
        .belief
        .current_view(&input.belief_key)
        .unwrap()
        .unwrap();
    view.planner_projection.confidence_field = "invalid field".to_string();
    runtime.belief.put_view(&view).unwrap();
    runtime.hydration_tick("lease-failed", 1);
    runtime.hydration_tick("lease-failed", 1);
    runtime.hydration_tick("lease-failed", 1);
    let traversal = Arc::new(TraversalStore::new(runtime.db.clone()).unwrap());
    let projection = PlannerProjectionActor::new(
        Arc::clone(&runtime.planner),
        Arc::clone(&runtime.belief),
        traversal,
    )
    .tick(PlannerProjectionTickRequest { max_items: 1 });
    assert_eq!(projection.failed_count, 1, "{projection:?}");
    let failed = runtime.hydration_tick("lease-failed", 1);
    assert_eq!(failed.failed_count, 1);
    assert_eq!(failed.fatal_errors[0].code, "planner_projection_failed");
}

#[test]
fn missing_rule_and_divergent_request_are_fatal_without_activation() {
    let temp = tempfile::tempdir().unwrap();
    initialize(temp.path(), &["a"]);
    let runtime = OpenRuntime::open(temp.path());
    runtime
        .db
        .open_tree("agent_curation_rules")
        .unwrap()
        .remove(b"rule-a")
        .unwrap();
    let missing = runtime.hydration_tick("lease-a", 1);
    assert_eq!(missing.fatal_errors[0].code, "missing_curation_rule");
    assert_eq!(
        runtime.agent.get_agent("seed-a").unwrap().unwrap().status,
        AgentStatus::Registered
    );

    let temp = tempfile::tempdir().unwrap();
    initialize(temp.path(), &["a"]);
    let runtime = OpenRuntime::open(temp.path());
    runtime.hydration_tick("lease-a", 1);
    runtime.hydration_tick("lease-a", 1);
    runtime.hydration_tick("lease-a", 1);
    let hydration = only_hydration(&runtime, "seed-a");
    let checkpoint = runtime
        .agent
        .hydration_checkpoint(&hydration.hydration_id)
        .unwrap()
        .unwrap();
    let request_id = checkpoint.planner_request_id.unwrap();
    let tree = runtime.db.open_tree("planner_projection_requests").unwrap();
    let raw = tree.get(request_id.as_bytes()).unwrap().unwrap();
    let mut record: PlannerProjectionRequestRecord = serde_json::from_slice(&raw).unwrap();
    record.created_at_seq += 5;
    record.updated_at_seq += 5;
    tree.insert(request_id.as_bytes(), serde_json::to_vec(&record).unwrap())
        .unwrap();
    let divergent = runtime.hydration_tick("lease-a", 1);
    assert_eq!(divergent.fatal_errors[0].code, "divergent_planner_request");
    assert_eq!(
        runtime.agent.get_agent("seed-a").unwrap().unwrap().status,
        AgentStatus::Registered
    );
}

#[test]
fn actor_order_converges_and_budget_exhaustion_is_truthful() {
    fn complete(path: &Path, planner_first: bool) -> meld_world_model::AgentReadinessProof {
        initialize(path, &["a"]);
        let runtime = OpenRuntime::open(path);
        if planner_first {
            runtime.planner_tick(1);
        }
        runtime.hydration_tick("lease-order", 1);
        runtime.hydration_tick("lease-order", 1);
        runtime.hydration_tick("lease-order", 1);
        if !planner_first {
            let pending = runtime.hydration_tick("lease-order", 1);
            assert_eq!(pending.projection_pending_count, 1);
        }
        runtime.planner_tick(1);
        runtime.hydration_tick("lease-order", 1);
        let hydration = only_hydration(&runtime, "seed-a");
        runtime
            .agent
            .get_readiness_proof(hydration.readiness_proof_id.as_deref().unwrap())
            .unwrap()
            .unwrap()
    }

    let left = tempfile::tempdir().unwrap();
    let right = tempfile::tempdir().unwrap();
    assert_eq!(complete(left.path(), false), complete(right.path(), true));

    let budget = tempfile::tempdir().unwrap();
    initialize(budget.path(), &["a", "b"]);
    let runtime = OpenRuntime::open(budget.path());
    let first = runtime.hydration_tick("lease-budget", 1);
    assert_eq!(first.selected_count, 1);
    assert_eq!(first.started_count, 1);
    assert!(first.budget_exhausted);
    assert_eq!(
        runtime
            .agent
            .agents_by_status(AgentStatus::Registered)
            .unwrap()
            .len(),
        2
    );
    let second = runtime.hydration_tick("lease-budget", 1);
    assert_eq!(second.selected_count, 1);
    assert_eq!(second.started_count, 1);
    assert_eq!(
        runtime
            .agent
            .process_hydrations_for_agent("seed-b")
            .unwrap()
            .len(),
        1
    );
    drop(runtime);
    let reopened = OpenRuntime::open(budget.path());
    let wrapped = reopened.hydration_tick("lease-budget", 1);
    assert_eq!(wrapped.attested_count, 1);
}

#[test]
fn causal_sequences_advance_through_failure_and_replacement() {
    let temp = tempfile::tempdir().unwrap();
    initialize(temp.path(), &["a"]);
    let runtime = OpenRuntime::open(temp.path());

    runtime.hydration_tick("lease-old", 1);
    let old = only_hydration(&runtime, "seed-a");
    let selected = runtime
        .agent
        .hydration_checkpoint(&old.hydration_id)
        .unwrap()
        .unwrap();
    assert!(selected.updated_at_seq > old.started_at_seq);

    runtime.hydration_tick("lease-old", 1);
    let attested_checkpoint = runtime
        .agent
        .hydration_checkpoint(&old.hydration_id)
        .unwrap()
        .unwrap();
    let attestation = runtime
        .belief
        .get_readiness_attestation(attested_checkpoint.attestation_id.as_deref().unwrap())
        .unwrap()
        .unwrap();
    assert!(attestation.attested_at_seq > selected.updated_at_seq);
    assert!(attested_checkpoint.updated_at_seq > attestation.attested_at_seq);

    runtime.hydration_tick("lease-old", 1);
    let requested_checkpoint = runtime
        .agent
        .hydration_checkpoint(&old.hydration_id)
        .unwrap()
        .unwrap();
    let request = runtime
        .planner
        .get_request(requested_checkpoint.planner_request_id.as_deref().unwrap())
        .unwrap()
        .unwrap();
    assert!(request.created_at_seq > attested_checkpoint.updated_at_seq);
    assert!(requested_checkpoint.updated_at_seq > request.created_at_seq);

    let replacement = runtime.hydration_tick("lease-new", 1);
    assert_eq!(replacement.started_count, 1);
    let current = runtime
        .agent
        .current_process_hydration("seed-a")
        .unwrap()
        .unwrap();
    assert_eq!(current.attempt_epoch, 2);
    assert!(current.started_at_seq > requested_checkpoint.updated_at_seq);
    assert!(current.started_at_seq > request.updated_at_seq);

    let temp = tempfile::tempdir().unwrap();
    initialize(temp.path(), &["a"]);
    let runtime = OpenRuntime::open(temp.path());
    let input = activation("a");
    let mut view = runtime
        .belief
        .current_view(&input.belief_key)
        .unwrap()
        .unwrap();
    view.planner_projection.confidence_field = "invalid field".to_string();
    runtime.belief.put_view(&view).unwrap();
    runtime.hydration_tick("lease-failure", 1);
    runtime.hydration_tick("lease-failure", 1);
    runtime.hydration_tick("lease-failure", 1);
    let hydration = only_hydration(&runtime, "seed-a");
    let checkpoint = runtime
        .agent
        .hydration_checkpoint(&hydration.hydration_id)
        .unwrap()
        .unwrap();
    let request_id = checkpoint.planner_request_id.as_deref().unwrap();
    let pending = runtime.planner.get_request(request_id).unwrap().unwrap();
    let traversal = Arc::new(TraversalStore::new(runtime.db.clone()).unwrap());
    let projection = PlannerProjectionActor::new(
        Arc::clone(&runtime.planner),
        Arc::clone(&runtime.belief),
        traversal,
    )
    .tick(PlannerProjectionTickRequest { max_items: 1 });
    assert_eq!(projection.failed_count, 1, "{projection:?}");
    let failed_request = runtime.planner.get_request(request_id).unwrap().unwrap();
    assert!(failed_request.updated_at_seq > pending.updated_at_seq);
    let report = runtime.hydration_tick("lease-failure", 1);
    assert_eq!(report.failed_count, 1);
    let failed_hydration = only_hydration(&runtime, "seed-a");
    assert!(failed_hydration.updated_at_seq > checkpoint.updated_at_seq);
    assert!(failed_hydration.updated_at_seq > failed_request.updated_at_seq);
    assert!(report.output_sequence >= failed_hydration.updated_at_seq);
}

#[test]
fn malformed_owner_products_are_fatal_without_operational_transition() {
    let temp = tempfile::tempdir().unwrap();
    initialize(temp.path(), &["a"]);
    let runtime = OpenRuntime::open(temp.path());
    let receipt_tree = runtime.db.open_tree("agent_bootstrap_receipts").unwrap();
    let raw = receipt_tree.get(b"bootstrap-a").unwrap().unwrap();
    let mut receipt: meld_world_model::activation::AgentBootstrapReceipt =
        serde_json::from_slice(&raw).unwrap();
    receipt.receipt_id = "divergent-receipt".to_string();
    receipt_tree
        .insert(b"bootstrap-a", serde_json::to_vec(&receipt).unwrap())
        .unwrap();
    let malformed_receipt = runtime.hydration_tick("lease-a", 1);
    assert_eq!(malformed_receipt.fatal_errors[0].code, "storage_fatal");
    assert!(malformed_receipt.retryable_errors.is_empty());

    let temp = tempfile::tempdir().unwrap();
    initialize(temp.path(), &["a"]);
    let runtime = OpenRuntime::open(temp.path());
    runtime.hydration_tick("lease-a", 1);
    let hydration = only_hydration(&runtime, "seed-a");
    let hydration_tree = runtime.db.open_tree("agent_process_hydrations").unwrap();
    let raw = hydration_tree
        .get(hydration.hydration_id.as_bytes())
        .unwrap()
        .unwrap();
    let mut malformed: meld_world_model::AgentProcessHydrationRecord =
        serde_json::from_slice(&raw).unwrap();
    malformed.attempt_epoch = 0;
    hydration_tree
        .insert(
            hydration.hydration_id.as_bytes(),
            serde_json::to_vec(&malformed).unwrap(),
        )
        .unwrap();
    let malformed_hydration = runtime.hydration_tick("lease-a", 1);
    assert_eq!(malformed_hydration.fatal_errors[0].code, "storage_fatal");
    assert!(malformed_hydration.retryable_errors.is_empty());

    let temp = tempfile::tempdir().unwrap();
    initialize(temp.path(), &["a"]);
    let runtime = OpenRuntime::open(temp.path());
    runtime.hydration_tick("lease-a", 1);
    let hydration = only_hydration(&runtime, "seed-a");
    runtime
        .db
        .open_tree("agent_hydration_checkpoints")
        .unwrap()
        .insert(hydration.hydration_id.as_bytes(), b"not-json")
        .unwrap();
    let malformed_checkpoint = runtime.hydration_tick("lease-a", 1);
    assert_eq!(
        malformed_checkpoint.fatal_errors[0].code,
        "durable_data_corrupt"
    );
    assert!(malformed_checkpoint.retryable_errors.is_empty());
}

#[test]
fn exact_seed_join_rejects_embedded_identity_drift() {
    let temp = tempfile::tempdir().unwrap();
    initialize(temp.path(), &["a"]);
    let runtime = OpenRuntime::open(temp.path());
    let rule_tree = runtime.db.open_tree("agent_curation_rules").unwrap();
    let raw = rule_tree.get(b"rule-a").unwrap().unwrap();
    let mut rule: AgentCurationRuleRecord = serde_json::from_slice(&raw).unwrap();
    rule.rule_id = "rule-divergent".to_string();
    rule_tree
        .insert(b"rule-a", serde_json::to_vec(&rule).unwrap())
        .unwrap();
    let divergent_rule = runtime.hydration_tick("lease-a", 1);
    assert_eq!(
        divergent_rule.fatal_errors[0].code,
        "curation_rule_mismatch"
    );

    let temp = tempfile::tempdir().unwrap();
    initialize(temp.path(), &["a"]);
    let runtime = OpenRuntime::open(temp.path());
    let receipt = runtime
        .agent
        .bootstrap_receipt_for_agent("seed-a")
        .unwrap()
        .unwrap();
    let subscription_tree = runtime.db.open_tree("agent_subscriptions").unwrap();
    let raw = subscription_tree
        .get(receipt.subscription_id.as_bytes())
        .unwrap()
        .unwrap();
    let mut subscription: meld_world_model::agent::AgentSubscriptionRecord =
        serde_json::from_slice(&raw).unwrap();
    subscription.subscription_id = "subscription-divergent".to_string();
    subscription_tree
        .insert(
            receipt.subscription_id.as_bytes(),
            serde_json::to_vec(&subscription).unwrap(),
        )
        .unwrap();
    let divergent_subscription = runtime.hydration_tick("lease-a", 1);
    assert_eq!(
        divergent_subscription.fatal_errors[0].code,
        "subscription_mismatch"
    );
}

#[test]
fn direct_receipt_lookup_does_not_scan_unselected_agents() {
    let temp = tempfile::tempdir().unwrap();
    initialize(temp.path(), &["a", "b"]);
    let runtime = OpenRuntime::open(temp.path());
    runtime
        .db
        .open_tree("agent_bootstrap_receipts")
        .unwrap()
        .insert(b"bootstrap-b", b"not-json")
        .unwrap();

    let first = runtime.hydration_tick("lease-bounded", 1);
    assert_eq!(first.selected_count, 1);
    assert_eq!(first.started_count, 1);
    assert!(first.fatal_errors.is_empty(), "{first:?}");
    assert_eq!(
        runtime
            .agent
            .process_hydrations_for_agent("seed-a")
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn malformed_attestation_request_and_frame_fail_closed() {
    let temp = tempfile::tempdir().unwrap();
    initialize(temp.path(), &["a"]);
    let runtime = OpenRuntime::open(temp.path());
    runtime.hydration_tick("lease-a", 1);
    runtime.hydration_tick("lease-a", 1);
    let hydration = only_hydration(&runtime, "seed-a");
    let checkpoint = runtime
        .agent
        .hydration_checkpoint(&hydration.hydration_id)
        .unwrap()
        .unwrap();
    let attestation_id = checkpoint.attestation_id.as_deref().unwrap();
    let tree = runtime
        .db
        .open_tree("belief_readiness_attestations")
        .unwrap();
    let raw = tree.get(attestation_id.as_bytes()).unwrap().unwrap();
    let mut attestation: meld_world_model::belief::BeliefReadinessAttestation =
        serde_json::from_slice(&raw).unwrap();
    attestation.belief_view_hash = "divergent-view".to_string();
    tree.insert(
        attestation_id.as_bytes(),
        serde_json::to_vec(&attestation).unwrap(),
    )
    .unwrap();
    let malformed_attestation = runtime.hydration_tick("lease-a", 1);
    assert_eq!(
        malformed_attestation.fatal_errors[0].code,
        "invalid_attestation"
    );

    let temp = tempfile::tempdir().unwrap();
    initialize(temp.path(), &["a"]);
    let runtime = OpenRuntime::open(temp.path());
    runtime.hydration_tick("lease-a", 1);
    runtime.hydration_tick("lease-a", 1);
    runtime.hydration_tick("lease-a", 1);
    let hydration = only_hydration(&runtime, "seed-a");
    let checkpoint = runtime
        .agent
        .hydration_checkpoint(&hydration.hydration_id)
        .unwrap()
        .unwrap();
    let request_id = checkpoint.planner_request_id.as_deref().unwrap();
    let request_tree = runtime.db.open_tree("planner_projection_requests").unwrap();
    let raw = request_tree.get(request_id.as_bytes()).unwrap().unwrap();
    let mut record: PlannerProjectionRequestRecord = serde_json::from_slice(&raw).unwrap();
    record.created_at_seq += 1;
    record.updated_at_seq += 1;
    request_tree
        .insert(request_id.as_bytes(), serde_json::to_vec(&record).unwrap())
        .unwrap();
    let malformed_request = runtime.hydration_tick("lease-a", 1);
    assert_eq!(
        malformed_request.fatal_errors[0].code,
        "divergent_planner_request"
    );

    let temp = tempfile::tempdir().unwrap();
    initialize(temp.path(), &["a"]);
    let runtime = OpenRuntime::open(temp.path());
    runtime.hydration_tick("lease-a", 1);
    runtime.hydration_tick("lease-a", 1);
    runtime.hydration_tick("lease-a", 1);
    runtime.planner_tick(1);
    let hydration = only_hydration(&runtime, "seed-a");
    let checkpoint = runtime
        .agent
        .hydration_checkpoint(&hydration.hydration_id)
        .unwrap()
        .unwrap();
    let request = runtime
        .planner
        .get_request(checkpoint.planner_request_id.as_deref().unwrap())
        .unwrap()
        .unwrap();
    let frame_id = request.frame_id.as_deref().unwrap();
    let frame_tree = runtime.db.open_tree("planner_projection_frames").unwrap();
    let raw = frame_tree.get(frame_id.as_bytes()).unwrap().unwrap();
    let mut frame: meld_world_model::planner::PlannerProjectionFrame =
        serde_json::from_slice(&raw).unwrap();
    frame.completed_at_seq += 1;
    frame_tree
        .insert(frame_id.as_bytes(), serde_json::to_vec(&frame).unwrap())
        .unwrap();
    let malformed_frame = runtime.hydration_tick("lease-a", 1);
    assert_eq!(
        malformed_frame.fatal_errors[0].code,
        "malformed_planner_products"
    );
    assert_eq!(
        runtime.agent.get_agent("seed-a").unwrap().unwrap().status,
        AgentStatus::Registered
    );
}

#[test]
fn direct_revision_head_read_is_bounded_across_long_malformed_history() {
    let temp = tempfile::tempdir().unwrap();
    initialize(temp.path(), &["a"]);
    let runtime = OpenRuntime::open(temp.path());
    let input = activation("a");
    let config_hash = runtime
        .agent
        .bootstrap_receipt_for_agent("seed-a")
        .unwrap()
        .unwrap()
        .belief
        .config_snapshot_hash;
    let mut prior = "revision-a-1".to_string();
    for offset in 0..128_u64 {
        let revision_id = format!("revision-a-long-{offset:03}");
        seed_revision_in_store(
            runtime.belief.as_ref(),
            &input,
            &config_hash,
            &revision_id,
            Some(prior),
            21 + offset,
            2 + offset,
        );
        prior = revision_id;
    }
    runtime
        .db
        .open_tree("belief_revisions")
        .unwrap()
        .insert(b"revision-a-1", b"{malformed-history")
        .unwrap();

    let report = runtime.hydration_tick("lease-bounded-head", 1);
    assert_eq!(report.started_count, 1, "{report:?}");
    assert!(report.fatal_errors.is_empty(), "{report:?}");
    assert!(report.input_sequence >= 148);
    runtime.hydration_tick("lease-bounded-head", 1);
    runtime.hydration_tick("lease-bounded-head", 1);
    runtime.planner_tick(1);
    let completed = runtime.hydration_tick("lease-bounded-head", 1);
    assert_eq!(completed.operational_count, 1, "{completed:?}");
}

#[test]
fn durable_seed_content_must_match_bootstrap_input_hash() {
    let temp = tempfile::tempdir().unwrap();
    initialize(temp.path(), &["a"]);
    let runtime = OpenRuntime::open(temp.path());
    let tree = runtime.db.open_tree("agent_curation_rules").unwrap();
    let raw = tree.get(b"rule-a").unwrap().unwrap();
    let mut rule: AgentCurationRuleRecord = serde_json::from_slice(&raw).unwrap();
    rule.config.priority_urgency += 1;
    tree.insert(b"rule-a", serde_json::to_vec(&rule).unwrap())
        .unwrap();

    let report = runtime.hydration_tick("lease-owner-drift", 1);
    assert_eq!(report.fatal_errors[0].code, "bootstrap_input_hash_mismatch");
    assert!(runtime
        .agent
        .process_hydrations_for_agent("seed-a")
        .unwrap()
        .is_empty());
}

#[test]
fn epoch_qualified_lease_index_migrates_legacy_key() {
    let temp = tempfile::tempdir().unwrap();
    initialize(temp.path(), &["a"]);
    let runtime = OpenRuntime::open(temp.path());
    runtime.hydration_tick("lease-reused", 1);
    let first = only_hydration(&runtime, "seed-a");
    drop(runtime);

    let db = reopen(temp.path());
    let lease_tree = db.open_tree("agent_process_hydrations_by_lease").unwrap();
    lease_tree.clear().unwrap();
    let legacy_key = format!("{:020}{}{}", "seed-a".len(), "seed-a", "lease-reused");
    lease_tree
        .insert(legacy_key.as_bytes(), first.hydration_id.as_bytes())
        .unwrap();
    db.flush().unwrap();
    drop(lease_tree);
    drop(db);

    let runtime = OpenRuntime::open(temp.path());
    assert_eq!(
        runtime
            .agent
            .process_hydration_for_lease("seed-a", "lease-reused")
            .unwrap()
            .unwrap()
            .attempt_epoch,
        1
    );
    assert_eq!(
        runtime
            .agent
            .process_hydration_for_lease("seed-a", "lease-reused")
            .unwrap()
            .unwrap(),
        first
    );
}

#[test]
fn pending_request_is_revoked_after_owner_drift_across_reopen() {
    let temp = tempfile::tempdir().unwrap();
    initialize(temp.path(), &["a"]);
    let request_id = {
        let runtime = OpenRuntime::open(temp.path());
        for _ in 0..3 {
            runtime.hydration_tick("lease-owner-drift", 1);
        }
        let hydration = only_hydration(&runtime, "seed-a");
        runtime
            .agent
            .hydration_checkpoint(&hydration.hydration_id)
            .unwrap()
            .unwrap()
            .planner_request_id
            .unwrap()
    };

    let runtime = OpenRuntime::open(temp.path());
    let directives = runtime.db.open_tree("agent_directives").unwrap();
    let raw = directives.get(b"directive-a").unwrap().unwrap();
    let mut directive: DirectiveRecord = serde_json::from_slice(&raw).unwrap();
    directive.text.push_str(" drift after planner activation");
    directives
        .insert(b"directive-a", serde_json::to_vec(&directive).unwrap())
        .unwrap();
    runtime.db.flush().unwrap();
    let traversal = Arc::new(TraversalStore::new(runtime.db.clone()).unwrap());
    let report = PlannerProjectionActor::new(
        Arc::clone(&runtime.planner),
        Arc::clone(&runtime.belief),
        traversal,
    )
    .tick(PlannerProjectionTickRequest { max_items: 1 });
    assert_eq!(report.completed_count, 0, "{report:?}");
    assert_eq!(report.failed_count, 0, "{report:?}");
    assert_eq!(report.retryable_errors.len(), 1, "{report:?}");
    assert!(runtime.planner.get_request(&request_id).unwrap().is_none());
    assert!(runtime
        .planner
        .pending_requests_bounded(1)
        .unwrap()
        .records
        .is_empty());
}

#[test]
fn pending_request_is_revoked_after_replacement_lease_across_reopen() {
    let temp = tempfile::tempdir().unwrap();
    initialize(temp.path(), &["a"]);
    let request_id = {
        let runtime = OpenRuntime::open(temp.path());
        for _ in 0..3 {
            runtime.hydration_tick("lease-old-pending", 1);
        }
        let hydration = only_hydration(&runtime, "seed-a");
        runtime
            .agent
            .hydration_checkpoint(&hydration.hydration_id)
            .unwrap()
            .unwrap()
            .planner_request_id
            .unwrap()
    };

    let runtime = OpenRuntime::open(temp.path());
    assert_eq!(
        runtime.hydration_tick("lease-replacement", 1).started_count,
        1
    );
    drop(runtime);

    let runtime = OpenRuntime::open(temp.path());
    let traversal = Arc::new(TraversalStore::new(runtime.db.clone()).unwrap());
    let report = PlannerProjectionActor::new(
        Arc::clone(&runtime.planner),
        Arc::clone(&runtime.belief),
        traversal,
    )
    .tick(PlannerProjectionTickRequest { max_items: 1 });
    assert_eq!(report.completed_count, 0, "{report:?}");
    assert_eq!(report.failed_count, 0, "{report:?}");
    assert_eq!(report.retryable_errors.len(), 1, "{report:?}");
    assert!(runtime.planner.get_request(&request_id).unwrap().is_none());
    assert_eq!(
        runtime
            .agent
            .current_process_hydration("seed-a")
            .unwrap()
            .unwrap()
            .attempt_epoch,
        2
    );
}

#[test]
fn stale_pending_failure_is_revoked_without_a_terminal_record() {
    let temp = tempfile::tempdir().unwrap();
    initialize(temp.path(), &["a"]);
    let request_id = {
        let runtime = OpenRuntime::open(temp.path());
        let input = activation("a");
        let mut view = runtime
            .belief
            .current_view(&input.belief_key)
            .unwrap()
            .unwrap();
        view.planner_projection.confidence_field = "invalid field".to_string();
        runtime.belief.put_view(&view).unwrap();
        for _ in 0..3 {
            runtime.hydration_tick("lease-stale-failure", 1);
        }
        let hydration = only_hydration(&runtime, "seed-a");
        runtime
            .agent
            .hydration_checkpoint(&hydration.hydration_id)
            .unwrap()
            .unwrap()
            .planner_request_id
            .unwrap()
    };

    let runtime = OpenRuntime::open(temp.path());
    assert_eq!(
        runtime
            .hydration_tick("lease-stale-failure-replacement", 1)
            .started_count,
        1
    );
    let traversal = Arc::new(TraversalStore::new(runtime.db.clone()).unwrap());
    let report = PlannerProjectionActor::new(
        Arc::clone(&runtime.planner),
        Arc::clone(&runtime.belief),
        traversal,
    )
    .tick(PlannerProjectionTickRequest { max_items: 1 });
    assert_eq!(report.completed_count, 0, "{report:?}");
    assert_eq!(report.failed_count, 0, "{report:?}");
    assert_eq!(report.retryable_errors.len(), 1, "{report:?}");
    assert!(runtime.planner.get_request(&request_id).unwrap().is_none());
}

#[test]
fn attested_revision_snapshot_survives_revision_drift_and_reopen() {
    let temp = tempfile::tempdir().unwrap();
    initialize(temp.path(), &["a"]);
    let request_id = {
        let runtime = OpenRuntime::open(temp.path());
        for _ in 0..3 {
            runtime.hydration_tick("lease-revision-snapshot", 1);
        }
        let hydration = only_hydration(&runtime, "seed-a");
        let checkpoint = runtime
            .agent
            .hydration_checkpoint(&hydration.hydration_id)
            .unwrap()
            .unwrap();
        let request_id = checkpoint.planner_request_id.unwrap();
        let input = activation("a");
        let receipt = runtime
            .agent
            .bootstrap_receipt_for_agent("seed-a")
            .unwrap()
            .unwrap();
        seed_revision(
            &runtime.db,
            &input,
            &receipt.belief.config_snapshot_hash,
            "revision-a-2",
            Some("revision-a-1".to_string()),
            40,
            2,
        );
        request_id
    };

    let runtime = OpenRuntime::open(temp.path());
    runtime.planner_tick(1);
    let completed = runtime.planner.get_request(&request_id).unwrap().unwrap();
    assert_eq!(completed.status, PlannerProjectionRequestStatus::Completed);
    let frame = runtime
        .planner
        .get_frame(completed.frame_id.as_deref().unwrap())
        .unwrap()
        .unwrap();
    assert_eq!(
        frame.output.hydration_refs.revision_ids,
        vec!["revision-a-1".to_string()]
    );
    assert_eq!(
        completed
            .request
            .attested_belief
            .as_ref()
            .unwrap()
            .revision_id,
        "revision-a-1"
    );
    let operational = runtime.hydration_tick("lease-revision-snapshot", 1);
    assert_eq!(operational.operational_count, 1, "{operational:?}");
}

#[test]
fn early_hydration_branches_report_every_observed_record_sequence() {
    let temp = tempfile::tempdir().unwrap();
    initialize(temp.path(), &["a"]);
    let runtime = OpenRuntime::open(temp.path());
    runtime.hydration_tick("lease-old-sequence", 1);
    runtime.hydration_tick("lease-new-sequence", 1);
    let current = runtime
        .agent
        .current_process_hydration("seed-a")
        .unwrap()
        .unwrap();
    let historical = runtime
        .agent
        .process_hydration_for_lease("seed-a", "lease-old-sequence")
        .unwrap()
        .unwrap();
    let stale = runtime.hydration_tick("lease-old-sequence", 1);
    assert_eq!(stale.fatal_errors[0].code, "stale_lease");
    assert!(stale.input_sequence >= current.updated_at_seq);
    assert!(stale.input_sequence >= historical.updated_at_seq);

    let temp = tempfile::tempdir().unwrap();
    initialize(temp.path(), &["a"]);
    let runtime = OpenRuntime::open(temp.path());
    for _ in 0..3 {
        runtime.hydration_tick("lease-ready-sequence", 1);
    }
    runtime.planner_tick(1);
    runtime.hydration_tick("lease-ready-sequence", 1);
    let ready = runtime
        .agent
        .current_process_hydration("seed-a")
        .unwrap()
        .unwrap();
    let agents = runtime.db.open_tree("agent_records").unwrap();
    let raw = agents.get(b"seed-a").unwrap().unwrap();
    let mut agent: meld_world_model::agent::AgentRecord = serde_json::from_slice(&raw).unwrap();
    agent.status = AgentStatus::Registered;
    agents
        .insert(b"seed-a", serde_json::to_vec(&agent).unwrap())
        .unwrap();
    runtime
        .db
        .open_tree("agent_by_status")
        .unwrap()
        .insert(
            format!("registered::{:020}::seed-a", agent.updated_at_seq).as_bytes(),
            b"seed-a",
        )
        .unwrap();
    let ready_report = runtime.hydration_tick("lease-ready-sequence", 1);
    assert_eq!(ready_report.fatal_errors[0].code, "registered_ready_agent");
    assert!(ready_report.input_sequence >= ready.updated_at_seq);

    let temp = tempfile::tempdir().unwrap();
    initialize(temp.path(), &["a"]);
    let runtime = OpenRuntime::open(temp.path());
    runtime.hydration_tick("lease-failed-sequence", 1);
    let input = activation("a");
    let receipt = runtime
        .agent
        .bootstrap_receipt_for_agent("seed-a")
        .unwrap()
        .unwrap();
    seed_revision(
        &runtime.db,
        &input,
        &receipt.belief.config_snapshot_hash,
        "revision-a-failed-sequence",
        Some("revision-a-1".to_string()),
        40,
        2,
    );
    runtime.hydration_tick("lease-failed-sequence", 1);
    let failed = runtime
        .agent
        .current_process_hydration("seed-a")
        .unwrap()
        .unwrap();
    assert_eq!(failed.status, AgentProcessHydrationStatus::Failed);
    let failed_report = runtime.hydration_tick("lease-failed-sequence", 1);
    assert_eq!(failed_report.fatal_errors[0].code, "failed_lease");
    assert!(failed_report.input_sequence >= failed.updated_at_seq);
}

#[test]
fn final_operational_transition_rejects_same_id_owner_drift() {
    let temp = tempfile::tempdir().unwrap();
    initialize(temp.path(), &["a"]);
    let runtime = OpenRuntime::open(temp.path());
    for _ in 0..3 {
        runtime.hydration_tick("lease-final-owner", 1);
    }
    runtime.planner_tick(1);
    let directives = runtime.db.open_tree("agent_directives").unwrap();
    let raw = directives.get(b"directive-a").unwrap().unwrap();
    let mut directive: DirectiveRecord = serde_json::from_slice(&raw).unwrap();
    directive.text.push_str(" final drift");
    directives
        .insert(b"directive-a", serde_json::to_vec(&directive).unwrap())
        .unwrap();

    let report = runtime.hydration_tick("lease-final-owner", 1);
    assert_eq!(report.fatal_errors[0].code, "bootstrap_input_hash_mismatch");
    assert_eq!(
        runtime.agent.get_agent("seed-a").unwrap().unwrap().status,
        AgentStatus::Registered
    );
}
