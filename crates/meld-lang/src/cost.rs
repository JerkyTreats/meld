use serde::{Deserialize, Serialize};

use crate::composition::{Composition, StepKind};

/// Multi-dimensional cost estimate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CostEstimate {
    /// Estimated wall-clock time in milliseconds.
    pub time_ms: u64,
    /// Estimated monetary cost in microdollars.
    pub money_microdollars: u64,
    /// Estimated provider call count.
    pub provider_calls: u32,
}

impl CostEstimate {
    /// Sum two estimates dimension-wise.
    pub fn add(&self, other: &CostEstimate) -> CostEstimate {
        CostEstimate {
            time_ms: self.time_ms + other.time_ms,
            money_microdollars: self.money_microdollars + other.money_microdollars,
            provider_calls: self.provider_calls + other.provider_calls,
        }
    }

    /// Returns true when any dimension exceeds the ceiling.
    pub fn exceeds(&self, ceiling: &CostEstimate) -> bool {
        self.time_ms > ceiling.time_ms
            || self.money_microdollars > ceiling.money_microdollars
            || self.provider_calls > ceiling.provider_calls
    }

    /// Sum operator costs across a composition.
    pub fn aggregate(composition: &Composition) -> CostEstimate {
        composition
            .steps
            .iter()
            .filter_map(|step| match &step.kind {
                StepKind::Op(operator) => Some(&operator.cost),
                StepKind::Goal(_) => None,
            })
            .fold(CostEstimate::zero(), |total, cost| total.add(cost))
    }

    /// Zero cost in all dimensions.
    pub fn zero() -> CostEstimate {
        CostEstimate {
            time_ms: 0,
            money_microdollars: 0,
            provider_calls: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cost_algebra_adds_and_compares_dimensions() {
        let a = CostEstimate {
            time_ms: 5,
            money_microdollars: 10,
            provider_calls: 1,
        };
        let b = CostEstimate {
            time_ms: 7,
            money_microdollars: 2,
            provider_calls: 3,
        };

        assert_eq!(
            a.add(&b),
            CostEstimate {
                time_ms: 12,
                money_microdollars: 12,
                provider_calls: 4
            }
        );
        assert_eq!(a.add(&CostEstimate::zero()), a);
        assert!(a.exceeds(&CostEstimate {
            time_ms: 4,
            money_microdollars: 100,
            provider_calls: 9
        }));
    }
}
