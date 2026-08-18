use super::contracts::*;
use serde::Serialize;

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
