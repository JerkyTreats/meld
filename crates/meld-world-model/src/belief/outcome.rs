//! Interpretation of execution's published outcomes as epistemic evidence.
//!
//! Owner: world model. Execution owns canonical publication outcomes;
//! deciding what they mean as evidence is world-model theory. The seam
//! splits in two: [`mapping`] is the frozen contract boundary — the
//! disposition vocabulary, the mapping trait, and the deterministic
//! promoted-evidence identity — and [`interpretation`] is the config-driven
//! implementation whose meaning arrives entirely as installed mapping data.

/// Config-driven outcome mapping implementation and its data vocabulary.
pub mod interpretation;
/// Frozen outcome-to-evidence contract: trait, dispositions, identity.
pub mod mapping;
