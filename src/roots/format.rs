use crate::roots::contracts::RootsStatusOutput;

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
