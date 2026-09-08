//! A refused startup must not retain partial ownership or disturb a live predecessor.

use super::*;

struct MissingCurationTemplate(meld_world_model::curation::CurationRuleSource);

impl meld_world_model::curation::CurationRuleSelectionPort for MissingCurationTemplate {
    fn select(
        &self,
        authority: &meld_world_model::CurationAuthority,
    ) -> Result<
        meld_world_model::curation::CurationRuleSelection,
        meld_world_model::error::StorageError,
    > {
        self.0.select(authority)
    }
    fn binding_refs(&self) -> Result<Vec<String>, meld_world_model::error::StorageError> {
        self.0.binding_refs()
    }
    fn template_refs(
        &self,
    ) -> Result<
        Vec<meld_world_model::belief::TheoryRevisionRef>,
        meld_world_model::error::StorageError,
    > {
        Err(meld_world_model::error::StorageError::InvalidPath(
            "selected Curation template is unavailable during readiness".into(),
        ))
    }
    fn resolves_wake(
        &self,
        _: &meld_world_model::waiting::StructuralWakeAddress,
    ) -> Result<bool, String> {
        Ok(false)
    }
}

#[test]
fn failed_native_start_releases_its_partial_leases_and_can_retry_immediately() {
    let harness = StewardshipHarness::new();
    {
        let assembly = harness.assembly();
        harness.run_world_genesis(&assembly);
    }
    let mut assembly = harness.assembly();
    assert!(assembly.bind_dispatch_routes(stub_routes()));
    let factory = assembly
        .handle_factories
        .factories
        .get_mut("world_model.standing_curation")
        .unwrap();
    let RuntimeSemanticHandleFactory::StandingCuration(curation) = &mut factory.semantic else {
        unreachable!()
    };
    let original = curation.rule.clone();
    curation.rule = meld_world_model::curation::CurationRuleSource::Producer(Arc::new(
        MissingCurationTemplate(original.clone()),
    ));
    let error = match RuntimeSupervisor::start(
        assembly.supervisor_startup_package(),
        SupervisorStartCommand::new("failed-start", 100),
    ) {
        Ok(_) => panic!("missing owner template must refuse readiness"),
        Err(error) => error,
    };
    assert!(
        error
            .to_string()
            .contains("selected Curation template is unavailable"),
        "{error}"
    );
    assert!(
        !error.to_string().contains("cleanup also failed"),
        "{error}"
    );
    for registration in &assembly.registration_set().unwrap().registrations {
        assert!(
            assembly
                .supervisor_store()
                .get_active_runtime_lease(
                    &crate::runtime::supervisor::RuntimeId::new(registration.runtime_id.clone())
                        .unwrap()
                )
                .unwrap()
                .is_none(),
            "{} kept a failed startup lease",
            registration.runtime_id
        );
    }
    let failed = assembly
        .supervisor_store()
        .get_runtime_instance("failed-start")
        .unwrap()
        .unwrap();
    assert_eq!(
        failed.status,
        crate::runtime::supervisor::RuntimeInstanceStatus::Failed
    );
    let factory = assembly
        .handle_factories
        .factories
        .get_mut("world_model.standing_curation")
        .unwrap();
    let RuntimeSemanticHandleFactory::StandingCuration(curation) = &mut factory.semantic else {
        unreachable!()
    };
    curation.rule = original;
    let mut resumed = RuntimeSupervisor::start(
        assembly.supervisor_startup_package(),
        SupervisorStartCommand::new("immediate-retry", 101),
    )
    .unwrap();
    resumed.request_shutdown(102).unwrap();
}

#[test]
fn contended_native_start_preserves_the_live_predecessor_and_names_its_lease() {
    let harness = StewardshipHarness::new();
    {
        let assembly = harness.assembly();
        harness.run_world_genesis(&assembly);
    }
    let assembly = harness.assembly();
    let mut current = harness.start_supervisor(&assembly);
    let assignment = &assembly
        .prepared_activation()
        .unwrap()
        .assignment
        .assignment_id;
    let before = assembly
        .lifecycle_store()
        .unwrap()
        .current_generation(assignment)
        .unwrap()
        .unwrap();
    assert!(before.admission_open());
    let error = match RuntimeSupervisor::start(
        assembly.supervisor_startup_package(),
        SupervisorStartCommand::new("contended-start", 101),
    ) {
        Ok(_) => panic!("the active predecessor owns this participant set"),
        Err(error) => error,
    };
    assert!(
        error
            .to_string()
            .contains("owned by instance 'epoch-fixture'"),
        "{error}"
    );
    assert!(error.to_string().contains("until"), "{error}");
    assert_eq!(
        assembly
            .lifecycle_store()
            .unwrap()
            .current_generation(assignment)
            .unwrap()
            .unwrap(),
        before
    );
    for registration in &assembly.registration_set().unwrap().registrations {
        if let Some(lease) = assembly
            .supervisor_store()
            .get_active_runtime_lease(
                &crate::runtime::supervisor::RuntimeId::new(registration.runtime_id.clone())
                    .unwrap(),
            )
            .unwrap()
        {
            assert_eq!(lease.instance_id, "epoch-fixture");
        }
    }
    current.request_shutdown(102).unwrap();
}
