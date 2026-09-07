use super::contracts::*;
use serde::{Deserialize, Serialize};

/// Source-reported knowledge before the Security owner authors canonical identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdvisorySourceDocumentV1 {
    pub source_id: String,
    pub source_revision: String,
    pub covered_ecosystem: PackageEcosystem,
    pub covered_components: Vec<ComponentCoverageV1>,
    pub advisories: Vec<NormalizedAdvisoryV1>,
    pub conflicts: Vec<String>,
    pub completeness: AdvisoryCompleteness,
    pub acquired_at: u64,
}

impl AdvisorySourceDocumentV1 {
    pub fn admit(self) -> Result<AdvisoryKnowledgeSnapshotV1, String> {
        AdvisoryKnowledgeSnapshotV1::canonical(
            self.source_id,
            self.source_revision,
            self.covered_ecosystem,
            self.covered_components,
            self.advisories,
            self.conflicts,
            self.completeness,
            self.acquired_at,
        )
    }
}

impl From<AdvisoryKnowledgeSnapshotV1> for AdvisorySourceDocumentV1 {
    fn from(snapshot: AdvisoryKnowledgeSnapshotV1) -> Self {
        Self {
            source_id: snapshot.source_id,
            source_revision: snapshot.source_revision,
            covered_ecosystem: snapshot.covered_ecosystem,
            covered_components: snapshot.covered_components,
            advisories: snapshot.advisories,
            conflicts: snapshot.conflicts,
            completeness: snapshot.completeness,
            acquired_at: snapshot.acquired_at,
        }
    }
}

#[derive(Serialize)]
struct Identity<'a> {
    source_id: &'a str,
    source_revision: &'a str,
    covered_ecosystem: &'a PackageEcosystem,
    covered_components: &'a [ComponentCoverageV1],
    advisories: &'a [NormalizedAdvisoryV1],
    conflicts: &'a [String],
    completeness: &'a AdvisoryCompleteness,
    acquired_at: u64,
}

impl AdvisoryKnowledgeSnapshotV1 {
    /// Verify exact source identity and normalized advisory meaning at consumption.
    pub fn validate(&self) -> Result<(), String> {
        let expected = Self::canonical(
            self.source_id.clone(),
            self.source_revision.clone(),
            self.covered_ecosystem.clone(),
            self.covered_components.clone(),
            self.advisories.clone(),
            self.conflicts.clone(),
            self.completeness.clone(),
            self.acquired_at,
        )?;
        if &expected != self {
            return Err("advisory identity or canonical content differs".into());
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn canonical(
        source_id: String,
        source_revision: String,
        covered_ecosystem: PackageEcosystem,
        mut covered_components: Vec<ComponentCoverageV1>,
        mut advisories: Vec<NormalizedAdvisoryV1>,
        mut conflicts: Vec<String>,
        completeness: AdvisoryCompleteness,
        acquired_at: u64,
    ) -> Result<Self, String> {
        if source_id.trim().is_empty() || source_revision.trim().is_empty() {
            return Err("advisory source identity is incomplete".into());
        }
        if covered_components.iter().any(|component| {
            component.package_name.trim().is_empty() || component.source_identity.trim().is_empty()
        }) {
            return Err("advisory coverage requires exact package and source identities".into());
        }
        let mut ids = std::collections::BTreeSet::new();
        if advisories.iter().any(|advisory| {
            advisory.source_advisory_id.trim().is_empty()
                || advisory.package_name.trim().is_empty()
                || advisory.affected_versions.is_empty()
                || advisory
                    .affected_versions
                    .iter()
                    .any(|version| semver::Version::parse(version).is_err())
                || !ids.insert(&advisory.source_advisory_id)
        }) {
            return Err("advisories require unique identities and exact semantic versions".into());
        }
        covered_components.sort();
        covered_components.dedup();
        conflicts.sort();
        conflicts.dedup();
        for advisory in &mut advisories {
            advisory.aliases.sort();
            advisory.affected_versions.sort();
        }
        advisories.sort_by(|a, b| a.source_advisory_id.cmp(&b.source_advisory_id));
        let identity = Identity {
            source_id: &source_id,
            source_revision: &source_revision,
            covered_ecosystem: &covered_ecosystem,
            covered_components: &covered_components,
            advisories: &advisories,
            conflicts: &conflicts,
            completeness: &completeness,
            acquired_at,
        };
        let snapshot_id = content_hash(&identity)?;
        Ok(Self {
            snapshot_id,
            source_id,
            source_revision,
            covered_ecosystem,
            covered_components,
            advisories,
            conflicts,
            completeness,
            acquired_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn advisory_ranges_cannot_be_misread_as_unaffected_exact_versions() {
        for version in ["<1.0.0", "*", "1.0", "1.0.0 || 2.0.0"] {
            assert!(AdvisoryKnowledgeSnapshotV1::canonical(
                "source".into(),
                "revision".into(),
                PackageEcosystem::Cargo,
                vec![],
                vec![NormalizedAdvisoryV1 {
                    source_advisory_id: "advisory".into(),
                    aliases: vec![],
                    package_name: "package".into(),
                    affected_versions: vec![version.into()],
                    severity: SeverityV1::High,
                }],
                vec![],
                AdvisoryCompleteness::CompleteForDeclaredCoverage,
                1,
            )
            .is_err());
        }
    }
    #[test]
    fn advisory_identity_retains_source_revision() {
        let make = |revision: &str| {
            AdvisoryKnowledgeSnapshotV1::canonical(
                "fixture".into(),
                revision.into(),
                PackageEcosystem::Cargo,
                vec![],
                vec![],
                vec![],
                AdvisoryCompleteness::CompleteForDeclaredCoverage,
                1,
            )
            .unwrap()
        };
        assert_ne!(make("a").snapshot_id, make("b").snapshot_id);
    }
}
