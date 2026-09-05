//! Situated PDS assignment over one complete product compilation.

use meld_events::DomainObjectRef;
use serde::{Deserialize, Serialize};

use crate::error::ApiError;

/// One product topology position bound to a durable Agent identity.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssignedAgentPositionV1 {
    pub position_id: String,
    pub agent_id: String,
}

/// One situated assignment of an exact compiled product.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StewardshipAssignmentV1 {
    pub assignment_id: String,
    pub product_compilation_receipt_id: String,
    pub product_revision_id: String,
    pub principal_id: String,
    pub subject: DomainObjectRef,
    pub perspective_id: String,
    pub branch_id: String,
    pub topology_id: String,
    pub agent_positions: Vec<AssignedAgentPositionV1>,
    pub requested_authority_ref: String,
    pub principal_grant_ref: String,
}

#[derive(Serialize)]
struct AssignmentIdentity<'a> {
    product_compilation_receipt_id: &'a str,
    product_revision_id: &'a str,
    principal_id: &'a str,
    subject: &'a DomainObjectRef,
    perspective_id: &'a str,
    branch_id: &'a str,
    topology_id: &'a str,
    agent_positions: &'a [AssignedAgentPositionV1],
    requested_authority_ref: &'a str,
    principal_grant_ref: &'a str,
}

impl StewardshipAssignmentV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        product_compilation_receipt_id: String,
        product_revision_id: String,
        principal_id: String,
        subject: DomainObjectRef,
        perspective_id: String,
        branch_id: String,
        topology_id: String,
        mut agent_positions: Vec<AssignedAgentPositionV1>,
        requested_authority_ref: String,
        principal_grant_ref: String,
    ) -> Result<Self, ApiError> {
        agent_positions.sort();
        let values = [
            product_compilation_receipt_id.as_str(),
            product_revision_id.as_str(),
            principal_id.as_str(),
            perspective_id.as_str(),
            branch_id.as_str(),
            topology_id.as_str(),
            requested_authority_ref.as_str(),
            principal_grant_ref.as_str(),
        ];
        if values.iter().any(|value| value.trim().is_empty())
            || agent_positions.is_empty()
            || agent_positions.iter().any(|position| {
                position.position_id.trim().is_empty() || position.agent_id.trim().is_empty()
            })
            || !agent_positions
                .windows(2)
                .all(|pair| pair[0].position_id != pair[1].position_id)
        {
            return Err(ApiError::ConfigError(
                "PDS assignment identity fields and Agent topology must be complete".to_string(),
            ));
        }
        let identity = AssignmentIdentity {
            product_compilation_receipt_id: &product_compilation_receipt_id,
            product_revision_id: &product_revision_id,
            principal_id: &principal_id,
            subject: &subject,
            perspective_id: &perspective_id,
            branch_id: &branch_id,
            topology_id: &topology_id,
            agent_positions: &agent_positions,
            requested_authority_ref: &requested_authority_ref,
            principal_grant_ref: &principal_grant_ref,
        };
        let assignment_id = identity_hash(&identity)?;
        Ok(Self {
            assignment_id,
            product_compilation_receipt_id,
            product_revision_id,
            principal_id,
            subject,
            perspective_id,
            branch_id,
            topology_id,
            agent_positions,
            requested_authority_ref,
            principal_grant_ref,
        })
    }

    pub fn verify_identity(&self) -> Result<(), ApiError> {
        let rebuilt = Self::new(
            self.product_compilation_receipt_id.clone(),
            self.product_revision_id.clone(),
            self.principal_id.clone(),
            self.subject.clone(),
            self.perspective_id.clone(),
            self.branch_id.clone(),
            self.topology_id.clone(),
            self.agent_positions.clone(),
            self.requested_authority_ref.clone(),
            self.principal_grant_ref.clone(),
        )?;
        if rebuilt != *self {
            return Err(ApiError::ConfigError(
                "PDS assignment identity mismatch".to_string(),
            ));
        }
        Ok(())
    }
}

fn identity_hash(value: &impl Serialize) -> Result<String, ApiError> {
    serde_json::to_vec(value)
        .map(|bytes| blake3::hash(&bytes).to_hex().to_string())
        .map_err(|failure| ApiError::ConfigError(failure.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_binds_complete_compilation_and_topology() {
        let assignment = StewardshipAssignmentV1::new(
            "compilation".into(),
            "product-revision".into(),
            "principal".into(),
            DomainObjectRef::new("workspace_fs", "node", "docs").unwrap(),
            "perspective".into(),
            "main".into(),
            "topology".into(),
            vec![AssignedAgentPositionV1 {
                position_id: "steward".into(),
                agent_id: "agent".into(),
            }],
            "authority".into(),
            "grant".into(),
        )
        .unwrap();
        assert_eq!(assignment.assignment_id.len(), 64);
        assignment.verify_identity().unwrap();
    }
}
