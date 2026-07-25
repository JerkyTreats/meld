//! Affordance-shaped available-action binding consumed by planning.
//!
//! Owner: execution. The stewardship package declares what actions the
//! domain affords; root resolves that declaration into this typed binding
//! and injects it. Execution owns the shape and how planning realizes an
//! action; it never interprets belief confidence or satisfaction policy.
//!
//! The shape is frozen affordance-shaped — action identity, artifact
//! meaning, outcome contract reference, and realization route — so it can
//! later publish as a standalone semantic action affordance in the Strategy
//! catalog without rework.

use serde::{Deserialize, Serialize};

/// One action the stewarded domain affords, bound for execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AvailableActionBinding {
    /// Stable action identity within the stewardship package.
    pub action_id: String,
    /// What artifact the action produces and what it means.
    pub artifact: ActionArtifactMeaning,
    /// Canonical outcome contract the action's completion publishes.
    pub outcome_contract: ActionOutcomeContractRef,
    /// How execution realizes the action.
    pub realization: ActionRealizationRoute,
}

/// Domain meaning of the artifact an action produces.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionArtifactMeaning {
    /// Artifact type produced by the action.
    pub artifact_type_id: String,
    /// Artifact schema version the producer commits to.
    pub schema_version: u32,
}

/// Reference to the canonical outcome contract an action publishes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionOutcomeContractRef {
    /// Identity of the outcome contract, such as
    /// `execution.package.aggregate.v1`.
    pub outcome_contract_id: String,
}

/// Route through which execution realizes an available action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionRealizationRoute {
    /// Compatibility route through an existing built-in task package and
    /// workflow. The docs writer rides this route in the first slice.
    TaskPackageWorkflow {
        /// Built-in task package identity.
        package_id: String,
        /// Workflow route identity.
        workflow_id: String,
    },
    /// Native route through an operator resolved to a capability. The
    /// Strategy affordance catalog publishes actions in this form.
    OperatorCapability {
        /// Operator identity in the method library.
        operator_id: String,
        /// Resolved capability type.
        capability_type_id: String,
        /// Resolved capability version.
        capability_version: u32,
    },
}

/// The set of available actions injected into planning.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AvailableActionSet {
    /// Actions the stewarded domain affords, in stable order.
    pub actions: Vec<AvailableActionBinding>,
}
