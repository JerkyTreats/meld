//! Durable Docs observation history and its single pending publication per bound scope.

use super::capability::DocsEvidenceBundle;
use super::publication::DocsObservationRevision;
use meld_events::{DomainObjectRef, LedgerCursor};
use meld_world_model::world_state::graph::contracts::OwnerPublicationScope;
use serde::{Deserialize, Serialize};

pub struct DocsObservationStore {
    db: sled::Db,
    records: sled::Tree,
    mutation: std::sync::Mutex<()>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocsObservationHead {
    pub revision_id: String,
    pub ledger_id: meld_events::LedgerIdentity,
    pub publication: Option<LedgerCursor>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observed_effects_through: Option<LedgerCursor>,
}

enum DocsReportUpdate {
    Readme(super::claim_observation::ObservedDocsClaimReport),
    Source(super::source_claims::DocsSourceClaimReport),
    Correspondence(super::correspondence::DocsCorrespondenceReport),
    Clear,
}

impl DocsObservationStore {
    pub fn new(db: sled::Db) -> Result<Self, String> {
        let records = db
            .open_tree("docs_observations_v1")
            .map_err(|e| e.to_string())?;
        Ok(Self {
            db,
            records,
            mutation: std::sync::Mutex::new(()),
        })
    }

    pub fn head(&self, binding_id: &str) -> Result<Option<DocsObservationHead>, String> {
        self.read(&format!("head::{binding_id}"))
    }

    pub fn revision(&self, revision_id: &str) -> Result<Option<DocsObservationRevision>, String> {
        let revision: Option<DocsObservationRevision> =
            self.read(&format!("revision::{revision_id}"))?;
        if let Some(value) = &revision {
            if value.revision_id != revision_id {
                return Err("Docs revision key disagrees with its body".into());
            }
            value.publication()?;
        }
        Ok(revision)
    }

    // Keep the capture and its independently proven effect window explicit.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn prepare(
        &self,
        binding_id: &str,
        ledger_id: meld_events::LedgerIdentity,
        subject: &DomainObjectRef,
        scope: &OwnerPublicationScope,
        evidence: DocsEvidenceBundle,
        policy_identity: &str,
        effects: &super::publication_return::ObservedPublicationEffects,
    ) -> Result<DocsObservationRevision, String> {
        let _guard = self
            .mutation
            .lock()
            .map_err(|_| "Docs observation lock poisoned")?;
        let head = self.head(binding_id)?;
        if head
            .as_ref()
            .is_some_and(|head| head.ledger_id != ledger_id)
        {
            return Err("Docs scope is bound to another Event ledger".into());
        }
        let expected_after = head
            .as_ref()
            .and_then(|head| head.observed_effects_through)
            .unwrap_or(LedgerCursor {
                ledger_id,
                after_seq: 0,
            });
        if effects.after != expected_after
            || effects.through.ledger_id != ledger_id
            || effects.through.after_seq < expected_after.after_seq
        {
            return Err("Docs effect observation belongs to another capture position".into());
        }
        let prior = match &head {
            Some(head) => Some(
                self.revision(&head.revision_id)?
                    .ok_or("Docs head revision is absent")?,
            ),
            None => None,
        };
        let basis = super::input_basis::observe_work_inputs(
            prior.as_ref(),
            &evidence,
            policy_identity,
            effects,
        )?;
        if let Some(prior) = &prior {
            if &prior.subject != subject || &prior.scope != scope {
                return Err("Docs binding changed semantic scope".into());
            }
            if head.as_ref().is_some_and(|head| head.publication.is_none()) {
                return Ok(prior.clone());
            }
            if prior.evidence == evidence && prior.work_input_basis.as_ref() == Some(&basis) {
                let mut next = head.clone().expect("prior revision has a head");
                next.observed_effects_through = Some(effects.through);
                if head.as_ref() != Some(&next) {
                    self.commit_head(binding_id, head.as_ref(), &next, None)?;
                    self.flush()?;
                }
                return Ok(prior.clone());
            }
        }
        let sequence = prior.as_ref().map_or(Ok(1), |prior| {
            prior
                .sequence
                .checked_add(1)
                .ok_or("Docs sequence exhausted")
        })?;
        let revision = DocsObservationRevision::new(
            prior.map(|prior| prior.revision_id),
            sequence,
            subject.clone(),
            scope.clone(),
            evidence,
            Some(basis),
        )?;
        let next = DocsObservationHead {
            revision_id: revision.revision_id.clone(),
            ledger_id,
            publication: None,
            observed_effects_through: Some(effects.through),
        };
        self.commit_head(binding_id, head.as_ref(), &next, Some(&revision))?;
        self.flush()?;
        Ok(revision)
    }

    pub(crate) fn source_progress(
        &self,
        binding_id: &str,
        input: &str,
    ) -> Result<Option<super::source_claims::DocsSourceClaimReport>, String> {
        self.read(&format!("source-progress::{binding_id}::{input}"))
    }

    pub(crate) fn save_source_progress(
        &self,
        binding_id: &str,
        predecessor: &str,
        report: &super::source_claims::DocsSourceClaimReport,
    ) -> Result<(), String> {
        let _guard = self
            .mutation
            .lock()
            .map_err(|_| "Docs observation lock poisoned")?;
        let head = self
            .head(binding_id)?
            .ok_or("Docs source claims have no source head")?;
        if head.revision_id != predecessor || head.publication.is_none() {
            return Err("Docs source claims name an unpublished or superseded source".into());
        }
        let source = self
            .revision(predecessor)?
            .ok_or("Docs source-claim capture is absent")?;
        report
            .validate_capture(&source.evidence)
            .map_err(|error| error.to_string())?;
        let key = format!("source-progress::{binding_id}::{}", report.input_id);
        let previous: Option<super::source_claims::DocsSourceClaimReport> = self.read(&key)?;
        if previous.as_ref().is_some_and(|prior| {
            prior.files.iter().any(|file| !report.files.contains(file))
                || (prior.complete && prior != report)
        }) {
            return Err("Docs source progress changed completed extraction".into());
        }
        self.commit_progress(
            binding_id,
            &head,
            &key,
            previous.as_ref().map(encode).transpose()?,
            encode(report)?,
        )
    }

    pub(crate) fn prepare_source_claims(
        &self,
        binding_id: &str,
        predecessor: &str,
        report: super::source_claims::DocsSourceClaimReport,
    ) -> Result<DocsObservationRevision, String> {
        self.set_reports(binding_id, predecessor, DocsReportUpdate::Source(report))
    }

    pub(crate) fn correspondence_progress(
        &self,
        binding_id: &str,
        input: &str,
    ) -> Result<Option<super::correspondence::DocsCorrespondenceReport>, String> {
        self.read(&format!("correspondence-progress::{binding_id}::{input}"))
    }

    pub(crate) fn save_correspondence_progress(
        &self,
        binding_id: &str,
        predecessor: &str,
        report: &super::correspondence::DocsCorrespondenceReport,
    ) -> Result<(), String> {
        let _guard = self
            .mutation
            .lock()
            .map_err(|_| "Docs observation lock poisoned")?;
        let head = self
            .head(binding_id)?
            .ok_or("Docs correspondence has no source head")?;
        if head.revision_id != predecessor || head.publication.is_none() {
            return Err("Docs correspondence names an unpublished or superseded source".into());
        }
        let source = self
            .revision(predecessor)?
            .ok_or("Docs correspondence capture absent")?;
        report
            .validate_capture(
                &source.evidence,
                source
                    .source_claims
                    .as_ref()
                    .ok_or("Docs source claims absent")?,
            )
            .map_err(|error| error.to_string())?;
        let key = format!("correspondence-progress::{binding_id}::{}", report.input_id);
        let previous: Option<super::correspondence::DocsCorrespondenceReport> = self.read(&key)?;
        if previous.as_ref().is_some_and(|prior| {
            !report.readmes.starts_with(&prior.readmes) || (prior.complete && prior != report)
        }) {
            return Err("Docs correspondence changed completed comparisons".into());
        }
        self.commit_progress(
            binding_id,
            &head,
            &key,
            previous.as_ref().map(encode).transpose()?,
            encode(report)?,
        )
    }

    pub(crate) fn prepare_correspondence(
        &self,
        binding_id: &str,
        predecessor: &str,
        report: super::correspondence::DocsCorrespondenceReport,
    ) -> Result<DocsObservationRevision, String> {
        self.set_reports(
            binding_id,
            predecessor,
            DocsReportUpdate::Correspondence(report),
        )
    }

    pub(crate) fn claim_progress(
        &self,
        binding_id: &str,
        observation: &str,
        policy: &str,
    ) -> Result<Option<super::claim_observation::ObservedDocsClaimReport>, String> {
        self.read(&format!(
            "claim-progress::{binding_id}::{observation}::{policy}"
        ))
    }

    pub(crate) fn save_claim_progress(
        &self,
        binding_id: &str,
        predecessor: &str,
        report: &super::claim_observation::ObservedDocsClaimReport,
    ) -> Result<(), String> {
        let _guard = self
            .mutation
            .lock()
            .map_err(|_| "Docs observation lock poisoned")?;
        let head = self
            .head(binding_id)?
            .ok_or("Docs claim progress has no source head")?;
        if head.revision_id != predecessor || head.publication.is_none() {
            return Err("Docs claim progress names an unpublished or superseded source".into());
        }
        let source = self
            .revision(predecessor)?
            .ok_or("Docs claim progress source is absent")?;
        if !report
            .matches_capture(&source.evidence)
            .map_err(|error| error.to_string())?
        {
            return Err("Docs claim progress names another capture".into());
        }
        let key = format!(
            "claim-progress::{binding_id}::{}::{}",
            report.observation_revision_id, report.policy_identity
        );
        let previous: Option<super::claim_observation::ObservedDocsClaimReport> =
            self.read(&key)?;
        if previous.as_ref().is_some_and(|prior| {
            prior
                .readmes
                .iter()
                .any(|judgment| !report.readmes.contains(judgment))
                || (prior.complete && prior != report)
        }) {
            return Err("Docs claim progress changed completed judgments".into());
        }
        self.commit_progress(
            binding_id,
            &head,
            &key,
            previous.as_ref().map(encode).transpose()?,
            encode(report)?,
        )
    }

    fn commit_progress(
        &self,
        binding_id: &str,
        head: &DocsObservationHead,
        key: &str,
        expected_progress: Option<Vec<u8>>,
        next: Vec<u8>,
    ) -> Result<(), String> {
        let expected_head = encode(head)?;
        let head_key = format!("head::{binding_id}");
        self.records
            .transaction(|tree| {
                if tree.get(head_key.as_bytes())?.as_deref() != Some(expected_head.as_slice())
                    || tree.get(key.as_bytes())?.as_deref() != expected_progress.as_deref()
                {
                    return Err(sled::transaction::ConflictableTransactionError::Abort(
                        "Docs source or claim progress advanced concurrently".to_string(),
                    ));
                }
                tree.insert(key.as_bytes(), next.as_slice())?;
                Ok(())
            })
            .map_err(|error| error.to_string())?;
        self.flush()
    }

    pub(crate) fn prepare_claim_report(
        &self,
        binding_id: &str,
        predecessor: &str,
        report: super::claim_observation::ObservedDocsClaimReport,
    ) -> Result<DocsObservationRevision, String> {
        self.set_reports(binding_id, predecessor, DocsReportUpdate::Readme(report))
    }

    pub(crate) fn invalidate_judgments(
        &self,
        binding_id: &str,
        predecessor: &str,
    ) -> Result<DocsObservationRevision, String> {
        self.set_reports(binding_id, predecessor, DocsReportUpdate::Clear)
    }

    fn set_reports(
        &self,
        binding_id: &str,
        predecessor: &str,
        update: DocsReportUpdate,
    ) -> Result<DocsObservationRevision, String> {
        let _guard = self
            .mutation
            .lock()
            .map_err(|_| "Docs observation lock poisoned")?;
        let head = self
            .head(binding_id)?
            .ok_or("Docs claim judgment has no source head")?;
        if head.revision_id != predecessor || head.publication.is_none() {
            return Err("Docs source advanced or is not published for claim judgment".into());
        }
        let prior = self
            .revision(predecessor)?
            .ok_or("Docs claim judgment source is absent")?;
        let mut readme = prior.claim_report.clone();
        let mut source_claims = prior.source_claims.clone();
        let correspondence;
        match update {
            DocsReportUpdate::Readme(report) => {
                readme = Some(report);
                correspondence = None;
            }
            DocsReportUpdate::Source(report) => {
                source_claims = Some(report);
                correspondence = None;
            }
            DocsReportUpdate::Correspondence(report) => correspondence = Some(report),
            DocsReportUpdate::Clear => {
                readme = None;
                source_claims = None;
                correspondence = None;
            }
        }
        if prior.claim_report == readme
            && prior.source_claims == source_claims
            && prior.correspondence == correspondence
        {
            return Ok(prior);
        }
        let mut revision = DocsObservationRevision::new(
            Some(prior.revision_id),
            prior
                .sequence
                .checked_add(1)
                .ok_or("Docs sequence exhausted")?,
            prior.subject,
            prior.scope,
            prior.evidence,
            prior.work_input_basis,
        )?;
        if let Some(report) = source_claims {
            revision = revision.with_source_claims(report)?;
        }
        if let Some(report) = readme {
            revision = revision.with_claim_report(report)?;
        }
        if let Some(report) = correspondence {
            revision = revision.with_correspondence(report)?;
        }
        let next = DocsObservationHead {
            revision_id: revision.revision_id.clone(),
            ledger_id: head.ledger_id,
            publication: None,
            observed_effects_through: head.observed_effects_through,
        };
        self.commit_head(binding_id, Some(&head), &next, Some(&revision))?;
        self.flush()?;
        Ok(revision)
    }

    pub(crate) fn record_publication(
        &self,
        binding_id: &str,
        revision_id: &str,
        proof: &meld_events::EventAppendProof,
    ) -> Result<(), String> {
        let _guard = self
            .mutation
            .lock()
            .map_err(|_| "Docs observation lock poisoned")?;
        let mut head = self
            .head(binding_id)?
            .ok_or("Docs publication has no prepared head")?;
        let expected_head = head.clone();
        let revision = self
            .revision(revision_id)?
            .ok_or("Docs publication revision is absent")?;
        if head.revision_id != revision_id
            || proof.ledger_id() != head.ledger_id
            || proof.record_id() != revision.publication()?.event_record_id()
        {
            return Err("Docs publication proof names another prepared observation".into());
        }
        let position = LedgerCursor {
            ledger_id: proof.ledger_id(),
            after_seq: proof.seq(),
        };
        if head
            .publication
            .as_ref()
            .is_some_and(|prior| prior != &position)
        {
            return Err("Docs publication has a conflicting Event position".into());
        }
        head.publication = Some(position);
        self.commit_head(binding_id, Some(&expected_head), &head, None)?;
        self.flush()
    }

    fn commit_head(
        &self,
        binding_id: &str,
        expected: Option<&DocsObservationHead>,
        next: &DocsObservationHead,
        revision: Option<&DocsObservationRevision>,
    ) -> Result<(), String> {
        let head_key = format!("head::{binding_id}");
        let expected = expected.map(encode).transpose()?;
        let next = encode(next)?;
        let revision = revision
            .map(|revision| {
                Ok::<_, String>((
                    format!("revision::{}", revision.revision_id),
                    encode(revision)?,
                ))
            })
            .transpose()?;
        self.records
            .transaction(|tree| {
                if tree.get(head_key.as_bytes())?.as_deref() != expected.as_deref() {
                    return Err(sled::transaction::ConflictableTransactionError::Abort(
                        "Docs observation head advanced concurrently".to_string(),
                    ));
                }
                if let Some((key, value)) = &revision {
                    if tree
                        .get(key.as_bytes())?
                        .is_some_and(|existing| existing.as_ref() != value.as_slice())
                    {
                        return Err(sled::transaction::ConflictableTransactionError::Abort(
                            "Docs revision body conflicts with history".to_string(),
                        ));
                    }
                    tree.insert(key.as_bytes(), value.as_slice())?;
                }
                tree.insert(head_key.as_bytes(), next.as_slice())?;
                Ok(())
            })
            .map_err(|error| error.to_string())
    }

    pub fn flush(&self) -> Result<(), String> {
        self.db.flush().map(|_| ()).map_err(|e| e.to_string())
    }

    fn read<T: serde::de::DeserializeOwned>(&self, key: &str) -> Result<Option<T>, String> {
        self.records
            .get(key)
            .map_err(|e| e.to_string())?
            .map(|bytes| serde_json::from_slice(&bytes).map_err(|e| e.to_string()))
            .transpose()
    }
}

fn encode(value: &impl Serialize) -> Result<Vec<u8>, String> {
    serde_json::to_vec(value).map_err(|e| e.to_string())
}
