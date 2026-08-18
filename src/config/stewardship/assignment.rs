//! Standing PDS assignment identity without physical activation data.

use meld_events::DomainObjectRef;
use serde::{Deserialize, Serialize};

use crate::error::ApiError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StewardshipAssignmentV1 {
    pub assignment_id: String,
    pub package_receipt_id: String,
    pub principal_id: String,
    pub agent_id: String,
    pub subject: DomainObjectRef,
    pub perspective_id: String,
    pub branch_id: String,
    pub requested_authority_ref: String,
    pub principal_grant_ref: String,
}

#[derive(Serialize)]
struct AssignmentIdentity<'a> {
    package_receipt_id: &'a str,
    principal_id: &'a str,
    agent_id: &'a str,
    subject: &'a DomainObjectRef,
    perspective_id: &'a str,
    branch_id: &'a str,
    requested_authority_ref: &'a str,
    principal_grant_ref: &'a str,
}

impl StewardshipAssignmentV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        package_receipt_id: String,
        principal_id: String,
        agent_id: String,
        subject: DomainObjectRef,
        perspective_id: String,
        branch_id: String,
        requested_authority_ref: String,
        principal_grant_ref: String,
    ) -> Result<Self, ApiError> {
        let values = [
            package_receipt_id.as_str(),
            principal_id.as_str(),
            agent_id.as_str(),
            perspective_id.as_str(),
            branch_id.as_str(),
            requested_authority_ref.as_str(),
            principal_grant_ref.as_str(),
        ];
        if values.iter().any(|value| value.trim().is_empty()) {
            return Err(ApiError::ConfigError(
                "PDS assignment identity fields must be non-empty".to_string(),
            ));
        }
        let identity = AssignmentIdentity {
            package_receipt_id: &package_receipt_id,
            principal_id: &principal_id,
            agent_id: &agent_id,
            subject: &subject,
            perspective_id: &perspective_id,
            branch_id: &branch_id,
            requested_authority_ref: &requested_authority_ref,
            principal_grant_ref: &principal_grant_ref,
        };
        let assignment_id = identity_hash(&identity)?;
        Ok(Self {
            assignment_id,
            package_receipt_id,
            principal_id,
            agent_id,
            subject,
            perspective_id,
            branch_id,
            requested_authority_ref,
            principal_grant_ref,
        })
    }

    pub fn verify_identity(&self) -> Result<(), ApiError> {
        let identity = AssignmentIdentity {
            package_receipt_id: &self.package_receipt_id,
            principal_id: &self.principal_id,
            agent_id: &self.agent_id,
            subject: &self.subject,
            perspective_id: &self.perspective_id,
            branch_id: &self.branch_id,
            requested_authority_ref: &self.requested_authority_ref,
            principal_grant_ref: &self.principal_grant_ref,
        };
        if identity_hash(&identity)? != self.assignment_id {
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
    fn identity_excludes_physical_bindings() {
        let assignment = StewardshipAssignmentV1::new(
            "receipt".into(),
            "principal".into(),
            "agent".into(),
            DomainObjectRef::new("workspace_fs", "node", "docs").unwrap(),
            "perspective".into(),
            "main".into(),
            "authority".into(),
            "grant".into(),
        )
        .unwrap();
        assert_eq!(assignment.assignment_id.len(), 64);
        assignment.verify_identity().unwrap();
    }
}
