//! Owner-authored materialization accounts, visible before successful Capability return.

use std::collections::BTreeMap;

use meld_events::{DomainObjectRef, EventEnvelope, EventReplayCapability};
use meld_execution::ExecutionEffectAuthority;
use meld_world_model::world_state::graph::{
    admission::GraphOwnerEventRoute, contracts::*, events::owner_publication_envelope,
};

use super::{contracts::hash, materialization_evidence};

pub const OWNER: &str = "code-change";
pub const EVENT: &str = "code_change.account_published.v1";
pub const SCHEMA: &str = "code_change.materialization_account.v1";

pub fn graph_route() -> GraphOwnerEventRoute {
    GraphOwnerEventRoute {
        complete_event_source: true,
        route_id: "code-change-materialization".into(),
        owner_id: OWNER.into(),
        event_type: EVENT.into(),
        enumeration_rule_revision: SCHEMA.into(),
    }
}

/// Opaque issuer and fence correlate an observation with its admitted effect scope.
/// Neither the desired change nor its result needs to be fabricated during preparation.
pub fn observation_source(
    issuer: &str,
    subject: &DomainObjectRef,
    fence: &str,
) -> Result<(OwnerPublicationScope, DomainObjectRef), String> {
    account_source("code-account", issuer, subject, fence)
}

fn account_source(
    kind: &str,
    issuer: &str,
    subject: &DomainObjectRef,
    correlation: &str,
) -> Result<(OwnerPublicationScope, DomainObjectRef), String> {
    subject.validate().map_err(|error| error.to_string())?;
    if issuer.trim().is_empty() || correlation.trim().is_empty() {
        return Err("code observation requires a complete issuer and correlation".into());
    }
    let id = format!("{kind}::{}", hash(&(issuer, subject, correlation))?);
    Ok((
        OwnerPublicationScope {
            scope_id: id.clone(),
            branch_id: None,
            perspective_id: None,
            valid_at: None,
        },
        DomainObjectRef::new(OWNER, "materialization_account", id)
            .map_err(|error| error.to_string())?,
    ))
}

/// A durable request keeps the same observed account when execution authority changes.
pub fn request_observation_source(
    issuer: &str,
    subject: &DomainObjectRef,
    request: &str,
) -> Result<(OwnerPublicationScope, DomainObjectRef), String> {
    account_source("code-request-account", issuer, subject, request)
}

/// Reconstruct solely from the exact retained intent and completion. This performs
/// no source reads and cannot turn later workspace edits into a different return.
pub(super) fn envelope(
    events: &EventReplayCapability,
    authority: &ExecutionEffectAuthority,
    artifact: &crate::task::ArtifactRecord,
) -> Result<EventEnvelope, String> {
    let evidence = materialization_evidence(events, artifact)?;
    let intent = events
        .committed_record(
            artifact.content["operation_id"]
                .as_str()
                .ok_or("code operation absent")?,
        )
        .map_err(|error| error.to_string())?
        .ok_or("code intent is not retained")?;
    let retained = &intent.data["binding"]["authority"];
    if retained["issuer"] != authority.issuer_ref
        || retained["principal"] != authority.principal_id
        || retained["subject"]
            != serde_json::to_value(&authority.subject).map_err(|error| error.to_string())?
        || retained["fence"] != authority.fence_ref
    {
        return Err("code account authority differs from the materialized intent".into());
    }
    let (scope, object_ref) = match retained.get("request_ref") {
        Some(value) => {
            let request = value.as_str().ok_or("retained code request is malformed")?;
            if authority.request_ref.as_deref() != Some(request) {
                return Err("code account belongs to another request".into());
            }
            request_observation_source(&authority.issuer_ref, &authority.subject, request)?
        }
        // Old intents predate request attribution. They retain their original epoch
        // account; a retry cannot promote them into a newly attributed request.
        None => observation_source(
            &authority.issuer_ref,
            &authority.subject,
            &authority.fence_ref,
        )?,
    };
    let positions = vec![evidence.intent, evidence.materialization];
    let revision = format!(
        "code-account-revision::{}",
        hash(&(&scope, artifact, &positions))?
    );
    let object_id = format!("{revision}::object");
    let operation = OwnerPublicationOperation::reconstruct(
        SCHEMA,
        OwnerPublicationBatch {
            work_input_basis_id: None,
            owner_id: OWNER.into(),
            revision_id: revision.clone(),
            scope: scope.clone(),
            objects: vec![OwnerObjectPublication {
                publication_id: object_id.clone(),
                object_ref,
                state: OwnerPublicationState::Observed,
                source_product_ref: evidence.change.change_id.clone(),
                hydration: HydrationReference {
                    owner_id: OWNER.into(),
                    product_kind: "materialization".into(),
                    product_id: evidence.change.change_id,
                    revision_id: revision.clone(),
                    role: "materialized_change".into(),
                },
                provenance_refs: vec![SCHEMA.into()],
                qualifications: BTreeMap::from([
                    ("materialized".into(), "true".into()),
                    (
                        "receipt".into(),
                        serde_json::to_string(&artifact.content)
                            .map_err(|error| error.to_string())?,
                    ),
                ]),
            }],
            relations: vec![],
            completeness: OwnerCompletenessReceipt {
                receipt_id: format!("{revision}::complete"),
                scope,
                included_ids: vec![object_id],
                exclusions: vec![],
                failures: vec![],
                status: OwnerCompletenessStatus::Complete,
            },
        },
    )
    .map_err(|error| error.to_string())?;
    let mut envelope =
        owner_publication_envelope("code-change", &operation).map_err(|error| error.to_string())?;
    envelope.event_type = EVENT.into();
    Ok(envelope.with_source_records(positions))
}
