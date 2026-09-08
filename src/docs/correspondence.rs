//! Observed source-to-README correspondence, independent of required coverage.

use super::capability::{DocsCapabilityConfig, DocsEvidenceBundle};
use super::claim_validation::{decode_json_response, DocsClaimJudge, DocsClaimPolicy, ReadmeClaim};
use super::observation::{validate_observation, ObservedReadme, ObservedReadmeState};
use super::source_claims::{DocsSourceClaimReport, ObservedSourceClaim};
use crate::error::ApiError;
use crate::execution::{ProviderExecutionPort, ProviderValidationPort};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const CORRESPONDENCE_CONTRACT: &str = "docs.claim-correspondence.v2";
const LEGACY_CORRESPONDENCE_CONTRACT: &str = "docs.claim-correspondence.v1";

/// A source assertion selected from this README's directory subtree. Selection
/// defines the comparison scope and does not make the assertion required.
#[derive(Debug, Serialize)]
pub struct CorrespondenceSource<'a> {
    pub path: &'a str,
    pub claim: &'a ObservedSourceClaim,
}

pub struct DocsCorrespondenceRequest<'a> {
    pub readme_path: &'a str,
    pub sources: &'a [CorrespondenceSource<'a>],
    pub readme_claims: &'a [ReadmeClaim],
    pub policy: &'a DocsClaimPolicy,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProposedCorrespondence {
    #[serde(skip)]
    pub execution: Option<super::judgment::DocsJudgmentExecution>,
    pub complete: bool,
    pub claims: Vec<SourceCorrespondence>,
}

/// Empty matches mean no representation was established. A proposed match is
/// not a correctness or materiality decision.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceCorrespondence {
    pub source_claim_id: String,
    pub readme_claim_ids: Vec<String>,
    pub confidence: f64,
    pub rationale: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReadmeCorrespondence {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution: Option<super::judgment::DocsJudgmentExecution>,
    pub path: String,
    pub content_hash: Option<String>,
    pub claims: Vec<SourceCorrespondence>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocsCorrespondenceReport {
    pub report_id: String,
    pub input_id: String,
    pub contract_revision: String,
    pub policy_identity: String,
    pub complete: bool,
    pub readmes: Vec<ReadmeCorrespondence>,
}

pub(crate) fn input_identity(
    policy: &DocsClaimPolicy,
    bundle: &DocsEvidenceBundle,
    sources: &DocsSourceClaimReport,
) -> Result<String, ApiError> {
    policy.validate()?;
    super::observation::validate_selected_scope(policy, bundle)?;
    sources.validate_capture(bundle)?;
    if !sources.complete || sources.policy_identity != policy.content_identity() {
        return Err(invalid(
            "Docs correspondence requires complete source claims under its policy",
        ));
    }
    captured_readmes(bundle)?;
    input_for_policy(
        CORRESPONDENCE_CONTRACT,
        &policy.content_identity(),
        bundle,
        sources,
    )
}

pub(crate) fn legacy_input_identity(
    policy: &DocsClaimPolicy,
    bundle: &DocsEvidenceBundle,
    sources: &DocsSourceClaimReport,
) -> Result<String, ApiError> {
    input_for_policy(
        LEGACY_CORRESPONDENCE_CONTRACT,
        &policy.content_identity(),
        bundle,
        sources,
    )
}

fn input_for_policy(
    contract: &str,
    policy: &str,
    bundle: &DocsEvidenceBundle,
    sources: &DocsSourceClaimReport,
) -> Result<String, ApiError> {
    identity(
        "docs-correspondence-input",
        &(
            contract,
            policy,
            &bundle
                .observation
                .as_ref()
                .ok_or_else(|| invalid("Docs capture absent"))?
                .revision_id,
            &sources.report_id,
        ),
    )
}

fn captured_readmes(bundle: &DocsEvidenceBundle) -> Result<Vec<&ObservedReadme>, ApiError> {
    validate_observation(bundle)?;
    let observed = bundle.observation.as_ref().expect("validated capture");
    if !observed.coverage_gaps.is_empty()
        || observed
            .readmes
            .iter()
            .any(|readme| matches!(readme.state, ObservedReadmeState::Unavailable { .. }))
    {
        return Err(invalid(
            "Docs correspondence requires complete captured source and README evidence",
        ));
    }
    let expected = bundle
        .directories
        .iter()
        .map(|directory| super::observation::document_path(bundle, &directory.path))
        .collect::<BTreeSet<_>>();
    let actual = observed
        .readmes
        .iter()
        .map(|readme| readme.path.clone())
        .collect::<BTreeSet<_>>();
    if expected != actual || expected.len() != bundle.directories.len() {
        return Err(invalid(
            "Docs correspondence must name every managed README exactly once",
        ));
    }
    let mut readmes = observed.readmes.iter().collect::<Vec<_>>();
    readmes.sort_by(|a, b| a.path.cmp(&b.path));
    if readmes.windows(2).any(|pair| pair[0].path == pair[1].path) {
        return Err(invalid("Docs correspondence repeats a README"));
    }
    Ok(readmes)
}

fn selected_sources<'a>(
    path: &str,
    sources: &'a DocsSourceClaimReport,
    bundle: &DocsEvidenceBundle,
) -> Vec<CorrespondenceSource<'a>> {
    let scope = bundle
        .observation
        .as_ref()
        .and_then(|capture| capture.scope.as_ref());
    let mut selected = sources
        .files
        .iter()
        .filter(|file| match scope {
            Some(scope) => scope.compares(path, &file.path),
            // Historical captures use the original subtree comparison contract.
            None => path
                .strip_suffix("README.md")
                .is_some_and(|prefix| file.path.starts_with(prefix)),
        })
        .flat_map(|file| {
            file.claims.iter().map(|claim| CorrespondenceSource {
                path: &file.path,
                claim,
            })
        })
        .collect::<Vec<_>>();
    selected.sort_by(|a, b| a.claim.claim_id.cmp(&b.claim.claim_id));
    selected
}

fn readme_claims(readme: &ObservedReadme) -> &[ReadmeClaim] {
    match &readme.state {
        ObservedReadmeState::Present { claims, .. } => claims,
        _ => &[],
    }
}
fn content_hash(readme: &ObservedReadme) -> Option<String> {
    match &readme.state {
        ObservedReadmeState::Present { content_hash, .. } => Some(content_hash.clone()),
        _ => None,
    }
}

pub(crate) async fn advance_correspondence(
    judge: &dyn DocsClaimJudge,
    policy: &DocsClaimPolicy,
    bundle: &DocsEvidenceBundle,
    sources: &DocsSourceClaimReport,
    prior: Option<&DocsCorrespondenceReport>,
    max_readmes: usize,
) -> Result<DocsCorrespondenceReport, ApiError> {
    let input_id = input_identity(policy, bundle, sources)?;
    if let Some(prior) = prior {
        prior.validate_capture(bundle, sources)?;
        if prior.input_id != input_id {
            return Err(invalid(
                "Docs correspondence progress belongs to another input",
            ));
        }
    }
    let readmes = captured_readmes(bundle)?;
    let mut results = prior.map_or_else(Vec::new, |report| report.readmes.clone());
    for readme in readmes.iter().skip(results.len()).take(max_readmes) {
        let selected = selected_sources(&readme.path, sources, bundle);
        let claims = readme_claims(readme);
        let proposed = if selected.is_empty() || claims.is_empty() {
            ProposedCorrespondence {
                execution: None,
                complete: true,
                claims: selected
                    .iter()
                    .map(|source| SourceCorrespondence {
                        source_claim_id: source.claim.claim_id.clone(),
                        readme_claim_ids: vec![],
                        confidence: 1.0,
                        rationale: "The captured README has no assertions.".into(),
                    })
                    .collect(),
            }
        } else {
            judge
                .correspond(&DocsCorrespondenceRequest {
                    readme_path: &readme.path,
                    sources: &selected,
                    readme_claims: claims,
                    policy,
                })
                .await?
        };
        if !proposed.complete {
            return Err(invalid("Docs correspondence proposal is incomplete"));
        }
        let mut result = ReadmeCorrespondence {
            execution: proposed.execution,
            path: readme.path.clone(),
            content_hash: content_hash(readme),
            claims: proposed.claims,
        };
        for claim in &mut result.claims {
            claim.readme_claim_ids.sort();
        }
        result
            .claims
            .sort_by(|a, b| a.source_claim_id.cmp(&b.source_claim_id));
        validate_readme(&result, readme, &selected, policy.minimum_claim_confidence)?;
        results.push(result);
    }
    let mut report = DocsCorrespondenceReport {
        report_id: String::new(),
        input_id,
        contract_revision: CORRESPONDENCE_CONTRACT.into(),
        policy_identity: policy.content_identity(),
        complete: results.len() == readmes.len(),
        readmes: results,
    };
    report.report_id = report.identity()?;
    Ok(report)
}

impl DocsCorrespondenceReport {
    #[cfg(test)]
    pub(crate) fn legacy_fixture(
        &self,
        bundle: &DocsEvidenceBundle,
        sources: &DocsSourceClaimReport,
    ) -> Self {
        let mut old = self.clone();
        old.contract_revision = LEGACY_CORRESPONDENCE_CONTRACT.into();
        old.input_id = input_for_policy(
            LEGACY_CORRESPONDENCE_CONTRACT,
            &self.policy_identity,
            bundle,
            sources,
        )
        .unwrap();
        old.report_id = old.identity().unwrap();
        old
    }

    /// Stable historical reports retain their original publication. The current
    /// projection adds native support predicates without repeating semantic judgment.
    pub(crate) fn upgraded(
        &self,
        bundle: &DocsEvidenceBundle,
        sources: &DocsSourceClaimReport,
    ) -> Result<Self, ApiError> {
        self.validate_capture(bundle, sources)?;
        let mut next = self.clone();
        next.contract_revision = CORRESPONDENCE_CONTRACT.into();
        next.input_id = input_for_policy(
            CORRESPONDENCE_CONTRACT,
            &self.policy_identity,
            bundle,
            sources,
        )?;
        next.report_id = next.identity()?;
        Ok(next)
    }

    fn identity(&self) -> Result<String, ApiError> {
        identity(
            "docs-correspondence",
            &(
                &self.input_id,
                &self.contract_revision,
                &self.policy_identity,
                self.complete,
                &self.readmes,
            ),
        )
    }

    pub(crate) fn validate_capture(
        &self,
        bundle: &DocsEvidenceBundle,
        sources: &DocsSourceClaimReport,
    ) -> Result<(), ApiError> {
        sources.validate_capture(bundle)?;
        let readmes = captured_readmes(bundle)?;
        if !sources.complete
            || self.policy_identity != sources.policy_identity
            || ![CORRESPONDENCE_CONTRACT, LEGACY_CORRESPONDENCE_CONTRACT]
                .contains(&self.contract_revision.as_str())
            || self.report_id != self.identity()?
            || self.input_id
                != input_for_policy(
                    &self.contract_revision,
                    &self.policy_identity,
                    bundle,
                    sources,
                )?
            || self.readmes.len() > readmes.len()
            || (self.complete && self.readmes.len() != readmes.len())
        {
            return Err(invalid(
                "Docs correspondence identity or input binding is invalid",
            ));
        }
        for (result, readme) in self.readmes.iter().zip(readmes) {
            if let Some(execution) = &result.execution {
                execution.validate(Some(&self.policy_identity))?;
            }
            validate_readme(
                result,
                readme,
                &selected_sources(&readme.path, sources, bundle),
                0.0,
            )?;
        }
        Ok(())
    }
}

fn validate_readme(
    result: &ReadmeCorrespondence,
    readme: &ObservedReadme,
    selected: &[CorrespondenceSource<'_>],
    minimum_confidence: f64,
) -> Result<(), ApiError> {
    let expected = selected
        .iter()
        .map(|source| source.claim.claim_id.as_str())
        .collect::<BTreeSet<_>>();
    let actual = result
        .claims
        .iter()
        .map(|claim| claim.source_claim_id.as_str())
        .collect::<BTreeSet<_>>();
    if result.path != readme.path
        || result.content_hash != content_hash(readme)
        || actual != expected
        || actual.len() != result.claims.len()
    {
        return Err(invalid(
            "Docs correspondence must account for every selected source claim exactly once",
        ));
    }
    let observed = readme_claims(readme)
        .iter()
        .map(|claim| claim.claim_id.as_str())
        .collect::<BTreeSet<_>>();
    for claim in &result.claims {
        let matches = claim
            .readme_claim_ids
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        if !matches.is_subset(&observed)
            || matches.len() != claim.readme_claim_ids.len()
            || claim.rationale.trim().is_empty()
            || !claim.confidence.is_finite()
            || !(0.0..=1.0).contains(&claim.confidence)
            || (!matches.is_empty() && claim.confidence < minimum_confidence)
        {
            return Err(invalid(
                "Docs correspondence names unknown claims or invalid evidence",
            ));
        }
    }
    Ok(())
}

pub(crate) async fn provider_correspondence<
    P: ProviderValidationPort + ProviderExecutionPort + ?Sized,
>(
    api: &P,
    config: &DocsCapabilityConfig,
    request: &DocsCorrespondenceRequest<'_>,
) -> Result<ProposedCorrespondence, ApiError> {
    request.policy.validate()?;
    let generation = request.policy.semantics()?.generation(
        config, &request.policy.content_identity(), super::semantics::DocsJudgmentOperation::Correspondence,
        serde_json::json!({"readme_path": request.readme_path, "sources": request.sources, "readme_claims": request.readme_claims}), 0, 0,
    )?;
    let preparation =
        crate::provider::executor::prepare_provider_for_request(api, &generation.request)?;
    let result = crate::provider::executor::execute_completion(
        api,
        &generation.request,
        &preparation,
        generation.messages,
        None,
    )
    .await?;
    let execution = super::judgment::DocsJudgmentExecution::capture(
        request.policy.content_identity(),
        &generation.request,
        &preparation,
        &result,
    )?;
    if let Some(claims) =
        super::claim_validation::decode_keyed_claims(&result.content, "claims", "source_claim_id")?
    {
        return Ok(ProposedCorrespondence {
            execution: Some(execution),
            complete: true,
            claims,
        });
    }
    let mut proposed: ProposedCorrespondence =
        decode_json_response(&result.content, "claim correspondence")?;
    proposed.execution = Some(execution);
    Ok(proposed)
}

fn identity(kind: &str, value: &impl Serialize) -> Result<String, ApiError> {
    Ok(format!(
        "{kind}::{}",
        blake3::hash(&serde_json::to_vec(value).map_err(|error| invalid(&error.to_string()))?)
            .to_hex()
    ))
}
fn invalid(message: &str) -> ApiError {
    ApiError::ConfigError(message.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::docs::claim_observation::test_support::{policy, FixtureJudge};
    use crate::docs::claim_validation::{DocsClaimJudgmentRequest, ProviderClaimAssessment};
    use std::sync::atomic::Ordering;

    #[tokio::test]
    async fn an_accurate_assertion_does_not_represent_an_omitted_source_claim() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(
            root.path().join("lib.rs"),
            "pub fn start() {}\npub fn stop() {}\n",
        )
        .unwrap();
        std::fs::write(root.path().join("README.md"), "`start` is declared.\n").unwrap();
        let bundle = crate::docs::observation::inspect_scope(root.path()).unwrap();
        let judge = FixtureJudge::default();
        let policy = policy();
        let sources =
            crate::docs::source_claims::advance_source_claims(&judge, &policy, &bundle, None, 1)
                .await
                .unwrap();
        let report = advance_correspondence(&judge, &policy, &bundle, &sources, None, 1)
            .await
            .unwrap();
        assert!(report.complete);
        let start = sources.files[0]
            .claims
            .iter()
            .find(|claim| claim.statement.contains("`start`"))
            .unwrap();
        let stop = sources.files[0]
            .claims
            .iter()
            .find(|claim| claim.statement.contains("`stop`"))
            .unwrap();
        let matches = &report.readmes[0].claims;
        assert!(!matches
            .iter()
            .find(|claim| claim.source_claim_id == start.claim_id)
            .unwrap()
            .readme_claim_ids
            .is_empty());
        assert!(matches
            .iter()
            .find(|claim| claim.source_claim_id == stop.claim_id)
            .unwrap()
            .readme_claim_ids
            .is_empty());
        assert_eq!(judge.correspondence_calls.load(Ordering::SeqCst), 1);
        std::fs::write(
            root.path().join("README.md"),
            "`start` is declared.\n`stop` is declared.\n",
        )
        .unwrap();
        let changed = crate::docs::observation::inspect_scope(root.path()).unwrap();
        assert!(report.validate_capture(&changed, &sources).is_err());
        assert!(
            advance_correspondence(&judge, &policy, &changed, &sources, Some(&report), 1)
                .await
                .is_err()
        );
        let complete = advance_correspondence(&judge, &policy, &changed, &sources, None, 1)
            .await
            .unwrap();
        assert!(complete.readmes[0]
            .claims
            .iter()
            .all(|claim| !claim.readme_claim_ids.is_empty()));
        assert_ne!(complete.input_id, report.input_id);
        assert_eq!(judge.source_calls.load(Ordering::SeqCst), 1);
        std::fs::write(root.path().join("lib.rs"), "pub fn replacement() {}\n").unwrap();
        let replaced = crate::docs::observation::inspect_scope(root.path()).unwrap();
        assert!(complete.validate_capture(&replaced, &sources).is_err());
    }

    #[tokio::test]
    async fn missing_readme_is_explicit_and_directory_selection_does_not_include_siblings() {
        let root = tempfile::tempdir().unwrap();
        for directory in ["a", "ab"] {
            std::fs::create_dir(root.path().join(directory)).unwrap();
            std::fs::write(
                root.path().join(directory).join("lib.rs"),
                "pub fn run() {}\n",
            )
            .unwrap();
        }
        let bundle = crate::docs::observation::inspect_scope(root.path()).unwrap();
        let judge = FixtureJudge::default();
        let policy = policy();
        let sources =
            crate::docs::source_claims::advance_source_claims(&judge, &policy, &bundle, None, 2)
                .await
                .unwrap();
        let report = advance_correspondence(&judge, &policy, &bundle, &sources, None, usize::MAX)
            .await
            .unwrap();
        assert!(report.complete);
        assert_eq!(
            report
                .readmes
                .iter()
                .find(|readme| readme.path == "a/README.md")
                .unwrap()
                .claims
                .len(),
            1
        );
        assert_eq!(
            report
                .readmes
                .iter()
                .find(|readme| readme.path == "README.md")
                .unwrap()
                .claims
                .len(),
            2
        );
        assert!(report
            .readmes
            .iter()
            .all(|readme| readme.content_hash.is_none()
                && readme
                    .claims
                    .iter()
                    .all(|claim| claim.readme_claim_ids.is_empty())));
        assert_eq!(judge.correspondence_calls.load(Ordering::SeqCst), 0);
    }

    struct BadJudge(&'static str);
    #[async_trait::async_trait]
    impl DocsClaimJudge for BadJudge {
        async fn assess(
            &self,
            _: &DocsClaimJudgmentRequest<'_>,
        ) -> Result<Vec<ProviderClaimAssessment>, ApiError> {
            unreachable!()
        }
        async fn correspond(
            &self,
            request: &DocsCorrespondenceRequest<'_>,
        ) -> Result<ProposedCorrespondence, ApiError> {
            let mut proposal = ProposedCorrespondence {
                execution: None,
                complete: true,
                claims: request
                    .sources
                    .iter()
                    .map(|source| SourceCorrespondence {
                        source_claim_id: source.claim.claim_id.clone(),
                        readme_claim_ids: vec![request.readme_claims[0].claim_id.clone()],
                        confidence: 1.0,
                        rationale: "proposed match".into(),
                    })
                    .collect(),
            };
            match self.0 {
                "omit" => {
                    proposal.claims.pop();
                }
                "duplicate" => proposal.claims.push(proposal.claims[0].clone()),
                "foreign_source" => proposal.claims[0].source_claim_id = "foreign".into(),
                "foreign_readme" => proposal.claims[0].readme_claim_ids = vec!["foreign".into()],
                "duplicate_match" => proposal.claims[0]
                    .readme_claim_ids
                    .push(request.readme_claims[0].claim_id.clone()),
                "incomplete" => proposal.complete = false,
                "uncertain" => proposal.claims[0].confidence = 0.01,
                "uncertain_missing" => {
                    proposal.claims[0].confidence = 0.01;
                    proposal.claims[0].readme_claim_ids.clear();
                }
                "unreasoned" => proposal.claims[0].rationale.clear(),
                _ => unreachable!(),
            }
            Ok(proposal)
        }
    }

    #[tokio::test]
    async fn malformed_or_partial_proposals_never_become_complete_correspondence() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("lib.rs"), "pub fn run() {}\n").unwrap();
        std::fs::write(root.path().join("README.md"), "`run` exists.\n").unwrap();
        let bundle = crate::docs::observation::inspect_scope(root.path()).unwrap();
        let policy = policy();
        let sources = crate::docs::source_claims::advance_source_claims(
            &FixtureJudge::default(),
            &policy,
            &bundle,
            None,
            1,
        )
        .await
        .unwrap();
        for defect in [
            "omit",
            "duplicate",
            "foreign_source",
            "foreign_readme",
            "duplicate_match",
            "incomplete",
            "uncertain",
            "unreasoned",
        ] {
            assert!(
                advance_correspondence(&BadJudge(defect), &policy, &bundle, &sources, None, 1)
                    .await
                    .is_err(),
                "{defect}"
            );
        }
        let unresolved = advance_correspondence(
            &BadJudge("uncertain_missing"),
            &policy,
            &bundle,
            &sources,
            None,
            1,
        )
        .await
        .unwrap();
        assert!(unresolved.complete);
        assert!(unresolved.readmes[0].claims[0].readme_claim_ids.is_empty());
        assert_eq!(unresolved.readmes[0].claims[0].confidence, 0.01);
    }
}
