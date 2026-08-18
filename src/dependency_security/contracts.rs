use meld_events::DomainObjectRef;
use serde::{Deserialize, Serialize};

use crate::theory::TheoryRevisionRef;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PackageEcosystem {
    Cargo,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InventoryScopeV1 {
    pub manifest_ref: DomainObjectRef,
    pub lockfile_ref: DomainObjectRef,
    pub include_transitive: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DependencySecuritySubjectV1 {
    pub subject: DomainObjectRef,
    pub ecosystem: PackageEcosystem,
    pub inventory_scope: InventoryScopeV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvedDependencyComponentV1 {
    pub component_id: String,
    pub ecosystem: PackageEcosystem,
    pub package_name: String,
    pub resolved_version: String,
    pub source_identity: String,
    pub dependency_paths: Vec<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InventoryCompleteness {
    CompleteTransitive,
    Incomplete { reasons: Vec<String> },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DependencyInventorySnapshotV1 {
    pub snapshot_id: String,
    pub subject: DependencySecuritySubjectV1,
    pub workspace_revision: String,
    pub manifest_content_hash: String,
    pub lockfile_content_hash: String,
    pub components: Vec<ResolvedDependencyComponentV1>,
    pub completeness: InventoryCompleteness,
    pub observed_at: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, PartialOrd, Ord)]
pub struct ComponentCoverageV1 {
    pub package_name: String,
    pub source_identity: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum SeverityV1 {
    Low,
    Moderate,
    High,
    Critical,
}

impl SeverityV1 {
    pub fn rank(&self) -> u8 {
        match self {
            Self::Low => 1,
            Self::Moderate => 2,
            Self::High => 3,
            Self::Critical => 4,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizedAdvisoryV1 {
    pub source_advisory_id: String,
    pub aliases: Vec<String>,
    pub package_name: String,
    pub affected_versions: Vec<String>,
    pub severity: SeverityV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdvisoryCompleteness {
    CompleteForDeclaredCoverage,
    Incomplete { reasons: Vec<String> },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdvisoryKnowledgeSnapshotV1 {
    pub snapshot_id: String,
    pub source_id: String,
    pub source_revision: String,
    pub covered_ecosystem: PackageEcosystem,
    pub covered_components: Vec<ComponentCoverageV1>,
    pub advisories: Vec<NormalizedAdvisoryV1>,
    pub conflicts: Vec<String>,
    pub completeness: AdvisoryCompleteness,
    pub acquired_at: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencySecurityPosture {
    Unknown,
    Insufficient,
    CleanWithinCoverage,
    Violated,
    Stale,
    Conflicted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecurityFindingV1 {
    pub finding_id: String,
    pub advisory_id: String,
    pub component_id: String,
    pub severity: SeverityV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssessmentCoverageV1 {
    pub component_count: usize,
    pub covered_component_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DependencySecurityAssessmentV1 {
    pub assessment_id: String,
    pub subject: DependencySecuritySubjectV1,
    pub inventory_snapshot_ref: String,
    pub advisory_snapshot_ref: String,
    pub policy_revision: TheoryRevisionRef,
    pub reference_time: u64,
    pub posture: DependencySecurityPosture,
    pub findings: Vec<SecurityFindingV1>,
    pub coverage: AssessmentCoverageV1,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DependencySecurityVerificationV1 {
    pub verification_id: String,
    pub assessment_ref: String,
    pub inventory_snapshot_ref: String,
    pub advisory_snapshot_ref: String,
    pub policy_revision: TheoryRevisionRef,
    pub independently_computed_posture: DependencySecurityPosture,
    pub independently_computed_finding_refs: Vec<String>,
    pub checks: Vec<String>,
    pub verified: bool,
}

pub(crate) fn content_hash(value: &impl Serialize) -> Result<String, String> {
    serde_json::to_vec(value)
        .map(|bytes| blake3::hash(&bytes).to_hex().to_string())
        .map_err(|e| e.to_string())
}
