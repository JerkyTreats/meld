//! Narrow belief-owned reopen verification for cross-domain readiness consumers.

use std::ops::Bound;
use std::sync::Arc;

use parking_lot::RwLock;
use sled::{
    transaction::{ConflictableTransactionError, TransactionError, Transactional},
    Db, Tree,
};

use crate::belief::store::{
    load_or_create_store_instance_id, require_product_authority_available, shared_write_gate,
    validate_readiness_owner_hash,
};
use crate::belief::{
    hash_readiness_view, BeliefReadinessAttestation, BeliefReadinessSnapshot, BeliefRevision,
    BeliefView, HydrationRefs,
};
use crate::error::StorageError;

const TREE_AUTHORITY_META: &str = "belief_authority_meta";
const TREE_AUTHORITY_MIGRATION: &str = "belief_authority_migration";
const TREE_REVISIONS: &str = "belief_revisions";
const TREE_VIEWS: &str = "belief_views";
const TREE_ATTESTATIONS: &str = "belief_readiness_attestations";
const TREE_INTENTS: &str = "belief_readiness_attestation_intents";
const TREE_OWNER_FENCES: &str = "belief_readiness_attestation_owner_fences";
const TREE_SNAPSHOTS: &str = "belief_readiness_attestation_snapshots";
const TREE_SCHEMA: &str = "belief_readiness_schema";
const KEY_SCHEMA_STATE: &[u8] = b"state";
const SCHEMA_VERSION: u16 = 2;
const MIGRATION_BATCH: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
enum MigrationPhase {
    Intents,
    Visible,
    Complete,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct MigrationState {
    schema_version: u16,
    phase: MigrationPhase,
    cursor: Option<Vec<u8>>,
}

impl MigrationState {
    fn initial() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            phase: MigrationPhase::Intents,
            cursor: None,
        }
    }

    fn complete() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            phase: MigrationPhase::Complete,
            cursor: None,
        }
    }

    fn validate(&self) -> Result<(), StorageError> {
        if self.schema_version != SCHEMA_VERSION
            || self.phase == MigrationPhase::Complete && self.cursor.is_some()
        {
            return Err(StorageError::MigrationConflict(
                "belief readiness schema marker is invalid".to_string(),
            ));
        }
        Ok(())
    }
}

/// Belief-owned contract for reopening one exact readiness attestation product.
pub(crate) struct BeliefReadinessReopenContract {
    db: Db,
    authority_meta: Tree,
    authority_migration: Tree,
    revisions: Tree,
    views: Tree,
    attestations: Tree,
    intents: Tree,
    owner_fences: Tree,
    snapshots: Tree,
    schema: Tree,
    write_gate: Arc<RwLock<()>>,
}

/// Proof that one cross-domain operation owns the available belief authority.
pub(crate) struct BeliefReadinessAuthority<'a> {
    contract: &'a BeliefReadinessReopenContract,
}

impl BeliefReadinessAuthority<'_> {
    /// Return one attestation after verifying its exact durable products.
    pub(crate) fn verified_attestation(
        &self,
        attestation_id: &str,
    ) -> Result<BeliefReadinessAttestation, StorageError> {
        self.contract
            .verified_attestation_exclusively(attestation_id)
    }
}

impl BeliefReadinessReopenContract {
    /// Open only readiness authority and complete its versioned migration.
    pub(crate) fn open(db: Db) -> Result<Self, StorageError> {
        let authority_meta = db.open_tree(TREE_AUTHORITY_META).map_err(to_storage_io)?;
        let authority_migration = db
            .open_tree(TREE_AUTHORITY_MIGRATION)
            .map_err(to_storage_io)?;
        require_product_authority_available(&authority_meta, &authority_migration)?;
        let store_instance_id = load_or_create_store_instance_id(&db, &authority_meta)?;
        let write_gate = shared_write_gate(&store_instance_id);
        let gate = Arc::clone(&write_gate);
        let _exclusive = gate.write();
        require_product_authority_available(&authority_meta, &authority_migration)?;
        let contract = Self {
            authority_meta,
            authority_migration,
            revisions: db.open_tree(TREE_REVISIONS).map_err(to_storage_io)?,
            views: db.open_tree(TREE_VIEWS).map_err(to_storage_io)?,
            attestations: db.open_tree(TREE_ATTESTATIONS).map_err(to_storage_io)?,
            intents: db.open_tree(TREE_INTENTS).map_err(to_storage_io)?,
            owner_fences: db.open_tree(TREE_OWNER_FENCES).map_err(to_storage_io)?,
            snapshots: db.open_tree(TREE_SNAPSHOTS).map_err(to_storage_io)?,
            schema: db.open_tree(TREE_SCHEMA).map_err(to_storage_io)?,
            write_gate,
            db,
        };
        contract.migrate_exclusively()?;
        Ok(contract)
    }

    /// Hold the available product authority across one cross-domain operation.
    pub(crate) fn with_product_authority_available<T>(
        &self,
        operation: impl FnOnce(&BeliefReadinessAuthority<'_>) -> Result<T, StorageError>,
    ) -> Result<T, StorageError> {
        let _exclusive = self.write_gate.write();
        self.require_product_authority_available()?;
        operation(&BeliefReadinessAuthority { contract: self })
    }

    fn verified_attestation_exclusively(
        &self,
        attestation_id: &str,
    ) -> Result<BeliefReadinessAttestation, StorageError> {
        let needs_upgrade = self
            .read_attestation(attestation_id)?
            .is_some_and(|attestation| attestation.requires_legacy_upgrade());
        if needs_upgrade {
            // TODO compat-shim: remove this late v1 rewrite after accepted W3A
            // fixture insertion is retired and every supported store carries a
            // completed marker from the final v1-capable release.
            self.migrate_identity(attestation_id.as_bytes())?;
        }

        let attestation = self.read_attestation(attestation_id)?.ok_or_else(|| {
            StorageError::MigrationConflict(format!(
                "readiness proof references missing belief attestation '{attestation_id}'"
            ))
        })?;
        attestation.validate()?;
        if attestation.attestation_id != attestation_id {
            return Err(StorageError::MigrationConflict(
                "readiness attestation key conflicts with its identity".to_string(),
            ));
        }
        self.validate_products(&attestation, true)?;
        Ok(attestation)
    }

    fn read_attestation(
        &self,
        attestation_id: &str,
    ) -> Result<Option<BeliefReadinessAttestation>, StorageError> {
        decode_optional(
            self.attestations
                .get(attestation_id.as_bytes())
                .map_err(to_storage_io)?,
        )
    }

    fn require_product_authority_available(&self) -> Result<(), StorageError> {
        require_product_authority_available(&self.authority_meta, &self.authority_migration)
    }

    fn migrate_exclusively(&self) -> Result<(), StorageError> {
        let mut state = match self.schema.get(KEY_SCHEMA_STATE).map_err(to_storage_io)? {
            Some(raw) => serde_json::from_slice(&raw).map_err(to_storage_data)?,
            None => {
                let initial = if self.intents.is_empty() && self.attestations.is_empty() {
                    MigrationState::complete()
                } else {
                    MigrationState::initial()
                };
                self.put_state(&initial)?;
                initial
            }
        };
        state.validate()?;
        while state.phase != MigrationPhase::Complete {
            state = self.advance(state)?;
        }
        Ok(())
    }

    fn advance(&self, mut state: MigrationState) -> Result<MigrationState, StorageError> {
        state.validate()?;
        let tree = match state.phase {
            MigrationPhase::Intents => &self.intents,
            MigrationPhase::Visible => &self.attestations,
            MigrationPhase::Complete => return Ok(state),
        };
        let mut keys = Vec::with_capacity(MIGRATION_BATCH);
        match state.cursor.as_deref() {
            Some(cursor) => {
                for item in tree
                    .range::<&[u8], _>((Bound::Excluded(cursor), Bound::Unbounded))
                    .take(MIGRATION_BATCH)
                {
                    let key = item.map_err(to_storage_io)?.0;
                    keys.push(key.to_vec());
                }
            }
            None => {
                for item in tree.iter().take(MIGRATION_BATCH) {
                    let key = item.map_err(to_storage_io)?.0;
                    keys.push(key.to_vec());
                }
            }
        }
        if keys.is_empty() {
            state.phase = match state.phase {
                MigrationPhase::Intents => MigrationPhase::Visible,
                MigrationPhase::Visible => MigrationPhase::Complete,
                MigrationPhase::Complete => MigrationPhase::Complete,
            };
            state.cursor = None;
        } else {
            for key in &keys {
                self.migrate_identity(key)?;
            }
            state.cursor = keys.last().cloned();
        }
        self.put_state(&state)?;
        Ok(state)
    }

    fn put_state(&self, state: &MigrationState) -> Result<(), StorageError> {
        state.validate()?;
        self.schema
            .insert(
                KEY_SCHEMA_STATE,
                serde_json::to_vec(state).map_err(to_storage_data)?,
            )
            .map_err(to_storage_io)?;
        self.db.flush().map_err(|error| {
            StorageError::DurabilityIndeterminate(format!(
                "belief readiness schema migration flush failed: {error}"
            ))
        })?;
        Ok(())
    }

    fn migrate_identity(&self, key: &[u8]) -> Result<(), StorageError> {
        let intent_raw = self.intents.get(key).map_err(to_storage_io)?;
        let visible_raw = self.attestations.get(key).map_err(to_storage_io)?;
        let source_raw = visible_raw
            .as_ref()
            .or(intent_raw.as_ref())
            .ok_or_else(|| {
                StorageError::MigrationConflict(
                    "readiness migration identity disappeared during scan".to_string(),
                )
            })?;
        let legacy: BeliefReadinessAttestation =
            serde_json::from_slice(source_raw).map_err(to_storage_data)?;
        legacy.validate()?;
        if key != legacy.attestation_id.as_bytes() {
            return Err(StorageError::MigrationConflict(
                "readiness attestation tree key conflicts with its identity".to_string(),
            ));
        }
        if !legacy.requires_legacy_upgrade() {
            return self.validate_products(&legacy, visible_raw.is_some());
        }
        for raw in [intent_raw.as_ref(), visible_raw.as_ref()]
            .into_iter()
            .flatten()
        {
            let decoded: BeliefReadinessAttestation =
                serde_json::from_slice(raw).map_err(to_storage_data)?;
            if decoded != legacy {
                return Err(StorageError::MigrationConflict(
                    "legacy readiness intent and visible record diverge".to_string(),
                ));
            }
        }
        let revision_raw = self
            .revisions
            .get(legacy.belief_revision_id.as_bytes())
            .map_err(to_storage_io)?
            .ok_or_else(|| {
                StorageError::MigrationConflict(
                    "legacy readiness attestation references a missing revision".to_string(),
                )
            })?;
        let revision: BeliefRevision =
            serde_json::from_slice(&revision_raw).map_err(to_storage_data)?;
        let view = self.legacy_view(&legacy, &revision)?;
        let upgraded = legacy.clone().upgrade_legacy(&revision, &view)?;
        let snapshot = BeliefReadinessSnapshot {
            attestation: upgraded.clone(),
            revision,
            view,
        };
        snapshot.validate()?;
        let upgraded_bytes = serde_json::to_vec(&upgraded).map_err(to_storage_data)?;
        let snapshot_bytes = serde_json::to_vec(&snapshot).map_err(to_storage_data)?;
        let intent_expected = intent_raw.as_ref().map(|raw| raw.as_ref());
        let visible_expected = visible_raw.as_ref().map(|raw| raw.as_ref());
        (
            &self.revisions,
            &self.intents,
            &self.owner_fences,
            &self.snapshots,
            &self.attestations,
        )
            .transaction(
                |(revisions, intents, owner_fences, snapshots, attestations)| {
                    require_value(
                        revisions,
                        legacy.belief_revision_id.as_bytes(),
                        revision_raw.as_ref(),
                        "legacy readiness revision",
                    )?;
                    require_optional_value(
                        intents,
                        key,
                        intent_expected,
                        "legacy readiness intent",
                    )?;
                    require_optional_value(
                        attestations,
                        key,
                        visible_expected,
                        "legacy readiness attestation",
                    )?;
                    match snapshots.get(key)? {
                        Some(existing) if existing.as_ref() != snapshot_bytes.as_slice() => {
                            return Err(ConflictableTransactionError::Abort(
                                "legacy readiness snapshot conflicts with migration".to_string(),
                            ));
                        }
                        Some(_) => {}
                        None => {
                            snapshots.insert(key, snapshot_bytes.as_slice())?;
                        }
                    }
                    match owner_fences.get(key)? {
                        Some(existing) if existing.as_ref() != b"legacy-unfenced" => {
                            validate_owner_transaction(existing.as_ref())?;
                        }
                        Some(_) => {}
                        None => {
                            // TODO compat-shim: remove the v1 owner sentinel after every
                            // accepted W3A identity has an exact capability claim and the
                            // fixture migration tests remain green without this branch.
                            owner_fences.insert(key, b"legacy-unfenced")?;
                        }
                    }
                    if intent_expected.is_some() || visible_expected.is_some() {
                        intents.insert(key, upgraded_bytes.as_slice())?;
                    }
                    if visible_expected.is_some() {
                        attestations.insert(key, upgraded_bytes.as_slice())?;
                    }
                    Ok(())
                },
            )
            .map_err(|error| match error {
                TransactionError::Abort(message) => StorageError::MigrationConflict(message),
                TransactionError::Storage(error) => to_storage_io(error),
            })?;
        self.validate_products(&upgraded, visible_expected.is_some())
    }

    fn legacy_view(
        &self,
        legacy: &BeliefReadinessAttestation,
        revision: &BeliefRevision,
    ) -> Result<BeliefView, StorageError> {
        if let Some(view) = decode_optional::<BeliefView>(
            self.views
                .get(legacy.belief_key.index_key().as_bytes())
                .map_err(to_storage_io)?,
        )? {
            if view.view_id == legacy.belief_view_id
                && hash_readiness_view(&view)? == legacy.belief_view_hash
            {
                return Ok(view);
            }
        }
        let projected = project_view(revision);
        if projected.view_id == legacy.belief_view_id
            && hash_readiness_view(&projected)? == legacy.belief_view_hash
        {
            Ok(projected)
        } else {
            Err(StorageError::MigrationConflict(
                "legacy readiness view cannot be reconstructed with hash parity".to_string(),
            ))
        }
    }

    fn validate_products(
        &self,
        expected: &BeliefReadinessAttestation,
        require_visible: bool,
    ) -> Result<(), StorageError> {
        expected.validate()?;
        let key = expected.attestation_id.as_bytes();
        let encoded = serde_json::to_vec(expected).map_err(to_storage_data)?;
        let intent = self
            .intents
            .get(key)
            .map_err(to_storage_io)?
            .ok_or_else(|| {
                StorageError::MigrationConflict(
                    "readiness attestation is missing its exact intent".to_string(),
                )
            })?;
        if intent.as_ref() != encoded.as_slice() {
            return Err(StorageError::MigrationConflict(
                "readiness attestation intent diverges from its product".to_string(),
            ));
        }
        let snapshot: BeliefReadinessSnapshot =
            decode_optional(self.snapshots.get(key).map_err(to_storage_io)?)?.ok_or_else(|| {
                StorageError::MigrationConflict(
                    "readiness attestation is missing its immutable snapshot".to_string(),
                )
            })?;
        snapshot.validate()?;
        if snapshot.attestation != *expected {
            return Err(StorageError::MigrationConflict(
                "readiness snapshot diverges from its attestation".to_string(),
            ));
        }
        let owner = self
            .owner_fences
            .get(key)
            .map_err(to_storage_io)?
            .ok_or_else(|| {
                StorageError::MigrationConflict(
                    "readiness attestation is missing its owner fence".to_string(),
                )
            })?;
        // TODO compat-shim: remove sentinel verification after all accepted W3A
        // identities have exact capability claims and narrow reopen tests remain
        // green without legacy owner products.
        if owner.as_ref() == b"legacy-unfenced" {
            if !expected.has_legacy_identity() {
                return Err(StorageError::MigrationConflict(
                    "current readiness attestation has no exact owner fence".to_string(),
                ));
            }
        } else {
            let owner = std::str::from_utf8(owner.as_ref()).map_err(|error| {
                StorageError::MigrationConflict(format!(
                    "readiness owner fence is not UTF-8: {error}"
                ))
            })?;
            validate_readiness_owner_hash(owner)?;
        }
        if require_visible {
            let visible = self
                .attestations
                .get(key)
                .map_err(to_storage_io)?
                .ok_or_else(|| {
                    StorageError::MigrationConflict(
                        "readiness attestation is not durably visible".to_string(),
                    )
                })?;
            if visible.as_ref() != encoded.as_slice() {
                return Err(StorageError::MigrationConflict(
                    "visible readiness attestation diverges from its identity".to_string(),
                ));
            }
        }
        Ok(())
    }
}

fn project_view(revision: &BeliefRevision) -> BeliefView {
    BeliefView {
        view_id: format!("view-{}", revision.revision_id),
        key: revision.belief_key.clone(),
        current_revision_id: Some(revision.revision_id.clone()),
        status: revision.status.clone(),
        posterior: revision.posterior.clone(),
        planner_projection: revision.planner_projection.clone(),
        uncertainty: revision.uncertainty,
        precision: revision.precision,
        freshness: revision.freshness.clone(),
        contradiction: revision.contradiction.clone(),
        observation: revision.observation.clone(),
        assessment_state: "complete".to_string(),
        advisory_posture: if revision.planner_projection.confidence
            < revision.planner_projection.threshold
        {
            "observe_or_repair".to_string()
        } else {
            "ready".to_string()
        },
        provenance: revision.provenance.clone(),
        hydration: HydrationRefs {
            evidence_ids: revision.evidence_ids.clone(),
            source_fact_ids: revision.provenance.source_fact_ids.clone(),
            graph_anchor_ids: revision.provenance.graph_anchor_ids.clone(),
            revision_id: Some(revision.revision_id.clone()),
        },
    }
}

fn require_value(
    tree: &sled::transaction::TransactionalTree,
    key: &[u8],
    expected: &[u8],
    product: &str,
) -> Result<(), ConflictableTransactionError<String>> {
    if tree.get(key)?.as_deref() != Some(expected) {
        return Err(ConflictableTransactionError::Abort(format!(
            "{product} changed during readiness migration"
        )));
    }
    Ok(())
}

fn require_optional_value(
    tree: &sled::transaction::TransactionalTree,
    key: &[u8],
    expected: Option<&[u8]>,
    product: &str,
) -> Result<(), ConflictableTransactionError<String>> {
    if tree.get(key)?.as_deref() != expected {
        return Err(ConflictableTransactionError::Abort(format!(
            "{product} changed during readiness migration"
        )));
    }
    Ok(())
}

fn validate_owner_transaction(owner: &[u8]) -> Result<(), ConflictableTransactionError<String>> {
    if owner.len() != 64
        || !owner
            .iter()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(byte))
    {
        return Err(ConflictableTransactionError::Abort(
            "legacy readiness owner fence is not a lowercase BLAKE3 digest".to_string(),
        ));
    }
    Ok(())
}

fn decode_optional<T: serde::de::DeserializeOwned>(
    raw: Option<sled::IVec>,
) -> Result<Option<T>, StorageError> {
    raw.map(|raw| serde_json::from_slice(&raw).map_err(to_storage_data))
        .transpose()
}

fn to_storage_io(error: sled::Error) -> StorageError {
    StorageError::IoError(std::io::Error::other(error.to_string()))
}

fn to_storage_data(error: serde_json::Error) -> StorageError {
    StorageError::IoError(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        error.to_string(),
    ))
}
