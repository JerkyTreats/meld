//! Bounded-step adaptation for supervised runtime handles.
//!
//! Owner: root runtime supervisor. Every supervised active actor is driven
//! through the public `ActorBoundedStep` contract; this module adapts the
//! existing bespoke handle tick entry behind it. The supervisor invokes
//! exactly one bounded step per active actor per maintenance pass, and a
//! step always yields a report — a missing report is a contract failure the
//! supervisor projects as a fatal step, never as health.

use crate::runtime::assembly::{
    InertRuntimeHandle, RuntimeHandleFlushReport, RuntimeHandleSafePointReport,
    RuntimeHandleStopReport,
};
use crate::runtime::contracts::{
    ActorBoundedStep, ActorBoundedStepError, WorkBudget, WorkerTickReport,
};
use crate::runtime::error::RuntimeAssemblyError;

/// One supervised active actor driven through the bounded-step contract.
///
/// Owns the started runtime handle and forwards the lifecycle hooks the
/// supervisor needs around stepping. Construction does not start the handle;
/// the supervisor starts it after lease acquisition, before wrapping.
pub struct BoundedActorHandle {
    runtime_id: String,
    handle: InertRuntimeHandle,
}

impl BoundedActorHandle {
    /// Wrap one runtime handle as a bounded-step actor.
    pub fn new(handle: InertRuntimeHandle) -> Self {
        Self {
            runtime_id: handle.runtime_id().to_string(),
            handle,
        }
    }

    /// Return whether the handle carries a concrete semantic body.
    pub fn has_semantic_body(&self) -> bool {
        self.handle.has_semantic_body()
    }

    /// Return whether the supervisor has started the handle.
    pub fn is_started(&self) -> bool {
        self.handle.is_started()
    }

    /// Request that the actor stop at its next safe point.
    pub fn request_stop(&mut self) -> RuntimeHandleStopReport {
        self.handle.request_stop()
    }

    /// Wait for the actor's safe point.
    pub fn wait_for_safe_point(&self) -> RuntimeHandleSafePointReport {
        self.handle.wait_for_safe_point()
    }

    /// Flush per-handle resources.
    pub fn flush_resources(&self) -> Result<RuntimeHandleFlushReport, RuntimeAssemblyError> {
        self.handle.flush_resources()
    }
}

impl ActorBoundedStep for BoundedActorHandle {
    fn actor_id(&self) -> &str {
        &self.runtime_id
    }

    /// Run one bounded step.
    ///
    /// The injected time satisfies the no-wall-clock contract; the currently
    /// adapted handles derive their positions from domain stores and do not
    /// consume it yet.
    fn bounded_step(
        &mut self,
        _now_ms: u64,
        budget: &WorkBudget,
    ) -> Result<WorkerTickReport, ActorBoundedStepError> {
        match self.handle.tick(budget.clone()) {
            Some(report) => Ok(report),
            // A supervised active actor always has a semantic body, so this
            // arm is a broken-contract surface: it is a non-retryable step
            // failure, and the caller must never project it as health.
            None => Err(ActorBoundedStepError {
                message: format!(
                    "runtime '{}' produced no bounded tick report",
                    self.runtime_id
                ),
                retryable: false,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::runtime::assembly::{ProductRuntimeAssembly, RuntimeLeaseContext};

    use super::*;

    #[test]
    fn bound_actor_steps_through_the_public_contract() {
        let temp = tempfile::tempdir().unwrap();
        let assembly = ProductRuntimeAssembly::load_for_product_root(temp.path()).unwrap();
        let mut handle = assembly
            .handle_factories()
            .get("event.append")
            .unwrap()
            .build_handle();
        handle
            .start_after_lease(RuntimeLeaseContext {
                runtime_id: "event.append".to_string(),
                lease_id: "lease-a".to_string(),
            })
            .unwrap();
        let mut actor = BoundedActorHandle::new(handle);

        let report = actor
            .bounded_step(100, &WorkBudget { max_items: 8 })
            .unwrap();

        assert_eq!(actor.actor_id(), "event.append");
        assert!(actor.has_semantic_body());
        assert_eq!(report.actor_id, "event.append");
        assert!(report.fatal_errors.is_empty());
    }

    #[test]
    fn body_less_handle_fails_the_step_instead_of_reporting_health() {
        let temp = tempfile::tempdir().unwrap();
        let assembly = ProductRuntimeAssembly::load_for_product_root(temp.path()).unwrap();
        let mut handle = assembly
            .handle_factories()
            .get("execution.task_admission")
            .unwrap()
            .build_handle();
        handle
            .start_after_lease(RuntimeLeaseContext {
                runtime_id: "execution.task_admission".to_string(),
                lease_id: "lease-a".to_string(),
            })
            .unwrap();
        let mut actor = BoundedActorHandle::new(handle);

        let error = actor
            .bounded_step(100, &WorkBudget { max_items: 8 })
            .unwrap_err();

        assert!(!actor.has_semantic_body());
        assert!(!error.retryable);
        assert!(error.message.contains("no bounded tick report"));
    }
}
