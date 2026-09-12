//! Resolve an Agent's WAD within an exact compiled package closure.
use super::*;

/// An explicit package selector names one WAD and its exact imports. Positions
/// written before this selector retain the strict whole-closure interpretation.
pub fn position_packages(
    position: &ProductAgentPositionV1,
    receipts: &[PdsPackageInstallationReceiptV1],
    store: &PdsPackageStore,
) -> Result<Vec<PdsPackageInstallationReceiptV1>, TheoryRouterError> {
    let Some(package_id) = &position.package_id else {
        return Ok(receipts.to_vec());
    };
    let matches = receipts
        .iter()
        .filter(|r| &r.package_id == package_id)
        .collect::<Vec<_>>();
    let [selected] = matches.as_slice() else {
        return Err(error(
            "agent_package_ambiguous",
            format!(
                "position '{}' requires one exact compiled package '{package_id}'",
                position.position_id
            ),
        ));
    };
    store.resolve_closure(std::slice::from_ref(&selected.receipt_id))
}

impl ProductCompilationReceiptV1 {
    /// Lower one position's exact native components to the existing runtime selection.
    pub(crate) fn position_selection(
        &self,
        position: &ProductAgentPositionV1,
        declaration: &ProductDeclarationV1,
        store: &PdsPackageStore,
    ) -> Result<crate::config::SelectedStewardshipPackage, TheoryRouterError> {
        let components = self
            .position_packages(position, store)?
            .into_iter()
            .flat_map(|p| p.components)
            .collect::<Vec<_>>();
        let one = |owner: &str, kind: &str, component_id: Option<&str>| {
            let found = components
                .iter()
                .filter(|c| {
                    c.route.owner_domain == owner
                        && c.route.component_kind == kind
                        && component_id.is_none_or(|id| c.component_id == id)
                })
                .collect::<Vec<_>>();
            match found.as_slice() {
                [component] => Ok(component.owner_revision.id.clone()),
                _ => Err(error(
                    "position_theory_ambiguous",
                    format!(
                        "position '{}' requires one exact {owner}/{kind}",
                        position.position_id
                    ),
                )),
            }
        };
        Ok(crate::config::SelectedStewardshipPackage {
            expression: declaration.product_id.clone(),
            principal_id: declaration.principal_id.clone(),
            belief_family_id: one(
                "world-model",
                "belief-family",
                Some(&position.observation_scope_component_id),
            )?,
            evidence_mapping_id: one("world-model", "outcome-mapping", None)?,
            curation_rule_id: one("world-model", "agent-curation-rule", None)?,
            maintained_condition_id: one("world-model", "agent-maintained-condition", None)?,
            strategy_theory_id: one("world-model", "strategy-theory", None)?,
            authority_policy_id: one("execution", "authority-policy", None)?,
            claim_policy_id: String::new(),
        })
    }

    /// Resolve from retained receipts, never the currently installed package heads.
    pub fn position_packages(
        &self,
        position: &ProductAgentPositionV1,
        store: &PdsPackageStore,
    ) -> Result<Vec<PdsPackageInstallationReceiptV1>, TheoryRouterError> {
        self.verify_identity()?;
        let receipts = store.resolve_closure(&self.package_receipt_ids)?;
        position_packages(position, &receipts, store)
    }
}
