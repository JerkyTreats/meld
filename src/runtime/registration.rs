//! Runtime registration identity and lifecycle projection contracts.
//!
//! Owner: root runtime. A registration is a required binding derived from
//! the selected stewardship expression and physical binding — distinct
//! from the descriptor catalog, which is an internal inventory with no
//! product cardinality meaning. Catalog-only descriptors receive no
//! registration and no lifecycle state; unavailable status is reserved for
//! a required binding that cannot be resolved.
//!
//! Registration-set composition is a public surface: the
//! stewardship-derived set is one producer, and harness or proof callers
//! may supply an explicit set to compose any actor subset. Store and port
//! opening is scoped to the composed set.
//!
//! The supervisor-truth workstream binds classification and lifecycle
//! projection here; factory binding stays with the actor-binding workstream.

use serde::{Deserialize, Serialize};

use crate::runtime::assembly::RuntimeResource;
use crate::runtime::contracts::WorkerTickReport;

/// What kind of runtime participant a registration binds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RegistrationKind {
    /// A bounded actor the supervisor ticks. Holds a lease and reports.
    ActiveActor,
    /// A passive capability other actors call. Never leased, never ticked,
    /// never assigned actor health.
    PassiveService,
}

/// One required runtime binding derived from composition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeRegistration {
    /// Root-local registration identity, used in lifecycle projection and
    /// diagnostics only. Durable correlation prefers existing session,
    /// stream, subject, event, and actor identities.
    pub registration_id: String,
    /// Internal runtime role this registration binds.
    pub runtime_id: String,
    /// Active actor or passive service.
    pub kind: RegistrationKind,
    /// Resources the binding requires before it can resolve.
    pub required_resources: Vec<RuntimeResource>,
}

/// An explicit set of registrations composing one runtime assembly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegistrationSet {
    /// Registrations in stable order.
    pub registrations: Vec<RuntimeRegistration>,
}

impl RegistrationSet {
    /// Return the registration for one runtime id when the set names it.
    pub fn get(&self, runtime_id: &str) -> Option<&RuntimeRegistration> {
        self.registrations
            .iter()
            .find(|registration| registration.runtime_id == runtime_id)
    }

    /// Return the declared kind for one runtime id when the set names it.
    pub fn kind_of(&self, runtime_id: &str) -> Option<RegistrationKind> {
        self.get(runtime_id).map(|registration| registration.kind)
    }

    /// Intersect this set with an operator's desired runtime ids.
    ///
    /// This is the contract every supervisor boot composes through: the
    /// complete selection leaves the set intact, a runtime-id subset keeps
    /// only registrations naming a desired id, and an empty intersection
    /// is `None` so the supervisor never sees a registration for an absent
    /// desired runtime.
    pub fn intersect_desired_runtime_ids<'a>(
        &self,
        desired_runtime_ids: impl IntoIterator<Item = &'a str>,
    ) -> Option<RegistrationSet> {
        let desired: std::collections::BTreeSet<&str> = desired_runtime_ids.into_iter().collect();
        let registrations = self
            .registrations
            .iter()
            .filter(|registration| desired.contains(registration.runtime_id.as_str()))
            .cloned()
            .collect::<Vec<_>>();
        if registrations.is_empty() {
            return None;
        }
        Some(RegistrationSet { registrations })
    }
}

/// Truthful lifecycle projection for one registration.
///
/// Projected by root from domain-owned reports; domains never depend on
/// this shape. A missing report is never projected as healthy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RegistrationLifecycle {
    /// A required resource or factory cannot be resolved. Never reported
    /// as healthy.
    UnresolvedRequiredBinding,
    /// The binding resolved and the participant is starting.
    Starting,
    /// The actor's last bounded tick committed work.
    ActiveWorking,
    /// The actor's last bounded tick truthfully found no eligible work.
    ActiveIdle,
    /// The actor's last bounded tick reported fatal errors.
    Unhealthy,
    /// The participant stopped and holds no lease.
    Stopped,
}

impl RegistrationLifecycle {
    /// Project one bounded tick report onto the registration lifecycle.
    ///
    /// The projection mirrors the operator-facing outcome classification in
    /// `RuntimeActionOutcome::from_worker_tick`: fatal issues dominate, any
    /// retryable issue or durable progress is active work, and only a clean
    /// zero-work report is truthful active idle.
    pub fn from_worker_tick(report: &WorkerTickReport) -> Self {
        if !report.fatal_errors.is_empty() {
            Self::Unhealthy
        } else if !report.retryable_errors.is_empty() || report.made_progress() {
            Self::ActiveWorking
        } else {
            Self::ActiveIdle
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::contracts::{WorkerCheckpoint, WorkerScope, WorkerTickIssue};

    fn zero_work_report() -> WorkerTickReport {
        WorkerTickReport {
            actor_id: "events.append".to_string(),
            scope: WorkerScope {
                domain_id: "events".to_string(),
                stream_id: None,
                work_key: None,
                agent_id: None,
                perspective_key: None,
                branch_id: None,
                subject_key: None,
            },
            input_checkpoint: WorkerCheckpoint {
                name: "event_commit_watermark".to_string(),
                value: 5,
            },
            output_checkpoint: WorkerCheckpoint {
                name: "event_commit_watermark".to_string(),
                value: 5,
            },
            items_attempted: 0,
            items_committed: 0,
            retryable_errors: Vec::new(),
            fatal_errors: Vec::new(),
            budget_exhausted: false,
        }
    }

    #[test]
    fn zero_work_report_projects_active_idle() {
        assert_eq!(
            RegistrationLifecycle::from_worker_tick(&zero_work_report()),
            RegistrationLifecycle::ActiveIdle
        );
    }

    #[test]
    fn progress_and_retryable_issues_project_active_working() {
        let mut progressed = zero_work_report();
        progressed.output_checkpoint.value = 6;
        assert_eq!(
            RegistrationLifecycle::from_worker_tick(&progressed),
            RegistrationLifecycle::ActiveWorking
        );

        let mut retryable = zero_work_report();
        retryable.retryable_errors.push(WorkerTickIssue {
            item_id: None,
            code: "retryable_io".to_string(),
            message: "retryable io".to_string(),
        });
        assert_eq!(
            RegistrationLifecycle::from_worker_tick(&retryable),
            RegistrationLifecycle::ActiveWorking
        );
    }

    #[test]
    fn fatal_report_projects_unhealthy_even_with_progress() {
        let mut fatal = zero_work_report();
        fatal.output_checkpoint.value = 6;
        fatal.fatal_errors.push(WorkerTickIssue {
            item_id: None,
            code: "fatal".to_string(),
            message: "fatal failure".to_string(),
        });
        assert_eq!(
            RegistrationLifecycle::from_worker_tick(&fatal),
            RegistrationLifecycle::Unhealthy
        );
    }

    #[test]
    fn registration_set_lookups_return_declared_kind() {
        let set = RegistrationSet {
            registrations: vec![RuntimeRegistration {
                registration_id: "registration-a".to_string(),
                runtime_id: "execution.task_network_command".to_string(),
                kind: RegistrationKind::PassiveService,
                required_resources: Vec::new(),
            }],
        };

        assert_eq!(
            set.kind_of("execution.task_network_command"),
            Some(RegistrationKind::PassiveService)
        );
        assert!(set.get("event.append").is_none());
    }
}
