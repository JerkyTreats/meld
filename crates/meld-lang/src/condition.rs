//! Proposition condition operators.
//!
//! Owner: planning language.
//! Inputs: query and stored belief values expressed as terms.
//! Outputs: comparison contracts used by evaluation, matching, validation, and
//! substitution.

use serde::{Deserialize, Serialize};

use crate::term::Term;

/// A comparison over a belief dimension value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Condition {
    /// The stored value must be greater than this threshold.
    Above(Term),
    /// The stored value must be less than this threshold.
    Below(Term),
    /// The stored value must exactly equal this term.
    Equals(Term),
    /// The stored value must be present in this set.
    In(Vec<Term>),
    /// The stored value must be less than this range bound.
    Within(Term),
    /// The stored value must be greater than this range bound.
    Exceeds(Term),
    /// Any value must be observed for the dimension.
    Present,
    /// No value must be observed for the dimension.
    Absent,
}

impl Condition {
    /// Returns true when every term in this condition is ground.
    pub fn is_ground(&self) -> bool {
        self.terms().iter().all(|term| term.is_ground())
    }

    /// Returns the first variable or derived marker that prevents grounding.
    pub fn grounding_issue(&self) -> Option<String> {
        self.terms().into_iter().find_map(Term::grounding_issue)
    }

    pub(crate) fn terms(&self) -> Vec<&Term> {
        match self {
            Condition::Above(term)
            | Condition::Below(term)
            | Condition::Equals(term)
            | Condition::Within(term)
            | Condition::Exceeds(term) => vec![term],
            Condition::In(terms) => terms.iter().collect(),
            Condition::Present | Condition::Absent => Vec::new(),
        }
    }
}
