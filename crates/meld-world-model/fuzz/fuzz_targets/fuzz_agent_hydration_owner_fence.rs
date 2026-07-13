#![no_main]

use libfuzzer_sys::fuzz_target;
use meld_world_model::activation::{
    AgentBootstrapReceipt, AgentCurationRuleRecord, BeliefActivationReceipt, DirectiveRecord,
};
use meld_world_model::agent::{
    fuzz_hydration_terminal_fence, AgentCurationRuleConfig, AgentHydrationOwnerSnapshot,
    AgentRecord, AgentStatus,
    AgentSubscriptionRecord, AgentSubscriptionStatus,
};
use meld_world_model::belief::{BeliefKey, BranchScope};
use meld_world_model::events::DomainObjectRef;
use meld_world_model::planner::{
    PlannerAttestedBeliefSnapshot, PlannerProjectionRequest,
};
use meld_world_model::world_state::graph::PerspectiveKey;

fuzz_target!(|data: &[u8]| {
    if data.is_empty() {
        return;
    }
    fuzz_hydration_terminal_fence(data);
    let mut snapshot = owner_snapshot(data);
    let baseline = snapshot.fence_hash().expect("canonical owner snapshot");
    let mode = data[0] % 16;
    let should_validate = match mode {
        0 => {
            snapshot.directive.text.push_str(&fragment(data));
            true
        }
        1 => {
            snapshot.rule.config.priority_urgency =
                snapshot.rule.config.priority_urgency.saturating_add(1);
            true
        }
        2 => {
            snapshot.agent.observation_scope.push_str(&fragment(data));
            true
        }
        3 => {
            snapshot.subscription.updated_at_seq =
                snapshot.subscription.updated_at_seq.saturating_add(1);
            true
        }
        4 => {
            snapshot.receipt.input_hash = alternate_digest(data, b'i');
            true
        }
        5 => {
            snapshot.belief_config_json = format!("{{\"seed\":\"{}-changed\"}}", fragment(data));
            true
        }
        6 => {
            let digest = alternate_digest(data, b'c');
            snapshot.receipt.belief.config_snapshot_hash = digest.clone();
            snapshot.belief_config_hash = digest;
            true
        }
        7 => {
            snapshot.directive.directive_id.push_str("-drift");
            false
        }
        8 => {
            snapshot.rule.agent_id.push_str("-drift");
            false
        }
        9 => {
            snapshot.subscription.agent_id.push_str("-drift");
            false
        }
        10 => {
            snapshot.receipt.belief.config_snapshot_hash = alternate_digest(data, b'x');
            false
        }
        11 => {
            snapshot.belief_config_json.clear();
            false
        }
        12 => {
            snapshot.agent.directive_id.push_str("-drift");
            false
        }
        13 => {
            snapshot.receipt.subscription_id.push_str("-drift");
            false
        }
        14 => {
            snapshot.receipt.agent_id.push_str("-drift");
            false
        }
        _ => {
            snapshot.receipt.rule_id.push_str("-drift");
            false
        }
    };

    let encoded = serde_json::to_vec(&snapshot).expect("owner snapshot serialization");
    let decoded: AgentHydrationOwnerSnapshot =
        serde_json::from_slice(&encoded).expect("owner snapshot round trip");
    let observed = decoded.fence_hash();
    if should_validate {
        let observed = observed.expect("consistent owner mutation");
        assert_ne!(observed, baseline);
        assert_eq!(observed, snapshot.fence_hash().expect("stable owner hash"));
    } else {
        assert!(observed.is_err());
    }

    let mut corrupted = encoded;
    let offset = usize::from(data[data.len() - 1]) % corrupted.len();
    corrupted[offset] ^= data.get(1).copied().unwrap_or(0x80) | 1;
    if let Ok(corrupted_snapshot) =
        serde_json::from_slice::<AgentHydrationOwnerSnapshot>(&corrupted)
    {
        let _ = corrupted_snapshot.fence_hash();
    }

    exercise_revision_fence(data);
});

fn exercise_revision_fence(data: &[u8]) {
    let suffix = fragment(data);
    let subject = DomainObjectRef::new("workspace", "node", format!("node-{suffix}"))
        .expect("revision subject fixture");
    let perspective =
        PerspectiveKey::new("agent", format!("perspective-{suffix}"))
            .expect("revision perspective fixture");
    let boundary = PlannerAttestedBeliefSnapshot {
        attestation_id: format!("attestation-{suffix}"),
        revision_id: format!("revision-{suffix}"),
        revision_hash: alternate_digest(data, b'r'),
        view_id: format!("view-{suffix}"),
        view_hash: alternate_digest(data, b'v'),
        source_cursor_start: 1,
        source_cursor_end: u64::from(data[0]).saturating_add(1),
    };
    let request = PlannerProjectionRequest::identified_attested(
        alternate_digest(data, b'p'),
        format!("agent-{suffix}"),
        subject.clone(),
        perspective.clone(),
        BranchScope::main(),
        vec![format!("dimension-{suffix}")],
        Vec::new(),
        boundary.clone(),
    )
    .expect("attested planner request");
    let mut drifted = request.clone();
    let drifted_boundary = {
        let drifted_boundary = drifted
            .attested_belief
            .as_mut()
            .expect("attested boundary");
        match data[0] % 4 {
            0 => drifted_boundary.revision_id.push_str("-drift"),
            1 => drifted_boundary.revision_hash = alternate_digest(data, b'R'),
            2 => drifted_boundary.view_hash = alternate_digest(data, b'V'),
            _ => {
                drifted_boundary.source_cursor_end =
                    drifted_boundary.source_cursor_end.saturating_add(1)
            }
        }
        drifted_boundary.clone()
    };
    assert!(drifted.validate().is_err());
    let rebuilt = PlannerProjectionRequest::identified_attested(
        request.source_request_hash.clone(),
        request.agent_id.clone(),
        subject,
        perspective,
        BranchScope::main(),
        request.requested_dimensions.clone(),
        Vec::new(),
        drifted_boundary,
    )
    .expect("drifted snapshot remains independently valid");
    assert_ne!(rebuilt.request_id, request.request_id);
}

fn owner_snapshot(data: &[u8]) -> AgentHydrationOwnerSnapshot {
    let suffix = fragment(data);
    let agent_id = format!("agent-{suffix}");
    let directive_id = format!("directive-{suffix}");
    let rule_id = format!("rule-{suffix}");
    let subscription_id = format!("subscription-{suffix}");
    let subject = DomainObjectRef::new("workspace", "node", format!("node-{suffix}"))
        .expect("subject fixture");
    let perspective =
        PerspectiveKey::new("agent", format!("perspective-{suffix}")).expect("perspective fixture");
    let agent = AgentRecord {
        agent_id: agent_id.clone(),
        perspective_key: perspective.clone(),
        subject: subject.clone(),
        branch_scope: BranchScope::main(),
        observation_scope: format!("scope-{suffix}"),
        directive_id: directive_id.clone(),
        seed_provenance: "fuzz".to_string(),
        status: AgentStatus::Registered,
        created_at_seq: 1,
        updated_at_seq: 1,
    };
    let rule = AgentCurationRuleRecord {
        rule_id: rule_id.clone(),
        agent_id: agent_id.clone(),
        config: AgentCurationRuleConfig {
            dimension_id: format!("dimension-{suffix}"),
            threshold: 0.7,
            priority_urgency: 10,
            desired_summary: "ready".to_string(),
            source_kind: "belief".to_string(),
        },
    };
    let directive = DirectiveRecord {
        directive_id: directive_id.clone(),
        text: "hydrate agent".to_string(),
    };
    let subscription = AgentSubscriptionRecord {
        subscription_id: subscription_id.clone(),
        agent_id: agent_id.clone(),
        belief_key: BeliefKey {
            subject,
            dimension_id: rule.config.dimension_id.clone(),
            predicate_id: "confidence".to_string(),
            perspective,
            branch_scope: BranchScope::main(),
            evidence_policy_id: format!("policy-{suffix}"),
        },
        status: AgentSubscriptionStatus::Active,
        last_delivered_revision_id: None,
        last_delivered_seq: 0,
        created_at_seq: 2,
        updated_at_seq: 2,
    };
    let activation_hash = "a".repeat(64);
    let config_hash = "c".repeat(64);
    let receipt = AgentBootstrapReceipt {
        receipt_id: format!("receipt-{suffix}"),
        bootstrap_id: format!("bootstrap-{suffix}"),
        activation_hash: activation_hash.clone(),
        activation_id: format!("activation-{suffix}"),
        input_hash: "b".repeat(64),
        belief: BeliefActivationReceipt {
            family_id: format!("family-{suffix}"),
            config_snapshot_hash: config_hash.clone(),
            activation_hash,
            activation_id: format!("activation-{suffix}"),
        },
        directive_id,
        agent_id,
        rule_id,
        subscription_id,
        completed_at_seq: 3,
    };
    AgentHydrationOwnerSnapshot {
        agent,
        receipt,
        directive,
        rule,
        subscription,
        belief_config_hash: config_hash,
        belief_config_json: format!("{{\"seed\":\"{suffix}\"}}"),
    }
}

fn fragment(data: &[u8]) -> String {
    let mut out = String::new();
    for byte in data.iter().copied().take(16) {
        use std::fmt::Write;
        write!(&mut out, "{byte:02x}").expect("string formatting");
    }
    out
}

fn alternate_digest(data: &[u8], domain: u8) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(&[domain]);
    hasher.update(data);
    hasher.finalize().to_hex().to_string()
}
