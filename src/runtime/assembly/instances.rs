//! Realize native factory descriptors from an exact prepared participant plan.

use std::collections::{BTreeMap, BTreeSet};

use super::{RuntimeAssemblyError, RuntimeFactoryRegistry};
use crate::config::StewardshipAssignmentV1;
use crate::theory::{ActivationParticipantPlanV1, ProductParticipantBindingV1};

pub(super) fn realize(
    registry: &mut RuntimeFactoryRegistry,
    assignment: &StewardshipAssignmentV1,
    plan: &ActivationParticipantPlanV1,
    bindings: &BTreeMap<String, ProductParticipantBindingV1>,
) -> Result<(), RuntimeAssemblyError> {
    let catalog = registry.descriptors.clone();
    let mut selected = BTreeSet::new();
    for participant in &plan.participants {
        super::validate_runtime_id(&participant.participant_id)
            .map_err(|error| invalid(error.to_string()))?;
        let binding = bindings.get(&participant.participant_id);
        let factory_id = binding
            .map(|b| b.factory_id.as_str())
            .unwrap_or(&participant.participant_id);
        let position = binding.map(|b| b.agent_position_id.as_str()).or_else(|| {
            let [position] = assignment.agent_positions.as_slice() else {
                return None;
            };
            Some(position.position_id.as_str())
        });
        if !selected.insert((factory_id, position)) {
            return Err(invalid(
                "one native responsibility cannot have two instances for the same Agent position",
            ));
        }
        if binding.is_none() {
            continue;
        }
        if !matches!(
            factory_id,
            "world_model.agent_reconciliation"
                | "world_model.standing_curation"
                | "world_model.belief_assessment"
                | "world_model.evidence_ingestion"
        ) {
            return Err(invalid(format!(
                "factory '{factory_id}' does not expose scoped native realization"
            )));
        }
        let position = position
            .ok_or_else(|| invalid("scoped native participant requires an Agent position"))?;
        if !assignment
            .agent_positions
            .iter()
            .any(|p| p.position_id == position)
        {
            return Err(invalid(
                "scoped native participant selects an unbound Agent position",
            ));
        }
        let mut descriptor = catalog
            .get(factory_id)
            .cloned()
            .ok_or_else(|| invalid(format!("unknown runtime factory '{factory_id}'")))?;
        if let Some(existing) = catalog.get(&participant.participant_id) {
            if existing.factory_id != factory_id {
                return Err(invalid(
                    "scoped participant cannot replace another runtime factory",
                ));
            }
        }
        descriptor.runtime_id = participant.participant_id.clone();
        descriptor.agent_position_id = Some(position.into());
        registry
            .descriptors
            .insert(descriptor.runtime_id.clone(), descriptor);
    }
    Ok(())
}

fn invalid(message: impl Into<String>) -> RuntimeAssemblyError {
    RuntimeAssemblyError::Config(message.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AssignedAgentPositionV1;
    use crate::theory::ActivationParticipantSpec;

    fn assignment() -> StewardshipAssignmentV1 {
        StewardshipAssignmentV1::new(
            "compilation".into(),
            "revision".into(),
            "principal".into(),
            meld_events::DomainObjectRef::new("workspace_fs", "node", "code").unwrap(),
            "default".into(),
            "main".into(),
            "topology".into(),
            vec![AssignedAgentPositionV1 {
                position_id: "producer".into(),
                agent_id: "agent".into(),
            }],
            "authority".into(),
            "grant".into(),
        )
        .unwrap()
    }

    fn selection(
        entries: &[(&str, &str, &str)],
    ) -> (
        ActivationParticipantPlanV1,
        BTreeMap<String, ProductParticipantBindingV1>,
    ) {
        let participants = entries.iter().map(|(id, _, _)| serde_json::from_value::<ActivationParticipantSpec>(serde_json::json!({
            "participant_id": id, "owner_domain": "world-model", "kind": "bounded_actor", "required": true,
            "depends_on": [], "readiness_contract_ref": "ready", "wake_contract_ref": "wake", "safe_point_contract_ref": "safe", "stop_contract_ref": "stop"
        })).unwrap()).collect();
        let bindings = entries
            .iter()
            .map(|(id, factory, position)| {
                (
                    id.to_string(),
                    ProductParticipantBindingV1 {
                        factory_id: factory.to_string(),
                        agent_position_id: position.to_string(),
                    },
                )
            })
            .collect();
        (
            ActivationParticipantPlanV1::new(participants).unwrap(),
            bindings,
        )
    }

    #[test]
    fn instance_selection_is_exact_and_repeatable() {
        let mut registry = RuntimeFactoryRegistry::first_proof_registry().unwrap();
        let factory = "world_model.agent_reconciliation";
        let original = registry.get(factory).unwrap().clone();
        let (plan, bindings) = selection(&[("producer.agent", factory, "producer")]);
        realize(&mut registry, &assignment(), &plan, &bindings).unwrap();
        realize(&mut registry, &assignment(), &plan, &bindings).unwrap();
        let instance = registry.get("producer.agent").unwrap();
        assert_eq!(instance.factory_id, factory);
        assert_eq!(instance.agent_position_id.as_deref(), Some("producer"));
        assert_eq!(instance.required_resources, original.required_resources);
        assert_eq!(registry.get(factory), Some(&original));
    }

    #[test]
    fn duplicate_authority_shared_factories_and_unbound_positions_are_rejected() {
        let factory = "world_model.agent_reconciliation";
        for entries in [
            vec![
                ("first.agent", factory, "producer"),
                ("second.agent", factory, "producer"),
            ],
            vec![("duplicate.graph", "world_model.graph_replay", "producer")],
            vec![("unknown.agent", "unknown.factory", "producer")],
            vec![("world_model.belief_assessment", factory, "producer")],
            vec![("absent.agent", factory, "absent")],
        ] {
            let mut registry = RuntimeFactoryRegistry::first_proof_registry().unwrap();
            let (plan, bindings) = selection(&entries);
            assert!(realize(&mut registry, &assignment(), &plan, &bindings).is_err());
        }
    }
}
