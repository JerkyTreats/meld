//! Claim-level validation for generated documentation artifacts.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use serde::{Deserialize, Serialize};

use super::semantics::{
    DocsClaimGuard, DocsJudgmentOperation, DocsRepairAction, DocsSemanticTheory,
};
use crate::docs::capability::{
    DirectoryEvidence, DocsCapabilityConfig, DocsEvidenceBundle, DocsPatchSet, ReadmePatch,
};
use crate::error::ApiError;
use crate::execution::ExecutionEventContext;
use crate::provider::ProviderCompletionPort;

const CLAIM_BATCH_SIZE: usize = 6;
const MAX_EVIDENCE_QUOTE_CHARS: usize = 160;

mod extraction;
mod historical;
mod registry;

pub use extraction::{extract_selected_claims, DocsClaimExtraction};
pub(crate) use historical::extract_historical_claims;

#[cfg(test)]
pub(crate) fn extract_claims(path: &str, content: &str) -> Vec<ReadmeClaim> {
    extract_selected_claims(DocsClaimExtraction::MarkdownSentencesV2, path, content)
}

pub use registry::{
    DocsClaimPolicyRegistryStore, DocsClaimPolicyRevision, DocsClaimPolicyRevisionRef,
};

/// PDS-owned policy for accepting a generated documentation artifact.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocsClaimPolicy {
    pub policy_id: String,
    /// Absent only in historical revisions, which cannot author new judgments.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acceptance_evaluator: Option<DocsAcceptanceEvaluator>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub semantic_theory: Option<DocsSemanticTheory>,
    pub minimum_claim_confidence: f64,
    pub minimum_groundedness: f64,
    pub maximum_unsupported_claim_mass: f64,
    pub maximum_contradiction_claim_mass: f64,
    pub maximum_revision_attempts: usize,
    pub title_weight: f64,
    pub prose_weight: f64,
    pub list_item_weight: f64,
    pub code_line_weight: f64,
    pub table_row_weight: f64,
}

/// Versioned owner evaluator selected by installed theory, with the policy's
/// confidence, groundedness, mass limits, and kind weights as its parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocsAcceptanceEvaluator {
    WeightedClaimMassV1,
    /// Document structure is judged explicitly but carries no factual support mass.
    DocumentClaimMassV2,
}

impl DocsClaimPolicy {
    pub fn validate(&self) -> Result<(), ApiError> {
        self.validate_historical()?;
        self.semantics()?.validate()?;
        if self.semantics()?.repair_actions()?.len() > self.maximum_revision_attempts {
            return Err(ApiError::ConfigError(
                "Docs repair response selection exceeds its attempt budget".into(),
            ));
        }
        if self.acceptance_evaluator.is_none() {
            return Err(ApiError::ConfigError(
                "Docs policy requires an explicit acceptance evaluator".into(),
            ));
        }
        Ok(())
    }

    pub(crate) fn semantics(&self) -> Result<&DocsSemanticTheory, ApiError> {
        self.semantic_theory.as_ref().ok_or_else(|| {
            ApiError::ConfigError("Docs policy requires installed semantic theory".into())
        })
    }

    fn validate_historical(&self) -> Result<(), ApiError> {
        if self.policy_id.trim().is_empty() {
            return Err(ApiError::ConfigError(
                "docs claim policy id must not be empty".to_string(),
            ));
        }
        for (name, value) in [
            ("minimum claim confidence", self.minimum_claim_confidence),
            ("minimum groundedness", self.minimum_groundedness),
            (
                "maximum unsupported claim mass",
                self.maximum_unsupported_claim_mass,
            ),
            (
                "maximum contradiction claim mass",
                self.maximum_contradiction_claim_mass,
            ),
        ] {
            if !value.is_finite() || !(0.0..=1.0).contains(&value) {
                return Err(ApiError::ConfigError(format!(
                    "docs claim policy {name} must be between zero and one"
                )));
            }
        }
        for (name, value) in [
            ("title weight", self.title_weight),
            ("prose weight", self.prose_weight),
            ("list item weight", self.list_item_weight),
            ("code line weight", self.code_line_weight),
            ("table row weight", self.table_row_weight),
        ] {
            if !value.is_finite() || value <= 0.0 {
                return Err(ApiError::ConfigError(format!(
                    "docs claim policy {name} must be finite and positive"
                )));
            }
        }
        Ok(())
    }

    pub fn content_identity(&self) -> String {
        let bytes = serde_json::to_vec(self).expect("claim policy serialization is infallible");
        format!("docs-claim-policy-{}", blake3::hash(&bytes).to_hex())
    }

    fn accepts(&self, assessments: &[ClaimAssessment]) -> Result<bool, ApiError> {
        validate_assessment_integrity(assessments, None)?;
        for assessment in assessments {
            if let Some(execution) = &assessment.execution {
                execution.validate(Some(&self.content_identity()))?;
            }
        }
        Ok(!assessments.is_empty()
            && assessments
                .iter()
                .all(|assessment| assessment.confidence >= self.minimum_claim_confidence)
            && self.accepts_metrics(aggregate_assessments(self, assessments))?)
    }

    fn retains_claim(&self, assessment: &ClaimAssessment) -> bool {
        matches!(
            assessment.verdict,
            ClaimVerdict::Supported | ClaimVerdict::NonAssertive
        ) && assessment.confidence >= self.minimum_claim_confidence
    }

    pub(crate) fn accepts_metrics(&self, metrics: (f64, f64, f64)) -> Result<bool, ApiError> {
        self.validate()?;
        let (groundedness, unsupported, contradiction) = metrics;
        if [groundedness, unsupported, contradiction]
            .iter()
            .any(|value| !value.is_finite() || !(0.0..=1.0).contains(value))
        {
            return Err(ApiError::ConfigError(
                "Docs acceptance metrics are invalid".into(),
            ));
        }
        match self.acceptance_evaluator.expect("validated evaluator") {
            DocsAcceptanceEvaluator::WeightedClaimMassV1
            | DocsAcceptanceEvaluator::DocumentClaimMassV2 => Ok(groundedness
                >= self.minimum_groundedness
                && unsupported <= self.maximum_unsupported_claim_mass
                && contradiction <= self.maximum_contradiction_claim_mass),
        }
    }

    fn weight(&self, kind: ClaimKind) -> f64 {
        match kind {
            ClaimKind::Title => self.title_weight,
            ClaimKind::Prose => self.prose_weight,
            ClaimKind::ListItem => self.list_item_weight,
            ClaimKind::CodeLine => self.code_line_weight,
            ClaimKind::TableRow => self.table_row_weight,
        }
    }
}

/// Deterministically extracted assertion kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaimKind {
    Title,
    Prose,
    ListItem,
    CodeLine,
    TableRow,
}

/// One addressable README statement candidate. Semantic judgment must support
/// every factual clause before the candidate can count as supported.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadmeClaim {
    pub claim_id: String,
    pub statement: String,
    pub kind: ClaimKind,
    pub source_line_start: usize,
    pub source_line_end: usize,
    pub literal_requirements: Vec<String>,
}

/// Semantic relationship between one README claim and admitted evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaimVerdict {
    Supported,
    Unsupported,
    Contradicted,
    /// Navigation or connective language with no factual assertion. This is not
    /// evidence and cannot establish correspondence to a required source claim.
    NonAssertive,
}

/// Evidence partition cited by the semantic verifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CitationScope {
    Inventory,
    Direct,
    Descendant,
}

/// Exact quotation from one admitted evidence partition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaimCitation {
    pub scope: CitationScope,
    pub quote: String,
}

/// Durable verdict for one deterministically extracted claim.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClaimAssessment {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution: Option<super::judgment::DocsJudgmentExecution>,
    pub claim: ReadmeClaim,
    pub verdict: ClaimVerdict,
    pub confidence: f64,
    pub citations: Vec<ClaimCitation>,
    pub rationale: String,
}

/// Claim validation result for one candidate README.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReadmeClaimReport {
    pub path: String,
    pub content_hash: String,
    pub revision_attempts: usize,
    pub assessments: Vec<ClaimAssessment>,
    pub weighted_groundedness: f64,
    pub unsupported_claim_mass: f64,
    pub contradiction_claim_mass: f64,
    pub accepted: bool,
}

/// Patch set whose exact bytes are fenced to successful claim reports.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValidatedDocsPatchSet {
    pub source_fingerprint: String,
    pub policy_id: String,
    pub policy_identity: String,
    pub validation_fingerprint: String,
    pub patches: Vec<ReadmePatch>,
    pub reports: Vec<ReadmeClaimReport>,
    pub weighted_groundedness: f64,
    pub unsupported_claim_mass: f64,
    pub contradiction_claim_mass: f64,
}

/// Reconcile a validation-bearing patch set before any filesystem mutation.
pub fn verify_validated_patch_set(
    policy: &DocsClaimPolicy,
    validated: &ValidatedDocsPatchSet,
) -> Result<(), ApiError> {
    policy.validate()?;
    if validated.policy_id != policy.policy_id
        || validated.policy_identity != policy.content_identity()
    {
        return Err(ApiError::ConfigError(
            "docs validation policy identity does not match the active PDS policy".to_string(),
        ));
    }
    if validated.patches.is_empty() || validated.patches.len() != validated.reports.len() {
        return Err(ApiError::ConfigError(
            "docs validation must cover every README patch".to_string(),
        ));
    }

    let mut patch_paths = BTreeSet::new();
    for patch in &validated.patches {
        if !patch_paths.insert(patch.path.as_str()) {
            return Err(ApiError::ConfigError(format!(
                "docs validation contains duplicate README patch '{}'",
                patch.path
            )));
        }
        let content_hash = blake3::hash(patch.content.as_bytes()).to_hex().to_string();
        if patch.content_hash != content_hash {
            return Err(ApiError::ConfigError(format!(
                "docs validation content hash is invalid for '{}'",
                patch.path
            )));
        }
    }

    let mut report_paths = BTreeSet::new();
    for report in &validated.reports {
        if !report_paths.insert(report.path.as_str()) || !patch_paths.contains(report.path.as_str())
        {
            return Err(ApiError::ConfigError(format!(
                "docs validation report path is invalid for '{}'",
                report.path
            )));
        }
        let patch = validated
            .patches
            .iter()
            .find(|patch| patch.path == report.path)
            .expect("report path membership was checked");
        if report.content_hash != patch.content_hash || report.assessments.is_empty() {
            return Err(ApiError::ConfigError(format!(
                "docs validation report does not fence exact bytes for '{}'",
                report.path
            )));
        }
        let mut actual = report
            .assessments
            .iter()
            .map(|assessment| assessment.claim.clone())
            .collect::<Vec<_>>();
        let mut expected = extract_selected_claims(
            policy.semantics()?.claim_extraction()?,
            &patch.path,
            &patch.content,
        );
        actual.sort_by(|a, b| a.claim_id.cmp(&b.claim_id));
        expected.sort_by(|a, b| a.claim_id.cmp(&b.claim_id));
        if actual != expected {
            return Err(ApiError::ConfigError(
                "docs validation assessments do not cover the exact README claims".into(),
            ));
        }
        if !report.accepted || !policy.accepts(&report.assessments)? {
            return Err(ApiError::ConfigError(format!(
                "docs validation report is not accepted for '{}'",
                report.path
            )));
        }
        let aggregates = aggregate_assessments(policy, &report.assessments);
        if aggregates
            != (
                report.weighted_groundedness,
                report.unsupported_claim_mass,
                report.contradiction_claim_mass,
            )
        {
            return Err(ApiError::ConfigError(format!(
                "docs validation report aggregates are invalid for '{}'",
                report.path
            )));
        }
    }
    if patch_paths != report_paths {
        return Err(ApiError::ConfigError(
            "docs validation reports do not cover every README patch".to_string(),
        ));
    }

    let aggregates = aggregate_reports(policy, &validated.reports);
    if aggregates
        != (
            validated.weighted_groundedness,
            validated.unsupported_claim_mass,
            validated.contradiction_claim_mass,
        )
    {
        return Err(ApiError::ConfigError(
            "docs validation aggregate claim metrics are invalid".to_string(),
        ));
    }
    let seed = serde_json::to_vec(&(
        &validated.source_fingerprint,
        &validated.policy_identity,
        &validated.patches,
        &validated.reports,
    ))
    .map_err(|error| ApiError::ConfigError(format!("cannot encode docs validation: {error}")))?;
    if validated.validation_fingerprint != blake3::hash(&seed).to_hex().to_string() {
        return Err(ApiError::ConfigError(
            "docs validation fingerprint is invalid".to_string(),
        ));
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub struct EvidencePartitions {
    pub inventory: String,
    pub direct: String,
    pub descendant: String,
}

#[derive(Debug, Deserialize)]
struct ProviderAssessmentBatch {
    assessments: Vec<ProviderClaimAssessment>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProviderClaimAssessment {
    #[serde(skip)]
    pub execution: Option<super::judgment::DocsJudgmentExecution>,
    pub claim_id: String,
    pub verdict: ClaimVerdict,
    pub confidence: f64,
    pub citations: Vec<ClaimCitation>,
    pub rationale: String,
}

/// Read-only evidence supplied to a claim judge. Proposals are reconciled and guarded by Docs.
pub struct DocsClaimJudgmentRequest<'a> {
    pub policy: &'a DocsClaimPolicy,
    pub directory: &'a DirectoryEvidence,
    pub patch: &'a ReadmePatch,
    pub evidence: &'a EvidencePartitions,
    pub claims: &'a [ReadmeClaim],
    pub revision_attempt: usize,
    pub batch_index: usize,
}

#[async_trait::async_trait]
pub trait DocsClaimJudge: Send + Sync {
    async fn correspond(
        &self,
        _request: &super::correspondence::DocsCorrespondenceRequest<'_>,
    ) -> Result<super::correspondence::ProposedCorrespondence, ApiError> {
        Err(ApiError::ConfigError(
            "Docs correspondence judgment is not bound".into(),
        ))
    }

    async fn extract_source(
        &self,
        _request: &super::source_claims::DocsSourceClaimRequest<'_>,
    ) -> Result<super::source_claims::ProposedSourceClaims, ApiError> {
        Err(ApiError::ConfigError(
            "Docs source-claim extraction is not bound".into(),
        ))
    }

    async fn assess(
        &self,
        request: &DocsClaimJudgmentRequest<'_>,
    ) -> Result<Vec<ProviderClaimAssessment>, ApiError>;
}

/// The ordinary provider route, usable without Task execution or repair authority.
pub struct ProviderDocsClaimJudge<'a, P: ?Sized> {
    pub api: &'a P,
    pub config: &'a DocsCapabilityConfig,
    pub event_context: Option<&'a ExecutionEventContext>,
}

#[async_trait::async_trait]
impl<P: ProviderCompletionPort + ?Sized> DocsClaimJudge for ProviderDocsClaimJudge<'_, P> {
    async fn correspond(
        &self,
        request: &super::correspondence::DocsCorrespondenceRequest<'_>,
    ) -> Result<super::correspondence::ProposedCorrespondence, ApiError> {
        super::correspondence::provider_correspondence(self.api, self.config, request).await
    }

    async fn extract_source(
        &self,
        request: &super::source_claims::DocsSourceClaimRequest<'_>,
    ) -> Result<super::source_claims::ProposedSourceClaims, ApiError> {
        super::source_claims::extract_provider_claims(self.api, self.config, request).await
    }

    async fn assess(
        &self,
        request: &DocsClaimJudgmentRequest<'_>,
    ) -> Result<Vec<ProviderClaimAssessment>, ApiError> {
        assess_claim_batch(
            self.api,
            self.config,
            request.policy,
            request.directory,
            request.patch,
            request.evidence,
            request.claims,
            request.revision_attempt,
            request.batch_index,
            self.event_context,
        )
        .await
    }
}

/// Validate and, when needed, revise every candidate README within policy bounds.
pub async fn validate_patch_set<P: ProviderCompletionPort + ?Sized>(
    api: &P,
    config: &DocsCapabilityConfig,
    policy: &DocsClaimPolicy,
    bundle: &DocsEvidenceBundle,
    patches: &DocsPatchSet,
    event_context: Option<&ExecutionEventContext>,
) -> Result<ValidatedDocsPatchSet, ApiError> {
    policy.validate()?;
    super::observation::validate_selected_scope(policy, bundle)?;
    if bundle.source_fingerprint != patches.source_fingerprint {
        return Err(ApiError::ConfigError(
            "docs evidence and patch source fingerprints do not match".to_string(),
        ));
    }

    let mut pending = patches
        .patches
        .iter()
        .map(|patch| (patch.path.clone(), patch.clone()))
        .collect::<BTreeMap<_, _>>();
    if pending.len() != patches.patches.len() {
        return Err(ApiError::ConfigError(
            "docs patch set contains duplicate README paths".to_string(),
        ));
    }

    let mut accepted_by_directory = BTreeMap::<String, String>::new();
    let mut accepted_patches = Vec::new();
    let mut reports = Vec::new();
    for directory in &bundle.directories {
        let path = super::observation::document_path(bundle, &directory.path);
        let mut patch = pending.remove(&path).ok_or_else(|| {
            ApiError::ConfigError(format!(
                "docs patch set is missing expected README '{path}'"
            ))
        })?;
        let partitions = evidence_partitions(directory, &accepted_by_directory, bundle, policy)?;
        let judge = ProviderDocsClaimJudge {
            api,
            config,
            event_context,
        };
        let assessment = ReadmeAssessmentContext {
            judge: &judge,
            policy,
            directory,
            evidence: &partitions,
        };
        let mut report = assess_readme(&assessment, &patch, 0).await?;
        for (index, action) in policy.semantics()?.repair_actions()?.iter().enumerate() {
            if report.accepted {
                break;
            }
            let revision_attempt = index + 1;
            match action {
                DocsRepairAction::ReviseV1 => {
                    patch = revise_readme(
                        api,
                        config,
                        policy,
                        directory,
                        &patch,
                        &partitions,
                        &report,
                        revision_attempt,
                        event_context,
                    )
                    .await?;
                    report = assess_readme(&assessment, &patch, revision_attempt).await?;
                }
                DocsRepairAction::PruneRejectedClaimsV1 => {
                    return Err(ApiError::ConfigError(
                        "historical line-pruning repair is no longer executable".into(),
                    ));
                }
            }
        }
        if !report.accepted {
            return Err(ApiError::ConfigError(format!(
                "installed Docs repair responses did not establish acceptance for '{}': {}",
                patch.path,
                rejection_summary(policy, &report)
            )));
        }
        accepted_by_directory.insert(
            directory.path.clone(),
            supported_readme_evidence(&report, &patch.content),
        );
        accepted_patches.push(patch);
        reports.push(report);
    }
    if !pending.is_empty() {
        return Err(ApiError::ConfigError(format!(
            "docs patch set contains unexpected README paths: {}",
            pending.keys().cloned().collect::<Vec<_>>().join(", ")
        )));
    }

    accepted_patches.sort_by(|left, right| left.path.cmp(&right.path));
    reports.sort_by(|left, right| left.path.cmp(&right.path));
    let (groundedness, unsupported, contradiction) = aggregate_reports(policy, &reports);
    let policy_identity = policy.content_identity();
    let validation_seed = serde_json::to_vec(&(
        &bundle.source_fingerprint,
        &policy_identity,
        &accepted_patches,
        &reports,
    ))
    .map_err(|error| ApiError::ConfigError(format!("cannot encode docs validation: {error}")))?;
    Ok(ValidatedDocsPatchSet {
        source_fingerprint: bundle.source_fingerprint.clone(),
        policy_id: policy.policy_id.clone(),
        policy_identity,
        validation_fingerprint: blake3::hash(&validation_seed).to_hex().to_string(),
        patches: accepted_patches,
        reports,
        weighted_groundedness: groundedness,
        unsupported_claim_mass: unsupported,
        contradiction_claim_mass: contradiction,
    })
}

pub(crate) async fn assess_captured_readme(
    judge: &dyn DocsClaimJudge,
    policy: &DocsClaimPolicy,
    directory: &DirectoryEvidence,
    evidence: &EvidencePartitions,
    patch: &ReadmePatch,
) -> Result<ReadmeClaimReport, ApiError> {
    assess_readme(
        &ReadmeAssessmentContext {
            judge,
            policy,
            directory,
            evidence,
        },
        patch,
        0,
    )
    .await
}

struct ReadmeAssessmentContext<'a> {
    judge: &'a dyn DocsClaimJudge,
    policy: &'a DocsClaimPolicy,
    directory: &'a DirectoryEvidence,
    evidence: &'a EvidencePartitions,
}

async fn assess_readme(
    context: &ReadmeAssessmentContext<'_>,
    patch: &ReadmePatch,
    revision_attempt: usize,
) -> Result<ReadmeClaimReport, ApiError> {
    context.policy.validate()?;
    let claims = extract_selected_claims(
        context.policy.semantics()?.claim_extraction()?,
        &patch.path,
        &patch.content,
    );
    if claims.is_empty() {
        return Err(ApiError::ConfigError(format!(
            "README '{}' contains no assessable claims",
            patch.path
        )));
    }
    let mut assessments = Vec::new();
    let mut pending_batches = claims
        .chunks(CLAIM_BATCH_SIZE)
        .map(|batch| batch.to_vec())
        .collect::<VecDeque<_>>();
    let mut batch_attempt = 0;
    while let Some(batch) = pending_batches.pop_front() {
        let request = DocsClaimJudgmentRequest {
            policy: context.policy,
            directory: context.directory,
            patch,
            evidence: context.evidence,
            claims: &batch,
            revision_attempt,
            batch_index: batch_attempt,
        };
        match context
            .judge
            .assess(&request)
            .await
            .and_then(|proposals| reconcile_provider_assessments(&batch, proposals))
        {
            Ok(batch_assessments) => assessments.extend(batch_assessments),
            Err(ApiError::ConfigError(_)) if batch.len() > 1 => {
                let right = batch[batch.len() / 2..].to_vec();
                let left = batch[..batch.len() / 2].to_vec();
                pending_batches.push_front(right);
                pending_batches.push_front(left);
            }
            Err(error) => return Err(error),
        }
        batch_attempt += 1;
    }
    assessments.sort_by(|left, right| left.claim.claim_id.cmp(&right.claim.claim_id));
    validate_assessment_integrity(&assessments, Some(context.evidence))?;
    apply_deterministic_guards(
        &context.policy.semantics()?.claim_guards,
        context.evidence,
        &mut assessments,
    );
    let (groundedness, unsupported, contradiction) =
        aggregate_assessments(context.policy, &assessments);
    let accepted = context.policy.accepts(&assessments)?;
    Ok(ReadmeClaimReport {
        path: patch.path.clone(),
        content_hash: patch.content_hash.clone(),
        revision_attempts: revision_attempt,
        assessments,
        weighted_groundedness: groundedness,
        unsupported_claim_mass: unsupported,
        contradiction_claim_mass: contradiction,
        accepted,
    })
}

#[allow(clippy::too_many_arguments)]
async fn assess_claim_batch<P: ProviderCompletionPort + ?Sized>(
    api: &P,
    config: &DocsCapabilityConfig,
    policy: &DocsClaimPolicy,
    directory: &DirectoryEvidence,
    patch: &ReadmePatch,
    evidence: &EvidencePartitions,
    claims: &[ReadmeClaim],
    revision_attempt: usize,
    batch_index: usize,
    event_context: Option<&ExecutionEventContext>,
) -> Result<Vec<ProviderClaimAssessment>, ApiError> {
    policy.validate()?;
    let generation = policy.semantics()?.generation(
        config,
        &policy.content_identity(),
        DocsJudgmentOperation::ReadmeJudgment,
        serde_json::json!({
            "directory": directory.path,
            "readme_path": patch.path,
            "readme_content_hash": patch.content_hash,
            "readme_context": patch.content,
            "claims": claims,
            "inventory": evidence.inventory,
            "direct_evidence": evidence.direct,
            "descendant_evidence": evidence.descendant,
        }),
        revision_attempt,
        batch_index,
    )?;
    let completion = api
        .complete_provider_request(&generation.request, generation.messages, event_context)
        .await?;
    let preparation = completion.preparation;
    let response = completion.response;
    let execution = super::judgment::DocsJudgmentExecution::capture(
        policy.content_identity(),
        &generation.request,
        &preparation,
        &response,
    )?;
    let mut assessments = decode_provider_assessments(&response.content)?;
    for assessment in &mut assessments {
        assessment.execution = Some(execution.clone());
    }
    Ok(assessments)
}

fn decode_provider_assessments(content: &str) -> Result<Vec<ProviderClaimAssessment>, ApiError> {
    if let Some(assessments) = decode_keyed_claims(content, "assessments", "claim_id")? {
        return Ok(assessments);
    }
    if let Ok(batch) = decode_json_response::<ProviderAssessmentBatch>(content, "claim assessment")
    {
        return Ok(batch.assessments);
    }
    if let Ok(assessments) =
        decode_json_response::<Vec<ProviderClaimAssessment>>(content, "claim assessment")
    {
        return Ok(assessments);
    }
    decode_json_response::<ProviderClaimAssessment>(content, "claim assessment")
        .map(|assessment| vec![assessment])
}

pub(crate) fn decode_keyed_claims<T: serde::de::DeserializeOwned>(
    content: &str,
    container: &str,
    identity: &str,
) -> Result<Option<Vec<T>>, ApiError> {
    let value: serde_json::Value = decode_json_response(content, "keyed claim response")?;
    let Some(_) = value.get(container).and_then(serde_json::Value::as_object) else {
        return Ok(None);
    };
    if value.as_object().is_none_or(|object| object.len() != 1) {
        return Err(ApiError::ConfigError(
            "keyed claim response has unexpected fields".into(),
        ));
    }
    let mut unique: UniqueClaimMap<UniqueClaimMap<serde_json::Value>> =
        decode_json_response(content, "keyed claim response")?;
    let records = unique
        .0
        .remove(container)
        .ok_or_else(|| ApiError::ConfigError("keyed claim container is absent".into()))?;
    records
        .0
        .iter()
        .map(|(key, body)| {
            let mut body = body
                .as_object()
                .cloned()
                .ok_or_else(|| ApiError::ConfigError("keyed claim is not an object".into()))?;
            if body
                .insert(identity.into(), serde_json::Value::String(key.clone()))
                .is_some()
            {
                return Err(ApiError::ConfigError(
                    "keyed claim repeats its enclosing identity".into(),
                ));
            }
            serde_json::from_value(serde_json::Value::Object(body))
                .map_err(|error| ApiError::ConfigError(error.to_string()))
        })
        .collect::<Result<Vec<_>, _>>()
        .map(Some)
}

struct UniqueClaimMap<T>(BTreeMap<String, T>);

impl<'de, T: Deserialize<'de>> Deserialize<'de> for UniqueClaimMap<T> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Visitor<T>(std::marker::PhantomData<T>);
        impl<'de, T: Deserialize<'de>> serde::de::Visitor<'de> for Visitor<T> {
            type Value = UniqueClaimMap<T>;
            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("an object with unique claim identities")
            }
            fn visit_map<M: serde::de::MapAccess<'de>>(
                self,
                mut map: M,
            ) -> Result<Self::Value, M::Error> {
                let mut values = BTreeMap::new();
                while let Some((key, value)) = map.next_entry::<String, T>()? {
                    if values.insert(key, value).is_some() {
                        return Err(serde::de::Error::custom(
                            "keyed claim response repeats an identity",
                        ));
                    }
                }
                Ok(UniqueClaimMap(values))
            }
        }
        deserializer.deserialize_map(Visitor(std::marker::PhantomData))
    }
}

fn reconcile_provider_assessments(
    claims: &[ReadmeClaim],
    provider: Vec<ProviderClaimAssessment>,
) -> Result<Vec<ClaimAssessment>, ApiError> {
    let expected = claims
        .iter()
        .map(|claim| (claim.claim_id.as_str(), claim))
        .collect::<BTreeMap<_, _>>();
    let mut seen = BTreeSet::new();
    let mut assessments = Vec::new();
    for assessment in provider {
        let claim = expected
            .get(assessment.claim_id.as_str())
            .copied()
            .ok_or_else(|| {
                ApiError::ConfigError(format!(
                    "claim verifier returned unknown claim id '{}'",
                    assessment.claim_id
                ))
            })?;
        if !seen.insert(claim.claim_id.clone()) {
            return Err(ApiError::ConfigError(format!(
                "claim verifier returned duplicate claim id '{}'",
                assessment.claim_id
            )));
        }
        if !assessment.confidence.is_finite() || !(0.0..=1.0).contains(&assessment.confidence) {
            return Err(ApiError::ConfigError(format!(
                "claim verifier returned invalid confidence for '{}'",
                assessment.claim_id
            )));
        }
        assessments.push(ClaimAssessment {
            execution: assessment.execution,
            claim: claim.clone(),
            verdict: assessment.verdict,
            confidence: assessment.confidence,
            citations: assessment.citations,
            rationale: assessment.rationale,
        });
    }
    if seen.len() != expected.len() {
        let missing = expected
            .keys()
            .filter(|claim_id| !seen.contains(**claim_id))
            .copied()
            .collect::<Vec<_>>();
        return Err(ApiError::ConfigError(format!(
            "claim verifier omitted claim ids: {}",
            missing.join(", ")
        )));
    }
    Ok(assessments)
}

fn validate_assessment_integrity(
    assessments: &[ClaimAssessment],
    evidence: Option<&EvidencePartitions>,
) -> Result<(), ApiError> {
    for assessment in assessments {
        if let Some(execution) = &assessment.execution {
            execution.validate(None)?;
        }
        if !assessment.confidence.is_finite() || !(0.0..=1.0).contains(&assessment.confidence) {
            return Err(ApiError::ConfigError(
                "claim verdict confidence is invalid".into(),
            ));
        }
        if matches!(
            assessment.verdict,
            ClaimVerdict::Supported | ClaimVerdict::Contradicted
        ) && assessment.citations.is_empty()
        {
            return Err(ApiError::ConfigError(
                "supported or contradicted verdict has no evidence citation".into(),
            ));
        }
        if assessment.verdict == ClaimVerdict::NonAssertive
            && (!assessment.citations.is_empty()
                || !assessment.claim.literal_requirements.is_empty()
                || assessment.claim.kind == ClaimKind::CodeLine)
        {
            return Err(ApiError::ConfigError("nonassertive text cannot carry evidence, literal requirements or executable examples".into()));
        }
        for citation in &assessment.citations {
            if citation.quote.trim().is_empty()
                || citation.quote.chars().count() > MAX_EVIDENCE_QUOTE_CHARS
            {
                return Err(ApiError::ConfigError(
                    "claim citation is empty or exceeds the quote bound".into(),
                ));
            }
            if let Some(evidence) = evidence {
                let partition = match citation.scope {
                    CitationScope::Inventory => &evidence.inventory,
                    CitationScope::Direct => &evidence.direct,
                    CitationScope::Descendant => &evidence.descendant,
                };
                if !partition.contains(&citation.quote) {
                    return Err(ApiError::ConfigError(
                        "claim citation is absent from its exact evidence partition".into(),
                    ));
                }
            }
        }
    }
    Ok(())
}

fn apply_deterministic_guards(
    guards: &[DocsClaimGuard],
    evidence: &EvidencePartitions,
    assessments: &mut [ClaimAssessment],
) {
    for assessment in assessments {
        if assessment.verdict != ClaimVerdict::Supported {
            continue;
        }
        let mut failures = Vec::new();
        let literal_evidence = format!(
            "{}\n{}\n{}",
            evidence.inventory, evidence.direct, evidence.descendant
        );
        for literal in assessment
            .claim
            .literal_requirements
            .iter()
            .filter(|_| guards.contains(&DocsClaimGuard::LiteralPresenceV1))
        {
            if !literal_evidence.contains(literal) {
                failures.push(format!(
                    "literal '{literal}' is absent from admitted evidence"
                ));
            }
        }
        if guards.contains(&DocsClaimGuard::CodeLineDirectPresenceV1)
            && assessment.claim.kind == ClaimKind::CodeLine
            && !evidence.direct.contains(assessment.claim.statement.trim())
        {
            failures.push("code or command line is absent from direct source evidence".to_string());
        }
        if guards.contains(&DocsClaimGuard::ClauseTermCoverageV1)
            && !citations_cover_each_clause(&assessment.claim.statement, &assessment.citations)
        {
            failures.push("citations do not cover every independent claim clause".to_string());
        }
        if !failures.is_empty() {
            assessment.verdict = ClaimVerdict::Unsupported;
            assessment.confidence = 1.0;
            assessment.rationale = failures.join("; ");
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn revise_readme<P: ProviderCompletionPort + ?Sized>(
    api: &P,
    config: &DocsCapabilityConfig,
    policy: &DocsClaimPolicy,
    directory: &DirectoryEvidence,
    patch: &ReadmePatch,
    evidence: &EvidencePartitions,
    report: &ReadmeClaimReport,
    revision_attempt: usize,
    event_context: Option<&ExecutionEventContext>,
) -> Result<ReadmePatch, ApiError> {
    let rejected = report
        .assessments
        .iter()
        .filter(|assessment| !policy.retains_claim(assessment))
        .map(|assessment| {
            format!(
                "- {}: {}\n  confidence: {}\n  reason: {}",
                assessment.claim.claim_id,
                assessment.claim.statement,
                assessment.confidence,
                assessment.rationale
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let generation = policy.semantics()?.generation(
        config,
        &policy.content_identity(),
        DocsJudgmentOperation::Revision,
        serde_json::json!({
            "directory": directory.path,
            "rejected_claim_report": truncate_chars(&rejected, policy.semantics()?.scope()?.capture_limits.as_ref().expect("validated capture limits").maximum_revision_report_chars),
            "inventory": evidence.inventory,
            "direct_evidence": evidence.direct,
            "descendant_evidence": evidence.descendant,
            "current_readme": patch.content,
        }),
        revision_attempt,
        0,
    )?;
    let response = api
        .complete_provider_request(&generation.request, generation.messages, event_context)
        .await?
        .response;
    let content = normalize_markdown(&response.content);
    if content.trim().is_empty() {
        return Err(ApiError::ConfigError(format!(
            "README revision for '{}' is invalid or empty",
            patch.path
        )));
    }
    Ok(ReadmePatch {
        path: patch.path.clone(),
        content_hash: blake3::hash(content.as_bytes()).to_hex().to_string(),
        content,
    })
}

fn citations_cover_each_clause(statement: &str, citations: &[ClaimCitation]) -> bool {
    const SCOPE_TERMS: &[&str] = &["all", "always", "every", "never", "none", "only"];
    let cited_terms = citations
        .iter()
        .flat_map(|citation| meaningful_terms(&citation.quote))
        .collect::<BTreeSet<_>>();
    statement_clauses(statement).into_iter().all(|clause| {
        let terms = meaningful_terms(clause);
        if terms.is_empty() {
            return true;
        }
        if terms
            .iter()
            .filter(|term| SCOPE_TERMS.contains(&term.as_str()))
            .any(|term| !cited_terms.contains(term))
        {
            return false;
        }
        let required = if terms.len() <= 2 {
            1
        } else {
            (terms.len() * 2).div_ceil(5)
        };
        terms
            .iter()
            .filter(|term| cited_terms.contains(*term))
            .count()
            >= required
    })
}

fn statement_clauses(statement: &str) -> Vec<&str> {
    let mut clauses = vec![statement];
    for separator in [" and ", " then ", " except ", ";"] {
        clauses = clauses
            .into_iter()
            .flat_map(|clause| clause.split(separator))
            .collect();
    }
    clauses
        .into_iter()
        .map(str::trim)
        .filter(|clause| !clause.is_empty())
        .collect()
}

fn meaningful_terms(value: &str) -> BTreeSet<String> {
    const STOP_WORDS: &[&str] = &[
        "a", "an", "as", "at", "be", "by", "for", "from", "in", "into", "is", "it", "of", "on",
        "or", "that", "the", "this", "to", "with",
    ];
    value
        .split(|character: char| !character.is_alphanumeric() && character != '$')
        .map(str::to_lowercase)
        .filter(|term| term.len() > 1 || term.starts_with('$'))
        .filter(|term| !STOP_WORDS.contains(&term.as_str()))
        .collect()
}

pub(crate) fn supported_readme_evidence(report: &ReadmeClaimReport, _content: &str) -> String {
    report
        .assessments
        .iter()
        .filter(|assessment| assessment.verdict == ClaimVerdict::Supported)
        .map(|assessment| assessment.claim.statement.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Recheck citation provenance for the entire proposed publication before any write.
pub(crate) fn verify_publication_evidence(
    bundle: &DocsEvidenceBundle,
    validated: &ValidatedDocsPatchSet,
    policy: &DocsClaimPolicy,
) -> Result<(), ApiError> {
    let mut descendants = BTreeMap::new();
    for directory in &bundle.directories {
        let path = super::observation::document_path(bundle, &directory.path);
        let report = validated
            .reports
            .iter()
            .find(|report| report.path == path)
            .ok_or_else(|| {
                ApiError::ConfigError("docs publication is missing a managed README report".into())
            })?;
        let patch = validated
            .patches
            .iter()
            .find(|patch| patch.path == path)
            .ok_or_else(|| {
                ApiError::ConfigError("docs publication is missing a managed README patch".into())
            })?;
        let evidence = evidence_partitions(directory, &descendants, bundle, policy)?;
        validate_assessment_integrity(&report.assessments, Some(&evidence))?;
        descendants.insert(
            directory.path.clone(),
            supported_readme_evidence(report, &patch.content),
        );
    }
    if bundle.directories.len() != validated.patches.len() {
        return Err(ApiError::ConfigError(
            "docs publication contains an unmanaged README".into(),
        ));
    }
    Ok(())
}

pub(crate) fn evidence_partitions(
    directory: &DirectoryEvidence,
    accepted_by_directory: &BTreeMap<String, String>,
    bundle: &DocsEvidenceBundle,
    policy: &DocsClaimPolicy,
) -> Result<EvidencePartitions, ApiError> {
    let scope = policy.semantics()?.scope()?;
    let limit = scope
        .capture_limits
        .as_ref()
        .ok_or_else(|| ApiError::ConfigError("Docs capture limits are absent".into()))?
        .maximum_child_readme_bytes;
    let mut inventory = format!(
        "current directory: {}\ndirect files:\n{}\nchild directories:\n{}",
        directory.path,
        list_or_none(&directory.direct_files),
        list_or_none(&directory.child_directories)
    );
    // Paths come from actual source captures, not expected entities or child prose.
    // This retains qualified child paths when the parent compares the whole subtree.
    if let Some(observation) = &bundle.observation {
        let document = scope.document_path(&directory.path);
        let paths = observation
            .sources
            .iter()
            .filter(|source| {
                scope.compares(&document, &source.path)
                    && !directory.direct_files.contains(&source.path)
            })
            .map(|source| source.path.clone())
            .collect::<Vec<_>>();
        if !paths.is_empty() {
            inventory.push_str(&format!(
                "\nobserved descendant source paths:\n{}",
                list_or_none(&paths)
            ));
        }
    }
    let descendant = directory
        .child_directories
        .iter()
        .filter_map(|child| {
            accepted_by_directory
                .get(child)
                .map(|content| (child, content))
        })
        .map(|(child, content)| {
            format!(
                "\n--- child {child} README ---\n{}\n",
                super::scope::truncate_utf8_bytes(content, limit)
            )
        })
        .collect::<String>();
    Ok(EvidencePartitions {
        inventory,
        direct: directory.evidence.clone(),
        descendant,
    })
}

fn list_or_none(values: &[String]) -> String {
    if values.is_empty() {
        "- none".to_string()
    } else {
        values
            .iter()
            .map(|value| format!("- {value}"))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

fn aggregate_assessments(
    policy: &DocsClaimPolicy,
    assessments: &[ClaimAssessment],
) -> (f64, f64, f64) {
    let total = assessments
        .iter()
        .filter(|assessment| {
            policy.acceptance_evaluator != Some(DocsAcceptanceEvaluator::DocumentClaimMassV2)
                || assessment.verdict != ClaimVerdict::NonAssertive
        })
        .map(|assessment| policy.weight(assessment.claim.kind))
        .sum::<f64>();
    if total <= 0.0 {
        return (0.0, 1.0, 0.0);
    }
    let supported = assessments
        .iter()
        .filter(|assessment| assessment.verdict == ClaimVerdict::Supported)
        .map(|assessment| policy.weight(assessment.claim.kind) * assessment.confidence)
        .sum::<f64>();
    let unsupported = assessments
        .iter()
        .filter(|assessment| assessment.verdict == ClaimVerdict::Unsupported)
        .map(|assessment| policy.weight(assessment.claim.kind))
        .sum::<f64>();
    let contradiction = assessments
        .iter()
        .filter(|assessment| assessment.verdict == ClaimVerdict::Contradicted)
        .map(|assessment| policy.weight(assessment.claim.kind))
        .sum::<f64>();
    (
        supported / total,
        unsupported / total,
        contradiction / total,
    )
}

fn aggregate_reports(policy: &DocsClaimPolicy, reports: &[ReadmeClaimReport]) -> (f64, f64, f64) {
    let assessments = reports
        .iter()
        .flat_map(|report| report.assessments.iter().cloned())
        .collect::<Vec<_>>();
    aggregate_assessments(policy, &assessments)
}

fn rejection_summary(policy: &DocsClaimPolicy, report: &ReadmeClaimReport) -> String {
    report
        .assessments
        .iter()
        .filter(|assessment| !policy.retains_claim(assessment))
        .take(6)
        .map(|assessment| {
            format!(
                "{} [{}]: {}",
                assessment.claim.claim_id, assessment.claim.statement, assessment.rationale
            )
        })
        .collect::<Vec<_>>()
        .join(" | ")
}

pub(crate) fn decode_json_response<T: serde::de::DeserializeOwned>(
    content: &str,
    label: &str,
) -> Result<T, ApiError> {
    let trimmed = content.trim();
    let candidate = if trimmed.starts_with("```") && trimmed.ends_with("```") {
        let body = trimmed
            .strip_prefix("```json")
            .or_else(|| trimmed.strip_prefix("```"))
            .unwrap_or(trimmed);
        body.strip_suffix("```").unwrap_or(body).trim()
    } else if (trimmed.starts_with('{') && trimmed.ends_with('}'))
        || (trimmed.starts_with('[') && trimmed.ends_with(']'))
    {
        trimmed
    } else if let (Some(start), Some(end)) = (trimmed.find('{'), trimmed.rfind('}')) {
        &trimmed[start..=end]
    } else {
        trimmed
    };
    serde_json::from_str(candidate).map_err(|error| {
        ApiError::ConfigError(format!("provider returned invalid {label} JSON: {error}"))
    })
}

fn normalize_markdown(value: &str) -> String {
    let trimmed = value.trim();
    let unwrapped = if trimmed.starts_with("```") && trimmed.ends_with("```") {
        let body = trimmed
            .strip_prefix("```markdown")
            .or_else(|| trimmed.strip_prefix("```md"))
            .or_else(|| trimmed.strip_prefix("```"))
            .unwrap_or(trimmed);
        body.strip_suffix("```").unwrap_or(body).trim()
    } else {
        trimmed
    };
    format!("{}\n", unwrapped.trim())
}

fn truncate_chars(value: &str, max: usize) -> String {
    value.chars().take(max).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> DocsClaimPolicy {
        DocsClaimPolicy {
            acceptance_evaluator: Some(DocsAcceptanceEvaluator::WeightedClaimMassV1),
            semantic_theory: crate::docs::claim_observation::test_support::policy().semantic_theory,
            policy_id: "test".to_string(),
            minimum_claim_confidence: 0.8,
            minimum_groundedness: 0.8,
            maximum_unsupported_claim_mass: 0.0,
            maximum_contradiction_claim_mass: 0.0,
            maximum_revision_attempts: 2,
            title_weight: 1.0,
            prose_weight: 1.0,
            list_item_weight: 1.0,
            code_line_weight: 2.0,
            table_row_weight: 1.0,
        }
    }

    #[test]
    fn extraction_covers_title_prose_lists_tables_and_code() {
        let claims = extract_claims(
            "README.md",
            "# Tool\n\nA useful tool.\n\n## API\n\n- `run` starts it.\n\n| Name | Meaning |\n| --- | --- |\n| api | HTTP |\n\n```sh\npython -m tool\n```\n",
        );
        assert_eq!(claims.len(), 7);
        assert_eq!(claims[0].kind, ClaimKind::Title);
        assert_eq!(claims[1].kind, ClaimKind::Prose);
        assert_eq!(claims[2].statement, "API");
        assert_eq!(claims[3].literal_requirements, vec!["run"]);
        assert_eq!(claims[4].kind, ClaimKind::TableRow);
        assert_eq!(claims[5].kind, ClaimKind::TableRow);
        assert_eq!(claims[6].kind, ClaimKind::CodeLine);
    }

    #[test]
    fn missing_literal_overrides_a_supported_model_verdict() {
        let claim = ReadmeClaim {
            claim_id: "claim".to_string(),
            statement: "Use `invented.yaml`.".to_string(),
            kind: ClaimKind::ListItem,
            source_line_start: 1,
            source_line_end: 1,
            literal_requirements: vec!["invented.yaml".to_string()],
        };
        let mut assessments = vec![ClaimAssessment {
            execution: None,
            claim,
            verdict: ClaimVerdict::Supported,
            confidence: 0.9,
            citations: vec![ClaimCitation {
                scope: CitationScope::Direct,
                quote: "config.example.yaml".to_string(),
            }],
            rationale: "supported".to_string(),
        }];
        apply_deterministic_guards(
            &policy().semantics().unwrap().claim_guards,
            &EvidencePartitions {
                inventory: String::new(),
                direct: "config.example.yaml".to_string(),
                descendant: String::new(),
            },
            &mut assessments,
        );
        assert_eq!(assessments[0].verdict, ClaimVerdict::Unsupported);
        assert!(assessments[0].rationale.contains("invented.yaml"));
    }

    #[test]
    fn markdown_destination_requires_literal_evidence_even_when_model_says_supported() {
        let claim = extract_selected_claims(
            DocsClaimExtraction::MarkdownSentencesV2,
            "README.md",
            "See the [guide](https://invented.example/guide).",
        )
        .remove(0);
        let mut assessments = vec![ClaimAssessment {
            execution: None,
            claim,
            verdict: ClaimVerdict::Supported,
            confidence: 1.0,
            citations: vec![],
            rationale: "proposed support".into(),
        }];
        let policy = policy();
        let guards = &policy.semantics().unwrap().claim_guards;
        apply_deterministic_guards(
            guards,
            &EvidencePartitions {
                inventory: "guide.md".into(),
                direct: "See the guide".into(),
                descendant: String::new(),
            },
            &mut assessments,
        );
        assert_eq!(assessments[0].verdict, ClaimVerdict::Unsupported);
        assessments[0].verdict = ClaimVerdict::Supported;
        apply_deterministic_guards(
            guards,
            &EvidencePartitions {
                inventory: String::new(),
                direct: "https://invented.example/guide".into(),
                descendant: String::new(),
            },
            &mut assessments,
        );
        assert_eq!(assessments[0].verdict, ClaimVerdict::Supported);
    }

    #[test]
    fn supported_citations_must_be_exact_admitted_quotes() {
        let claims = vec![ReadmeClaim {
            claim_id: "claim".to_string(),
            statement: "The service exposes health.".to_string(),
            kind: ClaimKind::Prose,
            source_line_start: 1,
            source_line_end: 1,
            literal_requirements: Vec::new(),
        }];
        let provider = vec![ProviderClaimAssessment {
            execution: None,
            claim_id: "claim".to_string(),
            verdict: ClaimVerdict::Supported,
            confidence: 0.95,
            citations: vec![ClaimCitation {
                scope: CitationScope::Direct,
                quote: "imagined quote".to_string(),
            }],
            rationale: "supported".to_string(),
        }];
        let assessments = reconcile_provider_assessments(&claims, provider).unwrap();
        let evidence = EvidencePartitions {
            inventory: String::new(),
            direct: "@app.get(\"/healthz\")".to_string(),
            descendant: String::new(),
        };
        let error = validate_assessment_integrity(&assessments, Some(&evidence)).unwrap_err();
        assert!(error.to_string().contains("absent"));
        assert_eq!(assessments[0].verdict, ClaimVerdict::Supported);
    }

    #[test]
    fn one_clause_citation_cannot_support_a_compound_claim() {
        let claim = ReadmeClaim {
            claim_id: "claim".to_string(),
            statement: "All scripts run from the root and rely on strict mode, except during parallel waits."
                .to_string(),
            kind: ClaimKind::Prose,
            source_line_start: 1,
            source_line_end: 1,
            literal_requirements: Vec::new(),
        };
        let mut assessments = vec![ClaimAssessment {
            execution: None,
            claim,
            verdict: ClaimVerdict::Supported,
            confidence: 0.95,
            citations: vec![ClaimCitation {
                scope: CitationScope::Direct,
                quote: "Assumed that it is being run from the root".to_string(),
            }],
            rationale: "supported".to_string(),
        }];
        apply_deterministic_guards(
            &[DocsClaimGuard::ClauseTermCoverageV1],
            &EvidencePartitions {
                inventory: String::new(),
                direct: "Assumed that it is being run from the root\nset -e\nset +e\nwait"
                    .to_string(),
                descendant: String::new(),
            },
            &mut assessments,
        );

        assert_eq!(assessments[0].verdict, ClaimVerdict::Unsupported);
        assert!(assessments[0].rationale.contains("every independent"));
    }

    #[test]
    fn installed_theory_retains_exactly_cited_constants_without_lexical_overlap_policy() {
        let installed: DocsClaimPolicy = serde_json::from_str(
            &std::fs::read_to_string(
                std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../..")
                    .join("theory/docs_freshness/claim_policy.docs-claims-strict-v1.json"),
            )
            .unwrap(),
        )
        .unwrap();
        let evidence = EvidencePartitions {
            inventory: "pricing.py".into(),
            direct: "SHIPPING_CENTS = 500\nFREE_SHIPPING_MINIMUM_CENTS = 7000".into(),
            descendant: String::new(),
        };
        let mut assessments = vec![ClaimAssessment {
                        execution: None,
            claim: extract_claims("README.md", "- `pricing.py` defines two module-level integer constants: `SHIPPING_CENTS = 500` and `FREE_SHIPPING_MINIMUM_CENTS = 7000`.\n").remove(0),
            verdict: ClaimVerdict::Supported, confidence: 1.0,
            citations: evidence.direct.lines().map(|line| ClaimCitation { scope: CitationScope::Direct, quote: line.into() }).collect(),
            rationale: "Both constants are exactly defined in the captured source".into(),
        }];
        validate_assessment_integrity(&assessments, Some(&evidence)).unwrap();
        let mut lexical = assessments.clone();
        apply_deterministic_guards(
            &[DocsClaimGuard::ClauseTermCoverageV1],
            &evidence,
            &mut lexical,
        );
        assert_eq!(lexical[0].verdict, ClaimVerdict::Unsupported);
        apply_deterministic_guards(
            &installed.semantics().unwrap().claim_guards,
            &evidence,
            &mut assessments,
        );
        assert!(installed.accepts(&assessments).unwrap());
    }

    #[test]
    fn universal_scope_requires_explicit_citation_support() {
        assert!(!citations_cover_each_clause(
            "All scripts enable strict mode.",
            &[ClaimCitation {
                scope: CitationScope::Direct,
                quote: "one script enables strict mode".to_string(),
            }]
        ));
        assert!(citations_cover_each_clause(
            "All scripts enable strict mode.",
            &[ClaimCitation {
                scope: CitationScope::Direct,
                quote: "All scripts enable strict mode".to_string(),
            }]
        ));
    }

    #[test]
    fn provider_must_assess_every_extracted_claim_exactly_once() {
        let claims = extract_claims("README.md", "# Tool\n\nIt runs.\n");
        let one = ProviderClaimAssessment {
            execution: None,
            claim_id: claims[0].claim_id.clone(),
            verdict: ClaimVerdict::Supported,
            confidence: 0.9,
            citations: vec![ClaimCitation {
                scope: CitationScope::Direct,
                quote: "tool".to_string(),
            }],
            rationale: "supported".to_string(),
        };

        assert!(reconcile_provider_assessments(&claims, vec![one.clone()]).is_err());
        assert!(reconcile_provider_assessments(&claims, vec![one.clone(), one]).is_err());

        let unknown = ProviderClaimAssessment {
            execution: None,
            claim_id: "unknown".to_string(),
            verdict: ClaimVerdict::Supported,
            confidence: 0.9,
            citations: Vec::new(),
            rationale: String::new(),
        };
        assert!(reconcile_provider_assessments(&claims, vec![unknown]).is_err());
    }

    #[test]
    fn singleton_assessment_rejects_another_claim_identity() {
        let claims = extract_claims("README.md", "# Tool\n");
        let provider = vec![ProviderClaimAssessment {
            execution: None,
            claim_id: "wrong-opaque-id".to_string(),
            verdict: ClaimVerdict::Supported,
            confidence: 0.9,
            citations: vec![ClaimCitation {
                scope: CitationScope::Inventory,
                quote: "current directory: .".to_string(),
            }],
            rationale: "supported".to_string(),
        }];

        assert!(reconcile_provider_assessments(&claims, provider).is_err());
    }

    #[test]
    fn verifier_accepts_equivalent_constrained_json_envelopes() {
        let assessment = r#"{"claim_id":"claim","verdict":"supported","confidence":0.9,"citations":[{"scope":"direct","quote":"run"}],"rationale":"supported"}"#;
        let wrapped = format!(r#"{{"assessments":[{assessment}]}}"#);
        let array = format!(r#"[{assessment}]"#);

        assert_eq!(decode_provider_assessments(&wrapped).unwrap().len(), 1);
        assert_eq!(decode_provider_assessments(&array).unwrap().len(), 1);
        assert_eq!(decode_provider_assessments(assessment).unwrap().len(), 1);
        let mut body: serde_json::Value = serde_json::from_str(assessment).unwrap();
        body.as_object_mut().unwrap().remove("claim_id");
        let mut keyed = serde_json::json!({"assessments":{"claim":body}});
        let decoded = decode_provider_assessments(&keyed.to_string()).unwrap();
        assert_eq!(decoded[0].claim_id, "claim");
        assert_eq!(decoded[0].verdict, ClaimVerdict::Supported);
        let duplicate = format!(r#"{{"assessments":{{"claim":{body},"claim":{body}}}}}"#);
        assert!(decode_provider_assessments(&duplicate).is_err());
        keyed["assessments"]["claim"]["claim_id"] = serde_json::json!("foreign");
        assert!(decode_provider_assessments(&keyed.to_string()).is_err());
    }

    #[tokio::test]
    async fn malformed_judgment_remains_an_error_without_inventing_a_claim_verdict() {
        struct InvalidJudge;
        #[async_trait::async_trait]
        impl DocsClaimJudge for InvalidJudge {
            async fn assess(
                &self,
                _: &DocsClaimJudgmentRequest<'_>,
            ) -> Result<Vec<ProviderClaimAssessment>, ApiError> {
                Err(ApiError::ConfigError(
                    "provider returned invalid claim assessment JSON: bad shape".into(),
                ))
            }
        }
        let directory = DirectoryEvidence {
            path: ".".into(),
            direct_files: vec!["lib.rs".into()],
            child_directories: vec![],
            evidence: "pub fn run() {}".into(),
        };
        let evidence = EvidencePartitions {
            inventory: String::new(),
            direct: directory.evidence.clone(),
            descendant: String::new(),
        };
        let patch = ReadmePatch {
            path: "README.md".into(),
            content: "# Tool\n\n`run` exists.\n".into(),
            content_hash: "test".into(),
        };
        assert!(
            assess_captured_readme(&InvalidJudge, &policy(), &directory, &evidence, &patch)
                .await
                .is_err()
        );
    }

    struct MixedVerdicts;
    #[async_trait::async_trait]
    impl DocsClaimJudge for MixedVerdicts {
        async fn assess(
            &self,
            request: &DocsClaimJudgmentRequest<'_>,
        ) -> Result<Vec<ProviderClaimAssessment>, ApiError> {
            Ok(request
                .claims
                .iter()
                .map(|claim| {
                    let verdict = if claim.statement.starts_with("Unknown") {
                        ClaimVerdict::Unsupported
                    } else if claim.statement.starts_with("Contradicted") {
                        ClaimVerdict::Contradicted
                    } else {
                        ClaimVerdict::Supported
                    };
                    ProviderClaimAssessment {
                        execution: None,
                        claim_id: claim.claim_id.clone(),
                        verdict,
                        confidence: 1.0,
                        citations: if verdict == ClaimVerdict::Unsupported {
                            vec![]
                        } else {
                            vec![ClaimCitation {
                                scope: CitationScope::Direct,
                                quote: if verdict == ClaimVerdict::Supported {
                                    "Supported fact."
                                } else {
                                    "Contradicting fact."
                                }
                                .into(),
                            }]
                        },
                        rationale: "controlled evidence judgment".into(),
                    }
                })
                .collect())
        }
    }

    #[tokio::test]
    async fn installed_mass_policy_controls_assessment_and_publication_without_hiding_verdicts() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(
            root.path().join("source.txt"),
            "Supported fact.\nContradicting fact.\n",
        )
        .unwrap();
        let bundle = crate::docs::capability::inspect_scope(root.path()).unwrap();
        let directory = &bundle.directories[0];
        let evidence =
            evidence_partitions(directory, &BTreeMap::new(), &bundle, &policy()).unwrap();
        let content = "# Supported fact\n\nUnknown fact.\n\nContradicted fact.\n".to_string();
        let patch = ReadmePatch {
            path: "README.md".into(),
            content_hash: blake3::hash(content.as_bytes()).to_hex().to_string(),
            content,
        };
        let db = sled::Config::new().temporary(true).open().unwrap();
        let store = DocsClaimPolicyRegistryStore::new(db).unwrap();
        let (_, strict) = store.install(policy(), 1).unwrap();
        let mut permissive = strict.policy.clone();
        permissive.minimum_groundedness = 0.3;
        permissive.maximum_unsupported_claim_mass = 0.34;
        permissive.maximum_contradiction_claim_mass = 0.34;
        let (_, permissive) = store.install(permissive, 2).unwrap();
        let rejected =
            assess_captured_readme(&MixedVerdicts, &strict.policy, directory, &evidence, &patch)
                .await
                .unwrap();
        let accepted = assess_captured_readme(
            &MixedVerdicts,
            &permissive.policy,
            directory,
            &evidence,
            &patch,
        )
        .await
        .unwrap();
        assert!(!rejected.accepted);
        assert!(accepted.accepted);
        assert_eq!(rejected.assessments, accepted.assessments);
        assert!(accepted
            .assessments
            .iter()
            .any(|assessment| assessment.verdict == ClaimVerdict::Contradicted));
        let child_evidence = supported_readme_evidence(&accepted, &patch.content);
        assert!(child_evidence.contains("Supported fact"));
        assert!(!child_evidence.contains("Unknown"));
        assert!(!child_evidence.contains("Contradicted"));
        let reports = vec![accepted];
        let patches = vec![patch.clone()];
        let (weighted_groundedness, unsupported_claim_mass, contradiction_claim_mass) =
            aggregate_reports(&permissive.policy, &reports);
        let policy_identity = permissive.policy.content_identity();
        let validation_fingerprint = blake3::hash(
            &serde_json::to_vec(&(
                &bundle.source_fingerprint,
                &policy_identity,
                &patches,
                &reports,
            ))
            .unwrap(),
        )
        .to_hex()
        .to_string();
        let validated = ValidatedDocsPatchSet {
            source_fingerprint: bundle.source_fingerprint.clone(),
            policy_id: permissive.policy.policy_id.clone(),
            policy_identity,
            validation_fingerprint,
            patches,
            reports,
            weighted_groundedness,
            unsupported_claim_mass,
            contradiction_claim_mass,
        };
        assert!(crate::docs::capability::publish_patch_set(
            root.path(),
            &strict.policy,
            &validated
        )
        .is_err());
        let receipt =
            crate::docs::capability::publish_patch_set(root.path(), &permissive.policy, &validated)
                .unwrap();
        assert_eq!(receipt.published.len(), 1);
        assert_eq!(
            std::fs::read_to_string(root.path().join("README.md")).unwrap(),
            patch.content
        );
        let mut corrupt = validated.clone();
        corrupt.reports[0].assessments[0].claim.claim_id = "foreign-claim".into();
        assert!(verify_validated_patch_set(&permissive.policy, &corrupt).is_err());
        let mut uncertain = validated.reports[0].assessments.clone();
        uncertain[0].confidence = 0.1;
        assert!(!permissive.policy.accepts(&uncertain).unwrap());
        for confidence in [f64::NAN, f64::INFINITY] {
            let mut uncertain = validated.reports[0].assessments.clone();
            uncertain[0].confidence = confidence;
            assert!(permissive.policy.accepts(&uncertain).is_err());
        }
        let mut false_quote = validated.reports[0].assessments.clone();
        false_quote
            .iter_mut()
            .find(|assessment| !assessment.citations.is_empty())
            .unwrap()
            .citations[0]
            .quote = "foreign evidence".into();
        assert!(validate_assessment_integrity(&false_quote, Some(&evidence)).is_err());
        let mut forged = validated.clone();
        forged.reports[0].assessments = false_quote;
        forged.validation_fingerprint = blake3::hash(
            &serde_json::to_vec(&(
                &forged.source_fingerprint,
                &forged.policy_identity,
                &forged.patches,
                &forged.reports,
            ))
            .unwrap(),
        )
        .to_hex()
        .to_string();
        assert!(crate::docs::capability::publish_patch_set(
            root.path(),
            &permissive.policy,
            &forged
        )
        .unwrap_err()
        .to_string()
        .contains("absent"));
        let mut missing_evaluator = permissive.policy.clone();
        missing_evaluator.acceptance_evaluator = None;
        assert!(store.install(missing_evaluator.clone(), 3).is_err());
        assert!(assess_captured_readme(
            &MixedVerdicts,
            &missing_evaluator,
            directory,
            &evidence,
            &patch
        )
        .await
        .is_err());
        let mut unknown_directive = serde_json::to_value(&permissive.policy).unwrap();
        unknown_directive["acceptance_override"] = true.into();
        assert!(serde_json::from_value::<DocsClaimPolicy>(unknown_directive).is_err());
        let mut unknown_evaluator = serde_json::to_value(&permissive.policy).unwrap();
        unknown_evaluator["acceptance_evaluator"] = "unknown_evaluator".into();
        assert!(serde_json::from_value::<DocsClaimPolicy>(unknown_evaluator).is_err());
    }

    #[test]
    fn hard_failures_are_not_hidden_by_weighted_support() {
        let supported = |id: &str| ClaimAssessment {
            execution: None,
            claim: ReadmeClaim {
                claim_id: id.to_string(),
                statement: "supported".to_string(),
                kind: ClaimKind::Prose,
                source_line_start: 1,
                source_line_end: 1,
                literal_requirements: Vec::new(),
            },
            verdict: ClaimVerdict::Supported,
            confidence: 1.0,
            citations: Vec::new(),
            rationale: String::new(),
        };
        let mut assessments = (0..20)
            .map(|index| supported(&format!("supported-{index}")))
            .collect::<Vec<_>>();
        let mut contradicted = supported("contradicted");
        contradicted.verdict = ClaimVerdict::Contradicted;
        assessments.push(contradicted);
        let (_, _, contradiction) = aggregate_assessments(&policy(), &assessments);
        assert!(contradiction > 0.0);
        assert!(contradiction > policy().maximum_contradiction_claim_mass);
    }

    #[test]
    fn navigation_cannot_inflate_factual_support_or_export_unverified_neighbours() {
        let mut selected = policy();
        selected.acceptance_evaluator = Some(DocsAcceptanceEvaluator::DocumentClaimMassV2);
        let claims = extract_claims("README.md", "## Usage\n\nSupported fact. Invented fact.\n");
        let mut assessments = claims
            .into_iter()
            .enumerate()
            .map(|(index, claim)| ClaimAssessment {
                execution: None,
                claim,
                verdict: if index == 0 {
                    ClaimVerdict::NonAssertive
                } else if index == 1 {
                    ClaimVerdict::Supported
                } else {
                    ClaimVerdict::Unsupported
                },
                confidence: 1.0,
                citations: if index == 1 {
                    vec![ClaimCitation {
                        scope: CitationScope::Direct,
                        quote: "Supported fact.".into(),
                    }]
                } else {
                    vec![]
                },
                rationale: String::new(),
            })
            .collect::<Vec<_>>();
        assert_eq!(
            aggregate_assessments(&selected, &assessments),
            (0.5, 0.5, 0.0)
        );
        assert!(!selected.accepts(&assessments).unwrap());
        assert!(!selected.accepts(&assessments[..1]).unwrap());
        let report = ReadmeClaimReport {
            path: "README.md".into(),
            content_hash: String::new(),
            revision_attempts: 0,
            assessments: assessments.clone(),
            weighted_groundedness: 0.5,
            unsupported_claim_mass: 0.5,
            contradiction_claim_mass: 0.0,
            accepted: false,
        };
        let exported = supported_readme_evidence(&report, "Supported fact. Invented fact.");
        assert_eq!(exported, "Supported fact.");
        assessments[1].verdict = ClaimVerdict::NonAssertive;
        assert!(validate_assessment_integrity(&assessments, None).is_err());
    }

    #[test]
    fn parent_grounding_uses_observed_qualified_paths_without_accepting_invented_children() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(root.path().join("pkg/sub")).unwrap();
        // Nested shape adapted from meld-eval's synthetic sample_nested fixture.
        std::fs::write(
            root.path().join("pkg/sub/notes.md"),
            "The helper normalizes records.\n",
        )
        .unwrap();
        let bundle = crate::docs::capability::inspect_scope(root.path()).unwrap();
        let parent = bundle
            .directories
            .iter()
            .find(|directory| directory.path == ".")
            .unwrap();
        let selected = policy();
        let accepted = BTreeMap::from([("pkg".into(), "The helper normalizes records.".into())]);
        let evidence = evidence_partitions(parent, &accepted, &bundle, &selected).unwrap();
        assert!(evidence.inventory.contains("pkg/sub/notes.md"));
        assert!(evidence
            .descendant
            .contains("The helper normalizes records."));
        assert!(!evidence.direct.contains("The helper"));
        let mut assessments = extract_claims(
            "README.md",
            "`pkg/sub/notes.md` describes normalization. `pkg/sub/missing.md` exists.",
        )
        .into_iter()
        .map(|claim| ClaimAssessment {
            execution: None,
            claim,
            verdict: ClaimVerdict::Supported,
            confidence: 1.0,
            citations: vec![ClaimCitation {
                scope: CitationScope::Descendant,
                quote: "The helper normalizes records.".into(),
            }],
            rationale: String::new(),
        })
        .collect::<Vec<_>>();
        apply_deterministic_guards(
            &[DocsClaimGuard::LiteralPresenceV1],
            &evidence,
            &mut assessments,
        );
        assert_eq!(assessments[0].verdict, ClaimVerdict::Supported);
        assert_eq!(assessments[1].verdict, ClaimVerdict::Unsupported);
        let mut direct = selected.clone();
        direct
            .semantic_theory
            .as_mut()
            .unwrap()
            .scope
            .as_mut()
            .unwrap()
            .comparison = super::super::scope::DocsSourceComparison::DirectDirectoryV1;
        assert!(!evidence_partitions(parent, &accepted, &bundle, &direct)
            .unwrap()
            .inventory
            .contains("pkg/sub/notes.md"));
    }

    #[test]
    fn child_readme_evidence_respects_multibyte_byte_limit() {
        let mut selected = policy();
        selected
            .semantic_theory
            .as_mut()
            .unwrap()
            .scope
            .as_mut()
            .unwrap()
            .capture_limits
            .as_mut()
            .unwrap()
            .maximum_child_readme_bytes = 5;
        let directory = DirectoryEvidence {
            path: ".".into(),
            direct_files: vec![],
            child_directories: vec!["child".into()],
            evidence: String::new(),
        };
        let bundle = DocsEvidenceBundle {
            source_fingerprint: String::new(),
            directories: vec![directory.clone()],
            observation: None,
        };
        let accepted = BTreeMap::from([("child".into(), "éé🙂".into())]);
        let evidence = evidence_partitions(&directory, &accepted, &bundle, &selected).unwrap();
        assert_eq!(evidence.descendant, "\n--- child child README ---\néé\n");
        assert!(!evidence.descendant.contains('🙂'));
    }
}
