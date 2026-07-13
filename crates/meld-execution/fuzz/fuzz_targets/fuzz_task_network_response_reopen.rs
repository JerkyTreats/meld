#![no_main]

use libfuzzer_sys::fuzz_target;
use meld_execution::{
    task::{CompiledTaskRecord, TaskRunContext},
    task_network::{
        command::{Command, Request, Response, ResponseAuthentication},
        mutation::{Inject, Mutation, ReadPrecondition, Set},
        state::{TaskLineage, TaskNode},
        store::SledTaskNetworkStore,
    },
};

fn task_node(byte: u8) -> TaskNode {
    let id = format!("task-{byte}");
    TaskNode {
        task_instance_id: id.clone(),
        lifecycle_epoch: 1,
        compiled_task: CompiledTaskRecord {
            task_id: format!("compiled-{byte}"),
            task_version: 1,
            init_slots: Vec::new(),
            capability_instances: Vec::new(),
            dependency_edges: Vec::new(),
        },
        init_sources: Vec::new(),
        task_run_context: TaskRunContext {
            task_run_id: format!("run-{byte}"),
            session_id: None,
            trigger: "fuzz response reopen".to_string(),
        },
        lineage: TaskLineage {
            composition_id: "composition-fuzz".to_string(),
            goal_id: "goal-fuzz".to_string(),
            method_id: "method-fuzz".to_string(),
            step_id: format!("step-{byte}"),
            operator_id: format!("operator-{byte}"),
            world_state_frame_id: "frame-fuzz".to_string(),
            subject: None,
            capability_type_id: "fuzz.run".to_string(),
            capability_version: 1,
        },
    }
}

fuzz_target!(|data: &[u8]| {
    if data.is_empty() {
        return;
    }
    let db = sled::Config::new().temporary(true).open().unwrap();
    let expected;
    {
        let mut store = SledTaskNetworkStore::open(db.clone(), "network-fuzz").unwrap();
        let node = task_node(data[0]);
        let request = Request {
            command_id: "command-fuzz".to_string(),
            network_id: "network-fuzz".to_string(),
            base_revision: 0,
            base_state_hash: store.state().state_hash.clone(),
            read_preconditions: Vec::new(),
            command: Command::ApplyMutationSet(Set::new(
                "network-fuzz",
                "composition-fuzz",
                "once-fuzz",
                vec![Mutation::Inject(Inject::new(node, Vec::new()))],
                Vec::new(),
            )),
        };
        assert!(matches!(
            store.submit(request).unwrap(),
            Response::Accepted { .. }
        ));
        expected = store.state().clone();
    }

    let mode = data.get(1).copied().unwrap_or(0) % 8;
    if mode == 0 {
        assert_eq!(
            SledTaskNetworkStore::open(db, "network-fuzz")
                .unwrap()
                .state(),
            &expected
        );
        return;
    }

    if mode == 5 {
        db.open_tree("task_network_command_authentications")
            .unwrap()
            .remove("command-fuzz")
            .unwrap();
        db.flush().unwrap();
        assert!(SledTaskNetworkStore::open(db, "network-fuzz").is_err());
        return;
    }

    if mode >= 6 {
        db.open_tree("task_network_command_schema")
            .unwrap()
            .remove("schema")
            .unwrap();
        db.open_tree("task_network_command_authentications")
            .unwrap()
            .clear()
            .unwrap();
        let responses = db.open_tree("task_network_command_responses").unwrap();
        let raw = responses.get("command-fuzz").unwrap().unwrap();
        let mut stored: serde_json::Value = serde_json::from_slice(&raw).unwrap();
        stored.as_object_mut().unwrap().remove("authentication");
        if mode == 7 {
            stored["response"] = serde_json::to_value(Response::Accepted {
                revision: 1,
                state_hash: format!("pre-auth-tampered-{}", data[0]),
            })
            .unwrap();
        }
        responses
            .insert("command-fuzz", serde_json::to_vec(&stored).unwrap())
            .unwrap();
        db.flush().unwrap();
        let reopened = SledTaskNetworkStore::open(db, "network-fuzz");
        if mode == 6 {
            assert_eq!(reopened.unwrap().state(), &expected);
        } else {
            assert!(reopened.is_err());
        }
        return;
    }

    if mode >= 3 {
        let requests = db.open_tree("task_network_command_requests").unwrap();
        let raw = requests.get("command-fuzz").unwrap().unwrap();
        let mut stored: serde_json::Value = serde_json::from_slice(&raw).unwrap();
        if mode == 3 {
            stored["request"]["base_state_hash"] =
                serde_json::Value::String(format!("foreign-base-{}", data[0]));
        }
        if mode == 4 {
            let mut request: Request = serde_json::from_value(stored["request"].clone()).unwrap();
            request.read_preconditions = vec![ReadPrecondition::RevisionIs(0)];
            stored["request"] = serde_json::to_value(&request).unwrap();
            stored["request_hash"] = serde_json::Value::String(
                meld_execution::task_network::command::request_hash(&request),
            );
        }
        requests
            .insert("command-fuzz", serde_json::to_vec(&stored).unwrap())
            .unwrap();
        if mode == 4 {
            let responses = db.open_tree("task_network_command_responses").unwrap();
            let raw = responses.get("command-fuzz").unwrap().unwrap();
            let mut response: serde_json::Value = serde_json::from_slice(&raw).unwrap();
            let request_hash = stored["request_hash"].as_str().unwrap();
            let durable_response: Response =
                serde_json::from_value(response["response"].clone()).unwrap();
            response["request_hash"] = serde_json::Value::String(request_hash.to_string());
            response["authentication"] = serde_json::to_value(
                ResponseAuthentication::bind(
                    "command-fuzz",
                    request_hash,
                    &durable_response,
                )
                .unwrap(),
            )
            .unwrap();
            responses
                .insert("command-fuzz", serde_json::to_vec(&response).unwrap())
                .unwrap();
        }
        db.flush().unwrap();
        assert!(SledTaskNetworkStore::open(db, "network-fuzz").is_err());
        return;
    }

    let responses = db.open_tree("task_network_command_responses").unwrap();
    let raw = responses.get("command-fuzz").unwrap().unwrap();
    let mut stored: serde_json::Value = serde_json::from_slice(&raw).unwrap();
    let substituted = Response::Accepted {
        revision: 1,
        state_hash: format!("tampered-{}", data[0]),
    };
    stored["response"] = serde_json::to_value(&substituted).unwrap();
    if mode == 2 {
        let request_hash = stored["request_hash"].as_str().unwrap().to_string();
        stored["authentication"] = serde_json::to_value(
            ResponseAuthentication::bind("command-fuzz", &request_hash, &substituted).unwrap(),
        )
        .unwrap();
    }
    responses
        .insert("command-fuzz", serde_json::to_vec(&stored).unwrap())
        .unwrap();
    db.flush().unwrap();

    assert!(SledTaskNetworkStore::open(db, "network-fuzz").is_err());
});
