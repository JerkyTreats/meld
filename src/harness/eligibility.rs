//! Eligibility walk: why a record does not exist (DBG-016).
//!
//! Owner: harness. The provenance walk answers why a record exists; this
//! walk answers the other half — what would have to exist for a quiet
//! actor to commit the absent record. It is a substrate derivation over
//! the waiting-on declarations the actors already persisted on their
//! durable action records, never consumer-side logic, so independent
//! consumers cannot diverge on the answer.
//!
//! The chain follows the flywheel couplings upstream: an absent task
//! completion resolves through dispatch to direct Task admission. Belief and
//! evidence questions remain directly queryable without inventing a producer.

use std::collections::BTreeSet;

use meld_events::DomainObjectRef;
use meld_world_model::world_state::graph::store::TraversalStore;
use serde::{Deserialize, Serialize};

use meld_execution::waiting::conditions as execution_conditions;
use meld_world_model::waiting::conditions as world_model_conditions;

use crate::harness::boot::HarnessError;
use crate::runtime::contracts::WaitingOnDeclaration;
use crate::runtime::supervisor::SupervisorReportStore;

/// Bound on chain links so a malformed vocabulary can never loop.
const MAX_CHAIN_LINKS: usize = 32;

/// The kind of record whose absence the walk explains.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AbsentRecordKind {
    /// A terminal task outcome from the dispatch route.
    TaskCompletion,
    /// A committed operational region from direct Task admission.
    TaskAdmission,
    /// A committed belief revision from assessment.
    BeliefRevision,
    /// Promoted evidence from ingestion.
    Evidence,
}

/// One absence question: a record kind, optionally about one subject.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EligibilityQuestion {
    /// Record kind that does not exist.
    pub kind: AbsentRecordKind,
    /// Exact subject index key the absence is about, when known.
    pub subject_key: Option<String>,
}

/// One recorded declaration the chain passed through.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EligibilityLink {
    /// Runtime whose tick recorded the declaration.
    pub runtime_id: String,
    /// Durable action record carrying the declaration.
    pub action_id: String,
    /// Observation time of that tick.
    pub observed_at_ms: u64,
    /// The declaration itself, in the emitting domain's vocabulary.
    pub declaration: WaitingOnDeclaration,
}

/// One resolved absence: the declaration chain and its divergences.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EligibilityChain {
    /// The absence the walk resolved.
    pub question: EligibilityQuestion,
    /// Declarations passed through, upstream order.
    pub links: Vec<EligibilityLink>,
    /// Nameable divergences the chain dead-ends at. Every chain ends in
    /// at least one entry: a divergence, a coupling that recorded no
    /// declaration, or an actor that never ran.
    pub divergences: Vec<String>,
}

/// Reader-side eligibility derivation over one run's preserved reports.
pub struct EligibilityWalker<'a> {
    reports: &'a SupervisorReportStore,
    traversal: Option<&'a TraversalStore>,
    /// Sequence floor fencing reads to one session; zero reads everything.
    floor: u64,
}

impl<'a> EligibilityWalker<'a> {
    /// Bind the walker to a run's preserved per-tick reports.
    pub fn new(reports: &'a SupervisorReportStore) -> Self {
        Self {
            reports,
            traversal: None,
            floor: 0,
        }
    }

    /// Fence reads to actions at or after a session's sequence floor, so
    /// a reused root's earlier boots cannot answer for this session.
    pub fn with_floor(mut self, floor: u64) -> Self {
        self.floor = floor;
        self
    }

    /// Deepen anchor divergences through the graph read surface.
    pub fn with_traversal(mut self, traversal: &'a TraversalStore) -> Self {
        self.traversal = Some(traversal);
        self
    }

    /// Resolve one absence to its declaration chain.
    ///
    /// From the producing actor's latest preserved tick, collect the
    /// declarations relevant to the subject, then follow the coupling
    /// vocabulary upstream until a declaration terminates the chain. A
    /// coupling visited twice or an exhausted link budget ends the walk
    /// truthfully rather than looping.
    pub fn why_absent(
        &self,
        question: EligibilityQuestion,
    ) -> Result<EligibilityChain, HarnessError> {
        let mut chain = EligibilityChain {
            question: question.clone(),
            links: Vec::new(),
            divergences: Vec::new(),
        };
        let mut visited: BTreeSet<AbsentRecordKind> = BTreeSet::new();
        let mut current = Some(question.kind);

        while let Some(kind) = current {
            if !visited.insert(kind) {
                chain
                    .divergences
                    .push(format!("coupling cycle at {kind:?}"));
                break;
            }
            if chain.links.len() >= MAX_CHAIN_LINKS {
                chain
                    .divergences
                    .push("declaration chain exceeded its link bound".to_string());
                break;
            }
            let Some(runtime_id) = producer_runtime_id(kind) else {
                chain
                    .divergences
                    .push("no producer runtime exists for this record kind".to_string());
                break;
            };
            let action = self
                .reports
                .latest_action_for_runtime_since(self.floor, runtime_id)
                .map_err(|error| HarnessError::Storage(error.to_string()))?;
            let Some(action) = action else {
                chain.divergences.push(format!(
                    "no preserved tick for {runtime_id}; the producing actor never ran"
                ));
                break;
            };
            let relevant = relevant_declarations(&action.waiting_on, &question.subject_key);
            if relevant.is_empty() {
                // Two different truths: a tick with no declarations found
                // eligible work; a tick whose declarations all concern
                // other subjects is quiet only about this subject.
                if action.waiting_on.is_empty() {
                    chain.divergences.push(format!(
                        "{runtime_id} recorded no waiting-on declaration on its last tick; \
                         its selector found eligible work or committed"
                    ));
                } else {
                    chain.divergences.push(format!(
                        "{runtime_id} recorded {} waiting-on declarations on its last tick, \
                         none concerning the asked subject",
                        action.waiting_on.len()
                    ));
                }
                break;
            }

            let mut next = None;
            for declaration in relevant {
                chain.links.push(EligibilityLink {
                    runtime_id: runtime_id.to_string(),
                    action_id: action.action_id.clone(),
                    observed_at_ms: action.observed_at_ms,
                    declaration: declaration.clone(),
                });
                match next_question_kind(&declaration.condition) {
                    // The first upstream coupling wins; declarations are in
                    // domain emission order, so this is deterministic.
                    Some(kind) if next.is_none() => next = Some(kind),
                    Some(_) => {}
                    None => chain.divergences.push(self.divergence_for(declaration)?),
                }
            }
            current = next;
        }

        Ok(chain)
    }

    /// Render one terminal declaration as a nameable divergence.
    ///
    /// The anchor condition deepens through the graph read surface when it
    /// is available: the survey's stall is not just a missing perspective
    /// but a subject key the anchor vocabulary has never contained.
    fn divergence_for(&self, declaration: &WaitingOnDeclaration) -> Result<String, HarnessError> {
        if declaration.condition != world_model_conditions::GRAPH_ANCHOR_ABSENT {
            return Ok(format!("{}: {}", declaration.condition, declaration.detail));
        }
        let Some((traversal, subject_key)) = self.traversal.zip(declaration.subject_key.as_deref())
        else {
            return Ok(format!("{}: {}", declaration.condition, declaration.detail));
        };
        let Some(subject) = parse_index_key(subject_key) else {
            return Ok(format!("{}: {}", declaration.condition, declaration.detail));
        };
        let anywhere = traversal
            .current_anchors_for_subject(&subject)
            .map_err(|error| HarnessError::Storage(error.to_string()))?;
        if anywhere.is_empty() {
            Ok(format!(
                "graph_anchor_absent: {}; the subject key {subject_key} appears in no anchor \
                 record — the selection subject vocabulary does not intersect the anchor \
                 vocabulary",
                declaration.detail
            ))
        } else {
            Ok(format!(
                "graph_anchor_absent: {}; {} anchors exist for {subject_key} under other \
                 perspectives",
                declaration.detail,
                anywhere.len()
            ))
        }
    }
}

/// The runtime whose commits would make the absent record exist, when one
/// exists in the current compiled runtime.
fn producer_runtime_id(kind: AbsentRecordKind) -> Option<&'static str> {
    match kind {
        AbsentRecordKind::TaskCompletion => Some("execution.task_dispatch"),
        AbsentRecordKind::TaskAdmission => Some("execution.task_admission"),
        AbsentRecordKind::BeliefRevision => Some("world_model.belief_assessment"),
        AbsentRecordKind::Evidence => Some("world_model.evidence_ingestion"),
    }
}

/// The upstream absence one waiting-on condition resolves to, when the
/// coupling vocabulary names one; `None` terminates the chain.
fn next_question_kind(condition: &str) -> Option<AbsentRecordKind> {
    // The vocabulary is compiled from the emitting domains' frozen
    // constants, so a domain rename breaks this match at build time
    // instead of silently changing the substrate's answer.
    if condition == execution_conditions::NO_READY_TASKS
        || condition == execution_conditions::UPSTREAM_ARTIFACT_UNAVAILABLE
    {
        Some(AbsentRecordKind::TaskAdmission)
    } else if condition == world_model_conditions::NO_UNDELIVERED_REVISIONS
        || condition == world_model_conditions::NO_PENDING_SATISFACTION_REVIEWS
    {
        Some(AbsentRecordKind::BeliefRevision)
    } else if condition == world_model_conditions::BELIEF_WORK_INELIGIBLE {
        Some(AbsentRecordKind::Evidence)
    } else {
        None
    }
}

/// Declarations relevant to one subject: exact matches plus broad ones.
fn relevant_declarations<'d>(
    declarations: &'d [WaitingOnDeclaration],
    subject_key: &Option<String>,
) -> Vec<&'d WaitingOnDeclaration> {
    declarations
        .iter()
        .filter(
            |declaration| match (&declaration.subject_key, subject_key) {
                (Some(declared), Some(asked)) => declared == asked,
                _ => true,
            },
        )
        .collect()
}

/// Parse a `domain::kind::id` index key back into a domain object ref.
fn parse_index_key(index_key: &str) -> Option<DomainObjectRef> {
    let mut parts = index_key.splitn(3, "::");
    let domain_id = parts.next()?;
    let object_kind = parts.next()?;
    let object_id = parts.next()?;
    DomainObjectRef::new(domain_id, object_kind, object_id).ok()
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use crate::runtime::contracts::{
        RuntimeActionRecord, RuntimeStatusPublisher, WorkerCheckpoint, WorkerScope,
        WorkerTickReport,
    };
    use crate::runtime::supervisor::SupervisorStore;

    use super::*;

    fn report_with(actor_id: &str, waiting_on: Vec<WaitingOnDeclaration>) -> WorkerTickReport {
        WorkerTickReport {
            actor_id: actor_id.to_string(),
            scope: WorkerScope {
                domain_id: actor_id.split('.').next().unwrap_or("test").to_string(),
                stream_id: None,
                work_key: None,
                agent_id: None,
                perspective_key: None,
                branch_id: None,
                subject_key: None,
            },
            input_checkpoint: WorkerCheckpoint {
                name: "checkpoint".to_string(),
                value: 0,
            },
            output_checkpoint: WorkerCheckpoint {
                name: "checkpoint".to_string(),
                value: 0,
            },
            items_attempted: 0,
            items_committed: 0,
            retryable_errors: Vec::new(),
            fatal_errors: Vec::new(),
            budget_exhausted: false,
            waiting_on,
        }
    }

    fn store_with_flywheel_stall() -> (TempDir, SupervisorReportStore) {
        let temp = tempfile::tempdir().unwrap();
        let store = SupervisorStore::open(temp.path().join("supervisor.sled")).unwrap();
        let mut reports = SupervisorReportStore::open(&store).unwrap();
        let publish = |reports: &mut SupervisorReportStore,
                       runtime_id: &str,
                       waiting_on: Vec<WaitingOnDeclaration>| {
            let action = RuntimeActionRecord::from_worker_tick(
                format!("action:{runtime_id}"),
                runtime_id,
                10,
                report_with(runtime_id, waiting_on),
            );
            reports.publish_action(&action).unwrap();
        };
        publish(
            &mut reports,
            "execution.task_dispatch",
            vec![WaitingOnDeclaration {
                condition: "no_ready_tasks".to_string(),
                subject_key: None,
                detail: "no claimable task at network revision 0".to_string(),
                wake_refs: Vec::new(),
            }],
        );
        publish(
            &mut reports,
            "execution.task_admission",
            vec![WaitingOnDeclaration {
                condition: "task_admission_available".to_string(),
                subject_key: None,
                detail: "no admitted unlowered Task at network revision 0".to_string(),
                wake_refs: Vec::new(),
            }],
        );
        publish(
            &mut reports,
            "world_model.agent_reconciliation",
            vec![WaitingOnDeclaration {
                condition: "future_execution_admission".to_string(),
                subject_key: Some("workspace_fs::node::docs".to_string()),
                detail: "eligible task remains unpublished until Execution admission exists"
                    .to_string(),
                wake_refs: Vec::new(),
            }],
        );
        publish(
            &mut reports,
            "world_model.belief_assessment",
            vec![WaitingOnDeclaration {
                condition: "graph_anchor_absent".to_string(),
                subject_key: Some("workspace_fs::node::docs".to_string()),
                detail: "no current anchor for subject workspace_fs::node::docs under \
                         frame_type::analysis"
                    .to_string(),
                wake_refs: Vec::new(),
            }],
        );
        (temp, reports)
    }

    #[test]
    fn an_absent_task_completion_stops_at_direct_task_admission() {
        let (_temp, reports) = store_with_flywheel_stall();
        let chain = EligibilityWalker::new(&reports)
            .why_absent(EligibilityQuestion {
                kind: AbsentRecordKind::TaskCompletion,
                subject_key: Some("workspace_fs::node::docs".to_string()),
            })
            .unwrap();

        let runtimes: Vec<&str> = chain
            .links
            .iter()
            .map(|link| link.runtime_id.as_str())
            .collect();
        assert_eq!(
            runtimes,
            vec!["execution.task_dispatch", "execution.task_admission"]
        );
        assert_eq!(chain.divergences.len(), 1);
        assert_eq!(
            chain.divergences[0],
            "task_admission_available: no admitted unlowered Task at network revision 0"
        );
    }

    #[test]
    fn direct_task_admission_is_a_real_producer_position() {
        let (_temp, reports) = store_with_flywheel_stall();
        let chain = EligibilityWalker::new(&reports)
            .why_absent(EligibilityQuestion {
                kind: AbsentRecordKind::TaskAdmission,
                subject_key: Some("workspace_fs::node::docs".to_string()),
            })
            .unwrap();

        assert_eq!(chain.links.len(), 1);
        assert_eq!(
            chain.divergences,
            vec!["task_admission_available: no admitted unlowered Task at network revision 0"]
        );
    }

    #[test]
    fn an_actor_that_never_ran_is_a_truthful_dead_end() {
        let temp = tempfile::tempdir().unwrap();
        let store = SupervisorStore::open(temp.path().join("supervisor.sled")).unwrap();
        let reports = SupervisorReportStore::open(&store).unwrap();
        let chain = EligibilityWalker::new(&reports)
            .why_absent(EligibilityQuestion {
                kind: AbsentRecordKind::BeliefRevision,
                subject_key: None,
            })
            .unwrap();
        assert!(chain.links.is_empty());
        assert_eq!(chain.divergences.len(), 1);
        assert!(chain.divergences[0].contains("never ran"));
    }

    #[test]
    fn a_quiet_declaration_free_tick_terminates_the_chain() {
        let temp = tempfile::tempdir().unwrap();
        let store = SupervisorStore::open(temp.path().join("supervisor.sled")).unwrap();
        let mut reports = SupervisorReportStore::open(&store).unwrap();
        let action = RuntimeActionRecord::from_worker_tick(
            "action:task-admission",
            "execution.task_admission",
            10,
            report_with("execution.task_admission", Vec::new()),
        );
        reports.publish_action(&action).unwrap();

        let chain = EligibilityWalker::new(&reports)
            .why_absent(EligibilityQuestion {
                kind: AbsentRecordKind::TaskAdmission,
                subject_key: None,
            })
            .unwrap();
        assert!(chain.links.is_empty());
        assert!(chain.divergences[0].contains("no waiting-on declaration"));
    }

    #[test]
    fn re_deriving_a_chain_yields_the_identical_chain() {
        let (_temp, reports) = store_with_flywheel_stall();
        let question = EligibilityQuestion {
            kind: AbsentRecordKind::TaskCompletion,
            subject_key: Some("workspace_fs::node::docs".to_string()),
        };
        let first = EligibilityWalker::new(&reports)
            .why_absent(question.clone())
            .unwrap();
        let second = EligibilityWalker::new(&reports)
            .why_absent(question)
            .unwrap();
        assert_eq!(first, second);
    }
}
