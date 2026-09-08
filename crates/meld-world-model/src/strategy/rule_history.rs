//! Preserve exact installed historical theory bytes while translating their layout once.
//! Runtime construction and verification consume only local product contracts.
use super::contracts::*;
use meld_lang::Proposition;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyRule {
    /// Installed permission to repeat complete work after these owners' input changes.
    /// Empty keeps repetition dependent on a different operation or explicit inputs.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub repeat_on_changed_owners: Vec<String>,
    /// When both contracts are selected, their distinct complete Tasks must progress in this order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub task_ordering: Vec<LegacyTaskOrdering>,
    /// Goal pattern to which this rule applies.
    pub goal_pattern: Proposition,
    /// Proposition that prospective action must contribute toward.
    pub settlement_obligation: Proposition,
    /// Evidence route required after action completes.
    pub evidence_route: ProspectiveEvidenceRoute,
    /// Whether the configured epistemic products prepare work or confirm its effects.
    #[serde(default, skip_serializing_if = "LegacyPlacement::is_prerequisite")]
    pub epistemic_placement: LegacyPlacement,
}

/// A causal requirement between complete Tasks containing exact Capability contracts.
/// The consumer waits for the producer Task's accepted operational return.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
struct LegacyTaskOrdering {
    pub before_contract_id: String,
    pub after_contract_id: String,
}

/// Causal placement of configured epistemic work relative to executable realization.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum LegacyPlacement {
    #[default]
    Prerequisite,
    /// Confirmation follows the Task's operational return.
    Confirmation,
    /// Confirmation follows exact Graph visibility; operational return remains separate.
    GraphConfirmation,
}

impl LegacyPlacement {
    pub fn is_confirmation(self) -> bool {
        matches!(self, Self::Confirmation | Self::GraphConfirmation)
    }
    fn is_prerequisite(&self) -> bool {
        *self == Self::Prerequisite
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CurrentRule {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    repeat_on_changed_owners: Vec<String>,
    goal_pattern: Proposition,
    settlement_obligation: Proposition,
    evidence_route: ProspectiveEvidenceRoute,
    epistemic_selections: Vec<StrategyEpistemicSelection>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    product_ordering: Vec<StrategyProductOrdering>,
    #[serde(default)]
    construction: StrategyConstruction,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct HistoricalRule {
    legacy: LegacyRule,
    canonical: CurrentRule,
}
impl From<&StrategySettlementRule> for CurrentRule {
    fn from(rule: &StrategySettlementRule) -> Self {
        Self {
            repeat_on_changed_owners: rule.repeat_on_changed_owners.clone(),
            goal_pattern: rule.goal_pattern.clone(),
            settlement_obligation: rule.settlement_obligation.clone(),
            evidence_route: rule.evidence_route.clone(),
            epistemic_selections: rule.epistemic_selections.clone(),
            product_ordering: rule.product_ordering.clone(),
            construction: rule.construction,
        }
    }
}
impl From<CurrentRule> for StrategySettlementRule {
    fn from(wire: CurrentRule) -> Self {
        Self {
            historical_wire: None,
            repeat_on_changed_owners: wire.repeat_on_changed_owners,
            goal_pattern: wire.goal_pattern,
            settlement_obligation: wire.settlement_obligation,
            evidence_route: wire.evidence_route,
            epistemic_selections: wire.epistemic_selections,
            product_ordering: wire.product_ordering,
            construction: wire.construction,
        }
    }
}
impl<'de> Deserialize<'de> for StrategySettlementRule {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Wire {
            Current(CurrentRule),
            Legacy(LegacyRule),
        }
        match Wire::deserialize(deserializer)? {
            Wire::Current(wire) => Ok(wire.into()),
            Wire::Legacy(wire) => {
                let mut ordering = vec![match wire.epistemic_placement {
                    LegacyPlacement::Prerequisite => StrategyProductOrdering {
                        condition: crate::strategy::StrategyOrderingCondition::ConsumerSelected,
                        before: StrategyProductSelector::AllEpistemic,
                        after: StrategyProductSelector::AllTasks,
                        milestone: StrategyDependencyMilestone::CurationVisible,
                    },
                    LegacyPlacement::Confirmation => StrategyProductOrdering {
                        condition: crate::strategy::StrategyOrderingCondition::ConsumerSelected,
                        before: StrategyProductSelector::AllTasks,
                        after: StrategyProductSelector::AllEpistemic,
                        milestone: StrategyDependencyMilestone::ExecutionTerminal,
                    },
                    LegacyPlacement::GraphConfirmation => StrategyProductOrdering {
                        condition: crate::strategy::StrategyOrderingCondition::ConsumerSelected,
                        before: StrategyProductSelector::AllTasks,
                        after: StrategyProductSelector::AllEpistemic,
                        milestone: StrategyDependencyMilestone::EffectVisible,
                    },
                }];
                ordering.extend(
                    wire.task_ordering
                        .iter()
                        .map(|entry| StrategyProductOrdering {
                            condition: crate::strategy::StrategyOrderingCondition::BothSelected,
                            before: StrategyProductSelector::Task {
                                contract_id: entry.before_contract_id.clone(),
                            },
                            after: StrategyProductSelector::Task {
                                contract_id: entry.after_contract_id.clone(),
                            },
                            milestone: StrategyDependencyMilestone::ExecutionTerminal,
                        }),
                );
                let canonical = CurrentRule {
                    repeat_on_changed_owners: wire.repeat_on_changed_owners.clone(),
                    goal_pattern: wire.goal_pattern.clone(),
                    settlement_obligation: wire.settlement_obligation.clone(),
                    evidence_route: wire.evidence_route.clone(),
                    epistemic_selections: vec![StrategyEpistemicSelection {
                        rule_revision: None,
                        evidence_return: wire.epistemic_placement.is_confirmation(),
                    }],
                    product_ordering: ordering,
                    construction: StrategyConstruction::Executable,
                };
                let mut rule: StrategySettlementRule = canonical.clone().into();
                rule.historical_wire = Some(Box::new(HistoricalRule {
                    legacy: wire,
                    canonical,
                }));
                Ok(rule)
            }
        }
    }
}
impl Serialize for StrategySettlementRule {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let canonical = CurrentRule::from(self);
        if let Some(history) = &self.historical_wire {
            if canonical == history.canonical {
                return history.legacy.serialize(serializer);
            }
        }
        canonical.serialize(serializer)
    }
}
