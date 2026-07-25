//! Realization-route selection from the injected available-action set.
//!
//! Owner: execution planning. The stewardship package affords actions; root
//! resolves them into the frozen [`AvailableActionSet`] and injects it
//! together with a data-driven method-to-action association. This module
//! only selects: it matches a composed method to its afforded action and,
//! for the task-package workflow route, validates the selection against the
//! existing built-in package registry and produces a handoff artifact.
//!
//! It does not own package compilation, dispatch, provider execution, or
//! outcome interpretation. The package's own traversal, fan-out, and
//! publication semantics stay inside the task package machinery that
//! consumes the handoff through the `WorkflowPackageTriggerRequest`
//! compatibility surface.

use crate::planning::action::{
    ActionArtifactMeaning, ActionRealizationRoute, AvailableActionBinding, AvailableActionSet,
};
use crate::planning::contracts::ExecutionComposition;
use crate::planning::world_state::PlanningWorldStateFrameRef;
use crate::task::package::load_builtin_task_package_spec;
use crate::task_network::AGGREGATE_OUTCOME_CONTRACT_ID;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Data-driven association from one method to one afforded action.
///
/// Associations are supplied by the caller from configuration or fixtures.
/// Generic planning code only ever compares these ids; domain vocabulary
/// such as the docs writer package never appears in planning modules.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MethodRealizationBinding {
    /// Method whose composed plan realizes through the associated action.
    pub method_id: String,
    /// Action identity inside the injected available-action set.
    pub action_id: String,
}

/// Deterministic failure while selecting or validating a realization route.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum RealizationSelectionError {
    /// One method was associated with more than one action.
    #[error("method '{method_id}' has more than one action association")]
    AmbiguousAssociation {
        /// Method with conflicting associations.
        method_id: String,
    },
    /// The associated action id was absent from the available-action set.
    #[error("action '{action_id}' associated with method '{method_id}' is not available")]
    ActionUnavailable {
        /// Method whose association could not be resolved.
        method_id: String,
        /// Action id missing from the injected set.
        action_id: String,
    },
    /// The routed package id did not resolve to a built-in task package.
    #[error("action '{action_id}' routes to unknown task package '{package_id}': {message}")]
    PackageUnknown {
        /// Action whose route failed validation.
        action_id: String,
        /// Package id that could not be loaded.
        package_id: String,
        /// Loader failure description.
        message: String,
    },
    /// The routed workflow id disagreed with the package's own declaration.
    #[error(
        "action '{action_id}' routes package '{package_id}' through workflow \
         '{route_workflow_id}' but the package declares '{package_workflow_id}'"
    )]
    WorkflowMismatch {
        /// Action whose route failed validation.
        action_id: String,
        /// Package id selected by the route.
        package_id: String,
        /// Workflow id carried by the route.
        route_workflow_id: String,
        /// Workflow id the package document declares.
        package_workflow_id: String,
    },
    /// A package-routed action declared a non-aggregate outcome contract.
    #[error(
        "action '{action_id}' rides the package route but publishes outcome \
         contract '{outcome_contract_id}' instead of '{expected}'",
        expected = AGGREGATE_OUTCOME_CONTRACT_ID
    )]
    OutcomeContractMismatch {
        /// Action whose outcome contract failed validation.
        action_id: String,
        /// Outcome contract the action declared.
        outcome_contract_id: String,
    },
}

/// Select the afforded action associated with one composed method.
///
/// Returns `Ok(None)` when the method has no association, which keeps the
/// existing operator-capability lowering path. Ambiguous associations and
/// associations naming an unavailable action fail deterministically.
pub fn select_action_for_method<'a>(
    method_id: &str,
    associations: &[MethodRealizationBinding],
    actions: &'a AvailableActionSet,
) -> Result<Option<&'a AvailableActionBinding>, RealizationSelectionError> {
    let mut matched = associations
        .iter()
        .filter(|binding| binding.method_id == method_id);
    let Some(binding) = matched.next() else {
        return Ok(None);
    };
    if matched.next().is_some() {
        return Err(RealizationSelectionError::AmbiguousAssociation {
            method_id: method_id.to_string(),
        });
    }
    actions
        .actions
        .iter()
        .find(|action| action.action_id == binding.action_id)
        .map(Some)
        .ok_or_else(|| RealizationSelectionError::ActionUnavailable {
            method_id: method_id.to_string(),
            action_id: binding.action_id.clone(),
        })
}

/// Handoff artifact for one plan realized through the task-package route.
///
/// Planning stops at selection: this artifact names the built-in package
/// and workflow route by id so the dispatch boundary can trigger the
/// package through the existing `WorkflowPackageTriggerRequest`
/// compatibility surface. No package graph is compiled here and no task
/// network mutation is proposed; the package owns its own traversal,
/// fan-out, and dependency semantics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskPackageRoutePlan {
    /// Deterministic plan identity derived from the goal, the selected
    /// action route, and the exact projection frame identity. Identical
    /// planning input reproduces this id byte for byte; a revised frame
    /// produces a causally distinct id.
    pub plan_id: String,
    /// Target task network the eventual package outcome binds to.
    pub network_id: String,
    /// Composition the selected method produced.
    pub composition_id: String,
    /// Goal the plan intends to satisfy.
    pub goal_id: String,
    /// Method whose realization selected the package route.
    pub method_id: String,
    /// Afforded action realized by this plan.
    pub action_id: String,
    /// Built-in task package selected by id.
    pub package_id: String,
    /// Workflow route validated against the package document.
    pub workflow_id: String,
    /// Canonical outcome contract the package run publishes.
    pub outcome_contract_id: String,
    /// Artifact meaning the afforded action commits to.
    pub artifact: ActionArtifactMeaning,
    /// Exact projection frame the plan was derived from.
    pub world_state_frame: PlanningWorldStateFrameRef,
}

/// Validate a package-routed action and build its handoff artifact.
///
/// The route is validated against the real built-in registry: the package
/// must load by id and its declared workflow must match the routed
/// workflow. Package-routed actions must publish the canonical aggregate
/// outcome contract because that is the only outcome the package route
/// produces for evidence mapping.
pub fn prepare_task_package_route(
    network_id: &str,
    composition: &ExecutionComposition,
    action: &AvailableActionBinding,
) -> Result<TaskPackageRoutePlan, RealizationSelectionError> {
    let ActionRealizationRoute::TaskPackageWorkflow {
        package_id,
        workflow_id,
    } = &action.realization
    else {
        unreachable!("prepare_task_package_route requires a package-routed action");
    };

    if action.outcome_contract.outcome_contract_id != AGGREGATE_OUTCOME_CONTRACT_ID {
        return Err(RealizationSelectionError::OutcomeContractMismatch {
            action_id: action.action_id.clone(),
            outcome_contract_id: action.outcome_contract.outcome_contract_id.clone(),
        });
    }
    let spec = load_builtin_task_package_spec(package_id).map_err(|error| {
        RealizationSelectionError::PackageUnknown {
            action_id: action.action_id.clone(),
            package_id: package_id.clone(),
            message: error.to_string(),
        }
    })?;
    if &spec.workflow_id != workflow_id {
        return Err(RealizationSelectionError::WorkflowMismatch {
            action_id: action.action_id.clone(),
            package_id: package_id.clone(),
            route_workflow_id: workflow_id.clone(),
            package_workflow_id: spec.workflow_id,
        });
    }

    Ok(TaskPackageRoutePlan {
        plan_id: package_route_plan_id(network_id, composition, action, package_id, workflow_id),
        network_id: network_id.to_string(),
        composition_id: composition.composition_id.clone(),
        goal_id: composition.goal.goal_id.clone(),
        method_id: composition.method_id.clone(),
        action_id: action.action_id.clone(),
        package_id: package_id.clone(),
        workflow_id: workflow_id.clone(),
        outcome_contract_id: action.outcome_contract.outcome_contract_id.clone(),
        artifact: action.artifact.clone(),
        world_state_frame: composition.world_state_frame.clone(),
    })
}

fn package_route_plan_id(
    network_id: &str,
    composition: &ExecutionComposition,
    action: &AvailableActionBinding,
    package_id: &str,
    workflow_id: &str,
) -> String {
    #[derive(Serialize)]
    struct Identity<'a> {
        network_id: &'a str,
        goal_id: &'a str,
        method_id: &'a str,
        action_id: &'a str,
        package_id: &'a str,
        workflow_id: &'a str,
        composition_id: &'a str,
        frame_id: &'a str,
        projection_version: &'a str,
        source_refs: &'a [String],
    }

    let frame = &composition.world_state_frame;
    let bytes = serde_json::to_vec(&Identity {
        network_id,
        goal_id: &composition.goal.goal_id,
        method_id: &composition.method_id,
        action_id: &action.action_id,
        package_id,
        workflow_id,
        composition_id: &composition.composition_id,
        frame_id: &frame.frame_id,
        projection_version: &frame.projection_version,
        source_refs: &frame.source_refs,
    })
    .expect("package route plan identity is serializable");
    format!(
        "execution-package-route-plan-{}",
        blake3::hash(&bytes).to_hex()
    )
}
