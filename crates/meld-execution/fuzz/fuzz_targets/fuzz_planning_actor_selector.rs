#![no_main]

use libfuzzer_sys::fuzz_target;
use meld_events::DomainObjectRef;
use meld_execution::{
    capability::CapabilityCatalog,
    goals::{AddGoalCommand, GoalCommandMetadata, PersistentGoalSetStore},
    planning::{
        ExecutionCompositionLowerer, MethodLibrary, PlanningPerspectiveRef,
        PlanningProjectionError, PlanningRuntime, PlanningRuntimeActor,
        PlanningRuntimeActorRequest,
    },
    task::TaskCompiler,
    task_network::{store::TaskNetworkStoreFactory, TaskNetworkAuthority},
};
use meld_lang::{Goal, GoalLifecycle, GoalPriority, GoalSource, Proposition, Term};
use std::collections::BTreeSet;
use std::sync::{Arc, Barrier};

fn planning_actor() -> PlanningRuntimeActor<meld_execution::task::TaskCompiler> {
    let catalog = CapabilityCatalog::new();
    let runtime = PlanningRuntime::new(
        MethodLibrary::from_methods(Vec::new(), &catalog),
        catalog.clone(),
    );
    PlanningRuntimeActor::new(
        runtime,
        ExecutionCompositionLowerer::new(TaskCompiler::new(), catalog),
    )
}

fn goal(index: usize, active: bool, urgency: u8) -> Goal {
    Goal {
        goal_id: format!("goal-{index}"),
        agent_id: "agent-fuzz".to_string(),
        target: Proposition::Accessible {
            scope: Term::Object(
                DomainObjectRef::new("workspace", "node", format!("node-{index}")).unwrap(),
            ),
        },
        priority: GoalPriority {
            urgency: urgency.into(),
            cost_ceiling: None,
        },
        source: GoalSource::UserDirected {
            directive: "fuzz bounded planning selection".to_string(),
        },
        lifecycle: if active {
            GoalLifecycle::Active
        } else {
            GoalLifecycle::Suspended {
                reason: "inactive fuzz history".to_string(),
            }
        },
    }
}

fuzz_target!(|data: &[u8]| {
    if data.is_empty() {
        return;
    }
    let active_count = usize::from(data[0] % 8) + 1;
    let inactive_count = data.get(1).copied().unwrap_or(0) as usize % 8;
    let limit = usize::from(data.get(2).copied().unwrap_or(0) % 4) + 1;
    let goal_db = sled::Config::new().temporary(true).open().unwrap();
    let goals = PersistentGoalSetStore::new(goal_db).unwrap();
    for index in 0..active_count + inactive_count {
        let urgency = data.get(index + 3).copied().unwrap_or(index as u8);
        goals
            .add_goal(AddGoalCommand {
                metadata: GoalCommandMetadata {
                    command_id: format!("command-add-{index}"),
                    source_identity: None,
                    seq: u64::try_from(index + 1).unwrap(),
                },
                goal: goal(index, index < active_count, urgency),
            })
            .unwrap();
    }

    let actor = planning_actor();
    let temp = tempfile::tempdir().unwrap();
    let factory = TaskNetworkStoreFactory::new(temp.path());
    let mut authority = TaskNetworkAuthority::open(&factory, "network-fuzz", 8).unwrap();
    let ports = authority.ports();
    let request = PlanningRuntimeActorRequest {
        network_id: "network-fuzz".to_string(),
        perspective: PlanningPerspectiveRef::new("agent", "default").unwrap(),
        branch_id: "main".to_string(),
        requested_dimensions: Vec::new(),
        required_preconditions: Vec::new(),
        limit: Some(limit),
    };
    let mut projection = |_| Err(PlanningProjectionError::retryable("fuzz projection"));
    let first = actor
        .run_once(&goals, &ports, &mut projection, request.clone())
        .unwrap();
    let second = actor
        .run_once(&goals, &ports, &mut projection, request.clone())
        .unwrap();

    assert_eq!(first.active_goal_count, active_count);
    assert_eq!(second.active_goal_count, active_count);
    assert_eq!(first.attempted, active_count.min(limit));
    assert_eq!(second.attempted, active_count.min(limit));
    let first_ids = first
        .results
        .iter()
        .map(|result| format!("{result:?}"))
        .collect::<BTreeSet<_>>();
    let second_ids = second
        .results
        .iter()
        .map(|result| format!("{result:?}"))
        .collect::<BTreeSet<_>>();
    if active_count > limit {
        assert_ne!(first_ids, second_ids);
    }

    if active_count >= 2 {
        let barrier = Arc::new(Barrier::new(2));
        let handles = (0..2)
            .map(|_| {
                let goals = goals.clone();
                let ports = ports.clone();
                let barrier = Arc::clone(&barrier);
                let mut concurrent_request = request.clone();
                concurrent_request.limit = Some(1);
                std::thread::spawn(move || {
                    let actor = planning_actor();
                    let mut projection = move |_| {
                        barrier.wait();
                        Err(PlanningProjectionError::retryable(
                            "fuzz concurrent projection",
                        ))
                    };
                    actor
                        .run_once(&goals, &ports, &mut projection, concurrent_request)
                        .unwrap()
                })
            })
            .collect::<Vec<_>>();
        let concurrent = handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(concurrent[0].attempted, 1);
        assert_eq!(concurrent[1].attempted, 1);
        assert_ne!(
            format!("{:?}", concurrent[0].results[0]),
            format!("{:?}", concurrent[1].results[0])
        );
    }
    authority.shutdown().unwrap();
});
