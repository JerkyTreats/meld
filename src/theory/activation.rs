//! Complete inert activation closures prepared from exact package inputs.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::config::{StewardshipActivationV1, StewardshipAssignmentV1};

use super::error::{error, TheoryRouterError};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParticipantKind {
    BoundedActor,
    DurableOperationAdapter,
    PassiveSource,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActivationParticipantSpec {
    pub participant_id: String,
    pub owner_domain: String,
    pub kind: ParticipantKind,
    pub required: bool,
    pub depends_on: BTreeSet<String>,
    pub readiness_contract_ref: String,
    pub wake_contract_ref: String,
    pub safe_point_contract_ref: String,
    pub stop_contract_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActivationParticipantPlanV1 {
    pub plan_id: String,
    pub participants: Vec<ActivationParticipantSpec>,
}

impl ActivationParticipantPlanV1 {
    pub fn new(
        mut participants: Vec<ActivationParticipantSpec>,
    ) -> Result<Self, TheoryRouterError> {
        participants.sort_by(|left, right| left.participant_id.cmp(&right.participant_id));
        if participants.is_empty()
            || participants
                .iter()
                .any(|participant| participant.participant_id.trim().is_empty())
            || !participants
                .windows(2)
                .all(|pair| pair[0].participant_id != pair[1].participant_id)
        {
            return Err(error(
                "activation_plan_invalid",
                "participant ids must be unique and non-empty",
            ));
        }
        let ids: BTreeSet<_> = participants
            .iter()
            .map(|participant| participant.participant_id.as_str())
            .collect();
        if participants.iter().any(|participant| {
            participant
                .depends_on
                .iter()
                .any(|dependency| !ids.contains(dependency.as_str()))
        }) {
            return Err(error(
                "activation_plan_invalid",
                "participant dependency is absent",
            ));
        }
        let by_id: BTreeMap<_, _> = participants
            .iter()
            .map(|participant| (participant.participant_id.as_str(), participant))
            .collect();
        for participant in &participants {
            let mut visiting = BTreeSet::new();
            verify_acyclic(participant.participant_id.as_str(), &by_id, &mut visiting)?;
        }
        let plan_id = hash(&participants)?;
        Ok(Self {
            plan_id,
            participants,
        })
    }
}

fn verify_acyclic<'a>(
    id: &'a str,
    participants: &BTreeMap<&'a str, &'a ActivationParticipantSpec>,
    visiting: &mut BTreeSet<&'a str>,
) -> Result<(), TheoryRouterError> {
    if !visiting.insert(id) {
        return Err(error(
            "activation_plan_cycle",
            "participant dependency cycle",
        ));
    }
    for dependency in &participants[id].depends_on {
        verify_acyclic(
            participants.get_key_value(dependency.as_str()).unwrap().0,
            participants,
            visiting,
        )?;
    }
    visiting.remove(id);
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreparedDomainActivationRef {
    pub owner_domain: String,
    pub receipt_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OwnerBindingRevisionRef {
    pub binding_id: String,
    pub revision_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EffectiveAuthorityInputRefs {
    pub requested_authority_ref: String,
    pub principal_grant_ref: String,
    pub current_judgment_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreparedActivationClosureV1 {
    pub prepared_id: String,
    pub assignment: StewardshipAssignmentV1,
    pub activation: StewardshipActivationV1,
    pub package_receipt_id: String,
    pub package_content_hash: String,
    pub owner_receipts: Vec<PreparedDomainActivationRef>,
    pub capability_closure_ref: String,
    pub participant_plan: ActivationParticipantPlanV1,
    pub binding_revision_refs: Vec<OwnerBindingRevisionRef>,
    pub effective_authority_inputs: EffectiveAuthorityInputRefs,
}

#[derive(Serialize)]
struct PreparedIdentity<'a> {
    assignment: &'a StewardshipAssignmentV1,
    activation: &'a StewardshipActivationV1,
    package_receipt_id: &'a str,
    package_content_hash: &'a str,
    owner_receipts: &'a [PreparedDomainActivationRef],
    capability_closure_ref: &'a str,
    participant_plan: &'a ActivationParticipantPlanV1,
    binding_revision_refs: &'a [OwnerBindingRevisionRef],
    effective_authority_inputs: &'a EffectiveAuthorityInputRefs,
}

impl PreparedActivationClosureV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        assignment: StewardshipAssignmentV1,
        activation: StewardshipActivationV1,
        package_content_hash: String,
        mut owner_receipts: Vec<PreparedDomainActivationRef>,
        capability_closure_ref: String,
        participant_plan: ActivationParticipantPlanV1,
        mut binding_revision_refs: Vec<OwnerBindingRevisionRef>,
        effective_authority_inputs: EffectiveAuthorityInputRefs,
    ) -> Result<Self, TheoryRouterError> {
        assignment
            .verify_identity()
            .map_err(|failure| error("assignment_invalid", failure.to_string()))?;
        activation
            .verify_identity()
            .map_err(|failure| error("activation_invalid", failure.to_string()))?;
        if activation.assignment_id != assignment.assignment_id {
            return Err(error(
                "activation_assignment_mismatch",
                "activation cites another assignment",
            ));
        }
        if owner_receipts.is_empty() || capability_closure_ref.trim().is_empty() {
            return Err(error(
                "prepared_closure_incomplete",
                "owner and capability receipts are required",
            ));
        }
        owner_receipts.sort_by(|left, right| left.owner_domain.cmp(&right.owner_domain));
        binding_revision_refs.sort_by(|left, right| left.binding_id.cmp(&right.binding_id));
        let package_receipt_id = assignment.package_receipt_id.clone();
        let identity = PreparedIdentity {
            assignment: &assignment,
            activation: &activation,
            package_receipt_id: &package_receipt_id,
            package_content_hash: &package_content_hash,
            owner_receipts: &owner_receipts,
            capability_closure_ref: &capability_closure_ref,
            participant_plan: &participant_plan,
            binding_revision_refs: &binding_revision_refs,
            effective_authority_inputs: &effective_authority_inputs,
        };
        let prepared_id = hash(&identity)?;
        Ok(Self {
            prepared_id,
            assignment,
            activation,
            package_receipt_id,
            package_content_hash,
            owner_receipts,
            capability_closure_ref,
            participant_plan,
            binding_revision_refs,
            effective_authority_inputs,
        })
    }
}

fn hash(value: &impl Serialize) -> Result<String, TheoryRouterError> {
    serde_json::to_vec(value)
        .map(|bytes| blake3::hash(&bytes).to_hex().to_string())
        .map_err(|failure| error("activation_identity_failed", failure.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AdapterPlacement, OperationalLimits, RuntimeIsolationRequirements};
    use meld_events::DomainObjectRef;

    fn assignment() -> StewardshipAssignmentV1 {
        StewardshipAssignmentV1::new(
            "package".into(),
            "principal".into(),
            "agent".into(),
            DomainObjectRef::new("workspace_fs", "node", "docs").unwrap(),
            "perspective".into(),
            "main".into(),
            "authority".into(),
            "grant".into(),
        )
        .unwrap()
    }

    fn plan() -> ActivationParticipantPlanV1 {
        ActivationParticipantPlanV1::new(vec![ActivationParticipantSpec {
            participant_id: "docs-agent".into(),
            owner_domain: "docs".into(),
            kind: ParticipantKind::BoundedActor,
            required: true,
            depends_on: BTreeSet::new(),
            readiness_contract_ref: "ready.v1".into(),
            wake_contract_ref: "wake.v1".into(),
            safe_point_contract_ref: "safe.v1".into(),
            stop_contract_ref: "stop.v1".into(),
        }])
        .unwrap()
    }

    #[test]
    fn prepared_closure_is_inert() {
        let assignment = assignment();
        let activation = StewardshipActivationV1::new(
            assignment.assignment_id.clone(),
            BTreeMap::new(),
            BTreeMap::new(),
            AdapterPlacement::InProcess,
            RuntimeIsolationRequirements::default(),
            OperationalLimits::default(),
        )
        .unwrap();
        let prepared = PreparedActivationClosureV1::new(
            assignment,
            activation,
            "content".into(),
            vec![PreparedDomainActivationRef {
                owner_domain: "docs".into(),
                receipt_ref: "owner".into(),
            }],
            "capability".into(),
            plan(),
            vec![],
            EffectiveAuthorityInputRefs {
                requested_authority_ref: "authority".into(),
                principal_grant_ref: "grant".into(),
                current_judgment_ref: "judgment".into(),
            },
        )
        .unwrap();
        assert_eq!(prepared.prepared_id.len(), 64);
    }

    #[test]
    fn missing_owner_receipt_prevents_closure() {
        let assignment = assignment();
        let activation = StewardshipActivationV1::new(
            assignment.assignment_id.clone(),
            BTreeMap::new(),
            BTreeMap::new(),
            AdapterPlacement::InProcess,
            RuntimeIsolationRequirements::default(),
            OperationalLimits::default(),
        )
        .unwrap();
        assert!(PreparedActivationClosureV1::new(
            assignment,
            activation,
            "content".into(),
            vec![],
            "capability".into(),
            plan(),
            vec![],
            EffectiveAuthorityInputRefs {
                requested_authority_ref: "authority".into(),
                principal_grant_ref: "grant".into(),
                current_judgment_ref: "judgment".into()
            },
        )
        .is_err());
    }
}
