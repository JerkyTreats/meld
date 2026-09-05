//! Durable manifest for one harness run: the session-as-artifact contract.
//!
//! Owner: harness. The manifest records what one run booted, injected, and
//! stepped — store roots, ledger identity, stimuli, step schedule, and
//! closing watermarks — so the run replays deterministically and hands off
//! between agents (DBG-002, DBG-008). It holds identities only: record
//! ids, ledger sequences, and action ids. Payloads stay in the durable
//! stores, so re-deriving any observation from those stores can never
//! disagree with the manifest.
//!
//! Type names deliberately avoid the `Session` stem: the CLI command
//! lifecycle records (`crate::session`) and the ledger session partition
//! (`EventEnvelope::session`) already own that vocabulary.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Manifest schema discriminator for forward-compatible readers.
pub const HARNESS_MANIFEST_SCHEMA_VERSION: u16 = 1;

/// Failure reading or writing a manifest artifact.
#[derive(Debug, Error)]
pub enum HarnessManifestError {
    /// Filesystem failure on the manifest path.
    #[error("manifest io failure: {0}")]
    Io(#[from] std::io::Error),
    /// The artifact is not a manifest this schema version reads.
    #[error("manifest encoding failure: {0}")]
    Json(#[from] serde_json::Error),
}

/// Durable record of one harness run over one isolated product root.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HarnessManifest {
    /// Manifest schema version, [`HARNESS_MANIFEST_SCHEMA_VERSION`].
    pub schema_version: u16,
    /// Stable caller-chosen identity for this run.
    pub manifest_id: String,
    /// Branch id bound into the product event binding for this root.
    pub branch_id: String,
    /// Supervisor boot time in milliseconds, injected by the caller.
    pub booted_at_ms: u64,
    /// Where the run's durable state lives.
    pub root: HarnessRootRecord,
    /// Ledger identity of the resolved event authority.
    pub ledger_identity: String,
    /// What the staged boot composed and initialized.
    pub boot: HarnessBootRecord,
    /// Stimuli appended through the canonical append port, in append order.
    pub stimuli: Vec<HarnessStimulusRecord>,
    /// Deterministic step schedule executed against the supervisor.
    pub steps: Vec<HarnessStepRecord>,
    /// Closing watermarks; present once the run has sealed.
    pub closing: Option<HarnessClosingRecord>,
}

/// Storage identity of one run root.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HarnessRootRecord {
    /// Product storage root; every store path derives from it.
    pub product_root: PathBuf,
    /// Supervisor lifecycle and report store path.
    pub supervisor_store_path: PathBuf,
    /// True when the run owns a temporary root created at boot.
    pub temporary: bool,
    /// True when the run opened an existing data root under the explicit
    /// unsafe flag (DBG-011).
    pub existing_data_root: bool,
}

/// Boot-time composition and initialization record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HarnessBootRecord {
    /// Registration ids composed into the assembly, in declaration order.
    pub registration_ids: Vec<String>,
    /// Desired runtime rows after assembly.
    pub desired_runtimes: Vec<HarnessDesiredRuntimeRecord>,
    /// Historical world-initialization reports retained for schema reads.
    /// New harness runs leave this compatibility field empty.
    pub world_init: Vec<HarnessStageRecord>,
    /// Default bounded budget the supervisor applies per actor invocation.
    pub default_budget_max_items: usize,
}

/// Desired state of one supervised runtime id at boot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HarnessDesiredRuntimeRecord {
    /// Stable runtime id.
    pub runtime_id: String,
    /// Whether desired state asks the supervisor to start it.
    pub enabled: bool,
    /// Whether assembly provided a factory for it.
    pub factory_available: bool,
}

/// One historical world-initialization stage outcome.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HarnessStageRecord {
    /// Stage name in the CLI vocabulary, e.g. `install-theory`.
    pub stage: String,
    /// `applied` or `unchanged`.
    pub disposition: String,
    /// Durable record ids the stage created or resolved.
    pub record_ids: Vec<String>,
}

/// One stimulus appended through the canonical append port (DBG-003).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HarnessStimulusRecord {
    /// Ledger that owns the sequence.
    pub ledger_id: String,
    /// Canonical ledger sequence of the appended record.
    pub seq: u64,
    /// Envelope record id when the stimulus carried one.
    pub record_id: Option<String>,
    /// Envelope event type, recorded for readability only.
    pub event_type: String,
    /// `inserted` or `duplicate`.
    pub disposition: String,
    /// Number of steps already executed when this stimulus was appended,
    /// so replay interleaves stimuli and steps identically.
    pub after_step: u64,
}

/// One supervisor maintenance pass in the deterministic step schedule.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HarnessStepRecord {
    /// One-based step ordinal.
    pub ordinal: u64,
    /// Injected supervisor time for this pass.
    pub now_ms: u64,
    /// Bounded work budget in effect for each actor invocation.
    pub budget_max_items: usize,
    /// Durable action record ids persisted by this pass.
    pub action_ids: Vec<String>,
}

/// Closing watermarks sealed after the run shut down.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HarnessClosingRecord {
    /// Injected supervisor time of the shutdown request.
    pub shutdown_at_ms: u64,
    /// Shutdown id written by the supervisor.
    pub shutdown_id: String,
    /// Highest sequence durably committed through the authority writer.
    pub committed_seq: u64,
    /// Highest sequence durable in the ledger at seal time.
    pub tip_seq: u64,
}

impl HarnessManifest {
    /// Write the manifest as pretty JSON, creating parent directories.
    pub fn save(&self, path: &Path) -> Result<(), HarnessManifestError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let body = serde_json::to_vec_pretty(self)?;
        fs::write(path, body)?;
        Ok(())
    }

    /// Read a manifest artifact back from disk.
    pub fn load(path: &Path) -> Result<Self, HarnessManifestError> {
        let body = fs::read(path)?;
        Ok(serde_json::from_slice(&body)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest() -> HarnessManifest {
        HarnessManifest {
            schema_version: HARNESS_MANIFEST_SCHEMA_VERSION,
            manifest_id: "run-1".to_string(),
            branch_id: "harness".to_string(),
            booted_at_ms: 10,
            root: HarnessRootRecord {
                product_root: PathBuf::from("/tmp/run-1/root"),
                supervisor_store_path: PathBuf::from("/tmp/run-1/root/supervisor.sled"),
                temporary: true,
                existing_data_root: false,
            },
            ledger_identity: "ledger-id".to_string(),
            boot: HarnessBootRecord {
                registration_ids: vec!["world_model.belief_assessment".to_string()],
                desired_runtimes: vec![HarnessDesiredRuntimeRecord {
                    runtime_id: "world_model.belief_assessment".to_string(),
                    enabled: true,
                    factory_available: true,
                }],
                world_init: vec![HarnessStageRecord {
                    stage: "install-theory".to_string(),
                    disposition: "applied".to_string(),
                    record_ids: vec!["belief_family::docs_freshness::hash".to_string()],
                }],
                default_budget_max_items: 8,
            },
            stimuli: vec![HarnessStimulusRecord {
                ledger_id: "ledger-id".to_string(),
                seq: 3,
                record_id: Some("genesis::world_model::observation::x".to_string()),
                event_type: "world_model.unobserved_scope".to_string(),
                disposition: "inserted".to_string(),
                after_step: 0,
            }],
            steps: vec![HarnessStepRecord {
                ordinal: 1,
                now_ms: 20,
                budget_max_items: 8,
                action_ids: vec!["action:run-1:world_model.belief_assessment:20:1".to_string()],
            }],
            closing: Some(HarnessClosingRecord {
                shutdown_at_ms: 30,
                shutdown_id: "shutdown-1".to_string(),
                committed_seq: 3,
                tip_seq: 3,
            }),
        }
    }

    #[test]
    fn manifest_round_trips_through_disk() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("artifacts/harness_manifest.json");
        let original = manifest();
        original.save(&path).unwrap();
        assert_eq!(HarnessManifest::load(&path).unwrap(), original);
    }

    #[test]
    fn loading_a_non_manifest_artifact_fails() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("harness_manifest.json");
        fs::write(&path, b"not json").unwrap();
        assert!(matches!(
            HarnessManifest::load(&path),
            Err(HarnessManifestError::Json(_))
        ));
    }
}
