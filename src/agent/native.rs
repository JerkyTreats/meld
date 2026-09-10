//! Native Agent command products. Profiles are configuration artifacts and never
//! supply an Agent identity, intention, Goal judgment or reconciliation receipt.

use crate::cli::AgentCommands;
use crate::error::ApiError;
use crate::runtime::assembly::ProductRuntimeAssembly;
use meld_world_model::agent::AgentStore;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case")]
pub enum AgentRead {
    List { limit: usize, after: Option<String> },
    Show { agent: Option<String> },
    RequestStatus { agent: Option<String>, key: String },
}

pub fn resolve_agent(store: &AgentStore, selected: Option<&str>) -> Result<String, String> {
    if let Some(id) = selected {
        return Ok(id.into());
    }
    let agents = store
        .list_genesis_intents(None, 2)
        .map_err(|e| e.to_string())?;
    match agents.as_slice() {
        [agent] => Ok(agent.registration.agent_id.clone()),
        [] => Err("no native Agent is prepared; run meld init".into()),
        _ => Err(
            "multiple native Agents are prepared; choose an identity from meld agent list".into(),
        ),
    }
}

pub fn read(store: &AgentStore, request: AgentRead) -> Result<Value, String> {
    match request {
        AgentRead::List { limit, after } => {
            if limit == 0 || limit > 1000 {
                return Err("limit must be between 1 and 1000".into());
            }
            let mut agents = store
                .list_genesis_intents(after.as_deref(), limit + 1)
                .map_err(|e| e.to_string())?;
            let more = agents.len() > limit;
            agents.truncate(limit);
            let next = if more {
                agents.last().map(|a| a.registration.agent_id.clone())
            } else {
                None
            };
            Ok(json!({"agents": agents, "next_cursor": next}))
        }
        AgentRead::Show { agent } => {
            let id = resolve_agent(store, agent.as_deref())?;
            let genesis = store
                .genesis_intent_for_agent(&id)
                .map_err(|e| e.to_string())?
                .ok_or("native Agent not found")?;
            let goals = store
                .reconciliation_goals_for_agent(&id)
                .map_err(|e| e.to_string())?;
            let mut plans = Vec::new();
            let mut dispositions = Vec::new();
            let mut authorizations = Vec::new();
            for goal in &goals {
                authorizations.extend(
                    store
                        .product_authorizations_for_goal(&goal.goal.goal_id)
                        .map_err(|e| e.to_string())?,
                );
                if let Some(plan) = store
                    .current_reconciliation_plan(&goal.goal.goal_id)
                    .map_err(|e| e.to_string())?
                {
                    if let Some(disposition) = store
                        .goal_disposition_for_plan(&plan.plan_revision_id)
                        .map_err(|e| e.to_string())?
                    {
                        dispositions.push(disposition);
                    }
                    plans.push(plan);
                }
            }
            let judgments = store
                .condition_judgments()
                .map_err(|e| e.to_string())?
                .into_iter()
                .filter(|judgment| judgment.agent_id == id)
                .collect::<Vec<_>>();
            Ok(
                json!({"genesis": genesis, "goals": goals, "condition_judgments": judgments, "plans": plans, "goal_dispositions": dispositions, "authorizations": authorizations}),
            )
        }
        AgentRead::RequestStatus { agent, key } => {
            let id = resolve_agent(store, agent.as_deref())?;
            let request = store
                .reconciliation_requests(&id)
                .map_err(|e| e.to_string())?
                .into_iter()
                .find(|r| r.request_key == key)
                .ok_or("reconciliation request not found; no request was submitted")?;
            let completed = store
                .reconciliation_request_completed(&request)
                .map_err(|e| e.to_string())?;
            Ok(json!({"request": request, "completed": completed}))
        }
    }
}

pub fn query(command: &AgentCommands) -> Option<AgentRead> {
    match command {
        AgentCommands::List { limit, after, .. } => Some(AgentRead::List {
            limit: *limit,
            after: after.clone(),
        }),
        AgentCommands::Show { agent, .. } => Some(AgentRead::Show {
            agent: agent.clone(),
        }),
        AgentCommands::RequestStatus { agent, key, .. } => Some(AgentRead::RequestStatus {
            agent: agent.clone(),
            key: key.clone(),
        }),
        AgentCommands::Request { .. } => None,
    }
}

pub fn format(value: Value, command: &AgentCommands) -> Result<String, ApiError> {
    let json = match command {
        AgentCommands::List { json, .. }
        | AgentCommands::Show { json, .. }
        | AgentCommands::Request { json, .. }
        | AgentCommands::RequestStatus { json, .. } => *json,
    };
    if !json {
        if let Some(genesis) = value.get("genesis") {
            let registration = &genesis["registration"];
            let mut lines = vec![
                format!("Agent {}", registration["agent_id"].as_str().unwrap_or("?")),
                format!(
                    "Intention: {}",
                    registration["directive"].as_str().unwrap_or("unavailable")
                ),
                format!(
                    "Assignment: {}",
                    genesis["assignment_id"].as_str().unwrap_or("?")
                ),
            ];
            if let Some(goals) = value["goals"].as_array() {
                for goal in goals {
                    let id = goal["goal"]["goal_id"].as_str().unwrap_or("?");
                    let disposition = value["goal_dispositions"]
                        .as_array()
                        .and_then(|rows| rows.iter().find(|row| row["goal_id"] == id));
                    lines.push(format!(
                        "Goal {id}: {}",
                        disposition
                            .map(|row| row["lifecycle"].to_string())
                            .unwrap_or_else(|| "no final disposition".into())
                    ));
                }
                if goals.is_empty() {
                    lines.push("No native Goals have been created.".into());
                }
            }
            return Ok(lines.join("\n"));
        }
        if let Some(request) = value.get("request") {
            return Ok(format!(
                "Request {} for {}: {}\nKey: {}",
                request["request_id"].as_str().unwrap_or("?"),
                request["agent_id"].as_str().unwrap_or("?"),
                if value["completed"].as_bool() == Some(true) {
                    "completed by native judgment"
                } else {
                    "accepted; completion pending"
                },
                request["request_key"].as_str().unwrap_or("?")
            ));
        }
        if let Some(agents) = value.get("agents").and_then(Value::as_array) {
            let mut lines: Vec<String> = agents
                .iter()
                .map(|a| {
                    format!(
                        "{}  assignment {}",
                        a["registration"]["agent_id"].as_str().unwrap_or("?"),
                        a["assignment_id"].as_str().unwrap_or("?")
                    )
                })
                .collect();
            if lines.is_empty() {
                lines.push("No native Agents prepared. Run meld init.".into());
            }
            if let Some(next) = value["next_cursor"].as_str() {
                lines.push(format!("More: meld agent list --after {next}"));
            }
            return Ok(lines.join("\n"));
        }
    }
    serde_json::to_string_pretty(&value).map_err(|e| ApiError::ConfigError(e.to_string()))
}

pub fn execute(
    assembly: &ProductRuntimeAssembly,
    command: &AgentCommands,
) -> Result<String, ApiError> {
    let store = assembly
        .stores()
        .agent_store
        .opened()
        .ok_or_else(|| ApiError::ConfigError("native Agent store unavailable".into()))?;
    let value = if let Some(query) = query(command) {
        read(store, query)
    } else if let AgentCommands::Request {
        agent, request_key, ..
    } = command
    {
        (|| {
            let id = resolve_agent(store, agent.as_deref())?;
            let key = request_key
                .clone()
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
            let request = store
                .request_reconciliation(&id, key)
                .map_err(|e| e.to_string())?;
            let completed = store
                .reconciliation_request_completed(&request)
                .map_err(|e| e.to_string())?;
            Ok(json!({"request": request, "completed": completed}))
        })()
    } else {
        unreachable!()
    };
    format(value.map_err(ApiError::ConfigError)?, command)
}

/// Serve native Agent commands through the live store owner when present.
pub fn try_live(
    workspace: &std::path::Path,
    config: &crate::config::MerkleConfig,
    command: &AgentCommands,
) -> Option<Result<String, ApiError>> {
    let target = ProductRuntimeAssembly::describe_for_workspace(workspace, config).ok()?;
    let (url, _) = match crate::runtime::managed::discover_live(&target) {
        Ok(Some(live)) => live,
        Ok(None) => return None,
        Err(error) => return Some(Err(error)),
    };
    Some((|| {
        let read_query = |query: &AgentRead| -> Result<Value, ApiError> {
            let response = ureq::post(&format!("{url}/v1/agents/query"))
                .timeout(std::time::Duration::from_secs(5))
                .send_json(query)
                .map_err(remote_error)?;
            response
                .into_json()
                .map_err(|e| ApiError::ConfigError(e.to_string()))
        };
        if let Some(query) = query(command) {
            return format(read_query(&query)?, command);
        }
        let AgentCommands::Request {
            agent,
            request_key,
            json,
        } = command
        else {
            unreachable!()
        };
        let agent = match agent {
            Some(id) => id.clone(),
            None => read_query(&AgentRead::Show { agent: None })?["genesis"]["registration"]
                ["agent_id"]
                .as_str()
                .ok_or_else(|| ApiError::ConfigError("live native Agent identity missing".into()))?
                .into(),
        };
        let key = request_key
            .clone()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        crate::runtime::tooling::try_live_runtime_request(
            workspace,
            config,
            &agent,
            &key,
            if *json { "json" } else { "text" },
        )
        .ok_or_else(|| {
            ApiError::ConfigError(format!(
                "live runtime disappeared before request {key}; no submission confirmed"
            ))
        })?
        .map_err(|e| ApiError::ConfigError(format!("request key {key}: {e}")))
    })())
}

fn remote_error(error: ureq::Error) -> ApiError {
    let message = match error {
        ureq::Error::Status(code, response) => response
            .into_json::<crate::serve::routes::RouteError>()
            .map(|body| body.error)
            .unwrap_or_else(|_| format!("Agent query returned HTTP {code}")),
        error => error.to_string(),
    };
    ApiError::ConfigError(message)
}
