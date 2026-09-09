//! Read-only judgments of assertions in an exact captured Docs observation.
//!
//! Supported README assertions do not by themselves establish required source
//! coverage. Curation requirements and their correspondence remain separate inputs.

use super::capability::{DocsEvidenceBundle, ReadmePatch};
use super::claim_validation::{
    assess_captured_readme, evidence_partitions, DocsClaimJudge, DocsClaimPolicy, ReadmeClaimReport,
};
use super::observation::{validate_observation, ObservedReadmeState};
use crate::error::ApiError;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObservedDocsClaimReport {
    pub report_id: String,
    pub observation_revision_id: String,
    pub source_fingerprint: String,
    pub policy_identity: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acceptance_evaluator: Option<super::claim_validation::DocsAcceptanceEvaluator>,
    pub complete: bool,
    pub readmes: Vec<ObservedReadmeJudgment>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObservedReadmeJudgment {
    pub path: String,
    pub disposition: ObservedClaimDisposition,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "disposition", rename_all = "snake_case")]
pub enum ObservedClaimDisposition {
    Missing,
    NoClaims { content_hash: String },
    Assessed { report: ReadmeClaimReport },
}

/// Judge current captured assertions without constructing a Task, revising text,
/// pruning assertions, writing files, or consuming a publication receipt.
pub async fn assess_observed_claims(
    judge: &dyn DocsClaimJudge,
    policy: &DocsClaimPolicy,
    bundle: &DocsEvidenceBundle,
) -> Result<ObservedDocsClaimReport, ApiError> {
    advance_observed_claims(judge, policy, bundle, None, usize::MAX).await
}

pub(crate) async fn advance_observed_claims(
    judge: &dyn DocsClaimJudge,
    policy: &DocsClaimPolicy,
    bundle: &DocsEvidenceBundle,
    prior: Option<&ObservedDocsClaimReport>,
    max_readmes: usize,
) -> Result<ObservedDocsClaimReport, ApiError> {
    policy.validate()?;
    super::observation::validate_selected_scope(policy, bundle)?;
    let observed = bundle.observation.as_ref().expect("validated observation");
    if !observed.coverage_gaps.is_empty() {
        return Err(invalid(
            "Docs claim judgment requires complete captured source evidence",
        ));
    }
    let readmes = observed
        .readmes
        .iter()
        .map(|readme| (readme.path.as_str(), &readme.state))
        .collect::<BTreeMap<_, _>>();
    if readmes.len() != observed.readmes.len() || readmes.len() != bundle.directories.len() {
        return Err(invalid(
            "Docs observation does not name each managed README exactly once",
        ));
    }
    if readmes
        .values()
        .any(|state| matches!(state, ObservedReadmeState::Unavailable { .. }))
    {
        return Err(invalid(
            "Docs claim judgment cannot use unavailable README evidence",
        ));
    }
    let mut directories = bundle.directories.iter().collect::<Vec<_>>();
    directories.sort_by(|a, b| {
        b.path
            .split('/')
            .count()
            .cmp(&a.path.split('/').count())
            .then_with(|| (a.path == ".").cmp(&(b.path == ".")))
            .then_with(|| a.path.cmp(&b.path))
    });
    if prior.is_some_and(|report| !report.matches_observation(policy, bundle).unwrap_or(false)) {
        return Err(invalid(
            "Docs claim progress belongs to another observation or policy",
        ));
    }
    let expected_readmes = directories.len();
    let mut retained = prior
        .into_iter()
        .flat_map(|report| report.readmes.iter().cloned())
        .map(|judgment| (judgment.path.clone(), judgment))
        .collect::<BTreeMap<_, _>>();
    if retained.len() != prior.map_or(0, |report| report.readmes.len()) {
        return Err(invalid("Docs claim progress repeats a README"));
    }
    let mut accepted_children = BTreeMap::new();
    let mut judgments = BTreeMap::new();
    let mut advanced = 0;
    for directory in directories {
        let path = super::observation::document_path(bundle, &directory.path);
        let state = readmes
            .get(path.as_str())
            .ok_or_else(|| invalid("Docs observation is missing a managed README disposition"))?;
        if let Some(judgment) = retained.remove(&path) {
            if let (
                ObservedClaimDisposition::Assessed { report },
                ObservedReadmeState::Present { content, .. },
            ) = (&judgment.disposition, *state)
            {
                if report.accepted {
                    accepted_children.insert(
                        directory.path.clone(),
                        super::claim_validation::supported_readme_evidence(report, content),
                    );
                }
            }
            judgments.insert(path, judgment);
            continue;
        }
        if advanced == max_readmes {
            break;
        }
        advanced += 1;
        let disposition = match state {
            ObservedReadmeState::Missing => ObservedClaimDisposition::Missing,
            ObservedReadmeState::Unavailable { .. } => unreachable!("checked before any judgment"),
            ObservedReadmeState::Present {
                content_hash,
                claims,
                ..
            } if claims.is_empty() => ObservedClaimDisposition::NoClaims {
                content_hash: content_hash.clone(),
            },
            ObservedReadmeState::Present {
                content,
                content_hash,
                ..
            } => {
                let evidence = evidence_partitions(directory, &accepted_children);
                let patch = ReadmePatch {
                    path: path.clone(),
                    content: content.clone(),
                    content_hash: content_hash.clone(),
                };
                let report =
                    assess_captured_readme(judge, policy, directory, &evidence, &patch).await?;
                if report.accepted {
                    accepted_children.insert(
                        directory.path.clone(),
                        super::claim_validation::supported_readme_evidence(&report, content),
                    );
                }
                ObservedClaimDisposition::Assessed { report }
            }
        };
        let judgment = ObservedReadmeJudgment {
            path: path.clone(),
            disposition,
        };
        if judgments.insert(path, judgment).is_some() {
            return Err(invalid("Docs observation repeats a managed directory"));
        }
    }
    if !retained.is_empty() {
        return Err(invalid(
            "Docs claim progress is not a valid completed prefix",
        ));
    }
    let mut report = ObservedDocsClaimReport {
        report_id: String::new(),
        observation_revision_id: observed.revision_id.clone(),
        source_fingerprint: bundle.source_fingerprint.clone(),
        policy_identity: policy.content_identity(),
        acceptance_evaluator: policy.acceptance_evaluator,
        complete: judgments.len() == expected_readmes,
        readmes: judgments.into_values().collect(),
    };
    report.report_id = report.identity()?;
    Ok(report)
}

impl ObservedDocsClaimReport {
    fn identity(&self) -> Result<String, ApiError> {
        let legacy_basis = (
            &self.observation_revision_id,
            &self.source_fingerprint,
            &self.policy_identity,
            self.complete,
            &self.readmes,
        );
        // Historical report identities keep their original tuple. New reports
        // name the admitted evaluator in both their body and hashed lineage.
        let bytes = match self.acceptance_evaluator {
            Some(evaluator) => serde_json::to_vec(&(legacy_basis, evaluator)),
            None => serde_json::to_vec(&legacy_basis),
        }
        .map_err(|error| invalid(&error.to_string()))?;
        Ok(format!(
            "docs-observed-claims::{}",
            blake3::hash(&bytes).to_hex()
        ))
    }

    pub fn matches_capture(&self, bundle: &DocsEvidenceBundle) -> Result<bool, ApiError> {
        validate_observation(bundle)?;
        for readme in &self.readmes {
            if let ObservedClaimDisposition::Assessed { report } = &readme.disposition {
                for assessment in &report.assessments {
                    if let Some(execution) = &assessment.execution {
                        execution.validate(Some(&self.policy_identity))?;
                    }
                }
            }
        }
        Ok(self.report_id == self.identity()?
            && self.source_fingerprint == bundle.source_fingerprint
            && bundle
                .observation
                .as_ref()
                .is_some_and(|observation| observation.revision_id == self.observation_revision_id))
    }

    /// Identity and exact input binding are necessary for reuse. This does not
    /// turn the report into a claim that every required source meaning is covered.
    pub fn matches_observation(
        &self,
        policy: &DocsClaimPolicy,
        bundle: &DocsEvidenceBundle,
    ) -> Result<bool, ApiError> {
        policy.validate()?;
        Ok(self.matches_capture(bundle)?
            && self.policy_identity == policy.content_identity()
            && self.acceptance_evaluator == policy.acceptance_evaluator)
    }
}

fn invalid(message: &str) -> ApiError {
    ApiError::ConfigError(message.into())
}

/// The native owner receives its read-only judge after the process provider is composed.
#[derive(Clone, Default)]
pub struct DocsClaimJudgeSlot(
    std::sync::Arc<std::sync::Mutex<Option<std::sync::Arc<dyn DocsClaimJudge>>>>,
);
impl DocsClaimJudgeSlot {
    pub fn bind(&self, judge: std::sync::Arc<dyn DocsClaimJudge>) -> bool {
        let Ok(mut current) = self.0.lock() else {
            return false;
        };
        if current.is_some() {
            return false;
        }
        *current = Some(judge);
        true
    }
    pub(crate) fn current(&self) -> Option<std::sync::Arc<dyn DocsClaimJudge>> {
        self.0.lock().ok().and_then(|judge| judge.clone())
    }
}

#[cfg(any(test, feature = "test-support"))]
pub mod test_support {
    use super::*;
    use crate::docs::claim_validation::{
        CitationScope, ClaimCitation, ClaimVerdict, DocsClaimJudgmentRequest,
        ProviderClaimAssessment,
    };
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[derive(Default)]
    pub struct FixtureJudge {
        pub calls: AtomicUsize,
        pub source_calls: AtomicUsize,
        pub correspondence_calls: AtomicUsize,
    }
    #[async_trait::async_trait]
    impl DocsClaimJudge for FixtureJudge {
        async fn correspond(
            &self,
            request: &super::super::correspondence::DocsCorrespondenceRequest<'_>,
        ) -> Result<super::super::correspondence::ProposedCorrespondence, ApiError> {
            use super::super::correspondence::{ProposedCorrespondence, SourceCorrespondence};
            self.correspondence_calls.fetch_add(1, Ordering::SeqCst);
            Ok(ProposedCorrespondence {
                execution: None,
                complete: true,
                claims: request
                    .sources
                    .iter()
                    .map(|source| {
                        let symbol = source.claim.statement.split('`').nth(1);
                        SourceCorrespondence {
                            source_claim_id: source.claim.claim_id.clone(),
                            readme_claim_ids: request
                                .readme_claims
                                .iter()
                                .filter(|claim| {
                                    symbol.is_some_and(|symbol| {
                                        claim
                                            .literal_requirements
                                            .iter()
                                            .any(|literal| literal == symbol)
                                    })
                                })
                                .map(|claim| claim.claim_id.clone())
                                .collect(),
                            confidence: 1.0,
                            rationale: "controlled fixture matches exact declared symbol".into(),
                        }
                    })
                    .collect(),
            })
        }

        async fn extract_source(
            &self,
            request: &super::super::source_claims::DocsSourceClaimRequest<'_>,
        ) -> Result<super::super::source_claims::ProposedSourceClaims, ApiError> {
            use super::super::source_claims::{ProposedSourceClaims, SourceClaimProposal};
            self.source_calls.fetch_add(1, Ordering::SeqCst);
            let text = request
                .source
                .text
                .as_deref()
                .expect("fixture receives captured source");
            let mut claims = text
                .lines()
                .filter_map(|line| {
                    line.trim()
                        .strip_prefix("pub fn ")
                        .and_then(|tail| tail.split_once('('))
                        .map(|(name, _)| SourceClaimProposal {
                            statement: format!("`{name}` is declared"),
                            confidence: 1.0,
                            quotes: vec![line.to_string()],
                        })
                })
                .collect::<Vec<_>>();
            if claims.is_empty() && !text.trim().is_empty() {
                claims.push(SourceClaimProposal {
                    statement: text.trim().into(),
                    confidence: 1.0,
                    quotes: vec![text.into()],
                });
            }
            let no_claims_reason = claims
                .is_empty()
                .then(|| "captured source is empty".to_string());
            Ok(ProposedSourceClaims {
                execution: None,
                complete: true,
                claims,
                no_claims_reason,
            })
        }

        async fn assess(
            &self,
            request: &DocsClaimJudgmentRequest<'_>,
        ) -> Result<Vec<ProviderClaimAssessment>, ApiError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(request
                .claims
                .iter()
                .map(|claim| {
                    let supported = !claim.statement.contains("invented")
                        && request.evidence.direct.contains("pub fn run");
                    ProviderClaimAssessment {
                        execution: None,
                        claim_id: claim.claim_id.clone(),
                        verdict: if supported {
                            ClaimVerdict::Supported
                        } else {
                            ClaimVerdict::Unsupported
                        },
                        confidence: 1.0,
                        citations: if supported {
                            vec![ClaimCitation {
                                scope: CitationScope::Direct,
                                quote: "pub fn run".into(),
                            }]
                        } else {
                            vec![]
                        },
                        rationale: "fixture verdict".into(),
                    }
                })
                .collect())
        }
    }
    pub fn policy() -> DocsClaimPolicy {
        serde_json::from_str(include_str!(
            "../../../../theory/docs_freshness/claim_policy.docs-claims-strict-v1.json"
        ))
        .unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::*;
    use super::*;
    use std::sync::atomic::Ordering;

    #[tokio::test]
    async fn acceptance_revision_invalidates_reuse_and_historical_report_identity_is_preserved() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("lib.rs"), "pub fn run() {}\n").unwrap();
        std::fs::write(
            root.path().join("README.md"),
            "`run` exists.\n\nAn invented assertion.\n",
        )
        .unwrap();
        let bundle = super::super::observation::inspect_scope(root.path()).unwrap();
        let strict = policy();
        let report = assess_observed_claims(&FixtureJudge::default(), &strict, &bundle)
            .await
            .unwrap();
        let mut permissive = strict.clone();
        permissive.minimum_groundedness = 0.4;
        permissive.maximum_unsupported_claim_mass = 0.6;
        assert!(!report.matches_observation(&permissive, &bundle).unwrap());
        assert!(advance_observed_claims(
            &FixtureJudge::default(),
            &permissive,
            &bundle,
            Some(&report),
            1
        )
        .await
        .is_err());
        let next = assess_observed_claims(&FixtureJudge::default(), &permissive, &bundle)
            .await
            .unwrap();
        assert_ne!(next.report_id, report.report_id);
        let mut historical = report.clone();
        historical.acceptance_evaluator = None;
        let bytes = serde_json::to_vec(&(
            &historical.observation_revision_id,
            &historical.source_fingerprint,
            &historical.policy_identity,
            historical.complete,
            &historical.readmes,
        ))
        .unwrap();
        historical.report_id = format!("docs-observed-claims::{}", blake3::hash(&bytes).to_hex());
        let historical_bytes = serde_json::to_vec(&historical).unwrap();
        assert!(!String::from_utf8_lossy(&historical_bytes).contains("acceptance_evaluator"));
        let decoded: ObservedDocsClaimReport = serde_json::from_slice(&historical_bytes).unwrap();
        assert!(decoded.matches_capture(&bundle).unwrap());
        assert!(!decoded.matches_observation(&strict, &bundle).unwrap());
        assert_eq!(decoded.identity().unwrap(), historical.report_id);
        let mut tampered = report;
        tampered.acceptance_evaluator = None;
        assert!(!tampered.matches_capture(&bundle).unwrap());
    }

    #[tokio::test]
    async fn captured_claim_judgment_preserves_incorrect_text_and_binds_exact_observation() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("lib.rs"), "pub fn run() {}\n").unwrap();
        let content = "# run\n\n`run` exists.\n\nAn invented behavior.\n";
        std::fs::write(root.path().join("README.md"), content).unwrap();
        let observed = super::super::observation::inspect_scope(root.path()).unwrap();
        let judge = FixtureJudge::default();
        let report = assess_observed_claims(&judge, &policy(), &observed)
            .await
            .unwrap();
        let ObservedClaimDisposition::Assessed { report: readme } = &report.readmes[0].disposition
        else {
            panic!("README was not assessed")
        };
        assert!(!readme.accepted);
        assert_eq!(readme.revision_attempts, 0);
        assert!(readme
            .assessments
            .iter()
            .any(|assessment| assessment.claim.statement.contains("invented")));
        assert_eq!(
            std::fs::read_to_string(root.path().join("README.md")).unwrap(),
            content
        );
        assert!(report.matches_observation(&policy(), &observed).unwrap());
        std::fs::write(root.path().join("README.md"), "# run\n\n`run` exists.\n").unwrap();
        let current = super::super::observation::inspect_scope(root.path()).unwrap();
        assert!(!report.matches_observation(&policy(), &current).unwrap());
        let correct = assess_observed_claims(&judge, &policy(), &current)
            .await
            .unwrap();
        assert!(
            matches!(&correct.readmes[0].disposition, ObservedClaimDisposition::Assessed { report } if report.accepted)
        );
        let mut tampered = correct;
        tampered.source_fingerprint = "another-source".into();
        assert!(!tampered.matches_observation(&policy(), &current).unwrap());
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 2);
    }

    #[tokio::test]
    async fn missing_and_empty_readmes_have_explicit_dispositions_without_provider_calls() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("lib.rs"), "pub fn run() {}\n").unwrap();
        let judge = FixtureJudge::default();
        let observed = super::super::observation::inspect_scope(root.path()).unwrap();
        let report = assess_observed_claims(&judge, &policy(), &observed)
            .await
            .unwrap();
        assert!(matches!(
            report.readmes[0].disposition,
            ObservedClaimDisposition::Missing
        ));
        std::fs::write(root.path().join("README.md"), "\n").unwrap();
        let empty = super::super::observation::inspect_scope(root.path()).unwrap();
        let report = assess_observed_claims(&judge, &policy(), &empty)
            .await
            .unwrap();
        assert!(matches!(
            report.readmes[0].disposition,
            ObservedClaimDisposition::NoClaims { .. }
        ));
        assert_eq!(judge.calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn incomplete_or_substituted_observation_cannot_start_claim_judgment() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(
            root.path().join("lib.rs"),
            "pub fn run() {}\n".repeat(1_000),
        )
        .unwrap();
        std::fs::write(root.path().join("README.md"), "# run\n").unwrap();
        let judge = FixtureJudge::default();
        let incomplete = super::super::observation::inspect_scope(root.path()).unwrap();
        assert!(assess_observed_claims(&judge, &policy(), &incomplete)
            .await
            .is_err());
        std::fs::write(root.path().join("lib.rs"), "pub fn run() {}\n").unwrap();
        let mut tampered = super::super::observation::inspect_scope(root.path()).unwrap();
        if let ObservedReadmeState::Present { claims, .. } =
            &mut tampered.observation.as_mut().unwrap().readmes[0].state
        {
            claims[0].statement = "substituted assertion".into();
        }
        assert!(assess_observed_claims(&judge, &policy(), &tampered)
            .await
            .is_err());
        assert_eq!(judge.calls.load(Ordering::SeqCst), 0);
    }
}
