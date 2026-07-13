//! One-shot activation bootstrap for configured world-model agent state.

mod compat;
mod contracts;
mod runtime;
mod store;

pub use contracts::{
    AgentBootstrapDiagnostic, AgentBootstrapError, AgentBootstrapErrorClass,
    AgentBootstrapProgress, AgentBootstrapProgressStatus, AgentBootstrapReport,
    AgentBootstrapStage, MAX_BOOTSTRAP_DIAGNOSTICS,
};
pub use runtime::AgentBootstrapRuntime;

pub(crate) fn decode_legacy_agent_for_read(
    raw: &[u8],
) -> Result<crate::agent::AgentRecord, String> {
    let legacy = compat::decode_legacy_agent_record(raw)?;
    let legacy_directive_id = format!(
        "legacy-directive-{}",
        blake3::hash(legacy.directive_text().as_bytes()).to_hex()
    );
    Ok(legacy.canonical(legacy_directive_id))
}
