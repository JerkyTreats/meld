//! Read the active branch through its existing store owner.
use super::*;
use crate::branches::query::{FederatedOwnerWalkOutput, OwnerWalkRead};
use crate::runtime::assembly::ProductRuntimeAssembly;

pub fn try_live(
    workspace: &Path,
    config: &crate::config::MerkleConfig,
    command: &BranchesCommands,
) -> Option<Result<String, ApiError>> {
    let BranchesCommands::GraphOwnerWalk {
        scope,
        owner_id,
        owner_scope_id,
        owner_branch_id,
        owner_perspective_id,
        owner_valid_at,
        domain,
        object_kind,
        object_id,
        direction,
        relation_types,
        max_depth,
        max_objects,
        max_occurrences,
        max_paths,
        format,
        ..
    } = command
    else {
        return None;
    };
    if scope != "active" {
        return None;
    }
    let target = ProductRuntimeAssembly::describe_for_workspace(workspace, config).ok()?;
    let (url, _) = match crate::runtime::managed::discover_live(&target) {
        Ok(Some(live)) => live,
        Ok(None) => return None,
        Err(error) => return Some(Err(error)),
    };
    Some((|| {
        let request = OwnerWalkRead {
            product_root: target.product_root.clone(),
            workspace_root: workspace
                .canonicalize()
                .map_err(|e| ApiError::ConfigError(e.to_string()))?,
            owner_id: owner_id.clone(),
            owner_scope: OwnerPublicationScope {
                scope_id: owner_scope_id.clone(),
                branch_id: owner_branch_id.clone(),
                perspective_id: owner_perspective_id.clone(),
                valid_at: owner_valid_at.clone(),
            },
            traversal: BoundedTraversalRequest {
                roots: vec![object_ref(domain, object_kind, object_id)?],
                direction: parse_direction(direction)?,
                relation_types: relation_types_filter(relation_types).map(|r| r.to_vec()),
                bounds: TraversalBounds {
                    max_depth: *max_depth,
                    max_objects: *max_objects,
                    max_occurrences: *max_occurrences,
                    max_paths: *max_paths,
                },
            },
        };
        let output: FederatedOwnerWalkOutput = ureq::post(&format!("{url}/v1/branches/owner_walk"))
            .timeout(std::time::Duration::from_secs(5))
            .send_json(&request)
            .map_err(|e| {
                ApiError::ConfigError(match e {
                    ureq::Error::Status(_, response) => response
                        .into_json::<crate::serve::routes::RouteError>()
                        .map(|e| e.error)
                        .unwrap_or_else(|e| e.to_string()),
                    e => e.to_string(),
                })
            })?
            .into_json()
            .map_err(|e| ApiError::ConfigError(e.to_string()))?;
        render_output(format, &output, format_federated_owner_walk_text)
    })())
}
