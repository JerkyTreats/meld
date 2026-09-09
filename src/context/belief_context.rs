//! Retained belief-context artifact contracts for bounded Context capabilities.
//!
//! Prompt assembly consumes only the explicitly supplied bundle. The retired
//! live-store hydration API and its implicit family selection are absent.
//! Bundle decoding preserves historical bytes and prompt-selection behavior.

use crate::error::ApiError;
use meld_execution::BeliefStatusLabel;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Artifact type id for the seeded belief context bundle.
pub const BELIEF_CONTEXT_BUNDLE_ARTIFACT_TYPE_ID: &str = "belief_context_bundle";

/// Retained belief context captured for a generated Context artifact.
/// Historical prompt artifacts keep their original subject and currency.
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
    /// by hex-encoded node id. The trigger
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

    const BELIEF_CONTEXT_FAMILY_ID: &str = "docs_freshness";

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

    // Preserve historical artifact round-trip and byte identity.
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
                            as_of_seq: subject_assertions
                                .values()
                                .map(|assertion| assertion.as_of_seq)
                                .max()
                                .unwrap_or(0),
                            subject_assertions,
                            omitted_subject_count: omitted,
                        }
                    },
                )
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
    }
}
