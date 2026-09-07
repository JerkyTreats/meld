//! Native preparation and startup must preserve predecessor lifecycle obligations.

use super::*;
use crate::runtime::lifecycle::ActivationGenerationStatus;

#[test]
fn prepared_binding_replacement_preserves_assignment_and_drains_predecessor() {
    let mut harness = StewardshipHarness::new();
    {
        let assembly = harness.assembly();
        harness.run_world_genesis(&assembly);
    }
    let predecessor;
    let assignment_id;
    {
        let assembly = harness.assembly();
        let supervisor = harness.start_supervisor(&assembly);
        let prepared = assembly.prepared_activation().unwrap();
        assignment_id = prepared.assignment.assignment_id.clone();
        predecessor = assembly
            .lifecycle_store()
            .unwrap()
            .current_generation(&assignment_id)
            .unwrap()
            .unwrap();
        assert!(predecessor.admission_open());
        harness.binding.provider_id = Some("replacement-provider".into());
        harness.run_world_genesis(&assembly);
        let head = assembly
            .stores()
            .pds_products
            .prepared_head(&harness.binding.package.expression)
            .unwrap()
            .unwrap();
        let successor = assembly
            .stores()
            .pds_products
            .prepared_closure(&head.prepared_id)
            .unwrap()
            .unwrap();
        assert_eq!(successor.assignment.assignment_id, assignment_id);
        assert_ne!(successor.prepared_id, prepared.prepared_id);
        assert_eq!(
            successor.expected_prior_prepared_id.as_ref(),
            Some(&prepared.prepared_id)
        );
        // Preparation does not close admission or publish its successor.
        assert_eq!(
            assembly
                .lifecycle_store()
                .unwrap()
                .current_generation(&assignment_id)
                .unwrap()
                .unwrap(),
            predecessor
        );
        drop(supervisor);
        assembly.flush_product_boundary().unwrap();
        assembly.flush_supervisor_store().unwrap();
    }
    let assembly = harness.assembly();
    assert!(assembly.bind_dispatch_routes(stub_routes()));
    let restart_at = 101
        + assembly
            .supervisor_startup_package()
            .lifecycle_config
            .lease_duration_ms;
    let mut command = SupervisorStartCommand::new("replacement", restart_at);
    command.registration_set = assembly.registration_set().cloned();
    let mut supervisor =
        RuntimeSupervisor::start(assembly.supervisor_startup_package(), command).unwrap();
    let lifecycle = assembly.lifecycle_store().unwrap();
    let current = lifecycle
        .current_generation(&assignment_id)
        .unwrap()
        .unwrap();
    assert_ne!(current.generation_id, predecessor.generation_id);
    assert!(current.admission_open());
    let RuntimeSemanticHandleFactory::AgentActor(agent) = &assembly
        .handle_factories()
        .get(AGENT_RECONCILIATION_RUNTIME_ID)
        .unwrap()
        .semantic
    else {
        panic!("native Agent absent")
    };
    let current_fence = agent.authority_port.observe().unwrap().unwrap();
    assert!(agent
        .authority_port
        .same_assignment(&predecessor.generation_id, &current_fence)
        .unwrap());
    assert!(!agent
        .authority_port
        .same_preparation(&predecessor.generation_id, &current_fence)
        .unwrap());
    assert!(!agent
        .authority_port
        .same_assignment("foreign-generation", &current_fence)
        .unwrap());
    let mut stale = current_fence.clone();
    stale.activation_generation = predecessor.generation_id.clone();
    assert!(!agent
        .authority_port
        .same_assignment(&predecessor.generation_id, &stale)
        .unwrap());
    assert_eq!(current.readiness.len(), predecessor.readiness.len());
    let prior = lifecycle
        .generation(&assignment_id, &predecessor.generation_id)
        .unwrap()
        .unwrap();
    assert_eq!(prior.status, ActivationGenerationStatus::Retired);
    assert_eq!(prior.readiness.len(), predecessor.readiness.len());
    assert_eq!(prior.stop_receipts.len(), predecessor.readiness.len());
    assert_eq!(prior.release_receipts.len(), predecessor.readiness.len());
    assert!(prior.retirement.is_some());
    assert!(prior.release_receipts.values().all(|receipt| receipt
        .released_binding_ref
        .starts_with(&format!(
            "retirement-session::{}::",
            predecessor.generation_id
        ))));
    let live = supervisor.status_snapshot(restart_at + 1).unwrap();
    assert!(
        live.runtimes
            .iter()
            .filter(|runtime| runtime.desired_enabled
                && runtime.registration_kind
                    == crate::runtime::registration::RegistrationKind::ActiveActor)
            .all(|runtime| runtime.active_lease_id.is_some()),
        "{live:?}"
    );
    for (id, incarnation) in &prior.incarnations {
        assert!(incarnation.incarnation_number > predecessor.incarnations[id].incarnation_number);
        assert_ne!(
            incarnation.incarnation_id,
            predecessor.incarnations[id].incarnation_id
        );
    }
    assert!(
        assembly
            .stores()
            .agent_store
            .reconciliation_goals_for_agent(STEWARD_AGENT_ID)
            .unwrap()
            .is_empty(),
        "retirement reconstruction must not run Agent work"
    );
    assert!(!prior.admission_open());
    supervisor.request_shutdown(restart_at + 1_000).unwrap();
    let history = lifecycle.assignment(&assignment_id).unwrap().unwrap();
    assert!(history.current_generation_id.is_none());
    assert!(
        history
            .generations
            .values()
            .all(|generation| generation.status == ActivationGenerationStatus::Retired),
        "replacement must not abandon its predecessor: {history:#?}"
    );
}
