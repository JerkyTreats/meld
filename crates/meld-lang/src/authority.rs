//! Pure contracts for separating action availability from permission.

use std::collections::BTreeSet;

use meld_events::DomainObjectRef;
use serde::{Deserialize, Serialize};

use crate::{Composition, StepKind};

/// One immutable policy whose intersection determines effective authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthorityPolicy {
    /// Stable policy identity.
    pub policy_id: String,
    /// Principal whose grant this policy carries.
    pub principal_id: String,
    /// Exact subject scope this policy governs.
    pub subject: DomainObjectRef,
    /// Action ids granted by the principal.
    pub principal_granted_action_ids: Vec<String>,
    /// Action ids allowed by the active runtime policy.
    pub runtime_allowed_action_ids: Vec<String>,
    /// Action ids explicitly restricted under this policy revision.
    #[serde(default)]
    pub restricted_action_ids: Vec<String>,
}

impl AuthorityPolicy {
    /// Validate identity and deterministic action sets.
    pub fn validate(&self) -> Result<(), AuthorityDenial> {
        require_non_empty("authority policy id", &self.policy_id)?;
        require_non_empty("authority principal id", &self.principal_id)?;
        validate_subject("authority policy subject", &self.subject)?;
        validate_action_set(
            "principal granted action ids",
            &self.principal_granted_action_ids,
        )?;
        validate_action_set(
            "runtime allowed action ids",
            &self.runtime_allowed_action_ids,
        )?;
        validate_action_set("restricted action ids", &self.restricted_action_ids)?;
        Ok(())
    }

    /// Return the canonical content hash for exact policy activation.
    pub fn content_hash(&self) -> Result<String, AuthorityDenial> {
        self.validate()?;
        let bytes = serde_json::to_vec(self)
            .map_err(|error| AuthorityDenial::InvalidPolicy(error.to_string()))?;
        Ok(blake3::hash(&bytes).to_hex().to_string())
    }
}

/// Exact policy body frozen for one runtime composition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthorityPolicyBinding {
    /// Intact policy body.
    pub policy: AuthorityPolicy,
    /// Canonical hash of the intact policy body.
    pub content_hash: String,
}

impl AuthorityPolicyBinding {
    /// Create and validate an exact policy binding.
    pub fn new(policy: AuthorityPolicy, content_hash: String) -> Result<Self, AuthorityDenial> {
        let binding = Self {
            policy,
            content_hash,
        };
        binding.validate()?;
        Ok(binding)
    }

    /// Validate policy content against the cited exact hash.
    pub fn validate(&self) -> Result<(), AuthorityDenial> {
        require_non_empty("authority policy content hash", &self.content_hash)?;
        if self.policy.content_hash()? != self.content_hash {
            return Err(AuthorityDenial::InvalidPolicy(
                "authority policy content hash does not match its body".to_string(),
            ));
        }
        Ok(())
    }
}

/// Effective authority retained with one Strategy authorization.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct AuthorityDecision {
    /// Exact policy identity used for the decision.
    pub policy_id: String,
    /// Exact policy content hash used for the decision.
    pub policy_content_hash: String,
    /// Principal whose grant was evaluated.
    pub principal_id: String,
    /// Exact subject scope authorized by the decision.
    pub subject: DomainObjectRef,
    /// Package-requested action ids considered by the decision.
    pub requested_action_ids: Vec<String>,
    /// Candidate-required action ids admitted by the full intersection.
    pub authorized_action_ids: Vec<String>,
}

impl AuthorityDecision {
    /// Validate deterministic decision identity and action sets.
    pub fn validate(&self) -> Result<(), AuthorityDenial> {
        require_non_empty("authority decision policy id", &self.policy_id)?;
        require_non_empty(
            "authority decision policy content hash",
            &self.policy_content_hash,
        )?;
        require_non_empty("authority decision principal id", &self.principal_id)?;
        validate_subject("authority decision subject", &self.subject)?;
        validate_action_set("requested action ids", &self.requested_action_ids)?;
        validate_action_set("authorized action ids", &self.authorized_action_ids)?;
        if self.authorized_action_ids.is_empty() {
            return Err(AuthorityDenial::InvalidDecision(
                "authority decision must authorize at least one action".to_string(),
            ));
        }
        Ok(())
    }
}

/// Stable reason effective authority could not be established.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthorityDenial {
    /// Policy identity, sets, or content were invalid.
    InvalidPolicy(String),
    /// Retained authority decision was invalid.
    InvalidDecision(String),
    /// Candidate did not carry a specific action identity.
    CandidateActionMissing(String),
    /// Candidate scope differs from policy scope.
    ScopeMismatch,
    /// Candidate action was not requested by the package.
    NotRequested(String),
    /// Candidate action was not granted by the principal.
    NotGranted(String),
    /// Candidate action was not allowed by runtime policy.
    RuntimeDenied(String),
    /// Candidate action was explicitly restricted.
    Restricted(String),
    /// Active policy differs from retained decision lineage.
    StalePolicy,
}

impl std::fmt::Display for AuthorityDenial {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidPolicy(reason) => write!(formatter, "invalid authority policy: {reason}"),
            Self::InvalidDecision(reason) => {
                write!(formatter, "invalid authority decision: {reason}")
            }
            Self::CandidateActionMissing(step) => {
                write!(
                    formatter,
                    "candidate step '{step}' has no specific action identity"
                )
            }
            Self::ScopeMismatch => write!(formatter, "authority scope does not match subject"),
            Self::NotRequested(action) => {
                write!(
                    formatter,
                    "action '{action}' was not requested by the package"
                )
            }
            Self::NotGranted(action) => {
                write!(
                    formatter,
                    "action '{action}' was not granted to the principal"
                )
            }
            Self::RuntimeDenied(action) => {
                write!(
                    formatter,
                    "action '{action}' is not allowed by runtime policy"
                )
            }
            Self::Restricted(action) => {
                write!(formatter, "action '{action}' is currently restricted")
            }
            Self::StalePolicy => write!(formatter, "authority decision cites a different policy"),
        }
    }
}

impl std::error::Error for AuthorityDenial {}

/// Return sorted unique capability type identities required by a composition.
pub fn required_action_ids(composition: &Composition) -> Result<Vec<String>, AuthorityDenial> {
    let mut actions = BTreeSet::new();
    for step in &composition.steps {
        let StepKind::Op(operator) = &step.kind else {
            continue;
        };
        let Some(specific) = &operator.resolution.specific else {
            return Err(AuthorityDenial::CandidateActionMissing(
                step.step_id.clone(),
            ));
        };
        require_non_empty("candidate action id", &specific.capability_type_id)?;
        actions.insert(specific.capability_type_id.clone());
    }
    if actions.is_empty() {
        return Err(AuthorityDenial::InvalidDecision(
            "candidate must require at least one action".to_string(),
        ));
    }
    Ok(actions.into_iter().collect())
}

/// Calculate effective authority for one exact candidate and subject.
pub fn evaluate_authority(
    binding: &AuthorityPolicyBinding,
    requested_action_ids: &[String],
    composition: &Composition,
    subject: &DomainObjectRef,
) -> Result<AuthorityDecision, AuthorityDenial> {
    binding.validate()?;
    validate_action_set("package requested action ids", requested_action_ids)?;
    if &binding.policy.subject != subject {
        return Err(AuthorityDenial::ScopeMismatch);
    }
    let required = required_action_ids(composition)?;
    let requested = requested_action_ids.iter().collect::<BTreeSet<_>>();
    let granted = binding
        .policy
        .principal_granted_action_ids
        .iter()
        .collect::<BTreeSet<_>>();
    let runtime_allowed = binding
        .policy
        .runtime_allowed_action_ids
        .iter()
        .collect::<BTreeSet<_>>();
    let restricted = binding
        .policy
        .restricted_action_ids
        .iter()
        .collect::<BTreeSet<_>>();
    for action in &required {
        if !requested.contains(action) {
            return Err(AuthorityDenial::NotRequested(action.clone()));
        }
        if !granted.contains(action) {
            return Err(AuthorityDenial::NotGranted(action.clone()));
        }
        if !runtime_allowed.contains(action) {
            return Err(AuthorityDenial::RuntimeDenied(action.clone()));
        }
        if restricted.contains(action) {
            return Err(AuthorityDenial::Restricted(action.clone()));
        }
    }
    let mut requested = requested_action_ids.to_vec();
    requested.sort();
    let decision = AuthorityDecision {
        policy_id: binding.policy.policy_id.clone(),
        policy_content_hash: binding.content_hash.clone(),
        principal_id: binding.policy.principal_id.clone(),
        subject: subject.clone(),
        requested_action_ids: requested,
        authorized_action_ids: required,
    };
    decision.validate()?;
    Ok(decision)
}

fn validate_action_set(label: &str, values: &[String]) -> Result<(), AuthorityDenial> {
    let mut seen = BTreeSet::new();
    for value in values {
        require_non_empty(label, value)?;
        if !seen.insert(value.as_str()) {
            return Err(AuthorityDenial::InvalidPolicy(format!(
                "{label} contains duplicate action '{value}'"
            )));
        }
    }
    Ok(())
}

fn require_non_empty(label: &str, value: &str) -> Result<(), AuthorityDenial> {
    if value.trim().is_empty() {
        return Err(AuthorityDenial::InvalidPolicy(format!(
            "{label} must be non-empty"
        )));
    }
    Ok(())
}

fn validate_subject(label: &str, subject: &DomainObjectRef) -> Result<(), AuthorityDenial> {
    require_non_empty(&format!("{label} domain id"), &subject.domain_id)?;
    require_non_empty(&format!("{label} object kind"), &subject.object_kind)?;
    require_non_empty(&format!("{label} object id"), &subject.object_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CapabilityRef, CostEstimate, Operator, Resolution, Step};

    fn subject() -> DomainObjectRef {
        DomainObjectRef::new("workspace_fs", "node", "docs").unwrap()
    }

    fn composition(action: &str) -> Composition {
        Composition {
            steps: vec![Step {
                step_id: "publish".to_string(),
                kind: StepKind::Op(Operator {
                    operator_id: "publish".to_string(),
                    preconditions: Vec::new(),
                    effects: Vec::new(),
                    cost: CostEstimate::zero(),
                    resolution: Resolution {
                        requires_inputs: Vec::new(),
                        requires_outputs: Vec::new(),
                        scope_kind: Some("repository".to_string()),
                        tags: Vec::new(),
                        specific: Some(CapabilityRef {
                            capability_type_id: action.to_string(),
                            capability_version: 1,
                        }),
                    },
                }),
            }],
            edges: Vec::new(),
        }
    }

    fn binding() -> AuthorityPolicyBinding {
        let policy = AuthorityPolicy {
            policy_id: "docs-local".to_string(),
            principal_id: "workspace-owner".to_string(),
            subject: subject(),
            principal_granted_action_ids: vec!["docs.publish".to_string()],
            runtime_allowed_action_ids: vec!["docs.publish".to_string()],
            restricted_action_ids: Vec::new(),
        };
        AuthorityPolicyBinding::new(policy.clone(), policy.content_hash().unwrap()).unwrap()
    }

    #[test]
    fn effective_authority_requires_every_intersection_input() {
        let decision = evaluate_authority(
            &binding(),
            &["docs.publish".to_string()],
            &composition("docs.publish"),
            &subject(),
        )
        .unwrap();
        assert_eq!(decision.authorized_action_ids, vec!["docs.publish"]);

        let denial = evaluate_authority(
            &binding(),
            &["docs.inspect".to_string()],
            &composition("docs.publish"),
            &subject(),
        )
        .unwrap_err();
        assert_eq!(denial, AuthorityDenial::NotRequested("docs.publish".into()));
    }

    #[test]
    fn explicit_restriction_denies_an_installed_action() {
        let mut binding = binding();
        binding.policy.restricted_action_ids = vec!["docs.publish".to_string()];
        binding.content_hash = binding.policy.content_hash().unwrap();
        assert_eq!(
            evaluate_authority(
                &binding,
                &["docs.publish".to_string()],
                &composition("docs.publish"),
                &subject(),
            )
            .unwrap_err(),
            AuthorityDenial::Restricted("docs.publish".into())
        );
    }
}
