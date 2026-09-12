//! Package-authored Agent positions and native participant selection.

use super::{
    ActivationParticipantPlanV1, ActivationParticipantSpec, PdsPackageInstallationReceiptV1,
    ProductAgentPositionV1, ProductDeclarationV1, ProductPackageSelectionV1, TheoryRevisionRef,
    TheoryRouteId, TheoryRouterError,
};
use serde::{Deserialize, Serialize};

pub const PRODUCT_TOPOLOGY_REGISTRY: &str = "pds_product_topology_v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProductTopologyV1 {
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub participant_bindings:
        std::collections::BTreeMap<String, super::ProductParticipantBindingV1>,
    pub topology_id: String,
    pub agent_positions: Vec<ProductAgentPositionV1>,
    pub participants: Vec<ActivationParticipantSpec>,
    pub requested_authority_ref: String,
    pub compilation_policy_revision: String,
}
impl ProductTopologyV1 {
    pub fn route() -> TheoryRouteId {
        TheoryRouteId::new("runtime", "product-topology", 1)
    }
    pub fn revision_ref(&self) -> Result<TheoryRevisionRef, TheoryRouterError> {
        self.validate()?;
        let bytes = serde_json::to_vec(self)
            .map_err(|e| super::error::error("product_topology_invalid", e.to_string()))?;
        Ok(TheoryRevisionRef {
            registry: PRODUCT_TOPOLOGY_REGISTRY.into(),
            id: self.topology_id.clone(),
            content_hash: blake3::hash(&bytes).to_hex().to_string(),
        })
    }
    pub fn validate(&self) -> Result<(), TheoryRouterError> {
        let plan = ActivationParticipantPlanV1::new(self.participants.clone())?;
        if self.topology_id.trim().is_empty()
            || !super::product::valid_agent_topology(&self.agent_positions, &plan)
            || !super::product::valid_participant_bindings(
                &self.participant_bindings,
                &self.agent_positions,
                &plan,
            )
            || self.requested_authority_ref.trim().is_empty()
            || self.compilation_policy_revision.trim().is_empty()
        {
            return Err(super::error::error(
                "product_topology_invalid",
                "topology identity, positions and authority must be declared",
            ));
        }
        Ok(())
    }
    pub fn declare(
        &self,
        product_id: &str,
        principal_id: &str,
        receipt: &PdsPackageInstallationReceiptV1,
    ) -> Result<ProductDeclarationV1, TheoryRouterError> {
        ProductDeclarationV1::new(
            product_id.into(),
            principal_id.into(),
            vec![ProductPackageSelectionV1 {
                package_id: receipt.package_id.clone(),
                package_version: receipt.package_version.clone(),
                package_content_hash: receipt.package_content_hash.clone(),
            }],
            self.agent_positions.clone(),
            ActivationParticipantPlanV1::new(self.participants.clone())?,
            self.requested_authority_ref.clone(),
            format!("principal-grant::{principal_id}"),
            self.compilation_policy_revision.clone(),
            self.participant_bindings.clone(),
        )
    }
}
