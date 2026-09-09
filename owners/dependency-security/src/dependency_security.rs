//! Native inventory and evidence-grounded dependency-security assessment.

pub mod advisory;
pub mod assessment;
pub mod capability;
pub mod condition;
pub mod contracts;
pub mod contribution;
pub mod currency;
pub mod events;
pub mod inventory;
pub mod observation;
pub mod policy;
pub mod publication;
pub mod returns;
pub mod runtime;
pub mod theory;
pub mod verification;

pub use contracts::*;

pub mod owner;

#[cfg(feature = "test-support")]
pub mod test_support {
    pub fn pending_inventory_returns(
        bindings: std::collections::BTreeMap<String, String>,
        events: &meld_events::EventReplayCapability,
        workspace_override: Option<std::path::PathBuf>,
    ) -> Result<Vec<super::returns::ExecutionObservationCause>, String> {
        let mut capability =
            super::contribution::DependencySecurityCapabilityContributor::default()
                .capability(
                    super::capability::OBSERVE_INVENTORY,
                    &crate::capability::OwnerBindingView::new(bindings),
                )
                .map_err(|error| error.to_string())?;
        if let Some(workspace) = workspace_override {
            capability.workspace = Some(workspace);
        }
        super::returns::pending(&capability, events)
    }
}
