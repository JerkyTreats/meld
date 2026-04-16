use crate::roots::contracts::RootsStatusOutput;
use crate::roots::query::{
    BranchGraphStatusOutput, FederatedNeighborsOutput, FederatedWalkOutput,
};

pub fn format_branch_status_text(output: &RootsStatusOutput) -> String {
    if output.roots.is_empty() {
        return "No branches registered".to_string();
    }

    let mut out = format!("Known branches: {}\n", output.roots.len());
    for root in &output.roots {
        out.push('\n');
        out.push_str(&format!("Branch ID: {}\n", root.root_id));
        out.push_str(&format!("Locator: {}\n", root.workspace_path));
        out.push_str(&format!("Data Home: {}\n", root.data_home_path));
        if let Some(store_path) = &root.store_path {
            out.push_str(&format!("Store: {}\n", store_path));
        }
        out.push_str(&format!("Attachment: {}\n", root.attachment_status));
        out.push_str(&format!("Inspection: {}\n", root.inspection_status));
        out.push_str(&format!("Migration: {}\n", root.migration_status));
        if let Some(last_seen_at) = &root.last_seen_at {
            out.push_str(&format!("Last Seen: {}\n", last_seen_at));
        }
        if let Some(last_migration_at) = &root.last_migration_at {
            out.push_str(&format!("Last Migration: {}\n", last_migration_at));
        }
    }
    out.trim_end().to_string()
}

pub fn format_roots_status_text(output: &RootsStatusOutput) -> String {
    format_branch_status_text(output)
}

pub fn format_branch_graph_status_text(output: &BranchGraphStatusOutput) -> String {
    if output.branches.is_empty() {
        return "No branch graph stores matched the requested scope".to_string();
    }

    let mut out = format!(
        "Federated branch graph status for scope '{}': {}\n",
        output.metadata.scope,
        output.branches.len()
    );
    for branch in &output.branches {
        out.push('\n');
        out.push_str(&format!("Branch ID: {}\n", branch.branch_id));
        out.push_str(&format!("Locator: {}\n", branch.canonical_locator));
        if let Some(store_path) = &branch.store_path {
            out.push_str(&format!("Store: {}\n", store_path));
        }
        out.push_str(&format!("Read Status: {}\n", branch.read_status));
        if let Some(last_reduced_seq) = branch.last_reduced_seq {
            out.push_str(&format!("Last Reduced Seq: {}\n", last_reduced_seq));
        }
        if let Some(error) = &branch.error {
            out.push_str(&format!("Error: {}\n", error));
        }
    }
    out.trim_end().to_string()
}

pub fn format_federated_neighbors_text(output: &FederatedNeighborsOutput) -> String {
    let mut out = format!(
        "Federated neighbors across {} readable branches\n",
        output.metadata.readable_branch_ids.len()
    );
    for neighbor in &output.neighbors {
        out.push_str(&format!(
            "- {}::{}::{}\n",
            neighbor.domain_id, neighbor.object_kind, neighbor.object_id
        ));
    }
    out.trim_end().to_string()
}

pub fn format_federated_walk_text(output: &FederatedWalkOutput) -> String {
    let mut out = format!(
        "Federated walk across {} readable branches\n",
        output.metadata.readable_branch_ids.len()
    );
    out.push_str(&format!(
        "Visited Objects: {}\n",
        output.walk.visited_objects.len()
    ));
    out.push_str(&format!(
        "Visited Facts: {}\n",
        output.walk.visited_facts.len()
    ));
    out.push_str(&format!(
        "Traversed Relations: {}\n",
        output.walk.traversed_relations.len()
    ));
    out.trim_end().to_string()
}
