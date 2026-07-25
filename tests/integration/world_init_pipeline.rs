//! World initialization pipeline integration: stages 2 through 4 over one
//! temporary product root.
//!
//! Proves the initialization command surface contract: every stage applies
//! once with durable record ids, a re-run reports `Unchanged` with the same
//! ids, the epistemic genesis fact exists exactly once on the ledger under
//! its frozen record identity, and stage selection normalizes to pipeline
//! order without duplicates.

use meld::init::world::pipeline::{normalized_stages, WorldInitContent, WorldInitPipeline};
use meld::init::world::{StageDisposition, WorldInitRequest, WorldInitStage};
use meld_events::{
    DomainObjectRef, EventAuthority, EventAuthorityOpenOptions, LedgerCursor, ReplayRequest,
};
use meld_world_model::agent::{AgentCurationRuleConfig, AgentQuery, AgentStatus, AgentStore};
use meld_world_model::belief::{BeliefFamilyRegistryStore, BranchScope};
use meld_world_model::PerspectiveKey;

const FAMILY_ID: &str = "docs_freshness";
const AGENT_ID: &str = "seed.docs_freshness";
const SUBJECT_ID: &str = "node-a";

/// Frozen stage 4 record identity for the fixture subject.
const EXPECTED_GENESIS_RECORD_ID: &str =
    "genesis::world_model::observation::workspace_fs::node::node-a";

fn family_config_json() -> &'static str {
    r#"{
        "family_id": "docs_freshness",
        "dimension_id": "docs_freshness",
        "predicate_id": "confidence",
        "evidence_policy_id": "default_policy",
        "evidence_schemas": [
            {
                "schema_id": "content_written_signal",
                "required": false,
                "role": "Support",
                "reliability": 1.0,
                "precision": 1.0
            }
        ],
        "source_mappings": [
            {
                "mapping_id": "content_written_to_signal",
                "source_kind": "content_written",
                "evidence_schema_id": "content_written_signal",
                "subject_from": "record.subject",
                "value_field": "stale_probability",
                "factor_id": "content_written_signal"
            }
        ],
        "comparator": {
            "engine_id": "weighted_bayesian",
            "engine_version": "1",
            "factors": [
                {
                    "factor_id": "content_written_signal",
                    "evidence_schema_id": "content_written_signal",
                    "weight": 1.0,
                    "polarity": "Supports"
                }
            ],
            "missing_evidence_uncertainty": 0.9
        },
        "default_prior": 0.8,
        "planner_projection": {
            "confidence_field": "confidence",
            "threshold": 0.7,
            "posterior_meaning": "stale_probability"
        },
        "config_version": "1"
    }"#
}

fn curation_rule() -> AgentCurationRuleConfig {
    AgentCurationRuleConfig {
        dimension_id: FAMILY_ID.to_string(),
        threshold: 0.7,
        priority_urgency: 50,
        desired_summary: "confidence>0.7".to_string(),
        source_kind: "belief_divergence".to_string(),
    }
}

fn content() -> WorldInitContent {
    WorldInitContent {
        family_config: serde_json::from_str(family_config_json()).unwrap(),
        curation_rule: curation_rule(),
        agent_id: AGENT_ID.to_string(),
        subject: DomainObjectRef::new("workspace_fs", "node", SUBJECT_ID).unwrap(),
        perspective: PerspectiveKey::new("default", "default").unwrap(),
        branch_scope: BranchScope::main(),
        observation_scope: FAMILY_ID.to_string(),
        directive: "steward docs freshness for node-a".to_string(),
        provenance: "world init pipeline test".to_string(),
        session_id: "world-init-test".to_string(),
        observed_seq: 1,
    }
}

/// One temporary product root with the ledger, registry, and agent store
/// the pipeline binds to, mirroring how machine initialization opens them.
struct TempWorld {
    _root: tempfile::TempDir,
    authority: EventAuthority,
    registry: BeliefFamilyRegistryStore,
    agent_store: AgentStore,
}

impl TempWorld {
    fn open() -> Self {
        let root = tempfile::tempdir().unwrap();
        let db = sled::open(root.path().join("world")).unwrap();
        let authority =
            EventAuthority::open(db.clone(), EventAuthorityOpenOptions::default()).unwrap();
        let registry = BeliefFamilyRegistryStore::new(db.clone()).unwrap();
        let agent_store = AgentStore::new(db).unwrap();
        Self {
            _root: root,
            authority,
            registry,
            agent_store,
        }
    }

    fn run(&mut self, request: &WorldInitRequest) -> meld::init::world::WorldInitReport {
        let append = self.authority.append_capability();
        WorldInitPipeline::new(&mut self.registry, &self.agent_store, &append)
            .run(request, &content())
            .unwrap()
    }

    fn genesis_fact_count(&self) -> usize {
        let replay = self.authority.replay_capability();
        let mut cursor = LedgerCursor {
            ledger_id: replay.ledger_identity(),
            after_seq: 0,
        };
        let mut count = 0;
        loop {
            let page = replay.replay(ReplayRequest { cursor, limit: 64 }).unwrap();
            if page.records.is_empty() {
                return count;
            }
            count += page
                .records
                .iter()
                .filter(|record| {
                    record.envelope.record_id.as_deref() == Some(EXPECTED_GENESIS_RECORD_ID)
                })
                .count();
            cursor = page.next_cursor;
        }
    }
}

/// A request carrying duplicates out of order, exercising normalization
/// through the full run rather than only through the pure helper.
fn scrambled_full_request() -> WorldInitRequest {
    WorldInitRequest {
        stages: vec![
            WorldInitStage::SeedEpistemicFacts,
            WorldInitStage::InstallTheory,
            WorldInitStage::SeedEpistemicFacts,
            WorldInitStage::GenesisIdentities,
            WorldInitStage::InstallTheory,
        ],
    }
}

#[test]
fn full_pipeline_applies_once_and_reruns_unchanged() {
    let mut world = TempWorld::open();

    let first = world.run(&scrambled_full_request());

    // Normalization: duplicates collapse and the report runs in pipeline
    // order regardless of request order.
    assert_eq!(
        first
            .stage_reports
            .iter()
            .map(|report| report.stage)
            .collect::<Vec<_>>(),
        vec![
            WorldInitStage::InstallTheory,
            WorldInitStage::GenesisIdentities,
            WorldInitStage::SeedEpistemicFacts,
        ]
    );
    for report in &first.stage_reports {
        assert_eq!(
            report.disposition,
            StageDisposition::Applied,
            "stage {:?} should apply on an empty world",
            report.stage
        );
        assert!(
            !report.record_ids.is_empty(),
            "stage {:?} must cite record ids",
            report.stage
        );
    }
    assert_eq!(
        first.stage_reports[2].record_ids,
        vec![EXPECTED_GENESIS_RECORD_ID.to_string()]
    );

    // The genesis identities are durably operational after the first run.
    let agent = AgentQuery::new(&world.agent_store)
        .agent(AGENT_ID)
        .unwrap()
        .unwrap();
    assert_eq!(agent.status, AgentStatus::Operational);
    assert_eq!(
        agent.installed_curation_rule().unwrap().clone(),
        curation_rule()
    );

    // Re-run: every stage is unchanged and cites identical record ids.
    let second = world.run(&scrambled_full_request());
    assert_eq!(second.stage_reports.len(), first.stage_reports.len());
    for (rerun, original) in second.stage_reports.iter().zip(&first.stage_reports) {
        assert_eq!(rerun.stage, original.stage);
        assert_eq!(
            rerun.disposition,
            StageDisposition::Unchanged,
            "stage {:?} must be unchanged on re-run",
            rerun.stage
        );
        assert_eq!(
            rerun.record_ids, original.record_ids,
            "stage {:?} must cite the same record ids on re-run",
            rerun.stage
        );
    }

    // The frozen genesis fact exists exactly once on the ledger.
    assert_eq!(world.genesis_fact_count(), 1);
}

#[test]
fn partial_stage_selection_runs_only_the_selected_stage() {
    let mut world = TempWorld::open();

    // Stage 2 alone installs the theory and nothing else.
    let install_only = world.run(&WorldInitRequest {
        stages: vec![WorldInitStage::InstallTheory],
    });
    assert_eq!(install_only.stage_reports.len(), 1);
    assert_eq!(
        install_only.stage_reports[0].stage,
        WorldInitStage::InstallTheory
    );
    assert_eq!(
        install_only.stage_reports[0].disposition,
        StageDisposition::Applied
    );
    assert!(AgentQuery::new(&world.agent_store)
        .agent(AGENT_ID)
        .unwrap()
        .is_none());
    assert_eq!(world.genesis_fact_count(), 0);

    // The remaining stages complete genesis against the installed theory.
    let rest = world.run(&WorldInitRequest {
        stages: vec![
            WorldInitStage::SeedEpistemicFacts,
            WorldInitStage::GenesisIdentities,
        ],
    });
    assert_eq!(
        rest.stage_reports
            .iter()
            .map(|report| (report.stage, report.disposition))
            .collect::<Vec<_>>(),
        vec![
            (WorldInitStage::GenesisIdentities, StageDisposition::Applied),
            (
                WorldInitStage::SeedEpistemicFacts,
                StageDisposition::Applied
            ),
        ]
    );
    assert_eq!(world.genesis_fact_count(), 1);
}

#[test]
fn genesis_identities_without_installed_theory_fail_truthfully() {
    let world = TempWorld::open();
    let append = world.authority.append_capability();
    let mut registry = world.registry.clone();
    let error = WorldInitPipeline::new(&mut registry, &world.agent_store, &append)
        .run(
            &WorldInitRequest {
                stages: vec![WorldInitStage::GenesisIdentities],
            },
            &content(),
        )
        .unwrap_err();
    assert!(error.to_string().contains("no installed revision"));
}

#[test]
fn normalized_stages_is_the_public_normalization_contract() {
    assert_eq!(
        normalized_stages(&scrambled_full_request()),
        vec![
            WorldInitStage::InstallTheory,
            WorldInitStage::GenesisIdentities,
            WorldInitStage::SeedEpistemicFacts,
        ]
    );
}
