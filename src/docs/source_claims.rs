//! Docs-owned source claims extracted independently of README content and requirements.

use super::capability::{DocsCapabilityConfig, DocsEvidenceBundle};
use super::claim_validation::{decode_json_response, DocsClaimJudge, DocsClaimPolicy};
use super::observation::{validate_observation, ObservedSource};
use crate::error::ApiError;
use crate::execution::{ProviderExecutionPort, ProviderValidationPort};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const SOURCE_CLAIM_CONTRACT: &str = "docs.source-claims.v1";

/// Extraction receives only one frozen source, never an existing or proposed README.
pub struct DocsSourceClaimRequest<'a> {
    pub source: &'a ObservedSource,
    pub policy: &'a DocsClaimPolicy,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProposedSourceClaims {
    pub complete: bool,
    pub claims: Vec<SourceClaimProposal>,
    #[serde(default)]
    pub no_claims_reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceClaimProposal {
    pub statement: String,
    pub confidence: f64,
    pub quotes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceClaimCitation {
    pub start_byte: usize,
    pub end_byte: usize,
    pub quote: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObservedSourceClaim {
    pub claim_id: String,
    pub statement: String,
    pub confidence: f64,
    pub citations: Vec<SourceClaimCitation>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SourceFileClaims {
    pub path: String,
    pub content_hash: String,
    pub claims: Vec<ObservedSourceClaim>,
    pub no_claims_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocsSourceClaimReport {
    pub report_id: String,
    pub input_id: String,
    pub policy_identity: String,
    pub contract_revision: String,
    pub complete: bool,
    pub files: Vec<SourceFileClaims>,
}

/// The source input excludes README observations, making completed extraction
/// reusable when documentation changes while its physical sources remain identical.
pub fn source_input_identity(
    policy: &DocsClaimPolicy,
    bundle: &DocsEvidenceBundle,
) -> Result<String, ApiError> {
    policy.validate()?;
    validate_observation(bundle)?;
    let observed = bundle.observation.as_ref().expect("validated capture");
    if observed.sources.iter().any(|source| source.text.is_none()) {
        return Err(invalid(
            "Docs source claims require complete captured text for every source",
        ));
    }
    let mut sources = observed.sources.iter().collect::<Vec<_>>();
    sources.sort_by(|a, b| a.path.cmp(&b.path));
    if sources.windows(2).any(|pair| pair[0].path == pair[1].path) {
        return Err(invalid("Docs capture repeats a source path"));
    }
    identity(
        "docs-source-input",
        &(SOURCE_CLAIM_CONTRACT, policy.content_identity(), &sources),
    )
}

pub(crate) async fn advance_source_claims(
    judge: &dyn DocsClaimJudge,
    policy: &DocsClaimPolicy,
    bundle: &DocsEvidenceBundle,
    prior: Option<&DocsSourceClaimReport>,
    max_sources: usize,
) -> Result<DocsSourceClaimReport, ApiError> {
    let input_id = source_input_identity(policy, bundle)?;
    if prior.is_some_and(|report| !report.matches_input(policy, bundle).unwrap_or(false)) {
        return Err(invalid(
            "Docs source-claim progress belongs to another captured source or policy",
        ));
    }
    let mut sources = bundle
        .observation
        .as_ref()
        .expect("validated capture")
        .sources
        .iter()
        .collect::<Vec<_>>();
    sources.sort_by(|a, b| a.path.cmp(&b.path));
    let mut retained = prior
        .into_iter()
        .flat_map(|report| report.files.iter().cloned())
        .map(|file| (file.path.clone(), file))
        .collect::<BTreeMap<_, _>>();
    if retained.len() != prior.map_or(0, |report| report.files.len()) {
        return Err(invalid("Docs source-claim progress repeats a file"));
    }
    let expected = sources.len();
    let mut files = Vec::new();
    let mut advanced = 0;
    for source in sources {
        if let Some(file) = retained.remove(&source.path) {
            files.push(file);
            continue;
        }
        if advanced == max_sources {
            break;
        }
        let proposed = judge
            .extract_source(&DocsSourceClaimRequest { source, policy })
            .await?;
        files.push(reconcile_source_claims(source, policy, proposed)?);
        advanced += 1;
    }
    if !retained.is_empty() {
        return Err(invalid(
            "Docs source-claim progress is not a completed prefix",
        ));
    }
    let mut report = DocsSourceClaimReport {
        report_id: String::new(),
        input_id,
        policy_identity: policy.content_identity(),
        contract_revision: SOURCE_CLAIM_CONTRACT.into(),
        complete: files.len() == expected,
        files,
    };
    report.report_id = report.identity()?;
    Ok(report)
}

impl DocsSourceClaimReport {
    fn identity(&self) -> Result<String, ApiError> {
        identity(
            "docs-source-claims",
            &(
                &self.input_id,
                &self.policy_identity,
                &self.contract_revision,
                self.complete,
                &self.files,
            ),
        )
    }

    pub fn matches_input(
        &self,
        policy: &DocsClaimPolicy,
        bundle: &DocsEvidenceBundle,
    ) -> Result<bool, ApiError> {
        Ok(self.report_id == self.identity()?
            && self.contract_revision == SOURCE_CLAIM_CONTRACT
            && self.policy_identity == policy.content_identity()
            && self.input_id == source_input_identity(policy, bundle)?)
    }

    pub(crate) fn validate_capture(&self, bundle: &DocsEvidenceBundle) -> Result<(), ApiError> {
        validate_observation(bundle)?;
        if self.report_id != self.identity()? || self.contract_revision != SOURCE_CLAIM_CONTRACT {
            return Err(invalid("Docs source-claim report identity is invalid"));
        }
        let sources = &bundle
            .observation
            .as_ref()
            .expect("validated capture")
            .sources;
        let mut selected = sources.iter().collect::<Vec<_>>();
        selected.sort_by(|a, b| a.path.cmp(&b.path));
        if self.input_id
            != identity(
                "docs-source-input",
                &(SOURCE_CLAIM_CONTRACT, &self.policy_identity, &selected),
            )?
        {
            return Err(invalid(
                "Docs source-claim report names another captured input",
            ));
        }
        if self.files.len() > selected.len()
            || (self.complete && self.files.len() != selected.len())
        {
            return Err(invalid(
                "Docs source-claim report does not cover its declared source set",
            ));
        }
        for (file, source) in self.files.iter().zip(selected) {
            if file.path != source.path || file.content_hash != source.content_hash {
                return Err(invalid(
                    "Docs source-claim report substituted source identity",
                ));
            }
            for claim in &file.claims {
                let text = source
                    .text
                    .as_deref()
                    .ok_or_else(|| invalid("source text is absent"))?;
                if claim.citations.iter().any(|citation| {
                    text.get(citation.start_byte..citation.end_byte)
                        != Some(citation.quote.as_str())
                }) {
                    return Err(invalid(
                        "Docs source claim no longer names its exact captured quotation",
                    ));
                }
            }
        }
        Ok(())
    }
}

fn reconcile_source_claims(
    source: &ObservedSource,
    policy: &DocsClaimPolicy,
    proposed: ProposedSourceClaims,
) -> Result<SourceFileClaims, ApiError> {
    if !proposed.complete {
        return Err(invalid("Docs source extraction is incomplete"));
    }
    let text = source
        .text
        .as_deref()
        .ok_or_else(|| invalid("Docs source text is absent"))?;
    if proposed.claims.is_empty()
        && proposed
            .no_claims_reason
            .as_ref()
            .is_none_or(|reason| reason.trim().is_empty())
    {
        return Err(invalid(
            "Docs source extraction omitted claims without an explicit disposition",
        ));
    }
    if !proposed.claims.is_empty() && proposed.no_claims_reason.is_some() {
        return Err(invalid(
            "Docs source extraction both supplied and denied claims",
        ));
    }
    let mut claims = BTreeMap::new();
    let mut statements = BTreeSet::new();
    for proposal in proposed.claims {
        if proposal.statement.trim().is_empty()
            || !statements.insert(proposal.statement.clone())
            || !proposal.confidence.is_finite()
            || !(policy.minimum_claim_confidence..=1.0).contains(&proposal.confidence)
            || proposal.quotes.is_empty()
        {
            return Err(invalid(
                "Docs source claim has invalid identity, confidence, or evidence",
            ));
        }
        let mut citations = Vec::new();
        for quote in proposal.quotes {
            if quote.trim().is_empty() {
                return Err(invalid("Docs source claim has an empty quotation"));
            }
            let mut matches = text.match_indices(&quote);
            let Some((start, _)) = matches.next() else {
                return Err(invalid(
                    "Docs source claim quotation is absent from captured bytes",
                ));
            };
            if matches.next().is_some() {
                return Err(invalid(
                    "Docs source claim quotation is ambiguous; a unique quotation is required",
                ));
            }
            citations.push(SourceClaimCitation {
                start_byte: start,
                end_byte: start + quote.len(),
                quote,
            });
        }
        citations.sort_by_key(|citation| (citation.start_byte, citation.end_byte));
        citations.dedup();
        let claim_id = identity(
            "docs-source-claim",
            &(
                SOURCE_CLAIM_CONTRACT,
                &source.path,
                &source.content_hash,
                &proposal.statement,
                &citations,
            ),
        )?;
        claims.insert(
            claim_id.clone(),
            ObservedSourceClaim {
                claim_id,
                statement: proposal.statement,
                confidence: proposal.confidence,
                citations,
            },
        );
    }
    Ok(SourceFileClaims {
        path: source.path.clone(),
        content_hash: source.content_hash.clone(),
        claims: claims.into_values().collect(),
        no_claims_reason: proposed.no_claims_reason,
    })
}

pub(crate) async fn extract_provider_claims<
    P: ProviderValidationPort + ProviderExecutionPort + ?Sized,
>(
    api: &P,
    config: &DocsCapabilityConfig,
    request: &DocsSourceClaimRequest<'_>,
) -> Result<ProposedSourceClaims, ApiError> {
    request.policy.validate()?;
    let generation = request.policy.semantics()?.generation(
        config,
        &request.policy.content_identity(),
        super::semantics::DocsJudgmentOperation::SourceExtraction,
        serde_json::json!({"source": request.source}),
        0,
        0,
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
    decode_json_response(&result.content, "source claims")
}

fn identity(kind: &str, value: &impl Serialize) -> Result<String, ApiError> {
    let bytes = serde_json::to_vec(value).map_err(|error| invalid(&error.to_string()))?;
    Ok(format!("{kind}::{}", blake3::hash(&bytes).to_hex()))
}
fn invalid(message: &str) -> ApiError {
    ApiError::ConfigError(message.into())
}

#[cfg(test)]
mod tests {
    use super::super::claim_observation::test_support::{policy, FixtureJudge};
    use super::*;
    use std::sync::atomic::Ordering;

    #[tokio::test]
    async fn source_claims_exist_without_readmes_and_are_reused_when_only_readmes_change() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(
            root.path().join("lib.rs"),
            "// π\npub fn start() {}\npub fn stop() {}\n",
        )
        .unwrap();
        let missing = super::super::observation::inspect_scope(root.path()).unwrap();
        let judge = FixtureJudge::default();
        let first = advance_source_claims(&judge, &policy(), &missing, None, 1)
            .await
            .unwrap();
        assert!(first.complete);
        assert_eq!(first.files.len(), 1);
        assert_eq!(first.files[0].claims.len(), 2);
        assert!(first.files[0]
            .claims
            .iter()
            .any(|claim| claim.statement.contains("stop")));
        first.validate_capture(&missing).unwrap();
        std::fs::write(root.path().join("README.md"), "`start` is declared.\n").unwrap();
        let changed = super::super::observation::inspect_scope(root.path()).unwrap();
        assert_ne!(
            missing.observation.as_ref().unwrap().revision_id,
            changed.observation.as_ref().unwrap().revision_id
        );
        assert!(first.matches_input(&policy(), &changed).unwrap());
        let reused = advance_source_claims(&judge, &policy(), &changed, Some(&first), 1)
            .await
            .unwrap();
        assert_eq!(first, reused);
        assert_eq!(judge.source_calls.load(Ordering::SeqCst), 1);
        assert_eq!(judge.calls.load(Ordering::SeqCst), 0);
        std::fs::write(root.path().join("lib.rs"), "pub fn changed() {}\n").unwrap();
        let changed_source = super::super::observation::inspect_scope(root.path()).unwrap();
        assert!(!first.matches_input(&policy(), &changed_source).unwrap());
        assert!(first.validate_capture(&changed_source).is_err());
    }

    #[test]
    fn source_claims_require_complete_unambiguous_captured_evidence() {
        let text = "// π\npub fn run() {}\nsame same";
        let source = ObservedSource {
            path: "lib.rs".into(),
            content_hash: blake3::hash(text.as_bytes()).to_hex().to_string(),
            byte_length: text.len(),
            text: Some(text.into()),
        };
        let proposal = || ProposedSourceClaims {
            complete: true,
            no_claims_reason: None,
            claims: vec![SourceClaimProposal {
                statement: "The source defines run".into(),
                confidence: 1.0,
                quotes: vec!["pub fn run() {}".into()],
            }],
        };
        let accepted = reconcile_source_claims(&source, &policy(), proposal()).unwrap();
        let citation = &accepted.claims[0].citations[0];
        assert_eq!(citation.start_byte, "// π\n".len());
        assert_eq!(
            source
                .text
                .as_ref()
                .unwrap()
                .get(citation.start_byte..citation.end_byte),
            Some(citation.quote.as_str())
        );
        for quote in ["README-only claim", "same", ""] {
            let mut invalid = proposal();
            invalid.claims[0].quotes = vec![quote.into()];
            assert!(reconcile_source_claims(&source, &policy(), invalid).is_err());
        }
        let mut incomplete = proposal();
        incomplete.complete = false;
        assert!(reconcile_source_claims(&source, &policy(), incomplete).is_err());
        let mut uncertain = proposal();
        uncertain.claims[0].confidence = 0.1;
        assert!(reconcile_source_claims(&source, &policy(), uncertain).is_err());
        assert!(reconcile_source_claims(
            &source,
            &policy(),
            ProposedSourceClaims {
                complete: true,
                claims: vec![],
                no_claims_reason: None
            }
        )
        .is_err());
    }

    #[tokio::test]
    async fn historical_hash_only_sources_do_not_invent_extraction_input() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("lib.rs"), "pub fn run() {}\n").unwrap();
        let mut capture = super::super::observation::inspect_scope(root.path()).unwrap();
        let observed = capture.observation.as_mut().unwrap();
        for source in &mut observed.sources {
            source.text = None;
        }
        let seed = serde_json::to_vec(&(
            &capture.source_fingerprint,
            &capture.directories,
            &observed.sources,
            &observed.readmes,
            &observed.exclusions,
            &observed.coverage_gaps,
        ))
        .unwrap();
        observed.revision_id = format!("docs-observation::{}", blake3::hash(&seed).to_hex());
        let bytes = serde_json::to_vec(&capture).unwrap();
        let historical: DocsEvidenceBundle = serde_json::from_slice(&bytes).unwrap();
        validate_observation(&historical).unwrap();
        let judge = FixtureJudge::default();
        assert!(
            advance_source_claims(&judge, &policy(), &historical, None, 1)
                .await
                .is_err()
        );
        assert_eq!(judge.source_calls.load(Ordering::SeqCst), 0);
    }
}
