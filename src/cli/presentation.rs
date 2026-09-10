//! CLI presentation: text and json formatters per command family.

mod agent;
mod context;
mod provider;
mod runtime_account;
mod shared;
mod world_init;

pub use agent::{
    format_agent_list_result_json, format_agent_list_result_text, format_agent_show_result_json,
    format_agent_show_result_text, format_validation_result, format_validation_results_all,
};
pub use context::{format_context_json_output, format_context_text_output};
pub use provider::{
    format_provider_list_result_json, format_provider_list_result_text,
    format_provider_show_result_json, format_provider_show_result_text,
    format_provider_test_result, format_provider_validation_result,
};
pub use runtime_account::{render_runtime_tick_account_json, render_runtime_tick_account_text};
pub use shared::{format_ignore_result, format_list_deleted_result, format_validate_result_text};
pub use world_init::{format_world_init_report, validate_world_init_format};
