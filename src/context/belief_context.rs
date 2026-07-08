//! Belief context contracts for generation conditioning.
//!
//! Owns the `belief_context_bundle` artifact shape that carries one current
//! assertion per covered subject in the target subtree, the trigger
//! subject's own view among them (up to fixed count and byte budgets), into
//! the task package. The seeded bundle is
//! the only belief source consulted during prompt assembly, so generation is
//! a deterministic function of belief state at hydration time. Belief state
//! is read through the execution-side [`BeliefContextReadPort`] at hydration
//! only; this module never mutates belief state and the world model never
//! sees prompt text produced from these records.

use crate::error::ApiError;
use crate::execution::{BeliefContextReadPort, ContextReadPort};
use crate::prompt_context::MAX_CONTEXT_ARTIFACT_BYTES;
use crate::types::NodeID;
use meld_execution::{BeliefStatusLabel, BeliefSubjectSignal};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Artifact type id for the seeded belief context bundle.
pub const BELIEF_CONTEXT_BUNDLE_ARTIFACT_TYPE_ID: &str = "belief_context_bundle";

/// Belief family consulted by the first slice. `docs_freshness` is the only
/// loaded family; selection falls back to recency for uncovered subjects.
pub const BELIEF_CONTEXT_FAMILY_ID: &str = "docs_freshness";

/// Maximum covered subjects seeded into one bundle. A coarse pre-filter, not
/// the byte-cap guarantee: assertion reference lists are unbounded upstream,
/// so [`hydrate_belief_context_bundle`] additionally trims the retained set
/// until the serialized bundle fits under [`MAX_CONTEXT_ARTIFACT_BYTES`].
pub const MAX_BELIEF_CONTEXT_SUBJECTS: usize = 512;

/// Headroom reserved under [`MAX_CONTEXT_ARTIFACT_BYTES`] for the artifact
/// envelope around the bundle's canonical JSON, so a budgeted bundle is never
/// rejected at write time by the lineage store's byte cap.
const BELIEF_BUNDLE_BYTE_MARGIN: usize = 4 * 1024;

/// Hydrated belief view for one subject, seeded into the task package at
/// trigger time and consumed by `context_generate_prepare`.
///
/// Each seeded assertion carries its own per-subject currency in its
/// `as_of_seq`; the bundle-level `as_of_seq` is the high-water sequence
/// across seeded subjects, not a single-snapshot claim. When no belief
/// covers the trigger subject its map entry is absent and prompt assembly
/// leaves the prompt unchanged.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BeliefContextBundle {
    pub schema_version: u32,
    /// Hex-encoded workspace node id of the target subject.
    pub subject_node_id: String,
    /// Workspace-relative path of the target subject, for prompt rendering.
    pub subject_path: String,
    /// Belief family the bundle was hydrated from.
    pub family_id: String,
    /// High-water `as_of_seq` across all seeded subject assertions (0 when
    /// no subject is covered). Each assertion carries its own per-subject
    /// currency; this field does not claim a single-snapshot read.
    pub as_of_seq: u64,
    /// Current assertions for covered subjects in the target subtree, keyed
    /// by hex-encoded node id and bounded by
    /// [`MAX_BELIEF_CONTEXT_SUBJECTS`] plus the byte budget. The trigger
    /// subject's own assertion lives here under `subject_node_id`.
    /// Belief-endorsed selection reads only this map at generation time;
    /// `BTreeMap` keeps `canonical_json` byte-stable for equal bundles.
    pub subject_assertions: BTreeMap<String, BeliefContextAssertion>,
    /// Count of covered subjects omitted by the count and byte budgets; zero
    /// when the bundle covers the whole subtree. Consumers treat omitted
    /// subjects as uncovered. Serialized (defaulting to zero for pre-budget
    /// bundles) so truncation participates in the bundle digest.
    #[serde(default)]
    pub omitted_subject_count: usize,
}

/// Current belief assertion carried by a non-empty bundle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BeliefContextAssertion {
    /// Belief revision the assertion settles on, when one exists.
    pub revision_id: Option<String>,
    /// Belief status label projected at hydration time.
    pub status: BeliefStatusLabel,
    /// Planner-facing confidence.
    pub confidence: f64,
    /// True when freshness tracking flags the view stale.
    pub stale: bool,
    /// True when unresolved counterevidence contradicts the view.
    pub contradicted: bool,
    /// Evidence ids surfaced as unresolved contradicted claims.
    pub contradicted_claims: Vec<String>,
    /// Hydrated evidence references from the belief view.
    pub evidence_refs: Vec<String>,
    /// Hydrated source fact references from the belief view.
    pub source_fact_refs: Vec<String>,
    /// Sequence this subject's belief view is current as of.
    pub as_of_seq: u64,
}

impl BeliefContextBundle {
    /// Canonical JSON used for digest-addressed lineage persistence.
    /// Struct field order is fixed and the assertion map is a `BTreeMap`,
    /// so equal bundles produce equal bytes.
    ///
    /// ```rust
    /// use meld::context::belief_context::BeliefContextBundle;
    /// use std::collections::BTreeMap;
    ///
    /// let bundle = BeliefContextBundle {
    ///     schema_version: 1,
    ///     subject_node_id: "ab".repeat(32),
    ///     subject_path: "src".to_string(),
    ///     family_id: "docs_freshness".to_string(),
    ///     as_of_seq: 7,
    ///     subject_assertions: BTreeMap::new(),
    ///     omitted_subject_count: 0,
    /// };
    ///
    /// assert_eq!(
    ///     bundle.canonical_json().unwrap(),
    ///     bundle.clone().canonical_json().unwrap()
    /// );
    /// ```
    pub fn canonical_json(&self) -> Result<String, ApiError> {
        serde_json::to_string(self).map_err(|err| {
            ApiError::ConfigError(format!("Failed to encode belief context bundle: {}", err))
        })
    }

    /// Decodes a bundle from a seeded artifact value.
    pub fn from_artifact_value(value: &serde_json::Value) -> Result<Self, ApiError> {
        serde_json::from_value(value.clone()).map_err(|err| {
            ApiError::ConfigError(format!(
                "Failed to decode belief context bundle artifact: {}",
                err
            ))
        })
    }

    /// Returns the seeded assertion for one subject, or `None` when the
    /// bundle does not cover it.
    pub fn assertion_for_subject(&self, node_id_hex: &str) -> Option<&BeliefContextAssertion> {
        self.subject_assertions.get(node_id_hex)
    }
}

/// Hydrates the belief context bundle for one subject from the belief read
/// port, walking the subject's subtree so covered descendants carry seeded
/// assertions. Deterministic for a fixed store state: the port projects each
/// current view at its committed as-of sequence, the map is keyed for
/// canonical ordering, and the [`MAX_BELIEF_CONTEXT_SUBJECTS`] count budget
/// plus the byte budget are applied as pure functions of the covered
/// subjects, with any truncation recorded in `omitted_subject_count`.
pub fn hydrate_belief_context_bundle(
    api: &(impl ContextReadPort + BeliefContextReadPort + ?Sized),
    node_id: NodeID,
    subject_path: &str,
) -> Result<BeliefContextBundle, ApiError> {
    let subject_node_id = hex::encode(node_id);
    let mut subject_assertions = BTreeMap::new();
    let mut pending = vec![node_id];
    while let Some(current) = pending.pop() {
        let Some(record) = api.read_node_record(&current)? else {
            continue;
        };
        pending.extend(record.children.iter().copied());
        let current_hex = hex::encode(current);
        if let Some(signal) = api.current_belief_signal(&current_hex, BELIEF_CONTEXT_FAMILY_ID)? {
            subject_assertions.insert(current_hex, assertion_from_signal(signal));
        }
    }
    let omitted_subject_count = apply_subject_budget(&mut subject_assertions, &subject_node_id);

    let mut bundle = BeliefContextBundle {
        schema_version: 1,
        subject_node_id,
        subject_path: subject_path.to_string(),
        family_id: BELIEF_CONTEXT_FAMILY_ID.to_string(),
        as_of_seq: high_water_seq(&subject_assertions),
        subject_assertions,
        omitted_subject_count,
    };
    apply_bundle_byte_budget(&mut bundle)?;
    Ok(bundle)
}

/// High-water `as_of_seq` across the retained subject assertions.
fn high_water_seq(subject_assertions: &BTreeMap<String, BeliefContextAssertion>) -> u64 {
    subject_assertions
        .values()
        .map(|assertion| assertion.as_of_seq)
        .max()
        .unwrap_or(0)
}

/// Retention priority under budget pressure, lower is kept longer.
/// Contradicted assertions outrank stale ones, which outrank endorsed and
/// other settled entries: dropping a contradicted assertion would silently
/// re-include content the belief state rejects, while dropping an endorsed
/// one merely loses an annotation.
fn retention_rank(assertion: &BeliefContextAssertion) -> u8 {
    match classify_belief_assertion(Some(assertion)) {
        BeliefSelectionClass::Contradicted => 0,
        BeliefSelectionClass::Stale => 1,
        BeliefSelectionClass::Endorsed | BeliefSelectionClass::Uncovered => 2,
    }
}

/// Enforces the bundle subject-count budget: the trigger subject's own
/// assertion is always kept, then remaining slots fill contradicted-first,
/// then stale, then endorsed entries, with ties inside a class broken by key
/// order. The retained set is a pure deterministic function of the covered
/// subjects. Returns the omitted entry count.
fn apply_subject_budget(
    subject_assertions: &mut BTreeMap<String, BeliefContextAssertion>,
    trigger_subject: &str,
) -> usize {
    if subject_assertions.len() <= MAX_BELIEF_CONTEXT_SUBJECTS {
        return 0;
    }
    let total = subject_assertions.len();
    let mut retained = BTreeMap::new();
    if let Some(assertion) = subject_assertions.remove(trigger_subject) {
        retained.insert(trigger_subject.to_string(), assertion);
    }
    let mut candidates: Vec<(String, BeliefContextAssertion)> =
        std::mem::take(subject_assertions).into_iter().collect();
    // Stable sort over the class rank keeps ascending key order within each
    // retention class.
    candidates.sort_by_key(|(_, assertion)| retention_rank(assertion));
    for (node_id, assertion) in candidates {
        if retained.len() >= MAX_BELIEF_CONTEXT_SUBJECTS {
            break;
        }
        retained.insert(node_id, assertion);
    }
    let omitted = total - retained.len();
    *subject_assertions = retained;
    omitted
}

/// Enforces the serialized byte budget after the count budget: while the
/// bundle's canonical JSON exceeds [`MAX_CONTEXT_ARTIFACT_BYTES`] minus the
/// envelope margin, the lowest-priority retained entry is evicted, never the
/// trigger subject. Within the lowest-priority class the largest key goes
/// first, mirroring the count budget's key-order fill, so the retained set
/// stays a pure deterministic function of the covered subjects. Every
/// eviction bumps `omitted_subject_count`.
fn apply_bundle_byte_budget(bundle: &mut BeliefContextBundle) -> Result<(), ApiError> {
    let byte_budget = MAX_CONTEXT_ARTIFACT_BYTES - BELIEF_BUNDLE_BYTE_MARGIN;
    let mut trimmed = false;
    while bundle.canonical_json()?.len() > byte_budget {
        let evicted = bundle
            .subject_assertions
            .iter()
            .filter(|(key, _)| key.as_str() != bundle.subject_node_id)
            .max_by(|(left_key, left), (right_key, right)| {
                (retention_rank(left), left_key.as_str())
                    .cmp(&(retention_rank(right), right_key.as_str()))
            })
            .map(|(key, _)| key.clone());
        // Only the trigger subject remains: nothing else may be evicted, so
        // an oversized single assertion surfaces at write time instead.
        let Some(evicted) = evicted else {
            break;
        };
        bundle.subject_assertions.remove(&evicted);
        bundle.omitted_subject_count += 1;
        trimmed = true;
    }
    if trimmed {
        bundle.as_of_seq = high_water_seq(&bundle.subject_assertions);
    }
    Ok(())
}

fn assertion_from_signal(signal: BeliefSubjectSignal) -> BeliefContextAssertion {
    BeliefContextAssertion {
        revision_id: signal.revision_id,
        status: signal.status,
        confidence: signal.confidence,
        stale: signal.stale,
        contradicted: signal.contradicted,
        contradicted_claims: signal.contradicted_evidence_ids,
        evidence_refs: signal.evidence_ids,
        source_fact_refs: signal.source_fact_ids,
        as_of_seq: signal.as_of_seq,
    }
}

/// Deterministic selection class for one candidate subject under
/// belief-endorsed frame collection.
///
/// Rank order (lower is preferred): endorsed, uncovered (recency fallback),
/// stale, contradicted. Contradicted subjects are excluded from content and
/// surfaced as unresolved.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BeliefSelectionClass {
    /// Settled, not stale, not contradicted: preferred and annotated.
    Endorsed,
    /// No belief family covers the subject: recency-fallback ordering.
    Uncovered,
    /// Covered but stale or in any non-settled status (for example
    /// `NeedsAssessment`, `NeedsObservation`, `AssessmentPending`,
    /// `Invalid`): kept, deprioritized.
    Stale,
    /// Unresolved contradiction: frame content excluded, subject flagged.
    Contradicted,
}

impl BeliefSelectionClass {
    /// Sort rank used by belief-endorsed selection; ties keep child order.
    pub fn rank(&self) -> u8 {
        match self {
            BeliefSelectionClass::Endorsed => 0,
            BeliefSelectionClass::Uncovered => 1,
            BeliefSelectionClass::Stale => 2,
            BeliefSelectionClass::Contradicted => 3,
        }
    }
}

/// Classifies one subject's seeded belief assertion for belief-endorsed
/// selection. Pure over the assertion, so selection replays byte-identically
/// for the same seeded bundle.
///
/// ```rust
/// use meld::context::belief_context::{
///     classify_belief_assertion, BeliefContextAssertion, BeliefSelectionClass,
/// };
/// use meld::execution::BeliefStatusLabel;
///
/// assert_eq!(
///     classify_belief_assertion(None),
///     BeliefSelectionClass::Uncovered
/// );
///
/// let contradicted = BeliefContextAssertion {
///     revision_id: None,
///     status: BeliefStatusLabel::Settled,
///     confidence: 0.4,
///     stale: false,
///     contradicted: true,
///     contradicted_claims: vec!["ev-1".to_string()],
///     evidence_refs: Vec::new(),
///     source_fact_refs: Vec::new(),
///     as_of_seq: 3,
/// };
/// assert_eq!(
///     classify_belief_assertion(Some(&contradicted)),
///     BeliefSelectionClass::Contradicted
/// );
/// ```
pub fn classify_belief_assertion(
    assertion: Option<&BeliefContextAssertion>,
) -> BeliefSelectionClass {
    match assertion {
        None => BeliefSelectionClass::Uncovered,
        Some(assertion) if assertion.contradicted => BeliefSelectionClass::Contradicted,
        Some(assertion) if assertion.stale || assertion.status != BeliefStatusLabel::Settled => {
            BeliefSelectionClass::Stale
        }
        Some(_) => BeliefSelectionClass::Endorsed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assertion(
        stale: bool,
        contradicted: bool,
        status: BeliefStatusLabel,
    ) -> BeliefContextAssertion {
        BeliefContextAssertion {
            revision_id: Some("rev-1".to_string()),
            status,
            confidence: 0.9,
            stale,
            contradicted,
            contradicted_claims: Vec::new(),
            evidence_refs: vec!["ev-1".to_string()],
            source_fact_refs: vec!["fact-1".to_string()],
            as_of_seq: 7,
        }
    }

    #[test]
    fn classification_orders_endorsed_before_uncovered_stale_contradicted() {
        let endorsed =
            classify_belief_assertion(Some(&assertion(false, false, BeliefStatusLabel::Settled)));
        let uncovered = classify_belief_assertion(None);
        let stale =
            classify_belief_assertion(Some(&assertion(true, false, BeliefStatusLabel::Settled)));
        let contradicted =
            classify_belief_assertion(Some(&assertion(false, true, BeliefStatusLabel::Settled)));

        assert_eq!(endorsed, BeliefSelectionClass::Endorsed);
        assert_eq!(uncovered, BeliefSelectionClass::Uncovered);
        assert_eq!(stale, BeliefSelectionClass::Stale);
        assert_eq!(contradicted, BeliefSelectionClass::Contradicted);
        assert!(endorsed.rank() < uncovered.rank());
        assert!(uncovered.rank() < stale.rank());
        assert!(stale.rank() < contradicted.rank());
    }

    #[test]
    fn classification_endorses_only_settled_status() {
        for status in [
            BeliefStatusLabel::Stale,
            BeliefStatusLabel::NeedsAssessment,
            BeliefStatusLabel::NeedsObservation,
            BeliefStatusLabel::AssessmentPending,
            BeliefStatusLabel::Invalid,
        ] {
            assert_eq!(
                classify_belief_assertion(Some(&assertion(false, false, status))),
                BeliefSelectionClass::Stale,
                "non-settled status '{status}' must be deprioritized"
            );
        }
        assert_eq!(
            classify_belief_assertion(Some(&assertion(false, false, BeliefStatusLabel::Settled))),
            BeliefSelectionClass::Endorsed
        );
    }

    #[test]
    fn canonical_json_is_stable_for_equal_bundles() {
        let subject_node_id = "ab".repeat(32);
        let bundle = BeliefContextBundle {
            schema_version: 1,
            subject_node_id: subject_node_id.clone(),
            subject_path: "src".to_string(),
            family_id: BELIEF_CONTEXT_FAMILY_ID.to_string(),
            as_of_seq: 7,
            subject_assertions: BTreeMap::from([
                (
                    subject_node_id,
                    assertion(false, false, BeliefStatusLabel::Settled),
                ),
                (
                    "cd".repeat(32),
                    assertion(true, false, BeliefStatusLabel::Stale),
                ),
            ]),
            omitted_subject_count: 0,
        };

        assert_eq!(
            bundle.canonical_json().unwrap(),
            bundle.clone().canonical_json().unwrap()
        );
        let decoded =
            BeliefContextBundle::from_artifact_value(&serde_json::to_value(&bundle).unwrap())
                .unwrap();
        assert_eq!(decoded, bundle);
    }

    #[test]
    fn subject_budget_is_noop_within_budget() {
        let trigger = "ab".repeat(32);
        let mut assertions = BTreeMap::from([(
            trigger.clone(),
            assertion(false, false, BeliefStatusLabel::Settled),
        )]);

        assert_eq!(apply_subject_budget(&mut assertions, &trigger), 0);
        assert_eq!(assertions.len(), 1);
    }

    #[test]
    fn subject_budget_truncates_deterministically_and_keeps_trigger_subject() {
        // Trigger key sorts after every generated key, so keeping it
        // exercises the always-retain rule rather than key order.
        let trigger = "ff".repeat(32);
        let mut assertions: BTreeMap<String, BeliefContextAssertion> = (0..600u32)
            .map(|index| {
                (
                    format!("{index:064x}"),
                    assertion(false, false, BeliefStatusLabel::Settled),
                )
            })
            .collect();
        assertions.insert(
            trigger.clone(),
            assertion(false, false, BeliefStatusLabel::Settled),
        );
        let total = assertions.len();

        let mut truncated = assertions.clone();
        let omitted = apply_subject_budget(&mut truncated, &trigger);

        assert_eq!(truncated.len(), MAX_BELIEF_CONTEXT_SUBJECTS);
        assert_eq!(omitted, total - MAX_BELIEF_CONTEXT_SUBJECTS);
        assert!(truncated.contains_key(&trigger));
        // Non-trigger slots fill with the smallest keys in map order.
        assert!(truncated.contains_key(&format!("{:064x}", 0)));
        assert!(!truncated.contains_key(&format!("{:064x}", 599)));

        let mut rerun = assertions;
        apply_subject_budget(&mut rerun, &trigger);
        assert_eq!(truncated, rerun);
    }

    #[test]
    fn subject_budget_retains_contradicted_and_stale_over_endorsed_under_pressure() {
        let trigger = format!("{:064x}", 0u32);
        // Contradicted and stale entries carry the largest keys, so key-order
        // filling alone would evict them before the endorsed entries.
        let contradicted_key = format!("{:064x}", 700u32);
        let stale_key = format!("{:064x}", 701u32);
        let mut assertions: BTreeMap<String, BeliefContextAssertion> = (1..=600u32)
            .map(|index| {
                (
                    format!("{index:064x}"),
                    assertion(false, false, BeliefStatusLabel::Settled),
                )
            })
            .collect();
        assertions.insert(
            trigger.clone(),
            assertion(false, false, BeliefStatusLabel::Settled),
        );
        assertions.insert(
            contradicted_key.clone(),
            assertion(false, true, BeliefStatusLabel::Settled),
        );
        assertions.insert(
            stale_key.clone(),
            assertion(true, false, BeliefStatusLabel::Settled),
        );
        let total = assertions.len();

        let omitted = apply_subject_budget(&mut assertions, &trigger);

        assert_eq!(assertions.len(), MAX_BELIEF_CONTEXT_SUBJECTS);
        assert_eq!(omitted, total - MAX_BELIEF_CONTEXT_SUBJECTS);
        assert!(assertions.contains_key(&trigger));
        assert!(assertions.contains_key(&contradicted_key));
        assert!(assertions.contains_key(&stale_key));
        // Endorsed entries fill the remaining slots in key order: trigger
        // plus two warning-class entries leave 509 endorsed slots.
        assert!(assertions.contains_key(&format!("{:064x}", 509u32)));
        assert!(!assertions.contains_key(&format!("{:064x}", 510u32)));
    }

    #[test]
    fn byte_budget_evicts_lowest_priority_entries_and_keeps_trigger() {
        let trigger = "ff".repeat(32);
        let oversized_endorsed = {
            let mut oversized = assertion(false, false, BeliefStatusLabel::Settled);
            oversized.evidence_refs = vec!["e".repeat(MAX_CONTEXT_ARTIFACT_BYTES)];
            oversized
        };
        let mut bundle = BeliefContextBundle {
            schema_version: 1,
            subject_node_id: trigger.clone(),
            subject_path: "src".to_string(),
            family_id: BELIEF_CONTEXT_FAMILY_ID.to_string(),
            as_of_seq: 7,
            subject_assertions: BTreeMap::from([
                (
                    trigger.clone(),
                    assertion(false, false, BeliefStatusLabel::Settled),
                ),
                ("aa".repeat(32), oversized_endorsed),
                (
                    "bb".repeat(32),
                    assertion(false, true, BeliefStatusLabel::Settled),
                ),
            ]),
            omitted_subject_count: 0,
        };

        apply_bundle_byte_budget(&mut bundle).unwrap();

        // The oversized endorsed entry is evicted before the contradicted
        // one, and the trigger subject is never a candidate.
        assert!(!bundle.subject_assertions.contains_key(&"aa".repeat(32)));
        assert!(bundle.subject_assertions.contains_key(&"bb".repeat(32)));
        assert!(bundle.subject_assertions.contains_key(&trigger));
        assert_eq!(bundle.omitted_subject_count, 1);
        assert!(bundle.canonical_json().unwrap().len() <= MAX_CONTEXT_ARTIFACT_BYTES);
    }

    #[test]
    fn byte_budget_never_evicts_the_trigger_subject() {
        let trigger = "ab".repeat(32);
        let oversized_trigger = {
            let mut oversized = assertion(false, false, BeliefStatusLabel::Settled);
            oversized.evidence_refs = vec!["e".repeat(MAX_CONTEXT_ARTIFACT_BYTES)];
            oversized
        };
        let mut bundle = BeliefContextBundle {
            schema_version: 1,
            subject_node_id: trigger.clone(),
            subject_path: "src".to_string(),
            family_id: BELIEF_CONTEXT_FAMILY_ID.to_string(),
            as_of_seq: 7,
            subject_assertions: BTreeMap::from([(trigger.clone(), oversized_trigger)]),
            omitted_subject_count: 0,
        };

        apply_bundle_byte_budget(&mut bundle).unwrap();

        // An oversized trigger assertion stays for write-time enforcement.
        assert!(bundle.subject_assertions.contains_key(&trigger));
        assert_eq!(bundle.omitted_subject_count, 0);
    }

    #[test]
    fn classification_is_total_with_contradiction_dominating_staleness() {
        assert_eq!(
            classify_belief_assertion(None),
            BeliefSelectionClass::Uncovered
        );
        let statuses = [
            BeliefStatusLabel::Settled,
            BeliefStatusLabel::Stale,
            BeliefStatusLabel::NeedsObservation,
            BeliefStatusLabel::NeedsAssessment,
            BeliefStatusLabel::AssessmentPending,
            BeliefStatusLabel::Invalid,
        ];
        for status in statuses {
            for stale in [false, true] {
                for contradicted in [false, true] {
                    // Spec precedence: an unresolved contradiction dominates
                    // staleness and status; staleness or any non-settled
                    // status demotes; only settled-and-clean is endorsed.
                    let expected = if contradicted {
                        BeliefSelectionClass::Contradicted
                    } else if stale || status != BeliefStatusLabel::Settled {
                        BeliefSelectionClass::Stale
                    } else {
                        BeliefSelectionClass::Endorsed
                    };
                    assert_eq!(
                        classify_belief_assertion(Some(&assertion(stale, contradicted, status))),
                        expected,
                        "status={status}, stale={stale}, contradicted={contradicted}"
                    );
                }
            }
        }
    }

    // Property coverage over the bundle contracts: canonical-json byte
    // stability and the budget functions' retention invariants. Runners are
    // deterministically seeded so generated cases are stable across runs.
    mod properties {
        use super::*;
        use proptest::prelude::*;
        use proptest::test_runner::{Config, RngAlgorithm, TestRng, TestRunner};

        fn deterministic_runner(cases: u32) -> TestRunner {
            TestRunner::new_with_rng(
                Config {
                    cases,
                    ..Config::default()
                },
                TestRng::deterministic_rng(RngAlgorithm::ChaCha),
            )
        }

        fn arb_status() -> impl Strategy<Value = BeliefStatusLabel> {
            prop_oneof![
                Just(BeliefStatusLabel::Settled),
                Just(BeliefStatusLabel::Stale),
                Just(BeliefStatusLabel::NeedsObservation),
                Just(BeliefStatusLabel::NeedsAssessment),
                Just(BeliefStatusLabel::AssessmentPending),
                Just(BeliefStatusLabel::Invalid),
            ]
        }

        /// Full-precision confidences in `[0, 1]`. Byte-stable decode and
        /// re-encode of arbitrary `f64` values requires the workspace-wide
        /// `serde_json` `float_roundtrip` feature; this strategy exercises
        /// that guarantee, so a regression to the imprecise parser fails
        /// here first. Digests should still be computed from originally
        /// serialized bundle bytes as defense in depth.
        fn arb_confidence() -> impl Strategy<Value = f64> {
            0.0f64..=1.0
        }

        fn arb_assertion() -> impl Strategy<Value = BeliefContextAssertion> {
            (
                proptest::option::of("rev-[a-f0-9]{4}"),
                arb_status(),
                arb_confidence(),
                any::<bool>(),
                any::<bool>(),
                prop::collection::vec("ev-[a-f0-9]{3}", 0..3),
                prop::collection::vec("ev-[a-f0-9]{3}", 0..3),
                prop::collection::vec("fact-[a-f0-9]{3}", 0..3),
                any::<u64>(),
            )
                .prop_map(
                    |(
                        revision_id,
                        status,
                        confidence,
                        stale,
                        contradicted,
                        contradicted_claims,
                        evidence_refs,
                        source_fact_refs,
                        as_of_seq,
                    )| BeliefContextAssertion {
                        revision_id,
                        status,
                        confidence,
                        stale,
                        contradicted,
                        contradicted_claims,
                        evidence_refs,
                        source_fact_refs,
                        as_of_seq,
                    },
                )
        }

        /// Hex node-id keys whose lexicographic order matches numeric order.
        fn arb_subject_key() -> impl Strategy<Value = String> {
            any::<u32>().prop_map(|value| format!("{value:064x}"))
        }

        fn arb_bundle() -> impl Strategy<Value = BeliefContextBundle> {
            (
                arb_subject_key(),
                prop::collection::btree_map(arb_subject_key(), arb_assertion(), 0..5),
                proptest::option::of(arb_assertion()),
                0usize..4,
            )
                .prop_map(
                    |(trigger, mut subject_assertions, trigger_assertion, omitted)| {
                        if let Some(assertion) = trigger_assertion {
                            subject_assertions.insert(trigger.clone(), assertion);
                        }
                        BeliefContextBundle {
                            schema_version: 1,
                            subject_node_id: trigger,
                            subject_path: "src".to_string(),
                            family_id: BELIEF_CONTEXT_FAMILY_ID.to_string(),
                            as_of_seq: high_water_seq(&subject_assertions),
                            subject_assertions,
                            omitted_subject_count: omitted,
                        }
                    },
                )
        }

        /// `(retention_rank, key)` priority; retention keeps the smallest
        /// prefix of this order, mirroring both budget functions.
        fn priority(key: &str, assertion: &BeliefContextAssertion) -> (u8, String) {
            (retention_rank(assertion), key.to_string())
        }

        #[test]
        fn canonical_json_is_byte_stable_and_roundtrips_for_arbitrary_bundles() {
            let mut runner = deterministic_runner(256);
            runner
                .run(&(arb_bundle(), arb_bundle()), |(left, right)| {
                    let left_json = left.canonical_json().unwrap();
                    prop_assert_eq!(&left_json, &left.canonical_json().unwrap());
                    let decoded = BeliefContextBundle::from_artifact_value(
                        &serde_json::from_str::<serde_json::Value>(&left_json).unwrap(),
                    )
                    .unwrap();
                    prop_assert_eq!(&decoded, &left);
                    prop_assert_eq!(&decoded.canonical_json().unwrap(), &left_json);
                    // Canonical bytes agree exactly when the bundles agree,
                    // so digest equality tracks semantic equality.
                    let right_json = right.canonical_json().unwrap();
                    prop_assert_eq!(left == right, left_json == right_json);
                    Ok(())
                })
                .unwrap();
        }

        #[test]
        fn subject_budget_invariants_hold_for_arbitrary_assertion_maps() {
            let mut runner = deterministic_runner(48);
            runner
                .run(
                    &(
                        prop::collection::btree_map(
                            arb_subject_key(),
                            arb_assertion(),
                            // Straddles MAX_BELIEF_CONTEXT_SUBJECTS so both
                            // the no-op and truncation paths are exercised.
                            500..=540usize,
                        ),
                        arb_subject_key(),
                        proptest::option::of(arb_assertion()),
                    ),
                    |(mut input, trigger, trigger_assertion)| {
                        if let Some(assertion) = trigger_assertion {
                            input.insert(trigger.clone(), assertion);
                        }
                        let total = input.len();
                        let trigger_covered = input.contains_key(&trigger);

                        let mut retained = input.clone();
                        let omitted = apply_subject_budget(&mut retained, &trigger);

                        prop_assert_eq!(retained.len(), total.min(MAX_BELIEF_CONTEXT_SUBJECTS));
                        prop_assert_eq!(omitted, total - retained.len());
                        prop_assert_eq!(retained.contains_key(&trigger), trigger_covered);
                        for (key, assertion) in &retained {
                            prop_assert_eq!(input.get(key), Some(assertion));
                        }
                        // Class-first retention: every retained non-trigger
                        // entry outranks every omitted entry.
                        let retained_max = retained
                            .iter()
                            .filter(|(key, _)| key.as_str() != trigger)
                            .map(|(key, assertion)| priority(key, assertion))
                            .max();
                        let omitted_min = input
                            .iter()
                            .filter(|(key, _)| !retained.contains_key(*key))
                            .map(|(key, assertion)| priority(key, assertion))
                            .min();
                        if let (Some(retained_max), Some(omitted_min)) = (retained_max, omitted_min)
                        {
                            prop_assert!(retained_max < omitted_min);
                        }
                        // Pure function: rerunning on the same input
                        // reproduces the retained set and omitted count.
                        let mut rerun = input.clone();
                        prop_assert_eq!(apply_subject_budget(&mut rerun, &trigger), omitted);
                        prop_assert_eq!(&rerun, &retained);
                        Ok(())
                    },
                )
                .unwrap();
        }

        #[test]
        fn byte_budget_invariants_hold_for_arbitrary_bundles() {
            // Padding pushes serialized bundles both under and far over the
            // byte budget; case count stays low because each case serializes
            // up to a megabyte-scale bundle.
            let arb_padded_assertion = || {
                (arb_assertion(), 0usize..200_000).prop_map(|(mut assertion, padding)| {
                    assertion.evidence_refs.push("e".repeat(padding));
                    assertion
                })
            };
            let mut runner = deterministic_runner(16);
            runner
                .run(
                    &(
                        arb_subject_key(),
                        proptest::option::of(arb_padded_assertion()),
                        prop::collection::btree_map(
                            arb_subject_key(),
                            arb_padded_assertion(),
                            0..8,
                        ),
                    ),
                    |(trigger, trigger_assertion, mut assertions)| {
                        if let Some(assertion) = trigger_assertion {
                            assertions.insert(trigger.clone(), assertion);
                        }
                        let trigger_covered = assertions.contains_key(&trigger);
                        let initial = assertions.clone();
                        let mut bundle = BeliefContextBundle {
                            schema_version: 1,
                            subject_node_id: trigger.clone(),
                            subject_path: "src".to_string(),
                            family_id: BELIEF_CONTEXT_FAMILY_ID.to_string(),
                            as_of_seq: high_water_seq(&assertions),
                            subject_assertions: assertions,
                            omitted_subject_count: 0,
                        };
                        let rerun_input = bundle.clone();

                        apply_bundle_byte_budget(&mut bundle).unwrap();

                        // The serialized bundle fits unless only the trigger
                        // subject (or nothing) is left to keep.
                        let byte_budget = MAX_CONTEXT_ARTIFACT_BYTES - BELIEF_BUNDLE_BYTE_MARGIN;
                        let only_trigger_left =
                            bundle.subject_assertions.keys().all(|key| key == &trigger);
                        prop_assert!(
                            bundle.canonical_json().unwrap().len() <= byte_budget
                                || only_trigger_left
                        );
                        prop_assert_eq!(
                            bundle.subject_assertions.contains_key(&trigger),
                            trigger_covered
                        );
                        prop_assert_eq!(
                            bundle.omitted_subject_count,
                            initial.len() - bundle.subject_assertions.len()
                        );
                        for (key, assertion) in &bundle.subject_assertions {
                            prop_assert_eq!(initial.get(key), Some(assertion));
                        }
                        prop_assert_eq!(
                            bundle.as_of_seq,
                            high_water_seq(&bundle.subject_assertions)
                        );
                        // Evictions follow the same class-first priority as
                        // the count budget.
                        let retained_max = bundle
                            .subject_assertions
                            .iter()
                            .filter(|(key, _)| key.as_str() != trigger)
                            .map(|(key, assertion)| priority(key, assertion))
                            .max();
                        let evicted_min = initial
                            .iter()
                            .filter(|(key, _)| !bundle.subject_assertions.contains_key(*key))
                            .map(|(key, assertion)| priority(key, assertion))
                            .min();
                        if let (Some(retained_max), Some(evicted_min)) = (retained_max, evicted_min)
                        {
                            prop_assert!(retained_max < evicted_min);
                        }
                        // Pure function: rerunning on the same input
                        // reproduces the trimmed bundle.
                        let mut rerun = rerun_input;
                        apply_bundle_byte_budget(&mut rerun).unwrap();
                        prop_assert_eq!(&rerun, &bundle);
                        Ok(())
                    },
                )
                .unwrap();
        }
    }
}
