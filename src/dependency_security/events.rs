pub const INVENTORY_OBSERVED_V1: &str = "dependency_security.inventory_observed.v1";
pub const ADVISORY_SNAPSHOT_OBSERVED_V1: &str = "dependency_security.advisory_snapshot_observed.v1";
pub const ASSESSMENT_COMPLETED_V1: &str = "dependency_security.assessment_completed.v1";
pub const ASSESSMENT_VERIFICATION_COMPLETED_V1: &str =
    "dependency_security.assessment_verification_completed.v1";
pub const SOURCE_ADVANCED_V1: &str = "dependency_security.source_advanced.v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceAdvanceOpportunity {
    pub source_id: String,
    pub source_revision: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_advance_is_observation_opportunity() {
        let opportunity = SourceAdvanceOpportunity {
            source_id: "fixture".into(),
            source_revision: "r2".into(),
        };
        assert_eq!(opportunity.source_revision, "r2");
        assert_ne!(SOURCE_ADVANCED_V1, ASSESSMENT_COMPLETED_V1);
    }
}
