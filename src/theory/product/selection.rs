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
