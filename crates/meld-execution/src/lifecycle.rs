//! Native lifecycle transitions shared by Execution runtime owners.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

/// Semantic lifecycle account authored from the native owner's bound state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeLifecycleEvidence {
    /// Exact durable owner position reconstructed for this account.
    pub checkpoint_ref: String,
    /// Native schema, policy, and catalog revisions actually bound.
    pub installed_revision_refs: Vec<String>,
    /// Physical and authority bindings observed by the owner.
    pub binding_refs: Vec<String>,
    /// Native delivery routes whose progress this instance observes.
    pub subscription_refs: Vec<String>,
    /// Durable owner position supporting the phase transition.
    pub proof_position_ref: String,
    /// Content identity of the durable work remaining at this boundary.
    pub unresolved_operation_summary_ref: String,
}

pub(crate) fn evidence_ref<T: serde::Serialize>(kind: &str, value: &T) -> Result<String, String> {
    let bytes = serde_json::to_vec(value).map_err(|error| error.to_string())?;
    Ok(format!("{kind}::{}", blake3::hash(&bytes).to_hex()))
}

/// Exact activation identity for one native owner lifecycle session.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct NativeLifecycleIdentity {
    /// Owning activation generation.
    pub generation_id: String,
    /// Exact participant incarnation.
    pub incarnation_id: String,
}

impl NativeLifecycleIdentity {
    /// Bind a native lifecycle session to an exact generation and incarnation.
    pub fn new(generation_id: String, incarnation_id: String) -> Result<Self, String> {
        if generation_id.trim().is_empty() || incarnation_id.trim().is_empty() {
            return Err("native lifecycle identity is incomplete".to_string());
        }
        Ok(Self {
            generation_id,
            incarnation_id,
        })
    }
}

impl From<(String, String)> for NativeLifecycleIdentity {
    fn from((generation_id, incarnation_id): (String, String)) -> Self {
        Self {
            generation_id,
            incarnation_id,
        }
    }
}

/// Native phase retained by one concrete Execution owner instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeLifecyclePhase {
    /// The owner exists but has not reconstructed its runtime state.
    Constructed,
    /// The owner reconstructed state and may run bounded work.
    Running,
    /// The owner closed its current bounded operation set.
    SafePoint,
    /// The owner accepted its stop contract.
    Stopped,
    /// The owner released its runtime binding.
    Released,
}

/// Owner-authored proof of one concrete native phase transition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeLifecycleTransition {
    /// Owning activation generation.
    pub generation_id: String,
    /// Exact participant incarnation.
    pub incarnation_id: String,
    /// Phase observed before the transition.
    pub from: NativeLifecyclePhase,
    /// Phase installed by the transition.
    pub to: NativeLifecyclePhase,
    /// Native durable checkpoint observed during the transition.
    pub checkpoint_ref: String,
    /// Stable owner proof over the transition and checkpoint.
    pub proof_ref: String,
}

#[derive(Debug)]
struct NativeLifecycleRecord {
    phase: NativeLifecyclePhase,
    last_transition: Option<NativeLifecycleTransition>,
}

/// Stateful lifecycle seam owned by one Execution actor instance.
#[derive(Debug, Clone)]
pub struct NativeLifecycle {
    owner_ref: String,
    records: Arc<Mutex<BTreeMap<NativeLifecycleIdentity, NativeLifecycleRecord>>>,
}

impl NativeLifecycle {
    /// Create a constructed native owner lifecycle.
    pub fn new(owner_ref: impl Into<String>) -> Self {
        Self {
            owner_ref: owner_ref.into(),
            records: Arc::new(Mutex::new(BTreeMap::new())),
        }
    }

    /// Record successful native reconstruction.
    pub fn start(
        &self,
        identity: NativeLifecycleIdentity,
        checkpoint_ref: impl Into<String>,
    ) -> Result<NativeLifecycleTransition, String> {
        self.transition(
            NativeLifecyclePhase::Constructed,
            NativeLifecyclePhase::Running,
            identity,
            checkpoint_ref.into(),
            true,
        )
    }

    /// Close the current bounded operation set at a native safe point.
    pub fn safe_point(
        &self,
        identity: NativeLifecycleIdentity,
        checkpoint_ref: impl Into<String>,
    ) -> Result<NativeLifecycleTransition, String> {
        self.transition(
            NativeLifecyclePhase::Running,
            NativeLifecyclePhase::SafePoint,
            identity,
            checkpoint_ref.into(),
            false,
        )
    }

    /// Accept the native stop contract after a safe point.
    pub fn stop(
        &self,
        identity: NativeLifecycleIdentity,
        checkpoint_ref: impl Into<String>,
    ) -> Result<NativeLifecycleTransition, String> {
        self.transition(
            NativeLifecyclePhase::SafePoint,
            NativeLifecyclePhase::Stopped,
            identity,
            checkpoint_ref.into(),
            false,
        )
    }

    /// Release the native runtime binding after stop completion.
    pub fn release(
        &self,
        identity: NativeLifecycleIdentity,
        checkpoint_ref: impl Into<String>,
    ) -> Result<NativeLifecycleTransition, String> {
        self.transition(
            NativeLifecyclePhase::Stopped,
            NativeLifecyclePhase::Released,
            identity,
            checkpoint_ref.into(),
            false,
        )
    }

    fn transition(
        &self,
        expected: NativeLifecyclePhase,
        next: NativeLifecyclePhase,
        identity: NativeLifecycleIdentity,
        checkpoint_ref: String,
        create: bool,
    ) -> Result<NativeLifecycleTransition, String> {
        if identity.generation_id.trim().is_empty() || identity.incarnation_id.trim().is_empty() {
            return Err("native lifecycle identity is incomplete".to_string());
        }
        if checkpoint_ref.trim().is_empty() {
            return Err("native lifecycle checkpoint is empty".to_string());
        }
        let mut records = self
            .records
            .lock()
            .map_err(|_| "native lifecycle state is poisoned".to_string())?;
        if create {
            records
                .entry(identity.clone())
                .or_insert(NativeLifecycleRecord {
                    phase: NativeLifecyclePhase::Constructed,
                    last_transition: None,
                });
        }
        let record = records.get_mut(&identity).ok_or_else(|| {
            format!(
                "native owner '{}' has no lifecycle session for incarnation '{}'",
                self.owner_ref, identity.incarnation_id
            )
        })?;
        if record.phase == next {
            let retained = record.last_transition.clone().ok_or_else(|| {
                "native lifecycle repeated a phase without retained proof".to_string()
            })?;
            if retained.checkpoint_ref == checkpoint_ref {
                return Ok(retained);
            }
            return Err("native lifecycle retry changed its checkpoint".to_string());
        }
        if record.phase != expected {
            return Err(format!(
                "native owner '{}' cannot transition from {:?} to {:?}",
                self.owner_ref, record.phase, next
            ));
        }
        let from = record.phase;
        let proof_ref = format!(
            "execution-native-transition::{}::{}::{}::{from:?}-to-{next:?}::{checkpoint_ref}",
            self.owner_ref, identity.generation_id, identity.incarnation_id
        );
        let transition = NativeLifecycleTransition {
            generation_id: identity.generation_id,
            incarnation_id: identity.incarnation_id,
            from,
            to: next,
            checkpoint_ref,
            proof_ref,
        };
        record.phase = next;
        record.last_transition = Some(transition.clone());
        Ok(transition)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_owner_requires_safe_point_before_stop_and_retains_transition_proofs() {
        let lifecycle = NativeLifecycle::new("dispatch.owner");
        let identity =
            NativeLifecycleIdentity::new("generation-1".into(), "incarnation-1".into()).unwrap();
        let start = lifecycle.start(identity.clone(), "network::11").unwrap();
        assert_eq!(start.to, NativeLifecyclePhase::Running);
        assert!(lifecycle.stop(identity.clone(), "network::11").is_err());
        let safe = lifecycle
            .safe_point(identity.clone(), "network::11")
            .unwrap();
        let stop = lifecycle.stop(identity.clone(), "network::11").unwrap();
        let release = lifecycle.release(identity, "network::11").unwrap();
        assert!(safe.proof_ref.contains("Running-to-SafePoint"));
        assert!(stop.proof_ref.contains("SafePoint-to-Stopped"));
        assert!(release.proof_ref.contains("Stopped-to-Released"));
    }

    #[test]
    fn lifecycle_retry_is_scoped_to_exact_incarnation_and_checkpoint() {
        let lifecycle = NativeLifecycle::new("dispatch.owner");
        let first =
            NativeLifecycleIdentity::new("generation-1".into(), "incarnation-1".into()).unwrap();
        let successor =
            NativeLifecycleIdentity::new("generation-1".into(), "incarnation-2".into()).unwrap();
        let first_proof = lifecycle.start(first.clone(), "network::11").unwrap();
        assert_eq!(
            lifecycle.start(first.clone(), "network::11").unwrap(),
            first_proof
        );
        assert!(lifecycle.start(first, "network::12").is_err());
        let successor_proof = lifecycle.start(successor, "network::11").unwrap();
        assert_ne!(first_proof.proof_ref, successor_proof.proof_ref);
    }
}
