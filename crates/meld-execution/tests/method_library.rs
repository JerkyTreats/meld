use meld_events::DomainObjectRef;
use meld_execution::capability::{
    ArtifactSchemaVersionRange, CapabilityCatalog, CapabilityTypeContract, ExecutionClass,
    ExecutionContract, InputCardinality, InputSlotSpec, OutputSlotSpec, ScopeContract,
};
use meld_execution::planning::{MethodLibrary, OperatorResolutionStatus, PlanningDiagnosticCode};
use meld_lang::{
    CapabilityRef, Composition, Condition, CostEstimate, Edge, EdgeKind, Effect, Literal, Method,
    Operator, Proposition, Resolution, SlotConstraint, Step, StepKind, Term,
};
use tempfile::tempdir;

fn node() -> Term {
    Term::Object(DomainObjectRef::new("workspace", "node", "readme").unwrap())
}

fn docs_target(scope: Term) -> Proposition {
    Proposition::Holds {
        subject: scope,
        dimension: Term::Dimension("docs_freshness".to_string()),
        condition: Condition::Above(Term::Literal(Literal::Number(0.7))),
    }
}

fn catalog() -> CapabilityCatalog {
    let mut catalog = CapabilityCatalog::new();
    catalog
        .register(CapabilityTypeContract {
            capability_type_id: "docs.write".to_string(),
            capability_version: 1,
            owning_domain: "docs".to_string(),
            scope_contract: ScopeContract {
                scope_kind: "filesystem".to_string(),
                scope_ref_kind: "node_id".to_string(),
                allow_fan_out: false,
            },
            binding_contract: vec![],
            input_contract: vec![InputSlotSpec {
                slot_id: "source".to_string(),
                accepted_artifact_type_ids: vec!["source_doc".to_string()],
                schema_versions: ArtifactSchemaVersionRange { min: 1, max: 1 },
                required: false,
                cardinality: InputCardinality::One,
            }],
            output_contract: vec![OutputSlotSpec {
                slot_id: "patch".to_string(),
                artifact_type_id: "docs_patch".to_string(),
                schema_version: 1,
                guaranteed: true,
            }],
            effect_contract: vec![],
            execution_contract: ExecutionContract {
                execution_class: ExecutionClass::Queued,
                completion_semantics: "result_or_failure".to_string(),
                retry_class: "provider_io".to_string(),
                cancellation_supported: true,
            },
        })
        .unwrap();
    catalog
}

fn docs_method(method_id: &str, preference: u32, time_ms: u64) -> Method {
    Method {
        method_id: method_id.to_string(),
        trigger: docs_target(Term::Variable("?node".to_string())),
        preconditions: vec![Proposition::Accessible {
            scope: Term::Variable("?node".to_string()),
        }],
        composition: Composition {
            steps: vec![Step {
                step_id: "write".to_string(),
                kind: StepKind::Op(Operator {
                    operator_id: "write".to_string(),
                    preconditions: vec![Proposition::Accessible {
                        scope: Term::Variable("?node".to_string()),
                    }],
                    effects: vec![
                        Effect::Assert(Proposition::Exists {
                            scope: Term::Variable("?node".to_string()),
                            artifact_type: Term::ArtifactType("docs_patch".to_string()),
                        }),
                        Effect::Update {
                            subject: Term::Variable("?node".to_string()),
                            dimension: Term::Dimension("docs_freshness".to_string()),
                            value: Term::Literal(Literal::Number(0.95)),
                        },
                    ],
                    cost: CostEstimate {
                        time_ms,
                        money_microdollars: 25_000,
                        provider_calls: 1,
                    },
                    resolution: Resolution {
                        requires_inputs: vec![],
                        requires_outputs: vec![SlotConstraint {
                            artifact_type_id: "docs_patch".to_string(),
                            required: true,
                        }],
                        scope_kind: Some("filesystem".to_string()),
                        tags: vec!["write".to_string(), "docs".to_string()],
                        specific: None,
                    },
                }),
            }],
            edges: vec![],
        },
        net_effects: vec![Effect::Update {
            subject: Term::Variable("?node".to_string()),
            dimension: Term::Dimension("docs_freshness".to_string()),
            value: Term::Literal(Literal::Number(0.95)),
        }],
        cost: CostEstimate {
            time_ms,
            money_microdollars: 25_000,
            provider_calls: 1,
        },
        preference,
    }
}

#[test]
fn load_pinned_refresh_docs_fixture() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/methods");
    let library = MethodLibrary::load_json_dir(path, &catalog()).unwrap();

    assert!(library.invalid.is_empty(), "{:?}", library.invalid);
    assert_eq!(library.entries.len(), 1);
    assert_eq!(library.entries[0].method.method_id, "refresh_docs_v1");
    assert_eq!(
        library.entries[0].verification.operator_resolutions[0].status,
        OperatorResolutionStatus::Resolved
    );
}

#[test]
fn method_load_order_is_deterministic() {
    let temp = tempdir().unwrap();
    let slow = docs_method("slow", 2, 20);
    let fast = docs_method("fast", 1, 10);
    std::fs::write(
        temp.path().join("z.json"),
        serde_json::to_string_pretty(&slow).unwrap(),
    )
    .unwrap();
    std::fs::write(
        temp.path().join("a.json"),
        serde_json::to_string_pretty(&fast).unwrap(),
    )
    .unwrap();

    let library = MethodLibrary::load_json_dir(temp.path(), &catalog()).unwrap();
    let ordered = library
        .sorted_verified_entries()
        .into_iter()
        .map(|entry| entry.method.method_id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(ordered, vec!["fast", "slow"]);
}

#[test]
fn duplicate_method_ids_become_invalid_reports() {
    let library = MethodLibrary::from_methods(
        vec![docs_method("same", 1, 10), docs_method("same", 2, 20)],
        &catalog(),
    );

    assert!(library.entries.is_empty());
    assert_eq!(library.invalid.len(), 2);
    assert!(library.invalid[0]
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == PlanningDiagnosticCode::MethodDuplicateId));
}

#[test]
fn unbound_precondition_variable_is_invalid() {
    let mut method = docs_method("bad", 1, 10);
    method.preconditions.push(Proposition::Accessible {
        scope: Term::Variable("?other".to_string()),
    });
    let library = MethodLibrary::from_methods(vec![method], &catalog());

    assert!(library.entries.is_empty());
    assert!(library.invalid[0].diagnostics.iter().any(|diagnostic| {
        diagnostic.code == PlanningDiagnosticCode::MethodVariableNotBoundByTrigger
    }));
}

#[test]
fn unbound_composition_variable_is_invalid() {
    let mut method = docs_method("bad", 1, 10);
    let StepKind::Op(operator) = &mut method.composition.steps[0].kind else {
        panic!("expected operator");
    };
    operator.effects.push(Effect::Update {
        subject: Term::Variable("?other".to_string()),
        dimension: Term::Dimension("docs_freshness".to_string()),
        value: Term::Literal(Literal::Number(0.95)),
    });
    let library = MethodLibrary::from_methods(vec![method], &catalog());

    assert!(library.entries.is_empty());
    assert!(library.invalid[0].diagnostics.iter().any(|diagnostic| {
        diagnostic.code == PlanningDiagnosticCode::MethodVariableNotBoundByTrigger
    }));
}

#[test]
fn invalid_composition_edge_is_invalid() {
    let mut method = docs_method("bad", 1, 10);
    method.composition.edges.push(Edge {
        from: "write".to_string(),
        to: "missing".to_string(),
        kind: EdgeKind::Ordering,
    });
    let library = MethodLibrary::from_methods(vec![method], &catalog());

    assert!(library.entries.is_empty());
    assert!(library.invalid[0]
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == PlanningDiagnosticCode::MethodTemplateInvalid));
}

#[test]
fn exact_capability_ref_resolves_and_missing_ref_reports_unresolved() {
    let mut exact = docs_method("exact", 1, 10);
    let StepKind::Op(operator) = &mut exact.composition.steps[0].kind else {
        panic!("expected operator");
    };
    operator.resolution.specific = Some(CapabilityRef {
        capability_type_id: "docs.write".to_string(),
        capability_version: 1,
    });
    let library = MethodLibrary::from_methods(vec![exact], &catalog());
    assert_eq!(
        library.entries[0].verification.operator_resolutions[0].status,
        OperatorResolutionStatus::Resolved
    );

    let mut missing = docs_method("missing", 1, 10);
    let StepKind::Op(operator) = &mut missing.composition.steps[0].kind else {
        panic!("expected operator");
    };
    operator.resolution.specific = Some(CapabilityRef {
        capability_type_id: "docs.missing".to_string(),
        capability_version: 1,
    });
    let library = MethodLibrary::from_methods(vec![missing], &catalog());
    assert_eq!(
        library.entries[0].verification.operator_resolutions[0].status,
        OperatorResolutionStatus::Unresolved
    );
}

#[test]
fn catalog_scan_matches_inputs_outputs_and_scope_with_tags_diagnostic_only() {
    let library = MethodLibrary::from_methods(vec![docs_method("scan", 1, 10)], &catalog());
    let report = &library.entries[0].verification.operator_resolutions[0];

    assert_eq!(report.status, OperatorResolutionStatus::Resolved);
    assert_eq!(report.capability_type_id.as_deref(), Some("docs.write"));
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == PlanningDiagnosticCode::OperatorTagsDiagnosticOnly
    }));
    assert!(docs_target(node()).is_ground());
}
