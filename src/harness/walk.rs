//! Causal thread walk: any record identity resolves to its transitive
//! provenance thread across domain boundaries (DBG-015).
//!
//! Owner: harness. The walk is derivation, not storage: it follows the
//! typed reference fields the domains already persist — event provenance,
//! traversal fact sources, anchor citations, evidence hydration, revision
//! lineage, decision input references, sink receipts, and task lineage —
//! through public domain read surfaces, and materializes nothing. Deleting
//! a walk result and re-deriving it yields an identical thread, so the
//! walk can never become a second source of truth.
//!
//! The walk goes upstream only ("why does this record exist"); the
//! downstream complement ("why does a record NOT exist") is the
//! eligibility walk over waiting-on declarations, delivered by phase two
//! of the runtime harness plan. A reference that does not resolve is
//! recorded truthfully as a cut — the absent anchor of the runtime survey
//! stall surfaces exactly there.

use std::collections::{BTreeMap, VecDeque};

use meld_events::DomainObjectRef;
use meld_execution::task_network::store::{
    network_storage_key, SledTaskNetworkStore, TaskNetworkStoreFactory,
};
use meld_execution::task_network::TaskInitSource;
use meld_world_model::agent::AgentStore;
use meld_world_model::belief::BeliefStore;
use meld_world_model::world_state::graph::store::TraversalStore;
use serde::{Deserialize, Serialize};

use crate::harness::boot::HarnessError;
use crate::runtime::assembly::ProductRuntimeAssembly;
use crate::runtime::ports::ProductEventReplayPort;

/// Default node bound for one walk; a thread that exceeds it reports
/// `bounded: true` rather than reading without limit.
pub const DEFAULT_MAX_THREAD_NODES: usize = 256;

/// One record identity the walk can start from or arrive at.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ThreadSubject {
    /// Canonical ledger record by sequence.
    Event { seq: u64 },
    /// Traversal fact projected from the ledger.
    Fact { fact_id: String },
    /// Graph anchor selection record.
    Anchor { anchor_id: String },
    /// The current anchor selection for a subject under one perspective —
    /// the record whose absence the runtime survey's anchor stall is
    /// about. Resolving it names the exact subject key either way.
    AnchorForSubject {
        subject_domain_id: String,
        subject_object_kind: String,
        subject_object_id: String,
        perspective_kind: String,
        perspective_id: String,
    },
    /// Evidence item admitted into a belief key.
    Evidence { evidence_id: String },
    /// Committed belief revision.
    BeliefRevision { revision_id: String },
    /// Agent curation decision.
    Decision { decision_id: String },
    /// Execution goal record.
    Goal { goal_id: String },
    /// Task node inside one committed task network.
    Task {
        network_id: String,
        task_instance_id: String,
    },
}

/// The typed reference one hop followed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThreadCitation {
    /// Event → event through structural source-record provenance.
    SourceRecord,
    /// Fact → the ledger event it was reduced from.
    FactSource,
    /// Fact → the spine fact it derives from.
    SpineFact,
    /// Anchor → the fact that created the selection.
    CreatedByFact,
    /// Anchor → its source facts.
    AnchorSourceFact,
    /// Anchor-for-subject → the concrete anchor selection it resolves to.
    CurrentAnchor,
    /// Evidence → the traversal facts it cites.
    EvidenceSourceFact,
    /// Evidence → the graph anchors it cites.
    EvidenceAnchor,
    /// Revision → the evidence items it settled over.
    RevisionEvidence,
    /// Revision → its prior revision.
    PriorRevision,
    /// Decision → the belief revision it consumed.
    DecisionInput,
    /// Revision → the installed theory revision that produced it. Always a
    /// cut today: the composition exposes no theory-registry read surface
    /// yet, and recording the reference as out of scope keeps threads
    /// stable when that surface lands.
    TheoryRevision,
    /// Goal → the decision whose command produced it, through the sink
    /// receipt recorded for the goal's source command.
    CommandReceipt,
    /// Task → the goal in its lineage.
    TaskGoal,
    /// Task → the upstream task an init slot selects an artifact from.
    UpstreamArtifact,
}

/// Why one reference could not be followed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThreadCutReason {
    /// The referenced record does not exist in its durable store.
    AbsentRecord,
    /// The store that owns the reference is not open in this composition.
    SourceOutOfScope,
}

/// One node of a resolved thread.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThreadNode {
    /// Record identity; the durable record is the truth this points at.
    pub subject: ThreadSubject,
    /// Short derived description in the owning domain's vocabulary.
    pub summary: String,
}

/// One resolved provenance hop between two thread nodes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThreadEdge {
    /// Index of the citing node in [`CausalThread::nodes`].
    pub from: usize,
    /// Index of the cited node in [`CausalThread::nodes`].
    pub to: usize,
    /// The reference field the hop followed.
    pub citation: ThreadCitation,
}

/// One truthfully recorded dead end.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThreadCut {
    /// Node whose expansion hit the dead end.
    pub at: usize,
    /// The reference that could not be followed, in display form.
    pub reference: String,
    /// The citation the reference would have carried.
    pub citation: ThreadCitation,
    /// Why the reference did not resolve.
    pub reason: ThreadCutReason,
}

/// One resolved causal thread.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CausalThread {
    /// The identity the walk started from.
    pub subject: ThreadSubject,
    /// Resolved records, subject first, in deterministic discovery order.
    pub nodes: Vec<ThreadNode>,
    /// Provenance hops between nodes.
    pub edges: Vec<ThreadEdge>,
    /// References that could not be followed.
    pub cuts: Vec<ThreadCut>,
    /// True when the walk stopped at its node bound before exhaustion.
    pub bounded: bool,
}

/// Reader-side thread derivation over one composition's read surfaces.
///
/// Every source is optional: a walk over a narrowed registration scope
/// records references into unopened stores as out-of-scope cuts instead of
/// failing or silently dropping them.
pub struct ThreadWalker<'a> {
    replay: Option<&'a ProductEventReplayPort>,
    belief: Option<&'a BeliefStore>,
    agent: Option<&'a AgentStore>,
    traversal: Option<&'a TraversalStore>,
    task_networks: Option<&'a TaskNetworkStoreFactory>,
    max_nodes: usize,
}

impl<'a> ThreadWalker<'a> {
    /// Bind the walker to whatever read surfaces an assembly opened.
    pub fn over_assembly(assembly: &'a ProductRuntimeAssembly) -> Self {
        let stores = assembly.stores();
        Self {
            replay: Some(assembly.ports().event_replay()),
            belief: stores.belief_store.opened().map(|store| store.as_ref()),
            agent: stores.agent_store.opened().map(|store| store.as_ref()),
            traversal: stores.traversal_store.opened().map(|store| store.as_ref()),
            task_networks: stores.task_networks.opened(),
            max_nodes: DEFAULT_MAX_THREAD_NODES,
        }
    }

    /// Bind the walker to explicitly owned read surfaces.
    ///
    /// The serving layer holds its own store handles rather than a live
    /// assembly borrow, so the same walk serves live and playback roots.
    #[allow(clippy::too_many_arguments)]
    pub fn from_parts(
        replay: Option<&'a ProductEventReplayPort>,
        belief: Option<&'a BeliefStore>,
        agent: Option<&'a AgentStore>,
        traversal: Option<&'a TraversalStore>,
        task_networks: Option<&'a TaskNetworkStoreFactory>,
    ) -> Self {
        Self {
            replay,
            belief,
            agent,
            traversal,
            task_networks,
            max_nodes: DEFAULT_MAX_THREAD_NODES,
        }
    }

    /// Replace the walk's node bound.
    pub fn with_max_nodes(mut self, max_nodes: usize) -> Self {
        self.max_nodes = max_nodes.max(1);
        self
    }

    /// Resolve one identity to its transitive provenance thread.
    ///
    /// Breadth-first over the typed references, deduplicated by identity,
    /// bounded by the node limit. Store read failures surface as errors;
    /// absent records and unopened stores are recorded as cuts because
    /// absence is an answer, not a failure.
    pub fn walk(&self, subject: ThreadSubject) -> Result<CausalThread, HarnessError> {
        let mut thread = CausalThread {
            subject: subject.clone(),
            nodes: Vec::new(),
            edges: Vec::new(),
            cuts: Vec::new(),
            bounded: false,
        };
        let mut index: BTreeMap<ThreadSubject, usize> = BTreeMap::new();
        let mut queue: VecDeque<usize> = VecDeque::new();

        match self.resolve(&subject)? {
            Resolution::Present(summary) => {
                thread.nodes.push(ThreadNode {
                    subject: subject.clone(),
                    summary,
                });
                queue.push_back(0);
            }
            Resolution::Missing { reference, reason } => {
                // The root subject keeps its node so the thread has an
                // anchor point, with the unresolvability recorded on it;
                // an unresolved root is never expanded.
                thread.nodes.push(ThreadNode {
                    subject: subject.clone(),
                    summary: "unresolved".to_string(),
                });
                thread.cuts.push(ThreadCut {
                    at: 0,
                    reference: reference.unwrap_or_else(|| display_subject(&subject)),
                    citation: owning_citation(&subject),
                    reason,
                });
            }
        }
        index.insert(subject, 0);

        while let Some(at) = queue.pop_front() {
            let node_subject = thread.nodes[at].subject.clone();
            for reference in self.expand(&node_subject)? {
                match reference {
                    Reference::Hop { subject, citation } => {
                        let to = match index.get(&subject) {
                            Some(existing) => *existing,
                            None => {
                                if thread.nodes.len() >= self.max_nodes {
                                    thread.bounded = true;
                                    continue;
                                }
                                let summary = match self.resolve(&subject)? {
                                    Resolution::Present(summary) => summary,
                                    Resolution::Missing { reference, reason } => {
                                        thread.cuts.push(ThreadCut {
                                            at,
                                            reference: reference
                                                .unwrap_or_else(|| display_subject(&subject)),
                                            citation,
                                            reason,
                                        });
                                        continue;
                                    }
                                };
                                let next = thread.nodes.len();
                                thread.nodes.push(ThreadNode {
                                    subject: subject.clone(),
                                    summary,
                                });
                                index.insert(subject, next);
                                queue.push_back(next);
                                next
                            }
                        };
                        thread.edges.push(ThreadEdge {
                            from: at,
                            to,
                            citation,
                        });
                    }
                    Reference::Cut {
                        reference,
                        citation,
                        reason,
                    } => thread.cuts.push(ThreadCut {
                        at,
                        reference,
                        citation,
                        reason,
                    }),
                }
            }
        }

        Ok(thread)
    }

    /// Resolve one identity to its display summary, absence, or an
    /// out-of-scope store.
    fn resolve(&self, subject: &ThreadSubject) -> Result<Resolution, HarnessError> {
        match subject {
            ThreadSubject::Event { seq } => {
                let Some(replay) = self.replay else {
                    return Ok(Resolution::out_of_scope(None));
                };
                let records = replay
                    .read_after_limit(seq.saturating_sub(1), 1)
                    .map_err(|error| HarnessError::Storage(error.to_string()))?;
                Ok(Resolution::from(
                    records
                        .into_iter()
                        .find(|record| record.seq == *seq)
                        .map(|record| {
                            format!(
                                "{} on {}::{}",
                                record.event_type, record.domain_id, record.stream_id
                            )
                        }),
                ))
            }
            ThreadSubject::Fact { fact_id } => {
                let Some(traversal) = self.traversal else {
                    return Ok(Resolution::out_of_scope(None));
                };
                Ok(Resolution::from(
                    traversal
                        .get_fact(fact_id)
                        .map_err(storage_error)?
                        .map(|fact| format!("fact {} at seq {}", fact.event_type, fact.seq)),
                ))
            }
            ThreadSubject::Anchor { anchor_id } => {
                let Some(traversal) = self.traversal else {
                    return Ok(Resolution::out_of_scope(None));
                };
                Ok(Resolution::from(
                    traversal
                        .get_anchor(anchor_id)
                        .map_err(storage_error)?
                        .map(|anchor| {
                            format!(
                                "anchor {} for subject {}",
                                anchor.anchor_ref.index_key(),
                                anchor.subject.index_key()
                            )
                        }),
                ))
            }
            ThreadSubject::AnchorForSubject {
                subject_domain_id,
                subject_object_kind,
                subject_object_id,
                perspective_kind,
                perspective_id,
            } => {
                let Some(traversal) = self.traversal else {
                    return Ok(Resolution::out_of_scope(None));
                };
                let subject =
                    DomainObjectRef::new(subject_domain_id, subject_object_kind, subject_object_id)
                        .map_err(storage_error)?;
                let current = traversal
                    .current_anchor_for_subject(&subject, perspective_kind, perspective_id)
                    .map_err(storage_error)?;
                match current {
                    Some(anchor) => Ok(Resolution::Present(format!(
                        "current anchor {} for subject {}",
                        anchor.anchor_id,
                        subject.index_key()
                    ))),
                    None => {
                        // The subject-key dead end from the runtime survey:
                        // when the subject key appears in no anchor record
                        // at all, the absence is a vocabulary mismatch, not
                        // just a missing perspective.
                        let anywhere = traversal
                            .current_anchors_for_subject(&subject)
                            .map_err(storage_error)?;
                        let reference = if anywhere.is_empty() {
                            format!(
                                "current anchor for subject {} under {}::{}; \
                                 the subject key appears in no anchor record",
                                subject.index_key(),
                                perspective_kind,
                                perspective_id
                            )
                        } else {
                            format!(
                                "current anchor for subject {} under {}::{}; \
                                 {} anchors exist under other perspectives",
                                subject.index_key(),
                                perspective_kind,
                                perspective_id,
                                anywhere.len()
                            )
                        };
                        Ok(Resolution::absent(Some(reference)))
                    }
                }
            }
            ThreadSubject::Evidence { evidence_id } => {
                let Some(belief) = self.belief else {
                    return Ok(Resolution::out_of_scope(None));
                };
                Ok(Resolution::from(
                    belief
                        .get_evidence(evidence_id)
                        .map_err(storage_error)?
                        .map(|item| {
                            format!(
                                "evidence {} for key {}",
                                item.evidence_schema_id,
                                item.candidate_key.index_key()
                            )
                        }),
                ))
            }
            ThreadSubject::BeliefRevision { revision_id } => {
                let Some(belief) = self.belief else {
                    return Ok(Resolution::out_of_scope(None));
                };
                Ok(Resolution::from(
                    belief
                        .get_revision(revision_id)
                        .map_err(storage_error)?
                        .map(|revision| {
                            format!(
                                "belief revision for {} (posterior {:.3})",
                                revision.belief_key.index_key(),
                                revision.posterior.probability
                            )
                        }),
                ))
            }
            ThreadSubject::Decision { decision_id } => {
                let Some(agent) = self.agent else {
                    return Ok(Resolution::out_of_scope(None));
                };
                Ok(Resolution::from(
                    agent
                        .legacy_decision(decision_id)
                        .map_err(storage_error)?
                        .map(|decision| {
                            format!("{:?} decision by {}", decision.decision, decision.agent_id)
                        }),
                ))
            }
            ThreadSubject::Goal { goal_id } => Ok(Resolution::out_of_scope(Some(format!(
                "Agent goal {goal_id}"
            )))),
            ThreadSubject::Task {
                network_id,
                task_instance_id,
            } => {
                let Some(factory) = self.task_networks else {
                    return Ok(Resolution::out_of_scope(None));
                };
                match open_existing_network(factory, network_id)? {
                    NetworkOpen::Missing => Ok(Resolution::absent(Some(format!(
                        "task network {network_id}"
                    )))),
                    NetworkOpen::Locked => Ok(Resolution::out_of_scope(Some(format!(
                        "task network {network_id} (locked by a live composition)"
                    )))),
                    NetworkOpen::Open(store) => Ok(Resolution::from(
                        store.state().tasks.get(task_instance_id).map(|task| {
                            format!(
                                "task {} via Capability {}",
                                task.task_instance_id, task.lineage.capability_type_id
                            )
                        }),
                    )),
                }
            }
        }
    }

    /// Enumerate one node's upstream references in deterministic order.
    fn expand(&self, subject: &ThreadSubject) -> Result<Vec<Reference>, HarnessError> {
        let mut refs = Vec::new();
        match subject {
            ThreadSubject::Event { seq } => {
                let Some(replay) = self.replay else {
                    return Ok(refs);
                };
                let records = replay
                    .read_after_limit(seq.saturating_sub(1), 1)
                    .map_err(|error| HarnessError::Storage(error.to_string()))?;
                if let Some(record) = records.into_iter().find(|record| record.seq == *seq) {
                    // Same-ledger walk: the product composes exactly one
                    // ledger, and the replay port rejects foreign cursors.
                    for source in &record.envelope.provenance.source_records {
                        refs.push(Reference::hop(
                            ThreadSubject::Event { seq: source.seq },
                            ThreadCitation::SourceRecord,
                        ));
                    }
                }
            }
            ThreadSubject::Fact { fact_id } => {
                let Some(traversal) = self.traversal else {
                    return Ok(refs);
                };
                if let Some(fact) = traversal.get_fact(fact_id).map_err(storage_error)? {
                    refs.push(Reference::hop(
                        ThreadSubject::Event { seq: fact.seq },
                        ThreadCitation::FactSource,
                    ));
                    if fact.source_spine_fact_id != *fact_id {
                        refs.push(Reference::hop(
                            ThreadSubject::Fact {
                                fact_id: fact.source_spine_fact_id.clone(),
                            },
                            ThreadCitation::SpineFact,
                        ));
                    }
                }
            }
            ThreadSubject::Anchor { anchor_id } => {
                let Some(traversal) = self.traversal else {
                    return Ok(refs);
                };
                if let Some(anchor) = traversal.get_anchor(anchor_id).map_err(storage_error)? {
                    refs.push(Reference::hop(
                        ThreadSubject::Fact {
                            fact_id: anchor.created_by_fact_id.clone(),
                        },
                        ThreadCitation::CreatedByFact,
                    ));
                    for fact_id in &anchor.source_fact_ids {
                        refs.push(Reference::hop(
                            ThreadSubject::Fact {
                                fact_id: fact_id.clone(),
                            },
                            ThreadCitation::AnchorSourceFact,
                        ));
                    }
                }
            }
            ThreadSubject::AnchorForSubject {
                subject_domain_id,
                subject_object_kind,
                subject_object_id,
                perspective_kind,
                perspective_id,
            } => {
                let Some(traversal) = self.traversal else {
                    return Ok(refs);
                };
                let subject =
                    DomainObjectRef::new(subject_domain_id, subject_object_kind, subject_object_id)
                        .map_err(storage_error)?;
                if let Some(anchor) = traversal
                    .current_anchor_for_subject(&subject, perspective_kind, perspective_id)
                    .map_err(storage_error)?
                {
                    refs.push(Reference::hop(
                        ThreadSubject::Anchor {
                            anchor_id: anchor.anchor_id.clone(),
                        },
                        ThreadCitation::CurrentAnchor,
                    ));
                }
            }
            ThreadSubject::Evidence { evidence_id } => {
                let Some(belief) = self.belief else {
                    return Ok(refs);
                };
                if let Some(item) = belief.get_evidence(evidence_id).map_err(storage_error)? {
                    for fact_id in &item.source_fact_ids {
                        refs.push(Reference::hop(
                            ThreadSubject::Fact {
                                fact_id: fact_id.clone(),
                            },
                            ThreadCitation::EvidenceSourceFact,
                        ));
                    }
                    for anchor_id in &item.graph_anchor_ids {
                        refs.push(Reference::hop(
                            ThreadSubject::Anchor {
                                anchor_id: anchor_id.clone(),
                            },
                            ThreadCitation::EvidenceAnchor,
                        ));
                    }
                }
            }
            ThreadSubject::BeliefRevision { revision_id } => {
                let Some(belief) = self.belief else {
                    return Ok(refs);
                };
                if let Some(revision) = belief.get_revision(revision_id).map_err(storage_error)? {
                    for evidence_id in &revision.evidence_ids {
                        refs.push(Reference::hop(
                            ThreadSubject::Evidence {
                                evidence_id: evidence_id.clone(),
                            },
                            ThreadCitation::RevisionEvidence,
                        ));
                    }
                    if let Some(prior) = &revision.prior_revision_id {
                        refs.push(Reference::hop(
                            ThreadSubject::BeliefRevision {
                                revision_id: prior.clone(),
                            },
                            ThreadCitation::PriorRevision,
                        ));
                    }
                    // The theory citation is real but the composition has
                    // no registry read surface yet, so the reference is
                    // recorded rather than silently dropped.
                    if let Some(theory) = &revision.theory_revision {
                        refs.push(Reference::Cut {
                            reference: format!(
                                "theory revision {}::{}::{}",
                                theory.registry, theory.id, theory.content_hash
                            ),
                            citation: ThreadCitation::TheoryRevision,
                            reason: ThreadCutReason::SourceOutOfScope,
                        });
                    }
                }
            }
            ThreadSubject::Decision { decision_id } => {
                let Some(agent) = self.agent else {
                    return Ok(refs);
                };
                if let Some(decision) = agent.legacy_decision(decision_id).map_err(storage_error)? {
                    if let Some(revision_id) = &decision.input_refs.belief_revision_id {
                        refs.push(Reference::hop(
                            ThreadSubject::BeliefRevision {
                                revision_id: revision_id.clone(),
                            },
                            ThreadCitation::DecisionInput,
                        ));
                    }
                }
            }
            ThreadSubject::Goal { .. } => {}
            ThreadSubject::Task {
                network_id,
                task_instance_id,
            } => {
                let Some(factory) = self.task_networks else {
                    return Ok(refs);
                };
                // Expansion runs only on resolved nodes, but the network
                // lock can be lost between resolve and expand; both
                // unreadable arms degrade to nothing rather than failing
                // the whole thread.
                let store = match open_existing_network(factory, network_id)? {
                    NetworkOpen::Open(store) => store,
                    NetworkOpen::Missing | NetworkOpen::Locked => return Ok(refs),
                };
                if let Some(task) = store.state().tasks.get(task_instance_id) {
                    if let Some(admission) = &task.lineage.admission {
                        refs.push(Reference::hop(
                            ThreadSubject::Goal {
                                goal_id: admission.goal_id.clone(),
                            },
                            ThreadCitation::TaskGoal,
                        ));
                    }
                    for source in &task.init_sources {
                        if let TaskInitSource::UpstreamArtifact(upstream) = source {
                            refs.push(Reference::hop(
                                ThreadSubject::Task {
                                    network_id: network_id.clone(),
                                    task_instance_id: upstream.upstream_task_instance_id.clone(),
                                },
                                ThreadCitation::UpstreamArtifact,
                            ));
                        }
                    }
                }
            }
        }
        Ok(refs)
    }
}

/// Outcome of resolving one identity against its owning read surface.
enum Resolution {
    /// The record exists; its derived display summary.
    Present(String),
    /// The record could not be resolved: absent from its open store, or
    /// its store is unreadable in this composition. The optional reference
    /// overrides the subject's display form when the unresolvable thing is
    /// narrower than the subject, such as a whole task network.
    Missing {
        reference: Option<String>,
        reason: ThreadCutReason,
    },
}

impl Resolution {
    fn absent(reference: Option<String>) -> Self {
        Resolution::Missing {
            reference,
            reason: ThreadCutReason::AbsentRecord,
        }
    }

    fn out_of_scope(reference: Option<String>) -> Self {
        Resolution::Missing {
            reference,
            reason: ThreadCutReason::SourceOutOfScope,
        }
    }
}

impl From<Option<String>> for Resolution {
    fn from(summary: Option<String>) -> Self {
        match summary {
            Some(summary) => Resolution::Present(summary),
            None => Resolution::absent(None),
        }
    }
}

/// One expansion result: a followable hop or a truthful dead end.
enum Reference {
    Hop {
        subject: ThreadSubject,
        citation: ThreadCitation,
    },
    Cut {
        reference: String,
        citation: ThreadCitation,
        reason: ThreadCutReason,
    },
}

impl Reference {
    fn hop(subject: ThreadSubject, citation: ThreadCitation) -> Self {
        Reference::Hop { subject, citation }
    }
}

/// One network open attempt through the read-only discipline.
enum NetworkOpen {
    /// The network database exists and opened.
    Open(Box<SledTaskNetworkStore>),
    /// No database exists for the network id.
    Missing,
    /// The database exists but another handle holds its exclusive lock,
    /// typically the live composition's own network handle.
    Locked,
}

/// Open a task network only when its database already exists.
///
/// The factory's `open_network` creates the database when missing, which a
/// read-only walk must never do; the existence check keeps the walk free
/// of durable side effects and keeps an absent network distinguishable
/// from an absent task inside an existing network.
fn open_existing_network(
    factory: &TaskNetworkStoreFactory,
    network_id: &str,
) -> Result<NetworkOpen, HarnessError> {
    let storage_key = network_storage_key(network_id)
        .map_err(|error| HarnessError::Storage(error.to_string()))?;
    if !factory.root().join(format!("{storage_key}.sled")).exists() {
        return Ok(NetworkOpen::Missing);
    }
    match factory.open_network(network_id) {
        Ok(store) => Ok(NetworkOpen::Open(Box::new(store))),
        Err(error) => {
            let message = error.to_string();
            if message.contains("could not acquire lock") {
                Ok(NetworkOpen::Locked)
            } else {
                Err(HarnessError::Storage(message))
            }
        }
    }
}

/// The citation an unexpandable subject's own store would have carried.
fn owning_citation(subject: &ThreadSubject) -> ThreadCitation {
    match subject {
        ThreadSubject::Event { .. } => ThreadCitation::SourceRecord,
        ThreadSubject::Fact { .. } => ThreadCitation::FactSource,
        ThreadSubject::Anchor { .. } => ThreadCitation::CreatedByFact,
        ThreadSubject::AnchorForSubject { .. } => ThreadCitation::CurrentAnchor,
        ThreadSubject::Evidence { .. } => ThreadCitation::EvidenceSourceFact,
        ThreadSubject::BeliefRevision { .. } => ThreadCitation::RevisionEvidence,
        ThreadSubject::Decision { .. } => ThreadCitation::DecisionInput,
        ThreadSubject::Goal { .. } => ThreadCitation::CommandReceipt,
        ThreadSubject::Task { .. } => ThreadCitation::TaskGoal,
    }
}

/// Display form of an identity for cut records.
fn display_subject(subject: &ThreadSubject) -> String {
    match subject {
        ThreadSubject::Event { seq } => format!("event seq {seq}"),
        ThreadSubject::Fact { fact_id } => format!("fact {fact_id}"),
        ThreadSubject::Anchor { anchor_id } => format!("anchor {anchor_id}"),
        ThreadSubject::AnchorForSubject {
            subject_domain_id,
            subject_object_kind,
            subject_object_id,
            perspective_kind,
            perspective_id,
        } => format!(
            "current anchor for {subject_domain_id}::{subject_object_kind}::\
             {subject_object_id} under {perspective_kind}::{perspective_id}"
        ),
        ThreadSubject::Evidence { evidence_id } => format!("evidence {evidence_id}"),
        ThreadSubject::BeliefRevision { revision_id } => {
            format!("belief revision {revision_id}")
        }
        ThreadSubject::Decision { decision_id } => format!("decision {decision_id}"),
        ThreadSubject::Goal { goal_id } => format!("goal {goal_id}"),
        ThreadSubject::Task {
            network_id,
            task_instance_id,
        } => format!("task {task_instance_id} in network {network_id}"),
    }
}

fn storage_error(error: meld_world_model::error::StorageError) -> HarnessError {
    HarnessError::Storage(error.to_string())
}

#[cfg(test)]
mod tests {
    use meld_world_model::agent::AgentStore;
    use meld_world_model::belief::{
        AssessmentLease, BeliefKey, BeliefProvenanceSummary, BeliefRevision, BeliefStatus,
        BeliefStore, BranchScope, ContradictionState, EvidenceItem, EvidenceRole, EvidenceValue,
        FreshnessState, LeaseStatus, PlannerProjectionSummary, PosteriorSummary, TheoryRevisionRef,
    };
    use meld_world_model::{AnchorSelectionRecord, PerspectiveKey, TraversalFactRecord};

    use super::*;
    use meld_events::DomainObjectRef;

    const REVISION_ID: &str = "revision-1";
    const EVIDENCE_ID: &str = "evidence-1";
    const FACT_ID: &str = "fact-1";
    const ANCHOR_ID: &str = "anchor-1";
    const MISSING_ANCHOR_ID: &str = "anchor-missing";
    const DECISION_ID: &str = "decision-1";

    struct ChainWorld {
        _dir: tempfile::TempDir,
        traversal: TraversalStore,
        belief: BeliefStore,
        agent: AgentStore,
    }

    fn subject() -> DomainObjectRef {
        DomainObjectRef::new("workspace_fs", "node", "node-a").unwrap()
    }

    fn belief_key() -> BeliefKey {
        BeliefKey {
            subject: subject(),
            dimension_id: "docs_freshness".to_string(),
            predicate_id: "confidence".to_string(),
            perspective: PerspectiveKey::new("default", "default").unwrap(),
            branch_scope: BranchScope::main(),
            evidence_policy_id: "default_policy".to_string(),
        }
    }

    /// One durable chain: decision -> revision -> evidence ->
    /// {fact -> event seq 5, anchor -> fact}, with one absent anchor
    /// reference left dangling on the evidence.
    fn chain_world() -> ChainWorld {
        let dir = tempfile::tempdir().unwrap();
        let db = sled::open(dir.path().join("world")).unwrap();
        let traversal = TraversalStore::new(db.clone()).unwrap();
        let belief = BeliefStore::new(db.clone()).unwrap();
        let agent = AgentStore::new(db.clone()).unwrap();
        let legacy_db = db.clone();

        traversal
            .put_fact(&TraversalFactRecord {
                fact_id: FACT_ID.to_string(),
                source_spine_fact_id: FACT_ID.to_string(),
                seq: 5,
                event_type: "context.frame.head_advanced".to_string(),
                objects: vec![subject()],
                relations: Vec::new(),
            })
            .unwrap();
        traversal
            .put_anchor(&AnchorSelectionRecord {
                anchor_id: ANCHOR_ID.to_string(),
                anchor_ref: DomainObjectRef::new("context", "head", "node-a::analysis").unwrap(),
                subject: subject(),
                perspective: PerspectiveKey::new("frame_type", "analysis").unwrap(),
                target: subject(),
                source_fact_ids: vec![FACT_ID.to_string()],
                created_by_fact_id: FACT_ID.to_string(),
                selected_at_seq: 5,
                ended_at_seq: None,
                ended_by_anchor_id: None,
                ended_by_fact_id: None,
            })
            .unwrap();
        belief
            .put_evidence(&EvidenceItem {
                publication_record_id: None,
                outcome_mapping_revision: None,
                evidence_id: EVIDENCE_ID.to_string(),
                candidate_key: belief_key(),
                source_fact_ids: vec![FACT_ID.to_string()],
                graph_anchor_ids: vec![ANCHOR_ID.to_string(), MISSING_ANCHOR_ID.to_string()],
                source_cursor_start: 1,
                source_cursor_end: 5,
                role: EvidenceRole::Support,
                evidence_schema_id: "content_written_signal".to_string(),
                typed_value: EvidenceValue::Scalar(0.9),
                reliability: 1.0,
                precision: 1.0,
                reference_time: None,
                transaction_seq: 5,
                content_hash: None,
                provenance: BeliefProvenanceSummary::empty(),
            })
            .unwrap();

        let lease = belief
            .acquire_lease(AssessmentLease {
                lease_id: "lease-1".to_string(),
                belief_key: belief_key(),
                epoch: 5,
                owner_id: "walk-test".to_string(),
                input_cursor_start: 0,
                input_cursor_end: 5,
                started_at_seq: 5,
                expires_at_seq: 100,
                comparator_engine_id: "weighted_bayesian".to_string(),
                config_snapshot_hash: "hash-1".to_string(),
                status: LeaseStatus::Queued,
            })
            .unwrap();
        belief
            .commit_revision(
                &lease,
                &BeliefRevision {
                    revision_id: REVISION_ID.to_string(),
                    belief_key: belief_key(),
                    prior_revision_id: None,
                    comparator_engine_id: "weighted_bayesian".to_string(),
                    comparator_engine_version: "1".to_string(),
                    config_snapshot_hash: "hash-1".to_string(),
                    evidence_ids: vec![EVIDENCE_ID.to_string()],
                    supporting_evidence_ids: vec![EVIDENCE_ID.to_string()],
                    contradicted_evidence_ids: Vec::new(),
                    source_cursor_start: 0,
                    source_cursor_end: 5,
                    posterior: PosteriorSummary {
                        probability: 0.9,
                        meaning: "stale_probability".to_string(),
                    },
                    planner_projection: PlannerProjectionSummary {
                        confidence_field: "confidence".to_string(),
                        confidence: 0.9,
                        threshold: 0.7,
                    },
                    uncertainty: 0.1,
                    precision: 1.0,
                    freshness: FreshnessState {
                        stale: false,
                        reasons: Vec::new(),
                        high_water_seq: 5,
                    },
                    contradiction: ContradictionState {
                        contradicted: false,
                        reasons: Vec::new(),
                        supporting_evidence_ids: Vec::new(),
                        contradicted_evidence_ids: Vec::new(),
                    },
                    status: BeliefStatus::Settled,
                    observation: None,
                    provenance: BeliefProvenanceSummary::empty(),
                    theory_revision: Some(TheoryRevisionRef {
                        registry: "belief_family".to_string(),
                        id: "docs_freshness".to_string(),
                        content_hash: "hash-1".to_string(),
                    }),
                },
            )
            .unwrap();

        legacy_db
            .open_tree("agent_curation_decisions")
            .unwrap()
            .insert(
                DECISION_ID.as_bytes(),
                serde_json::to_vec(&serde_json::json!({
                    "decision_id": DECISION_ID,
                    "agent_id": "seed.docs_freshness",
                    "decision": "GoalCommand",
                    "input_refs": { "belief_revision_id": REVISION_ID }
                }))
                .unwrap(),
            )
            .unwrap();
        ChainWorld {
            _dir: dir,
            traversal,
            belief,
            agent,
        }
    }

    fn walker(world: &ChainWorld) -> ThreadWalker<'_> {
        ThreadWalker {
            replay: None,
            belief: Some(&world.belief),
            agent: Some(&world.agent),
            traversal: Some(&world.traversal),
            task_networks: None,
            max_nodes: DEFAULT_MAX_THREAD_NODES,
        }
    }

    fn edge_exists(
        thread: &CausalThread,
        from: &ThreadSubject,
        to: &ThreadSubject,
        citation: ThreadCitation,
    ) -> bool {
        let index = |subject: &ThreadSubject| {
            thread
                .nodes
                .iter()
                .position(|node| node.subject == *subject)
        };
        match (index(from), index(to)) {
            (Some(from), Some(to)) => thread
                .edges
                .iter()
                .any(|edge| edge.from == from && edge.to == to && edge.citation == citation),
            _ => false,
        }
    }

    #[test]
    fn decision_thread_resolves_across_domain_boundaries_to_its_facts() {
        let world = chain_world();
        let thread = walker(&world)
            .walk(ThreadSubject::Decision {
                decision_id: DECISION_ID.to_string(),
            })
            .unwrap();

        assert!(edge_exists(
            &thread,
            &ThreadSubject::Decision {
                decision_id: DECISION_ID.to_string()
            },
            &ThreadSubject::BeliefRevision {
                revision_id: REVISION_ID.to_string()
            },
            ThreadCitation::DecisionInput,
        ));
        assert!(edge_exists(
            &thread,
            &ThreadSubject::BeliefRevision {
                revision_id: REVISION_ID.to_string()
            },
            &ThreadSubject::Evidence {
                evidence_id: EVIDENCE_ID.to_string()
            },
            ThreadCitation::RevisionEvidence,
        ));
        assert!(edge_exists(
            &thread,
            &ThreadSubject::Evidence {
                evidence_id: EVIDENCE_ID.to_string()
            },
            &ThreadSubject::Fact {
                fact_id: FACT_ID.to_string()
            },
            ThreadCitation::EvidenceSourceFact,
        ));
        assert!(edge_exists(
            &thread,
            &ThreadSubject::Evidence {
                evidence_id: EVIDENCE_ID.to_string()
            },
            &ThreadSubject::Anchor {
                anchor_id: ANCHOR_ID.to_string()
            },
            ThreadCitation::EvidenceAnchor,
        ));
        assert!(!thread.bounded);
    }

    #[test]
    fn absent_references_are_recorded_as_cuts_not_dropped() {
        let world = chain_world();
        let thread = walker(&world)
            .walk(ThreadSubject::Evidence {
                evidence_id: EVIDENCE_ID.to_string(),
            })
            .unwrap();

        // The dangling anchor reference surfaces as a truthful dead end.
        let cut = thread
            .cuts
            .iter()
            .find(|cut| cut.reference == format!("anchor {MISSING_ANCHOR_ID}"))
            .expect("missing anchor is recorded as a cut");
        assert_eq!(cut.reason, ThreadCutReason::AbsentRecord);
        assert_eq!(cut.citation, ThreadCitation::EvidenceAnchor);

        // The ledger is out of scope for this walker, so the fact's event
        // reference is a scope cut rather than a silent omission.
        assert!(thread.cuts.iter().any(|cut| {
            cut.reason == ThreadCutReason::SourceOutOfScope
                && cut.citation == ThreadCitation::FactSource
        }));
    }

    #[test]
    fn re_deriving_a_thread_yields_the_identical_thread() {
        let world = chain_world();
        let subject = ThreadSubject::Decision {
            decision_id: DECISION_ID.to_string(),
        };
        let first = walker(&world).walk(subject.clone()).unwrap();
        let second = walker(&world).walk(subject).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn the_node_bound_truncates_honestly() {
        let world = chain_world();
        let walker = ThreadWalker {
            max_nodes: 2,
            ..walker(&world)
        };
        let thread = walker
            .walk(ThreadSubject::Decision {
                decision_id: DECISION_ID.to_string(),
            })
            .unwrap();
        assert!(thread.bounded);
        assert_eq!(thread.nodes.len(), 2);
    }

    #[test]
    fn walking_an_out_of_scope_goal_keeps_the_anchor_node_and_records_the_cut() {
        let world = chain_world();
        let thread = walker(&world)
            .walk(ThreadSubject::Goal {
                goal_id: "goal-unknown".to_string(),
            })
            .unwrap();
        assert_eq!(thread.nodes.len(), 1);
        assert_eq!(thread.nodes[0].summary, "unresolved");
        assert_eq!(thread.cuts.len(), 1);
        assert_eq!(thread.cuts[0].reason, ThreadCutReason::SourceOutOfScope);
    }

    #[test]
    fn the_theory_citation_is_recorded_not_dropped() {
        let world = chain_world();
        let thread = walker(&world)
            .walk(ThreadSubject::BeliefRevision {
                revision_id: REVISION_ID.to_string(),
            })
            .unwrap();
        let cut = thread
            .cuts
            .iter()
            .find(|cut| cut.citation == ThreadCitation::TheoryRevision)
            .expect("theory revision reference is recorded");
        assert_eq!(cut.reason, ThreadCutReason::SourceOutOfScope);
        assert!(cut.reference.contains("belief_family::docs_freshness"));
    }

    #[test]
    fn a_present_anchor_for_subject_resolves_and_hops_to_the_anchor() {
        let world = chain_world();
        let thread = walker(&world)
            .walk(ThreadSubject::AnchorForSubject {
                subject_domain_id: "workspace_fs".to_string(),
                subject_object_kind: "node".to_string(),
                subject_object_id: "node-a".to_string(),
                perspective_kind: "frame_type".to_string(),
                perspective_id: "analysis".to_string(),
            })
            .unwrap();

        assert!(thread.nodes[0].summary.contains(ANCHOR_ID));
        let hop = thread
            .edges
            .iter()
            .find(|edge| edge.citation == ThreadCitation::CurrentAnchor)
            .expect("resolved anchor-for-subject hops to the anchor record");
        assert_eq!(
            thread.nodes[hop.to].subject,
            ThreadSubject::Anchor {
                anchor_id: ANCHOR_ID.to_string()
            }
        );
    }

    #[test]
    fn an_absent_perspective_reports_the_anchors_that_do_exist() {
        let world = chain_world();
        let thread = walker(&world)
            .walk(ThreadSubject::AnchorForSubject {
                subject_domain_id: "workspace_fs".to_string(),
                subject_object_kind: "node".to_string(),
                subject_object_id: "node-a".to_string(),
                perspective_kind: "frame_type".to_string(),
                perspective_id: "other-perspective".to_string(),
            })
            .unwrap();

        assert_eq!(thread.cuts.len(), 1);
        assert_eq!(thread.cuts[0].reason, ThreadCutReason::AbsentRecord);
        assert!(
            thread.cuts[0]
                .reference
                .contains("1 anchors exist under other perspectives"),
            "the absence names the perspectives that do exist: {}",
            thread.cuts[0].reference
        );
    }

    #[test]
    fn walking_an_unknown_network_creates_no_database_and_names_the_network() {
        let dir = tempfile::tempdir().unwrap();
        let factory = TaskNetworkStoreFactory::new(dir.path().join("networks"));
        let world = chain_world();
        let walker = ThreadWalker {
            task_networks: Some(&factory),
            ..walker(&world)
        };

        let thread = walker
            .walk(ThreadSubject::Task {
                network_id: "network-typo".to_string(),
                task_instance_id: "task-alpha".to_string(),
            })
            .unwrap();

        assert_eq!(thread.cuts.len(), 1);
        assert_eq!(thread.cuts[0].reason, ThreadCutReason::AbsentRecord);
        assert_eq!(thread.cuts[0].reference, "task network network-typo");
        // The read path must not have materialized the missing network.
        assert!(!dir.path().join("networks/network-typo.sled").exists());
    }
}
