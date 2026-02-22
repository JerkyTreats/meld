//! Format workspace, agent, provider, and unified status as text.

use crate::workspace::types::{
    AgentStatusEntry, ProviderStatusEntry, UnifiedStatusOutput, WorkspaceStatus,
};
use comfy_table::presets::UTF8_BORDERS_ONLY;
use comfy_table::Table;
use owo_colors::OwoColorize;

/// Format a section heading with bold/underline. Respects NO_COLOR and TTY.
pub fn format_section_heading(title: &str) -> String {
    format!("{}", title.bold().underline())
}

/// Format workspace status as human-readable text.
pub fn format_workspace_status_text(
    data: &WorkspaceStatus,
    include_breakdown: bool,
) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "{}\n\n",
        format_section_heading("Workspace Status")
    ));
    out.push_str(&format!("{}\n", format_section_heading("Tree")));
    if !data.scanned {
        out.push_str(&format!("  Store path: {}\n", data.store_path));
        out.push_str("  Scanned: no\n\n");
        if let Some(ref msg) = data.message {
            out.push_str(msg);
            out.push_str("\n");
        }
        return out;
    }
    let tree = data.tree.as_ref().unwrap();
    out.push_str(&format!("  Store path: {}\n", data.store_path));
    out.push_str(&format!(
        "  Root hash: {}...\n",
        &tree.root_hash[..tree.root_hash.len().min(7)]
    ));
    out.push_str(&format!("  Total nodes: {}\n", tree.total_nodes));
    out.push_str("  Scanned: yes\n\n");
    if include_breakdown {
        if let Some(ref breakdown) = tree.breakdown {
            out.push_str("  Top-level breakdown\n\n");
            let mut table = Table::new();
            table.load_preset(UTF8_BORDERS_ONLY);
            table.set_header(vec!["Path", "Nodes"]);
            for row in breakdown {
                table.add_row(vec![row.path.clone(), row.nodes.to_string()]);
            }
            out.push_str(&format!("{}\n\n", table));
        }
    }
    if let Some(ref coverage) = data.context_coverage {
        out.push_str(&format!(
            "{}\n\n",
            format_section_heading("Context coverage")
        ));
        let mut table = Table::new();
        table.load_preset(UTF8_BORDERS_ONLY);
        table.set_header(vec!["Agent", "With frame", "Without", "Coverage"]);
        for row in coverage {
            let pct = row
                .coverage_pct
                .map(|p| format!("{}%", p))
                .unwrap_or_else(|| "-".to_string());
            table.add_row(vec![
                row.agent_id.clone(),
                row.nodes_with_frame.to_string(),
                row.nodes_without_frame.to_string(),
                pct,
            ]);
        }
        out.push_str(&format!("{}\n\n", table));
    }
    if let Some(ref top_paths) = data.top_paths_by_node_count {
        out.push_str(&format!(
            "{}\n\n",
            format_section_heading("Top paths by node count")
        ));
        let mut table = Table::new();
        table.load_preset(UTF8_BORDERS_ONLY);
        table.set_header(vec!["Path", "Nodes"]);
        for row in top_paths {
            table.add_row(vec![row.path.clone(), row.nodes.to_string()]);
        }
        out.push_str(&format!("{}\n", table));
    }
    out
}

/// Format agent status as human-readable text.
pub fn format_agent_status_text(entries: &[AgentStatusEntry]) -> String {
    let mut out = String::new();
    out.push_str(&format!("{}\n\n", format_section_heading("Agents")));
    if entries.is_empty() {
        out.push_str("No agents configured.\n");
        return out;
    }
    let mut table = Table::new();
    table.load_preset(UTF8_BORDERS_ONLY);
    table.set_header(vec!["Agent", "Role", "Valid", "Prompt"]);
    for row in entries {
        let valid_str = if row.valid { "yes" } else { "no" };
        let prompt_str = if row.role == "Reader" {
            "n/a".to_string()
        } else if row.prompt_path_exists {
            "exists".to_string()
        } else {
            "missing".to_string()
        };
        table.add_row(vec![
            row.agent_id.clone(),
            row.role.clone(),
            valid_str.to_string(),
            prompt_str,
        ]);
    }
    out.push_str(&format!("{}\n\n", table));
    let valid_count = entries.iter().filter(|e| e.valid).count();
    out.push_str(&format!(
        "Total: {} agents, {} valid.\n",
        entries.len(),
        valid_count
    ));
    out
}

/// Format provider status as human-readable text.
pub fn format_provider_status_text(
    entries: &[ProviderStatusEntry],
    include_connectivity: bool,
) -> String {
    let mut out = String::new();
    out.push_str(&format!("{}\n\n", format_section_heading("Providers")));
    if entries.is_empty() {
        out.push_str("No providers configured.\n");
        return out;
    }
    let mut table = Table::new();
    table.load_preset(UTF8_BORDERS_ONLY);
    if include_connectivity {
        table.set_header(vec!["Provider", "Type", "Model", "Connectivity"]);
        for row in entries {
            let conn = row
                .connectivity
                .as_deref()
                .map(|c| match c {
                    "ok" => "OK",
                    "fail" => "Fail",
                    "skipped" => "Skipped",
                    _ => c,
                })
                .unwrap_or("-");
            table.add_row(vec![
                row.provider_name.clone(),
                row.provider_type.clone(),
                row.model.clone(),
                conn.to_owned(),
            ]);
        }
    } else {
        table.set_header(vec!["Provider", "Type", "Model"]);
        for row in entries {
            table.add_row(vec![
                row.provider_name.clone(),
                row.provider_type.clone(),
                row.model.clone(),
            ]);
        }
    }
    out.push_str(&format!("{}\n\n", table));
    out.push_str(&format!("Total: {} providers.\n", entries.len()));
    out
}

/// Format unified status as human-readable text.
pub fn format_unified_status_text(
    data: &UnifiedStatusOutput,
    include_breakdown: bool,
    include_connectivity: bool,
) -> String {
    let mut out = String::new();

    if let Some(ref workspace) = data.workspace {
        out.push_str(&format_workspace_status_text(workspace, include_breakdown));
        out.push('\n');
    }

    if let Some(ref agents) = data.agents {
        out.push_str(&format_agent_status_text(&agents.agents));
        out.push('\n');
    }

    if let Some(ref providers) = data.providers {
        out.push_str(&format_provider_status_text(
            &providers.providers,
            include_connectivity,
        ));
    }

    out
}
