use serde::{Deserialize, Serialize};

use crate::{
    composition::Composition, cost::CostEstimate, effect::Effect, proposition::Proposition,
};

/// Reusable composition template with a trigger pattern.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Method {
    /// Stable method identity.
    pub method_id: String,
    /// Proposition pattern that selects this method.
    pub trigger: Proposition,
    /// Additional conditions required for applicability.
    pub preconditions: Vec<Proposition>,
    /// Composition template to instantiate.
    pub composition: Composition,
    /// Projected net effects of the method.
    pub net_effects: Vec<Effect>,
    /// Estimated total cost.
    pub cost: CostEstimate,
    /// Preference ordering where lower is preferred.
    pub preference: u32,
}
