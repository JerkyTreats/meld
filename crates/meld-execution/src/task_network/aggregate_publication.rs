//! Aggregate package outcome production and publication.
//!
//! Owner: task network publication. One package run over a selected tree
//! becomes eligible for selected-tree evidence only as a whole. This module
//! detects aggregate completion over durable package-run progress, produces
//! the frozen [`AggregatePackageOutcome`], and appends it idempotently to the
//! event ledger through the existing append sink with a durable
//! identity-keyed receipt record.
//!
//! Invariants:
//!
//! - Completion detection is read-only over durable state: the persisted
//!   executor snapshot carries the run's own compiled fan-out, and the
//!   expected work-unit set is exactly that graph. Directory inventory or
//!   any other runtime input never widens or narrows the expected set.
//! - An incomplete run produces nothing. No partial aggregate is ever built,
//!   persisted, or appended, so ledger absence of the aggregate event types
//!   is itself ineligibility for the selected-tree question.
//! - The run's terminal state is the durable task-network outcome recorded
//!   through the command boundary. Durable per-folder progress alone never
//!   produces an aggregate, and a succeeded terminal outcome over incomplete
//!   durable progress is rejected rather than trusted.
//! - Per-folder subjects, the task instance, the outcome identity, and the
//!   artifact identities are carried into the aggregate intact from durable
//!   records. Nothing is summarized away.
//! - Folder classification arrives as data on the run binding: only work
//!   units whose durable capability type is named there contribute folder
//!   rows, so orchestration units with placeholder scope references never
//!   mint phantom folder subjects. Completion detection still spans the
//!   whole graph.
//! - Aggregate publication is idempotent at both layers: the durable receipt
//!   record keyed on the aggregate identity, and the deterministic event
//!   record id inside the ledger.
//!
//! This module does not own evidence interpretation, satisfaction policy, or
//! goal lifecycle. The world model maps the appended aggregate event types
//! into belief evidence; per-task publications continue to flow through
//! [`crate::task_network::publication`] untouched.
//!
//! # Example
//!
//! ```rust
//! use meld_execution::task_network::aggregate_publication::PublishAggregateRequest;
//!
//! let request = PublishAggregateRequest {
//!     session_id: "session-aggregate".to_string(),
//!     worker_id: "worker-aggregate".to_string(),
//! };
//!
//! assert_eq!(request.worker_id, "worker-aggregate");
//! ```

use std::collections::{BTreeMap, BTreeSet};

use meld_events::{AppendReceipt, DomainObjectRef, EventEnvelope, EventRelation, LedgerIdentity};
use serde::{Deserialize, Serialize};

use crate::task::{TaskExecutorSnapshot, TaskProgressStore};
use crate::task_network::{
    aggregate::{
        AggregatePackageOutcome, AggregatePackageStatus, FolderPublicationResult,
        SemanticYieldSummary, AGGREGATE_PACKAGE_COMPLETED_EVENT_TYPE,
        AGGREGATE_PACKAGE_FAILED_EVENT_TYPE,
    },
    contracts::has_text,
    dispatch::{Outcome, OutcomeStatus},
    publication::EventAppendSink,
};

/// Domain that owns per-folder workspace subjects, mirroring the task event
/// mapping so the aggregate and the per-task facts name folders identically.
const FOLDER_SUBJECT_DOMAIN: &str = "workspace_fs";
const FOLDER_SUBJECT_KIND: &str = "node";

const TREE_AGGREGATE_PUBLICATIONS: &str = "task_network_aggregate_publications_v1";
const AGGREGATE_PUBLICATION_SCHEMA_VERSION: u32 = 1;

/// Aggregate publication error.
#[derive(Debug, thiserror::Error)]
pub enum AggregatePublicationError {
    /// Request or binding validation failed before any work began.
    #[error("aggregate publication request is invalid: {0}")]
    InvalidRequest(String),

    /// Durable progress could not be read.
    #[error("aggregate completion read failed: {0}")]
    Progress(String),

    /// Aggregate publication storage failed.
    #[error("aggregate publication store error: {0}")]
    Storage(String),

    /// Persisted aggregate publication data could not be decoded.
    #[error("aggregate publication decode error: {0}")]
    Decode(String),

    /// A durable record under the same aggregate identity carries different
    /// content.
    #[error("aggregate '{aggregate_id}' already exists with different content")]
    IdentityDrift {
        /// Aggregate identity that collided.
        aggregate_id: String,
    },

    /// A completed run matched no folder work units, so the binding's
    /// classification and the compiled graph disagree. A completed
    /// selected-tree run must carry at least one folder row.
    #[error(
        "package run '{package_run_id}' completed with no work units matching the binding's folder classification"
    )]
    NoFolderWorkUnits {
        /// Package run whose compiled graph matched no classification entry.
        package_run_id: String,
    },

    /// A succeeded terminal outcome was supplied while durable progress still
    /// has incomplete work units. The aggregate must never trust a terminal
    /// claim that durable state contradicts.
    #[error(
        "package run '{package_run_id}' has a succeeded terminal outcome but incomplete durable progress"
    )]
    PrematureCompletion {
        /// Package run whose durable progress is incomplete.
        package_run_id: String,
        /// Work units still pending in durable progress.
        pending_instance_ids: Vec<String>,
    },

    /// A compiled work unit carries no scope reference, so no folder subject
    /// can be preserved for it.
    #[error("capability instance '{capability_instance_id}' has no folder scope reference")]
    MissingFolderSubject {
        /// Work unit without a durable folder subject.
        capability_instance_id: String,
    },

    /// A terminal outcome artifact names a producer outside the run's own
    /// compiled fan-out.
    #[error(
        "artifact '{artifact_id}' was produced by unknown work unit '{capability_instance_id}'"
    )]
    UnknownArtifactProducer {
        /// Artifact whose producer is not in the package graph.
        artifact_id: String,
        /// Producer instance missing from the compiled fan-out.
        capability_instance_id: String,
    },

    /// A receipt names a ledger other than the one the sink is bound to.
    #[error("aggregate receipt belongs to ledger {actual}, but sink is bound to {expected}")]
    LedgerIdentityMismatch {
        /// Ledger accepted by the supplied sink.
        expected: LedgerIdentity,
        /// Ledger named by the receipt.
        actual: LedgerIdentity,
    },
}

/// Caller-supplied identity binding for one package run.
///
/// The selected scope is the run's target binding carried as typed input.
/// This module never reattaches or re-derives it from runtime state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AggregateRunBinding {
    /// Package run whose aggregate this binding names.
    pub package_run_id: String,
    /// Task network the run executed in.
    pub network_id: String,
    /// Selected-tree subject the run targets.
    pub selected_scope: DomainObjectRef,
    /// Durable capability type ids whose work units carry per-folder work.
    ///
    /// The discriminator is `capability_type_id` on the snapshot's compiled
    /// instances, which persists inside the durable executor snapshot. A
    /// package graph may also contain orchestration units, such as a
    /// traversal seeder bound to a placeholder scope reference, whose scope
    /// is not a folder subject: those units still count toward completion
    /// but never contribute a folder result. The classification must name at
    /// least one capability type.
    pub folder_unit_capability_types: Vec<String>,
    /// Package-declared yield source: artifact type and array field whose
    /// cardinality measures per-folder semantic yield. Absent for packages
    /// without a countable yield; the aggregate outcome then carries no
    /// yield summary.
    pub semantic_yield_source: Option<(String, String)>,
}

/// Read-only completion projection over one durable executor snapshot.
///
/// The expected set is the run's own compiled fan-out: every capability
/// instance known to the durable graph, including instances added by applied
/// expansions. The projection never consults runtime inventory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AggregateCompletionCheck {
    /// Package run the projection describes.
    pub package_run_id: String,
    /// Work units known to the durable package graph.
    pub known_units: usize,
    /// Work units completed durably.
    pub completed_units: usize,
    /// Work units not yet completed, in stable order.
    pub pending_instance_ids: Vec<String>,
}

impl AggregateCompletionCheck {
    /// True when every known work unit completed durably.
    ///
    /// An empty graph is never complete: the package graph is authoritative
    /// for required work, and a run that never fanned out has proven nothing
    /// about the selected tree.
    pub fn is_complete(&self) -> bool {
        self.known_units > 0 && self.pending_instance_ids.is_empty()
    }
}

/// Projects completion state from one durable executor snapshot.
pub fn check_aggregate_completion(snapshot: &TaskExecutorSnapshot) -> AggregateCompletionCheck {
    let completed: BTreeSet<&str> = snapshot
        .completed_instance_ids
        .iter()
        .map(String::as_str)
        .collect();
    let pending_instance_ids: Vec<String> = snapshot
        .compiled_task
        .capability_instances
        .iter()
        .map(|instance| instance.capability_instance_id.clone())
        .filter(|instance_id| !completed.contains(instance_id.as_str()))
        .collect();

    AggregateCompletionCheck {
        package_run_id: snapshot.init_payload.task_run_context.task_run_id.clone(),
        known_units: snapshot.compiled_task.capability_instances.len(),
        completed_units: snapshot
            .compiled_task
            .capability_instances
            .iter()
            .filter(|instance| completed.contains(instance.capability_instance_id.as_str()))
            .count(),
        pending_instance_ids,
    }
}

/// Loads the durable snapshot for a run and projects its completion state.
///
/// Returns `None` when the run has no durable progress yet. The read is
/// side-effect free: detection never advances, repairs, or reorders work.
pub fn load_aggregate_completion(
    progress: &TaskProgressStore,
    package_run_id: &str,
) -> Result<Option<AggregateCompletionCheck>, AggregatePublicationError> {
    let snapshot = progress
        .load(package_run_id)
        .map_err(|error| AggregatePublicationError::Progress(error.to_string()))?;
    Ok(snapshot.as_ref().map(check_aggregate_completion))
}

/// Maps each expected folder to the folder-classified work units targeting it.
///
/// Folder identity is the durable scope reference bound to each compiled
/// capability instance whose durable capability type is named by the
/// classification. Non-classified units, such as traversal or other
/// orchestration instances with placeholder scope references, are excluded
/// here while still counting toward whole-graph completion. Multiple work
/// units may target one folder; every classified unit must carry a scope
/// reference.
pub fn expected_folder_units(
    snapshot: &TaskExecutorSnapshot,
    folder_unit_capability_types: &[String],
) -> Result<BTreeMap<String, Vec<String>>, AggregatePublicationError> {
    let classified: BTreeSet<&str> = folder_unit_capability_types
        .iter()
        .map(String::as_str)
        .collect();
    let mut folders: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for instance in &snapshot.compiled_task.capability_instances {
        if !classified.contains(instance.capability_type_id.as_str()) {
            continue;
        }
        if !has_text(&instance.scope_ref) {
            return Err(AggregatePublicationError::MissingFolderSubject {
                capability_instance_id: instance.capability_instance_id.clone(),
            });
        }
        folders
            .entry(instance.scope_ref.clone())
            .or_default()
            .push(instance.capability_instance_id.clone());
    }
    Ok(folders)
}

/// Production decision for one package run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AggregateProduction {
    /// The run is not terminal or not complete. Nothing was produced.
    Incomplete {
        /// Work units still pending in durable progress.
        pending_instance_ids: Vec<String>,
    },
    /// The run reached a terminal state and produced its aggregate.
    Produced(AggregatePackageOutcome),
}

/// Builds the frozen aggregate outcome for one terminal package run.
///
/// The terminal outcome is the durable task-network record accepted through
/// the command boundary; `None` means the run is not terminal and nothing is
/// produced regardless of durable progress. A succeeded terminal outcome
/// requires complete durable progress. A failed terminal outcome produces the
/// failed aggregate over the folders whose work units all completed.
pub fn produce_aggregate_outcome(
    snapshot: &TaskExecutorSnapshot,
    terminal_outcome: Option<&Outcome>,
    binding: &AggregateRunBinding,
) -> Result<AggregateProduction, AggregatePublicationError> {
    validate_binding(binding)?;
    let snapshot_run_id = &snapshot.init_payload.task_run_context.task_run_id;
    if snapshot_run_id != &binding.package_run_id {
        return Err(AggregatePublicationError::InvalidRequest(format!(
            "durable progress belongs to run '{snapshot_run_id}', binding names run '{}'",
            binding.package_run_id
        )));
    }

    let check = check_aggregate_completion(snapshot);
    let Some(outcome) = terminal_outcome else {
        return Ok(AggregateProduction::Incomplete {
            pending_instance_ids: check.pending_instance_ids,
        });
    };
    validate_outcome_identity(outcome)?;

    let status = match outcome.status {
        OutcomeStatus::Succeeded => {
            if !check.is_complete() {
                return Err(AggregatePublicationError::PrematureCompletion {
                    package_run_id: binding.package_run_id.clone(),
                    pending_instance_ids: check.pending_instance_ids,
                });
            }
            AggregatePackageStatus::Completed
        }
        OutcomeStatus::Failed => AggregatePackageStatus::Failed,
    };

    let folder_units = expected_folder_units(snapshot, &binding.folder_unit_capability_types)?;
    if status == AggregatePackageStatus::Completed && folder_units.is_empty() {
        // A completed selected-tree run without one folder row means the
        // classification and the compiled graph disagree; publishing an empty
        // aggregate would misread as satisfied folder work.
        return Err(AggregatePublicationError::NoFolderWorkUnits {
            package_run_id: binding.package_run_id.clone(),
        });
    }
    let known_units: BTreeSet<&str> = snapshot
        .compiled_task
        .capability_instances
        .iter()
        .map(|instance| instance.capability_instance_id.as_str())
        .collect();
    let mut folder_artifacts: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    let mut unit_to_folder: BTreeMap<&str, &str> = BTreeMap::new();
    for (folder, units) in &folder_units {
        for unit in units {
            unit_to_folder.insert(unit.as_str(), folder.as_str());
        }
    }
    // Artifact identities transfer intact and in durable record order from
    // the terminal outcome; grouping by folder is the only transformation.
    // Artifacts from known non-folder units are orchestration products, not
    // per-folder facts, and stay out of the folder rows.
    let mut folder_yields: BTreeMap<&str, u64> = BTreeMap::new();
    for artifact in &outcome.artifact_records {
        let producer = artifact.producer.capability_instance_id.as_str();
        let Some(folder) = unit_to_folder.get(producer) else {
            if known_units.contains(producer) {
                continue;
            }
            return Err(AggregatePublicationError::UnknownArtifactProducer {
                artifact_id: artifact.artifact_id.clone(),
                capability_instance_id: producer.to_string(),
            });
        };
        folder_artifacts
            .entry(folder)
            .or_default()
            .push(artifact.artifact_id.clone());
        // Yield rides the artifact content the ledger already carries; the
        // package declares which artifact and field measure it.
        if let Some((yield_type, yield_field)) = &binding.semantic_yield_source {
            if &artifact.artifact_type_id == yield_type {
                let count = artifact
                    .content
                    .get(yield_field)
                    .and_then(|value| value.as_array())
                    .map(|items| items.len() as u64)
                    .unwrap_or(0);
                *folder_yields.entry(folder).or_default() += count;
            }
        }
    }

    let completed: BTreeSet<&str> = snapshot
        .completed_instance_ids
        .iter()
        .map(String::as_str)
        .collect();
    let mut folder_results = Vec::with_capacity(folder_units.len());
    for (folder, units) in &folder_units {
        // A failed run preserves only folders whose work fully completed;
        // folders with pending or failed units contributed no folder result
        // and must not read as finished work inside the failed aggregate.
        let folder_complete = units.iter().all(|unit| completed.contains(unit.as_str()));
        if !folder_complete {
            debug_assert!(status == AggregatePackageStatus::Failed);
            continue;
        }
        folder_results.push(FolderPublicationResult {
            folder: folder_subject(folder)?,
            task_instance_id: outcome.task_instance_id.clone(),
            outcome_id: outcome.outcome_id.clone(),
            artifact_ids: folder_artifacts.remove(folder.as_str()).unwrap_or_default(),
            verified_yield: folder_yields.get(folder.as_str()).copied().unwrap_or(0),
        });
    }

    let semantic_yield = binding.semantic_yield_source.as_ref().map(|_| {
        let hollow_folder_count = folder_results
            .iter()
            .filter(|result| result.verified_yield == 0)
            .count() as u64;
        SemanticYieldSummary {
            verified_yield_total: folder_results
                .iter()
                .map(|result| result.verified_yield)
                .sum(),
            folder_count: folder_results.len() as u64,
            hollow_folder_count,
            class: if folder_results.is_empty() || hollow_folder_count > 0 {
                "hollow".to_string()
            } else {
                "substantive".to_string()
            },
        }
    });

    Ok(AggregateProduction::Produced(AggregatePackageOutcome {
        aggregate_id: AggregatePackageOutcome::derive_aggregate_id(
            &binding.package_run_id,
            &binding.network_id,
        ),
        package_run_id: binding.package_run_id.clone(),
        network_id: binding.network_id.clone(),
        selected_scope: binding.selected_scope.clone(),
        status,
        folder_results,
        semantic_yield,
    }))
}

/// Durable lifecycle state for one aggregate publication.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AggregatePublicationState {
    /// Publication is waiting for an external append attempt.
    Pending,
    /// Publication was appended and carries its complete authority receipt.
    Published {
        /// Identity-bearing durable authority acknowledgement.
        receipt: AppendReceipt,
    },
    /// Last append attempt failed and can be retried.
    Failed {
        /// Failure summary from the last append attempt.
        error: String,
    },
}

/// Durable aggregate publication record keyed on the aggregate identity.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AggregatePublication {
    /// Canonical aggregate outcome carried by this publication.
    pub aggregate: AggregatePackageOutcome,
    /// Worker identity that persisted the current state, kept for audit.
    pub worker_id: String,
    /// Current publication state.
    pub state: AggregatePublicationState,
}

#[derive(Debug, Serialize, Deserialize)]
struct StoredAggregatePublication {
    record_schema_version: u32,
    publication: AggregatePublication,
}

/// Sled-backed outbox holding one publication record per aggregate identity.
///
/// The store persists aggregates only after completion detection allows
/// production, so its content never includes a partial aggregate. Writes
/// flush before returning so a reopen observes what the caller believes is
/// durable, matching the per-task publication route's receipt persistence.
#[derive(Debug, Clone)]
pub struct AggregatePublicationStore {
    db: sled::Db,
    tree: sled::Tree,
}

impl AggregatePublicationStore {
    /// Opens the aggregate publication store within a caller-owned database.
    pub fn open(db: sled::Db) -> Result<Self, AggregatePublicationError> {
        let tree = db
            .open_tree(TREE_AGGREGATE_PUBLICATIONS)
            .map_err(to_storage)?;
        Ok(Self { db, tree })
    }

    /// Loads the publication record for one aggregate identity.
    pub fn load(
        &self,
        aggregate_id: &str,
    ) -> Result<Option<AggregatePublication>, AggregatePublicationError> {
        let Some(raw) = self.tree.get(aggregate_id.as_bytes()).map_err(to_storage)? else {
            return Ok(None);
        };
        let stored: StoredAggregatePublication = serde_json::from_slice(&raw).map_err(to_decode)?;
        if stored.record_schema_version != AGGREGATE_PUBLICATION_SCHEMA_VERSION {
            return Err(AggregatePublicationError::Decode(format!(
                "unsupported aggregate publication schema version '{}'",
                stored.record_schema_version
            )));
        }
        if stored.publication.aggregate.aggregate_id != aggregate_id {
            return Err(AggregatePublicationError::Decode(
                "stored aggregate publication identity mismatch".to_string(),
            ));
        }
        Ok(Some(stored.publication))
    }

    fn save(&self, publication: &AggregatePublication) -> Result<(), AggregatePublicationError> {
        let value = serde_json::to_vec(&StoredAggregatePublication {
            record_schema_version: AGGREGATE_PUBLICATION_SCHEMA_VERSION,
            publication: publication.clone(),
        })
        .map_err(to_decode)?;
        self.tree
            .insert(publication.aggregate.aggregate_id.as_bytes(), value)
            .map_err(to_storage)?;
        self.db.flush().map_err(to_storage)?;
        Ok(())
    }
}

/// Request to publish one aggregate package outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishAggregateRequest {
    /// Event ledger session partition for the produced envelope.
    pub session_id: String,
    /// Worker identity persisted on the durable publication record.
    pub worker_id: String,
}

/// Reason a publication attempt produced nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AggregateSkipReason {
    /// The run has no durable progress at all.
    RunProgressNotFound,
    /// The run has no durable terminal outcome or incomplete durable work.
    RunNotTerminal {
        /// Work units still pending in durable progress.
        pending_instance_ids: Vec<String>,
    },
}

/// Result for one aggregate publication attempt.
#[derive(Debug, Clone, PartialEq)]
pub enum AggregatePublishResult {
    /// The run is not eligible; nothing was produced, persisted, or appended.
    Skipped(AggregateSkipReason),
    /// Aggregate appended and its receipt persisted.
    Published {
        /// Stable aggregate identity.
        aggregate_id: String,
        /// Deterministic event ledger record id.
        event_record_id: String,
        /// Complete durable authority acknowledgement.
        receipt: AppendReceipt,
    },
    /// A durable receipt for this aggregate already exists; no append ran.
    AlreadyPublished {
        /// Stable aggregate identity.
        aggregate_id: String,
        /// Previously persisted authority acknowledgement.
        receipt: AppendReceipt,
    },
    /// Event append failed and remains retryable through the durable record.
    AppendFailed {
        /// Stable aggregate identity.
        aggregate_id: String,
        /// Append failure summary.
        error: String,
    },
}

/// Returns the deterministic ledger record id for one aggregate identity.
pub fn aggregate_event_record_id(aggregate_id: &str) -> String {
    format!("execution::task_network_aggregate::{aggregate_id}")
}

/// Builds the canonical ledger envelope for one aggregate outcome.
pub fn build_aggregate_envelope(
    session_id: &str,
    aggregate: &AggregatePackageOutcome,
) -> Result<EventEnvelope, AggregatePublicationError> {
    require_text("session_id", session_id)?;
    require_text("aggregate_id", &aggregate.aggregate_id)?;
    require_text("package_run_id", &aggregate.package_run_id)?;
    require_text("network_id", &aggregate.network_id)?;

    let event_type = match aggregate.status {
        AggregatePackageStatus::Completed => AGGREGATE_PACKAGE_COMPLETED_EVENT_TYPE,
        AggregatePackageStatus::Failed => AGGREGATE_PACKAGE_FAILED_EVENT_TYPE,
    };
    let aggregate_object = object("package_aggregate", &aggregate.aggregate_id)?;
    let network = object("task_network", &aggregate.network_id)?;
    let mut objects = vec![
        aggregate_object.clone(),
        network.clone(),
        aggregate.selected_scope.clone(),
    ];
    let mut relations = vec![
        relation("member_of", aggregate_object.clone(), network)?,
        relation(
            "targets",
            aggregate_object.clone(),
            aggregate.selected_scope.clone(),
        )?,
    ];

    let mut outcome_ids = BTreeSet::new();
    for folder_result in &aggregate.folder_results {
        objects.push(folder_result.folder.clone());
        relations.push(relation(
            "covers",
            aggregate_object.clone(),
            folder_result.folder.clone(),
        )?);
        // Folder results of one run share the terminal outcome; the graph
        // names each distinct outcome once.
        if outcome_ids.insert(folder_result.outcome_id.as_str()) {
            let outcome = object("task_outcome", &folder_result.outcome_id)?;
            objects.push(outcome.clone());
            relations.push(relation(
                "published_from",
                aggregate_object.clone(),
                outcome,
            )?);
        }
    }

    Ok(EventEnvelope::with_now_domain(
        session_id,
        "execution",
        format!(
            "task_network::{}::package::{}",
            aggregate.network_id, aggregate.package_run_id
        ),
        event_type,
        None,
        serde_json::to_value(aggregate).map_err(to_decode)?,
    )
    .with_record_id(aggregate_event_record_id(&aggregate.aggregate_id))
    .with_graph(objects, relations))
}

/// Publishes one produced aggregate outcome idempotently.
///
/// Duplicate attempts short-circuit on the persisted receipt; retried appends
/// reuse the deterministic record id so the ledger deduplicates even when the
/// durable receipt was lost.
pub fn publish_aggregate<E: EventAppendSink>(
    outbox: &AggregatePublicationStore,
    events: &E,
    request: &PublishAggregateRequest,
    aggregate: &AggregatePackageOutcome,
) -> Result<AggregatePublishResult, AggregatePublicationError> {
    validate_request(request)?;

    match outbox.load(&aggregate.aggregate_id)? {
        Some(existing) if existing.aggregate != *aggregate => {
            return Err(AggregatePublicationError::IdentityDrift {
                aggregate_id: aggregate.aggregate_id.clone(),
            });
        }
        Some(AggregatePublication {
            state: AggregatePublicationState::Published { receipt },
            ..
        }) => {
            if receipt.ledger_id != events.ledger_identity() {
                return Err(AggregatePublicationError::LedgerIdentityMismatch {
                    expected: events.ledger_identity(),
                    actual: receipt.ledger_id,
                });
            }
            return Ok(AggregatePublishResult::AlreadyPublished {
                aggregate_id: aggregate.aggregate_id.clone(),
                receipt,
            });
        }
        Some(_) | None => {}
    }

    // The pending record is the durable publication intent; it exists only
    // for terminal aggregates, so no partial aggregate ever reaches storage.
    outbox.save(&AggregatePublication {
        aggregate: aggregate.clone(),
        worker_id: request.worker_id.clone(),
        state: AggregatePublicationState::Pending,
    })?;

    let envelope = build_aggregate_envelope(&request.session_id, aggregate)?;
    match events.append_envelope_idempotent(envelope) {
        Ok(receipt) if receipt.ledger_id != events.ledger_identity() => {
            Err(AggregatePublicationError::LedgerIdentityMismatch {
                expected: events.ledger_identity(),
                actual: receipt.ledger_id,
            })
        }
        Ok(receipt) => {
            outbox.save(&AggregatePublication {
                aggregate: aggregate.clone(),
                worker_id: request.worker_id.clone(),
                state: AggregatePublicationState::Published { receipt },
            })?;
            Ok(AggregatePublishResult::Published {
                aggregate_id: aggregate.aggregate_id.clone(),
                event_record_id: aggregate_event_record_id(&aggregate.aggregate_id),
                receipt,
            })
        }
        Err(error) => {
            outbox.save(&AggregatePublication {
                aggregate: aggregate.clone(),
                worker_id: request.worker_id.clone(),
                state: AggregatePublicationState::Failed {
                    error: error.clone(),
                },
            })?;
            Ok(AggregatePublishResult::AppendFailed {
                aggregate_id: aggregate.aggregate_id.clone(),
                error,
            })
        }
    }
}

/// Detects completion, produces, and publishes the aggregate for one run.
///
/// The read path is durable-only: the progress store snapshot supplies the
/// expected work-unit set and completion, the supplied terminal outcome is
/// the durable command-boundary record, and ineligible runs return
/// [`AggregatePublishResult::Skipped`] without touching storage or the
/// ledger.
pub fn publish_aggregate_for_run<E: EventAppendSink>(
    progress: &TaskProgressStore,
    terminal_outcome: Option<&Outcome>,
    binding: &AggregateRunBinding,
    outbox: &AggregatePublicationStore,
    events: &E,
    request: &PublishAggregateRequest,
) -> Result<AggregatePublishResult, AggregatePublicationError> {
    validate_request(request)?;
    validate_binding(binding)?;

    let snapshot = progress
        .load(&binding.package_run_id)
        .map_err(|error| AggregatePublicationError::Progress(error.to_string()))?;
    let Some(snapshot) = snapshot else {
        return Ok(AggregatePublishResult::Skipped(
            AggregateSkipReason::RunProgressNotFound,
        ));
    };

    match produce_aggregate_outcome(&snapshot, terminal_outcome, binding)? {
        AggregateProduction::Incomplete {
            pending_instance_ids,
        } => Ok(AggregatePublishResult::Skipped(
            AggregateSkipReason::RunNotTerminal {
                pending_instance_ids,
            },
        )),
        AggregateProduction::Produced(aggregate) => {
            publish_aggregate(outbox, events, request, &aggregate)
        }
    }
}

fn folder_subject(scope_ref: &str) -> Result<DomainObjectRef, AggregatePublicationError> {
    DomainObjectRef::new(FOLDER_SUBJECT_DOMAIN, FOLDER_SUBJECT_KIND, scope_ref)
        .map_err(|error| AggregatePublicationError::InvalidRequest(error.to_string()))
}

fn validate_binding(binding: &AggregateRunBinding) -> Result<(), AggregatePublicationError> {
    require_text("package_run_id", &binding.package_run_id)?;
    require_text("network_id", &binding.network_id)?;
    if binding.folder_unit_capability_types.is_empty() {
        return Err(AggregatePublicationError::InvalidRequest(
            "folder_unit_capability_types must name at least one capability type".to_string(),
        ));
    }
    for capability_type_id in &binding.folder_unit_capability_types {
        require_text("folder_unit_capability_type", capability_type_id)?;
    }
    binding
        .selected_scope
        .validate()
        .map_err(|error| AggregatePublicationError::InvalidRequest(error.to_string()))
}

fn validate_outcome_identity(outcome: &Outcome) -> Result<(), AggregatePublicationError> {
    require_text("outcome_id", &outcome.outcome_id)?;
    require_text("task_instance_id", &outcome.task_instance_id)
}

fn validate_request(request: &PublishAggregateRequest) -> Result<(), AggregatePublicationError> {
    require_text("session_id", &request.session_id)?;
    require_text("worker_id", &request.worker_id)
}

fn require_text(name: &str, value: &str) -> Result<(), AggregatePublicationError> {
    if has_text(value) {
        Ok(())
    } else {
        Err(AggregatePublicationError::InvalidRequest(format!(
            "{name} must be non-empty"
        )))
    }
}

fn object(kind: &str, id: &str) -> Result<DomainObjectRef, AggregatePublicationError> {
    DomainObjectRef::new("execution", kind, id)
        .map_err(|error| AggregatePublicationError::InvalidRequest(error.to_string()))
}

fn relation(
    relation_type: &str,
    src: DomainObjectRef,
    dst: DomainObjectRef,
) -> Result<EventRelation, AggregatePublicationError> {
    EventRelation::new(relation_type, src, dst)
        .map_err(|error| AggregatePublicationError::InvalidRequest(error.to_string()))
}

fn to_storage(error: sled::Error) -> AggregatePublicationError {
    AggregatePublicationError::Storage(error.to_string())
}

fn to_decode(error: serde_json::Error) -> AggregatePublicationError {
    AggregatePublicationError::Decode(error.to_string())
}
