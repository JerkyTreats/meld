//! Private compatibility decoder for embedded directive agent records.

use serde::{Deserialize, Serialize};

use crate::agent::{AgentRecord, AgentStatus};
use crate::belief::BranchScope;
use crate::events::DomainObjectRef;
use crate::world_state::graph::PerspectiveKey;

const LEGACY_AGENT_RECORD_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct LegacyAgentRecordV1 {
    agent_id: String,
    perspective_key: PerspectiveKey,
    subject: DomainObjectRef,
    branch_scope: BranchScope,
    observation_scope: String,
    directive: String,
    seed_provenance: String,
    status: AgentStatus,
    created_at_seq: u64,
    updated_at_seq: u64,
}

impl LegacyAgentRecordV1 {
    fn canonical(&self, directive_id: String) -> AgentRecord {
        AgentRecord {
            agent_id: self.agent_id.clone(),
            perspective_key: self.perspective_key.clone(),
            subject: self.subject.clone(),
            branch_scope: self.branch_scope.clone(),
            observation_scope: self.observation_scope.clone(),
            directive_id,
            seed_provenance: self.seed_provenance.clone(),
            status: self.status.clone(),
            created_at_seq: self.created_at_seq,
            updated_at_seq: self.updated_at_seq,
        }
    }
}

/// Decoded compatibility record retained only for transactional migration.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct DecodedLegacyAgent {
    record: LegacyAgentRecordV1,
    raw_hash: String,
}

impl DecodedLegacyAgent {
    pub(super) fn agent_id(&self) -> &str {
        &self.record.agent_id
    }

    pub(super) fn perspective_key(&self) -> &PerspectiveKey {
        &self.record.perspective_key
    }

    pub(super) fn subject(&self) -> &DomainObjectRef {
        &self.record.subject
    }

    pub(super) fn branch_scope(&self) -> &BranchScope {
        &self.record.branch_scope
    }

    pub(super) fn observation_scope(&self) -> &str {
        &self.record.observation_scope
    }

    pub(super) fn directive_text(&self) -> &str {
        &self.record.directive
    }

    pub(super) fn seed_provenance(&self) -> &str {
        &self.record.seed_provenance
    }

    pub(super) fn raw_hash(&self) -> &str {
        &self.raw_hash
    }

    pub(super) fn canonical(&self, directive_id: String) -> AgentRecord {
        self.record.canonical(directive_id)
    }
}

pub(super) fn decode_legacy_agent_record(raw: &[u8]) -> Result<DecodedLegacyAgent, String> {
    decode_versioned_legacy_agent_record(LEGACY_AGENT_RECORD_SCHEMA_VERSION, raw)
}

// TODO compat-shim: Remove after every supported store has a versioned migration receipt and no embedded directive records remain.
fn decode_versioned_legacy_agent_record(
    schema_version: u32,
    raw: &[u8],
) -> Result<DecodedLegacyAgent, String> {
    if schema_version != LEGACY_AGENT_RECORD_SCHEMA_VERSION {
        return Err(format!(
            "unsupported legacy agent record schema version {schema_version}"
        ));
    }
    let record: LegacyAgentRecordV1 =
        serde_json::from_slice(raw).map_err(|error| error.to_string())?;
    Ok(DecodedLegacyAgent {
        record,
        raw_hash: blake3::hash(raw).to_hex().to_string(),
    })
}

#[cfg(test)]
pub(super) fn encode_legacy_agent_record(canonical: &AgentRecord, directive_text: &str) -> Vec<u8> {
    serde_json::to_vec(&LegacyAgentRecordV1 {
        agent_id: canonical.agent_id.clone(),
        perspective_key: canonical.perspective_key.clone(),
        subject: canonical.subject.clone(),
        branch_scope: canonical.branch_scope.clone(),
        observation_scope: canonical.observation_scope.clone(),
        directive: directive_text.to_string(),
        seed_provenance: canonical.seed_provenance.clone(),
        status: canonical.status.clone(),
        created_at_seq: canonical.created_at_seq,
        updated_at_seq: canonical.updated_at_seq,
    })
    .expect("legacy agent fixture must encode")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::belief::BranchScope;
    use crate::events::DomainObjectRef;

    #[test]
    fn legacy_embedded_directive_wire_shape_is_characterized() {
        let raw = serde_json::to_vec(&serde_json::json!({
            "agent_id": "seed-a",
            "perspective_key": {
                "perspective_kind": "default",
                "perspective_id": "default"
            },
            "subject": {
                "domain_id": "workspace_fs",
                "object_kind": "node",
                "object_id": "node-a"
            },
            "branch_scope": {"branch_id": "main"},
            "observation_scope": "family_a",
            "directive": "curate configured family goals",
            "seed_provenance": "trusted init",
            "status": "Registered",
            "created_at_seq": 11,
            "updated_at_seq": 11
        }))
        .unwrap();

        let decoded = decode_versioned_legacy_agent_record(1, &raw).unwrap();
        assert_eq!(decoded.agent_id(), "seed-a");
        assert_eq!(decoded.directive_text(), "curate configured family goals");
        assert_eq!(decoded.branch_scope(), &BranchScope::main());
        assert_eq!(
            decoded.subject(),
            &DomainObjectRef::new("workspace_fs", "node", "node-a").unwrap()
        );
        assert_eq!(decoded.raw_hash(), blake3::hash(&raw).to_hex().as_str());
        assert!(decode_versioned_legacy_agent_record(2, &raw).is_err());
    }
}
