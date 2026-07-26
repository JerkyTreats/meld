//! The three customer projections over one shared causal record.
//!
//! Owner: harness. Three customers consume the same durable records at
//! three altitudes: a subagent asks whether its last action moved the
//! coupling it works on (high frequency, scoped, machine-shaped); a
//! parent asks whether anything crossed the boundary it delegated or
//! whether in-scope progress looks wrong (exceptions and trajectory
//! signatures, silent when quiet); the user asks for the flow and shape
//! of the whole loop. Every projection cites shared record identities —
//! action ids, ledger sequences, subject keys — so any party drops to the
//! identical evidence on demand. Projections present only what durable
//! records confirm; they are substrate derivations, never consumer logic.
//!
//! Delegation scope is a named filter of subjects and actors granted by a
//! parent when it spawns a worker. It is a harness concept: filtering
//! happens here over records the runtime already persists, and no domain
//! knows scopes exist.

use serde::{Deserialize, Serialize};

use crate::harness::boot::HarnessError;
use crate::runtime::contracts::{RuntimeActionOutcome, RuntimeActionRecord, WaitingOnDeclaration};
use crate::runtime::ports::ProductEventReplayPort;
use crate::runtime::supervisor::SupervisorReportStore;
use meld_world_model::belief::BeliefStore;

/// Bounded action window one projection derivation reads.
const ACTION_WINDOW: usize = 256;

/// Bounded ledger page size per scoped-diff request.
const EVENT_PAGE_LIMIT: usize = 512;

/// Ticks of one repeated issue before it becomes a trajectory signature.
const SIGNATURE_THRESHOLD: usize = 2;

/// A named delegation: the subjects and actors a parent granted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DelegationScope {
    /// Scope name, chosen by the granting parent.
    pub name: String,
    /// Exact subject index keys inside the scope.
    pub subject_keys: Vec<String>,
    /// Runtime ids of the actors the worker owns.
    pub actor_ids: Vec<String>,
}

impl DelegationScope {
    fn contains_subject(&self, subject_key: &str) -> bool {
        self.subject_keys.iter().any(|key| key == subject_key)
    }
}

/// One in-scope ledger record reference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScopedEventRef {
    /// Canonical ledger sequence.
    pub seq: u64,
    /// Envelope event type.
    pub event_type: String,
    /// Envelope record id when present.
    pub record_id: Option<String>,
    /// Subject keys of the envelope objects that matched the scope.
    pub matched_subject_keys: Vec<String>,
}

/// One owned actor's latest durable tick, scoped for the subagent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScopedActorReport {
    /// Runtime id of the owned actor.
    pub runtime_id: String,
    /// Durable action record id — the shared identity every altitude cites.
    pub action_id: String,
    /// Observation time of the tick.
    pub observed_at_ms: u64,
    /// Items attempted by the tick.
    pub items_attempted: u64,
    /// Items committed by the tick.
    pub items_committed: u64,
    /// Waiting-on declarations scoped to the delegation.
    pub waiting_on: Vec<WaitingOnDeclaration>,
}

/// The subagent's scoped causal diff since its watermark.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubagentProjection {
    /// Scope this diff was filtered by.
    pub scope_name: String,
    /// Exclusive sequence the diff starts after.
    pub after_seq: u64,
    /// Highest sequence this diff covers.
    pub through_seq: u64,
    /// In-scope records appended since the watermark.
    pub events: Vec<ScopedEventRef>,
    /// Latest durable tick per owned actor.
    pub actor_reports: Vec<ScopedActorReport>,
}

/// Request for one scoped subagent diff.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubagentProjectionRequest {
    /// The delegation granted to the asking worker.
    pub scope: DelegationScope,
    /// The worker's last consumed sequence, exclusive.
    pub after_seq: u64,
}

/// One trajectory anomaly inside a delegated scope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrajectorySignature {
    /// Runtime whose trajectory looks wrong.
    pub runtime_id: String,
    /// Anomaly kind.
    pub kind: TrajectoryKind,
    /// Stable grouping signature: issue code or condition plus subject.
    pub signature: String,
    /// Consecutive ticks matching the signature.
    pub consecutive_ticks: usize,
    /// Durable action ids citing the evidence, oldest first.
    pub action_ids: Vec<String>,
}

/// The trajectory anomaly vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrajectoryKind {
    /// Ticks attempt work and commit nothing, repeatedly.
    AttemptedWithoutCommitted,
    /// The same issue signature repeats across ticks.
    RepeatingIssue,
    /// Consecutive ticks exhaust their budget.
    BudgetExhaustion,
}

/// The parent's exception view: complement events plus in-scope anomalies.
///
/// Both lists are empty when nothing crossed the boundary and no in-scope
/// trajectory looks wrong — silence is the healthy answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParentProjection {
    /// Scope whose complement was watched.
    pub scope_name: String,
    /// Exclusive sequence the exception watch starts after.
    pub after_seq: u64,
    /// Highest sequence this projection covers.
    pub through_seq: u64,
    /// Records outside the delegated scope since the watermark.
    pub out_of_scope_events: Vec<ScopedEventRef>,
    /// In-scope trajectory anomalies.
    pub trajectory_signatures: Vec<TrajectorySignature>,
}

/// Request for one parent exception view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParentProjectionRequest {
    /// The delegation whose complement is watched.
    pub scope: DelegationScope,
    /// The parent's last consumed sequence, exclusive.
    pub after_seq: u64,
}

/// One flywheel coupling's observed flow and current waits.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CouplingStatus {
    /// Runtime id owning the coupling's committing side.
    pub runtime_id: String,
    /// Items attempted across the retained action window.
    pub attempted: u64,
    /// Items committed across the retained action window.
    pub committed: u64,
    /// Ticks observed in the window.
    pub ticks: usize,
    /// Latest tick's waiting-on declarations.
    pub waiting_on: Vec<WaitingOnDeclaration>,
    /// Latest tick's durable action id.
    pub latest_action_id: Option<String>,
}

/// The user's whole-loop view: flow per coupling plus queue depths.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserProjection {
    /// One row per runtime observed in the retained window.
    pub couplings: Vec<CouplingStatus>,
    /// Dirty belief keys currently queued, when the store is readable.
    /// The count saturates at the read bound; the flag says so.
    pub dirty_key_depth: Option<u64>,
    /// True when more dirty keys exist past the reported depth.
    pub dirty_key_depth_saturated: bool,
}

/// Read surfaces the projection derivations consume.
pub struct ProjectionSources<'a> {
    /// Preserved per-tick reports.
    pub reports: &'a SupervisorReportStore,
    /// Bounded ledger replay for scoped diffs.
    pub replay: Option<&'a ProductEventReplayPort>,
    /// Belief store for queue depths, when the composition opened it.
    pub belief: Option<&'a BeliefStore>,
    /// Sequence floor fencing report reads to one session; zero reads
    /// the whole retained window.
    pub action_floor: u64,
}

/// Derive the subagent's scoped diff since its watermark.
pub fn subagent(
    sources: &ProjectionSources<'_>,
    request: &SubagentProjectionRequest,
) -> Result<SubagentProjection, HarnessError> {
    let (events, through_seq) = scoped_events(
        sources,
        request.after_seq,
        |matched| !matched.is_empty(),
        &request.scope,
    )?;
    let actor_reports = request
        .scope
        .actor_ids
        .iter()
        .filter_map(|runtime_id| {
            sources
                .reports
                .latest_action_for_runtime_since(sources.action_floor, runtime_id)
                .map_err(|error| HarnessError::Storage(error.to_string()))
                .transpose()
        })
        .map(|action| {
            action.map(|action| ScopedActorReport {
                runtime_id: action.runtime_id.clone(),
                action_id: action.action_id.clone(),
                observed_at_ms: action.observed_at_ms,
                items_attempted: action.metrics.attempted,
                items_committed: action.metrics.committed,
                waiting_on: action
                    .waiting_on
                    .into_iter()
                    .filter(|declaration| match &declaration.subject_key {
                        Some(key) => request.scope.contains_subject(key),
                        None => true,
                    })
                    .collect(),
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(SubagentProjection {
        scope_name: request.scope.name.clone(),
        after_seq: request.after_seq,
        through_seq,
        events,
        actor_reports,
    })
}

/// Derive the parent's complement exceptions and trajectory signatures.
pub fn parent(
    sources: &ProjectionSources<'_>,
    request: &ParentProjectionRequest,
) -> Result<ParentProjection, HarnessError> {
    let (out_of_scope_events, through_seq) = scoped_events(
        sources,
        request.after_seq,
        |matched| matched.is_empty(),
        &request.scope,
    )?;
    let recent = sources
        .reports
        .read_recent_actions_since(sources.action_floor, ACTION_WINDOW)
        .map_err(|error| HarnessError::Storage(error.to_string()))?;
    let mut trajectory_signatures = Vec::new();
    for runtime_id in &request.scope.actor_ids {
        let ticks: Vec<&RuntimeActionRecord> = recent
            .iter()
            .filter(|action| &action.runtime_id == runtime_id)
            .collect();
        trajectory_signatures.extend(signatures_for(runtime_id, &ticks));
    }
    Ok(ParentProjection {
        scope_name: request.scope.name.clone(),
        after_seq: request.after_seq,
        through_seq,
        out_of_scope_events,
        trajectory_signatures,
    })
}

/// Derive the user's whole-loop coupling view.
pub fn user(sources: &ProjectionSources<'_>) -> Result<UserProjection, HarnessError> {
    let recent = sources
        .reports
        .read_recent_actions_since(sources.action_floor, ACTION_WINDOW)
        .map_err(|error| HarnessError::Storage(error.to_string()))?;
    let mut couplings: std::collections::BTreeMap<String, CouplingStatus> =
        std::collections::BTreeMap::new();
    for action in &recent {
        let row = couplings
            .entry(action.runtime_id.clone())
            .or_insert_with(|| CouplingStatus {
                runtime_id: action.runtime_id.clone(),
                attempted: 0,
                committed: 0,
                ticks: 0,
                waiting_on: Vec::new(),
                latest_action_id: None,
            });
        row.attempted += action.metrics.attempted;
        row.committed += action.metrics.committed;
        row.ticks += 1;
        // Recent actions are ascending, so the last write per runtime is
        // its latest tick.
        row.waiting_on = action.waiting_on.clone();
        row.latest_action_id = Some(action.action_id.clone());
    }
    let (dirty_key_depth, dirty_key_depth_saturated) = match sources.belief {
        Some(belief) => {
            let (states, more_available) = belief
                .dirty_key_states_bounded(ACTION_WINDOW)
                .map_err(|error| HarnessError::Storage(error.to_string()))?;
            (Some(states.len() as u64), more_available)
        }
        None => (None, false),
    };
    Ok(UserProjection {
        couplings: couplings.into_values().collect(),
        dirty_key_depth,
        dirty_key_depth_saturated,
    })
}

/// Bounded scoped ledger diff: records after the watermark whose subject
/// match passes the filter, plus the coverage watermark reached.
fn scoped_events(
    sources: &ProjectionSources<'_>,
    after_seq: u64,
    keep: impl Fn(&[String]) -> bool,
    scope: &DelegationScope,
) -> Result<(Vec<ScopedEventRef>, u64), HarnessError> {
    let Some(replay) = sources.replay else {
        return Ok((Vec::new(), after_seq));
    };
    let records = replay
        .read_after_limit(after_seq, EVENT_PAGE_LIMIT)
        .map_err(|error| HarnessError::Storage(error.to_string()))?;
    let through_seq = records.last().map(|record| record.seq).unwrap_or(after_seq);
    let events = records
        .into_iter()
        .filter_map(|record| {
            let matched: Vec<String> = record
                .envelope
                .objects
                .iter()
                .map(|object| object.index_key())
                .filter(|key| scope.contains_subject(key))
                .collect();
            keep(&matched).then(|| ScopedEventRef {
                seq: record.seq,
                event_type: record.envelope.event_type.clone(),
                record_id: record.envelope.record_id.clone(),
                matched_subject_keys: matched,
            })
        })
        .collect();
    Ok((events, through_seq))
}

/// Derive the trajectory signatures for one runtime's tick window.
fn signatures_for(runtime_id: &str, ticks: &[&RuntimeActionRecord]) -> Vec<TrajectorySignature> {
    let mut signatures = Vec::new();
    push_streak(
        &mut signatures,
        runtime_id,
        TrajectoryKind::AttemptedWithoutCommitted,
        ticks,
        |action| {
            (action.metrics.attempted > 0 && action.metrics.committed == 0)
                .then(|| "attempted_without_committed".to_string())
        },
    );
    push_streak(
        &mut signatures,
        runtime_id,
        TrajectoryKind::RepeatingIssue,
        ticks,
        |action| {
            action.issues.first().map(|issue| {
                format!(
                    "{}::{}",
                    issue.code,
                    issue.item_id.as_deref().unwrap_or("-")
                )
            })
        },
    );
    push_streak(
        &mut signatures,
        runtime_id,
        TrajectoryKind::BudgetExhaustion,
        ticks,
        |action| {
            matches!(action.outcome, RuntimeActionOutcome::Succeeded if action.metrics.budget_exhausted)
                .then(|| "budget_exhausted".to_string())
        },
    );
    signatures
}

/// Record the longest trailing streak of one signature, when it crosses
/// the threshold: the parent steers on what is wrong NOW, so only a
/// streak still live at the newest tick becomes an exception.
fn push_streak(
    signatures: &mut Vec<TrajectorySignature>,
    runtime_id: &str,
    kind: TrajectoryKind,
    ticks: &[&RuntimeActionRecord],
    signature_of: impl Fn(&RuntimeActionRecord) -> Option<String>,
) {
    let mut streak: Vec<(&RuntimeActionRecord, String)> = Vec::new();
    for action in ticks.iter().rev() {
        match signature_of(action) {
            Some(signature) => {
                if let Some((_, current)) = streak.first() {
                    if current != &signature {
                        break;
                    }
                }
                streak.push((action, signature));
            }
            None => break,
        }
    }
    if streak.len() >= SIGNATURE_THRESHOLD {
        let signature = streak[0].1.clone();
        let mut action_ids: Vec<String> = streak
            .iter()
            .map(|(action, _)| action.action_id.clone())
            .collect();
        action_ids.reverse();
        signatures.push(TrajectorySignature {
            runtime_id: runtime_id.to_string(),
            kind,
            signature,
            consecutive_ticks: action_ids.len(),
            action_ids,
        });
    }
}
