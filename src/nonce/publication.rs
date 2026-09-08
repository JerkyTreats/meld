use std::collections::BTreeMap;

use meld_events::{
    AppendMode, DomainObjectRef, EventAppendCapability, EventEnvelope, EventRecord, LedgerCursor,
};
use meld_world_model::world_state::graph::admission::GraphOwnerEventRoute;
use meld_world_model::world_state::graph::contracts::{
    HydrationReference, OwnerCompletenessReceipt, OwnerCompletenessStatus, OwnerObjectPublication,
    OwnerPublicationBatch, OwnerPublicationOperation, OwnerPublicationScope, OwnerPublicationState,
    OwnerRelationOccurrence,
};
use meld_world_model::world_state::graph::events::owner_publication_envelope;

use super::{
    NonceEmissionReceipt, NonceError, NonceRequest, CONTRACT_REVISION, EVENT_TYPE, OWNER_ID,
};

pub fn graph_route() -> GraphOwnerEventRoute {
    GraphOwnerEventRoute {
        complete_event_source: true,
        route_id: "nonce-owner-publication".into(),
        owner_id: OWNER_ID.into(),
        event_type: EVENT_TYPE.into(),
        enumeration_rule_revision: CONTRACT_REVISION.into(),
    }
}

impl NonceRequest {
    pub fn object_ref(&self) -> Result<DomainObjectRef, NonceError> {
        self.validate()?;
        DomainObjectRef::new(OWNER_ID, "nonce", &self.nonce_id)
            .map_err(|error| NonceError::Invalid(error.to_string()))
    }

    /// A singleton owner scope makes completeness specific to the requested nonce.
    pub fn publication_scope(&self) -> OwnerPublicationScope {
        OwnerPublicationScope {
            scope_id: format!("nonce-scope::{}", self.nonce_id),
            branch_id: None,
            perspective_id: None,
            valid_at: None,
        }
    }

    pub fn publication(&self) -> Result<OwnerPublicationOperation, NonceError> {
        self.validate()?;
        let object_ref = self.object_ref()?;
        let scope = self.publication_scope();
        let hydration = HydrationReference {
            owner_id: OWNER_ID.into(),
            product_kind: "nonce".into(),
            product_id: self.nonce_id.clone(),
            revision_id: self.nonce_id.clone(),
            role: "emitted_nonce".into(),
        };
        let object_id = format!("{}::object", self.nonce_id);
        let mut relations = Vec::new();
        let mut add_reference = |role: &str, index: usize, target: DomainObjectRef| {
            relations.push(OwnerRelationOccurrence {
                occurrence_id: format!("{}::{role}::{index}", self.nonce_id),
                relation_type: format!("nonce_{role}"),
                src: object_ref.clone(),
                dst: target,
                source_product_ref: self.nonce_id.clone(),
                hydration: hydration.clone(),
                qualifications: BTreeMap::new(),
                provenance_refs: vec![self.contract_revision.clone()],
            });
        };
        add_reference("subject", 0, self.subject_ref.clone());
        for (role, values) in [
            ("issuer", vec![self.issuer_ref.as_str()]),
            ("fence", vec![self.fence_ref.as_str()]),
            (
                "correlation",
                self.correlation_refs.iter().map(String::as_str).collect(),
            ),
        ] {
            for (index, value) in values.into_iter().enumerate() {
                let target = DomainObjectRef::new("reference", "opaque", value)
                    .map_err(|error| NonceError::Invalid(error.to_string()))?;
                add_reference(role, index, target);
            }
        }
        let mut included_ids = vec![object_id.clone()];
        included_ids.extend(
            relations
                .iter()
                .map(|relation| relation.occurrence_id.clone()),
        );
        let batch = OwnerPublicationBatch {
            work_input_basis_id: None,
            owner_id: OWNER_ID.into(),
            revision_id: self.nonce_id.clone(),
            scope: scope.clone(),
            objects: vec![OwnerObjectPublication {
                publication_id: object_id,
                object_ref,
                state: OwnerPublicationState::Observed,
                source_product_ref: self.nonce_id.clone(),
                hydration,
                provenance_refs: vec![self.contract_revision.clone()],
                qualifications: BTreeMap::from([
                    ("nonce_request".into(), serde_json::to_string(self)?),
                    ("fence_ref".into(), self.fence_ref.clone()),
                    ("nonce_id".into(), self.nonce_id.clone()),
                ]),
            }],
            relations,
            completeness: OwnerCompletenessReceipt {
                receipt_id: format!("{}::complete", self.nonce_id),
                scope,
                included_ids,
                exclusions: Vec::new(),
                failures: Vec::new(),
                status: OwnerCompletenessStatus::Complete,
            },
        };
        Ok(OwnerPublicationOperation::reconstruct(
            CONTRACT_REVISION,
            batch,
        )?)
    }

    pub fn expected_event_id(&self) -> Result<String, NonceError> {
        Ok(self.publication()?.event_record_id())
    }

    pub fn envelope(&self, session_id: &str) -> Result<EventEnvelope, NonceError> {
        let mut envelope = owner_publication_envelope(session_id, &self.publication()?)?;
        envelope.event_type = EVENT_TYPE.into();
        Ok(envelope)
    }
}

/// Append one exact owner Event and prove that its canonical position contains this body.
pub fn emit(
    append: &EventAppendCapability,
    session_id: &str,
    request: &NonceRequest,
) -> Result<NonceEmissionReceipt, NonceError> {
    let proof =
        append.append_durable_proven(request.envelope(session_id)?, AppendMode::Idempotent)?;
    Ok(NonceEmissionReceipt {
        nonce_id: request.nonce_id.clone(),
        event_record_id: proof.record_id().into(),
        position: LedgerCursor {
            ledger_id: proof.ledger_id(),
            after_seq: proof.seq(),
        },
    })
}

/// Hydrate only a canonical nonce publication, retaining owner validation outside Graph.
pub fn hydrate(record: &EventRecord) -> Result<NonceRequest, NonceError> {
    if record.domain_id != OWNER_ID || record.event_type != EVENT_TYPE {
        return Err(NonceError::Invalid(
            "Event is not a nonce owner publication".into(),
        ));
    }
    let operation: OwnerPublicationOperation = serde_json::from_value(record.data.clone())?;
    let body = operation
        .batch
        .objects
        .first()
        .and_then(|object| object.qualifications.get("nonce_request"))
        .ok_or_else(|| {
            NonceError::Invalid("nonce request is absent from its owner publication".into())
        })?;
    let request: NonceRequest = serde_json::from_str(body)?;
    let expected = request.envelope(&record.session)?;
    if operation != request.publication()?
        || record.record_id != expected.record_id
        || record.objects != expected.objects
        || record.relations != expected.relations
    {
        return Err(NonceError::Invalid(
            "Event does not match its complete nonce request".into(),
        ));
    }
    Ok(request)
}
