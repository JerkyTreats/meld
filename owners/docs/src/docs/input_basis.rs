//! Docs-owned distinction between changed work inputs and our own published effects.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::capability::DocsEvidenceBundle;
use super::observation::ObservedReadmeState;
use super::publication::DocsObservationRevision;
use super::publication_return::ObservedPublicationEffects;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocsWorkInputBasis {
    pub basis_id: String,
    pub policy_identity: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
enum TargetInput {
    Present(String),
    Missing,
    Unavailable(String),
}

fn targets(evidence: &DocsEvidenceBundle) -> BTreeMap<String, TargetInput> {
    evidence
        .observation
        .iter()
        .flat_map(|capture| &capture.readmes)
        .map(|readme| {
            let state = match &readme.state {
                ObservedReadmeState::Present { content_hash, .. } => {
                    TargetInput::Present(content_hash.clone())
                }
                ObservedReadmeState::Missing => TargetInput::Missing,
                ObservedReadmeState::Unavailable { reason } => {
                    TargetInput::Unavailable(reason.clone())
                }
            };
            (readme.path.clone(), state)
        })
        .collect()
}

pub(super) fn observe_work_inputs(
    prior: Option<&DocsObservationRevision>,
    evidence: &DocsEvidenceBundle,
    policy_identity: &str,
    effects: &ObservedPublicationEffects,
) -> Result<DocsWorkInputBasis, String> {
    let actual = targets(evidence);
    if let Some(prior) = prior {
        if let Some(basis) = &prior.work_input_basis {
            let mut expected = targets(&prior.evidence);
            for effect in &effects.receipts {
                for output in effect
                    .receipt
                    .published
                    .iter()
                    .filter(|output| output.changed)
                {
                    if expected.contains_key(&output.path) {
                        expected.insert(
                            output.path.clone(),
                            TargetInput::Present(output.content_hash.clone()),
                        );
                    }
                }
            }
            if basis.policy_identity == policy_identity
                && prior.evidence.source_fingerprint == evidence.source_fingerprint
                && actual == expected
            {
                return Ok(basis.clone());
            }
        }
    }
    // Receipt identity matters here even when the before and after captures are
    // both missing: a durable write followed by deletion is a new external input.
    let positions = effects
        .receipts
        .iter()
        .map(|effect| (&effect.record_id, effect.seq))
        .collect::<Vec<_>>();
    let bytes = serde_json::to_vec(&(
        prior.and_then(|prior| prior.work_input_basis.as_ref()),
        policy_identity,
        &evidence.source_fingerprint,
        actual,
        effects.through.ledger_id,
        positions,
    ))
    .map_err(|error| error.to_string())?;
    Ok(DocsWorkInputBasis {
        basis_id: format!("docs-work-input-v1::{}", blake3::hash(&bytes).to_hex()),
        policy_identity: policy_identity.into(),
    })
}

#[cfg(test)]
mod tests {
    use super::super::capability::{DocsPublicationReceipt, PublishedReadme};
    use super::super::publication_return::ObservedPublicationEffect;
    use super::*;
    use meld_events::{DomainObjectRef, LedgerCursor, LedgerIdentity};
    use meld_world_model::world_state::graph::contracts::OwnerPublicationScope;

    #[test]
    fn own_publication_is_stable_but_deletion_before_capture_is_new_input() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("source.py"), "price = 9000\n").unwrap();
        let missing = super::super::observation::inspect_scope(root.path()).unwrap();
        let start = LedgerCursor {
            ledger_id: LedgerIdentity::new(),
            after_seq: 0,
        };
        let mut effects = ObservedPublicationEffects {
            after: start,
            through: start,
            receipts: vec![],
        };
        let basis = observe_work_inputs(None, &missing, "policy", &effects).unwrap();
        let subject = DomainObjectRef::new("workspace_fs", "node", "scope").unwrap();
        let scope = OwnerPublicationScope {
            scope_id: "scope".into(),
            branch_id: None,
            perspective_id: None,
            valid_at: None,
        };
        let prior = DocsObservationRevision::new(
            None,
            1,
            subject,
            scope,
            missing.clone(),
            Some(basis.clone()),
        )
        .unwrap();
        let content = "# Price\nThe price is 9000.\n";
        std::fs::write(root.path().join("README.md"), content).unwrap();
        effects.through.after_seq = 1;
        effects.receipts.push(ObservedPublicationEffect {
            seq: 1,
            record_id: "verified-publication".into(),
            receipt: DocsPublicationReceipt {
                source_fingerprint: missing.source_fingerprint.clone(),
                policy_identity: "policy".into(),
                validation_fingerprint: "validation".into(),
                weighted_groundedness: 1.0,
                unsupported_claim_mass: 0.0,
                contradiction_claim_mass: 0.0,
                published: vec![PublishedReadme {
                    path: "README.md".into(),
                    content_hash: blake3::hash(content.as_bytes()).to_hex().to_string(),
                    changed: true,
                }],
            },
        });
        let repaired = super::super::observation::inspect_scope(root.path()).unwrap();
        assert_eq!(
            observe_work_inputs(Some(&prior), &repaired, "policy", &effects).unwrap(),
            basis
        );
        effects.receipts[0].receipt.published[0].changed = false;
        assert_ne!(
            observe_work_inputs(Some(&prior), &repaired, "policy", &effects).unwrap(),
            basis
        );
        effects.receipts[0].receipt.published[0].changed = true;
        std::fs::remove_file(root.path().join("README.md")).unwrap();
        let deleted = super::super::observation::inspect_scope(root.path()).unwrap();
        assert_eq!(deleted, missing);
        let changed = observe_work_inputs(Some(&prior), &deleted, "policy", &effects).unwrap();
        assert_ne!(changed, basis);

        let consumed = ObservedPublicationEffects {
            after: effects.through,
            through: effects.through,
            receipts: vec![],
        };
        let next = DocsObservationRevision::new(
            Some(prior.revision_id),
            2,
            prior.subject,
            prior.scope,
            deleted.clone(),
            Some(changed.clone()),
        )
        .unwrap();
        assert_eq!(
            observe_work_inputs(Some(&next), &deleted, "policy", &consumed).unwrap(),
            changed
        );
        std::fs::write(root.path().join("README.md"), content).unwrap();
        let recreated = super::super::observation::inspect_scope(root.path()).unwrap();
        assert_ne!(
            observe_work_inputs(Some(&next), &recreated, "policy", &consumed).unwrap(),
            changed
        );
    }
}
