//! Installed Method selection over independently assessed Security coverage.

use super::*;

#[test]
fn native_security_method_requires_coverage_and_retains_independent_verification() {
    assert_native_security_reconciliation(true, true, SecuritySourceAdvance::GuardedMethod);
}

pub(super) fn genesis(harness: &StewardshipHarness, assembly: &ProductRuntimeAssembly) {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("theory/dependency_security");
    let package = tempfile::tempdir().unwrap();
    for entry in std::fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_file() {
            std::fs::copy(entry.path(), package.path().join(entry.file_name())).unwrap();
        }
    }
    let path = package.path().join("strategy_theory.security.json");
    let mut strategy: meld_world_model::strategy::StrategyTheoryPackage =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    let steps: Vec<_> = strategy
        .capabilities
        .iter()
        .map(|capability| meld_lang::Step {
            step_id: capability.operator.operator_id.clone(),
            kind: meld_lang::StepKind::Op(capability.operator.clone()),
        })
        .collect();
    let mut edges = Vec::new();
    for consumer in &strategy.capabilities {
        for input in &consumer.operator.resolution.requires_inputs {
            let producer = strategy
                .capabilities
                .iter()
                .find(|producer| {
                    producer
                        .operator
                        .resolution
                        .requires_outputs
                        .iter()
                        .any(|output| output.artifact_type == input.artifact_type)
                })
                .unwrap();
            edges.push(meld_lang::Edge {
                from: producer.operator.operator_id.clone(),
                to: consumer.operator.operator_id.clone(),
                kind: meld_lang::EdgeKind::DataFlow {
                    artifact_type: input.artifact_type.clone(),
                },
            });
        }
    }
    strategy.methods.push(meld_lang::Method {
        method_id: "verify-covered-security-posture".into(),
        trigger: strategy.snapshot.settlement_rules[0].goal_pattern.clone(),
        preconditions: vec![meld_lang::Proposition::Holds {
            subject: meld_lang::Term::Variable("?subject".into()),
            dimension: meld_lang::Term::Dimension("dependency_security_coverage".into()),
            condition: meld_lang::Condition::Above(meld_lang::Term::Literal(
                meld_lang::Literal::Number(0.9),
            )),
        }],
        composition: meld_lang::Composition { steps, edges },
        net_effects: Vec::new(),
        cost: meld_lang::CostEstimate::zero(),
        preference: 0,
    });
    // One expansion proves that the installed template reaches Agent; direct
    // construction cannot hide a dropped Method or an unavailable coverage view.
    strategy.search_bounds.max_expansions = 1;
    let bytes = serde_json::to_vec(&strategy).unwrap();
    std::fs::write(&path, &bytes).unwrap();
    let manifest_path = package.path().join("pds-package.json");
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&manifest_path).unwrap()).unwrap();
    for component in manifest["components"].as_array_mut().unwrap() {
        if component["content"]["path"] == "strategy_theory.security.json" {
            component["content"]["content_hash"] = blake3::hash(&bytes).to_hex().to_string().into();
        }
    }
    std::fs::write(manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();
    harness.run_world_genesis_from(assembly, package.path());
}
