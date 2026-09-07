use super::{contracts::*, mutation::Workspace};
use meld_events::{AppendMode, EventAppendCapability, EventEnvelope, EventReplayCapability};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;

pub const OWNER: &str = "code-change";
pub const INTENT: &str = "code_change.intent.v1";
pub const COMPLETED: &str = "code_change.materialized.v1";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Intent {
    binding: Value,
    workspace_identity: (u64, u64),
    change: CodeChangeSet,
}

pub(super) struct Operation<'a> {
    pub binding: Value,
    pub change: CodeChangeSet,
    pub root: &'a Path,
}

impl Operation<'_> {
    fn id(&self, events: &EventReplayCapability) -> Result<String, String> {
        Ok(format!(
            "code-operation::{}",
            hash(&(events.ledger_identity(), &self.binding, &self.change))?
        ))
    }

    fn envelope<T: Serialize>(
        &self,
        kind: &str,
        id: String,
        data: &T,
    ) -> Result<EventEnvelope, String> {
        Ok(EventEnvelope::with_now_domain(
            "code-change",
            OWNER,
            &self.change.subject.object_id,
            kind,
            None,
            serde_json::to_value(data).map_err(|e| e.to_string())?,
        )
        .with_record_id(id))
    }

    fn intent(&self, events: &EventReplayCapability) -> Result<Option<Intent>, String> {
        let id = self.id(events)?;
        let Some(record) = events.committed_record(&id).map_err(|e| e.to_string())? else {
            return Ok(None);
        };
        let intent: Intent =
            serde_json::from_value(record.data.clone()).map_err(|e| e.to_string())?;
        if intent.binding != self.binding || intent.change != self.change {
            return Err("retained code intent differs from its exact invocation".into());
        }
        if events
            .prove_existing(&self.envelope(INTENT, id, &intent)?)
            .map_err(|e| e.to_string())?
            .is_none()
        {
            return Err("code intent has no durable proof".into());
        }
        Ok(Some(intent))
    }

    fn completed_envelope(&self, events: &EventReplayCapability) -> Result<EventEnvelope, String> {
        let id = self.id(events)?;
        let intent = events
            .committed_record(&id)
            .map_err(|e| e.to_string())?
            .ok_or("code completion has no intent")?;
        Ok(self
            .envelope(
                COMPLETED,
                format!("{id}::materialized"),
                &CodeChangeReceipt::expected(id, &self.change),
            )?
            .with_source_records(vec![meld_events::EventRecordRef {
                ledger_id: events.ledger_identity(),
                seq: intent.seq,
            }]))
    }

    pub fn recover(
        &self,
        events: &EventReplayCapability,
    ) -> Result<Option<CodeChangeReceipt>, String> {
        self.change.validate()?;
        if self.intent(events)?.is_none() {
            return Ok(None);
        }
        let envelope = self.completed_envelope(events)?;
        if events
            .prove_existing(&envelope)
            .map_err(|e| e.to_string())?
            .is_none()
        {
            return Ok(None);
        }
        Ok(Some(CodeChangeReceipt::expected(
            self.id(events)?,
            &self.change,
        )))
    }

    fn prepare(&self, events: &EventAppendCapability) -> Result<Workspace, String> {
        self.change.validate()?;
        let replay = events.replay_capability();
        let retained = self.intent(&replay)?;
        let workspace = Workspace::open(self.root)?;
        let identity = workspace.identity()?;
        if retained
            .as_ref()
            .is_some_and(|intent| intent.workspace_identity != identity)
        {
            return Err("code change workspace differs from its retained physical resource".into());
        }
        // Check the whole source set before continuing any file, including partial recovery.
        for file in &self.change.files {
            let current = workspace.content_hash(&file.relative_path)?;
            let after = blake3::hash(file.replacement.as_bytes())
                .to_hex()
                .to_string();
            if current != file.expected_content_hash && !(retained.is_some() && current == after) {
                return Err(format!(
                    "code change source conflicts with its exact basis: {}",
                    file.relative_path
                ));
            }
        }
        if retained.is_none() {
            for file in &self.change.files {
                workspace.require_unclaimed_stage(file, &self.id(&replay)?)?;
            }
            let intent = Intent {
                binding: self.binding.clone(),
                workspace_identity: identity,
                change: self.change.clone(),
            };
            events
                .append_durable_proven(
                    self.envelope(INTENT, self.id(&replay)?, &intent)?,
                    AppendMode::Idempotent,
                )
                .map_err(|e| e.to_string())?;
        }
        Ok(workspace)
    }

    pub fn apply(&self, events: &EventAppendCapability) -> Result<CodeChangeReceipt, String> {
        if let Some(receipt) = self.recover(&events.replay_capability())? {
            return Ok(receipt);
        }
        let workspace = self.prepare(events)?;
        let id = self.id(&events.replay_capability())?;
        for file in &self.change.files {
            workspace.replace(file, &id)?;
        }
        for file in &self.change.files {
            if workspace.content_hash(&file.relative_path)?
                != blake3::hash(file.replacement.as_bytes()).to_hex().as_str()
            {
                return Err("code change differs after materialization".into());
            }
        }
        events
            .append_durable_proven(
                self.completed_envelope(&events.replay_capability())?,
                AppendMode::Idempotent,
            )
            .map_err(|e| e.to_string())?;
        self.recover(&events.replay_capability())?
            .ok_or("code completion is not durably proven".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use meld_events::{DomainObjectRef, EventAuthority, EventAuthorityOpenOptions};

    fn change(root: &Path) -> CodeChangeSet {
        let mut files = Vec::new();
        for path in ["Cargo.toml", "Cargo.lock"] {
            std::fs::write(root.join(path), "version = 1\n").unwrap();
            files.push(FileReplacement {
                relative_path: path.into(),
                expected_content_hash: blake3::hash(b"version = 1\n").to_hex().to_string(),
                replacement: "version = 2\n".into(),
            });
        }
        CodeChangeSet::new(
            DomainObjectRef::new("workspace_fs", "node", "repo").unwrap(),
            vec![],
            files,
        )
        .unwrap()
    }

    #[test]
    fn partial_materialization_reopens_without_repeating_completed_files_or_borrowing_verification()
    {
        use std::os::unix::fs::MetadataExt;
        let workspace = tempfile::tempdir().unwrap();
        let external = tempfile::tempdir().unwrap();
        let change = change(workspace.path());
        let operation = Operation {
            binding: serde_json::json!({"invocation": "first"}),
            change,
            root: workspace.path(),
        };
        let applied_inode;
        {
            let events = EventAuthority::open(
                sled::open(external.path()).unwrap(),
                EventAuthorityOpenOptions::default(),
            )
            .unwrap();
            let target = operation.prepare(&events.append_capability()).unwrap();
            target
                .replace(
                    &operation.change.files[0],
                    &operation.id(&events.replay_capability()).unwrap(),
                )
                .unwrap();
            applied_inode = std::fs::metadata(
                workspace
                    .path()
                    .join(&operation.change.files[0].relative_path),
            )
            .unwrap()
            .ino();
            assert!(operation
                .recover(&events.replay_capability())
                .unwrap()
                .is_none());
        }
        let events = EventAuthority::open(
            sled::open(external.path()).unwrap(),
            EventAuthorityOpenOptions::default(),
        )
        .unwrap();
        let receipt = operation.apply(&events.append_capability()).unwrap();
        assert_eq!(receipt.materialized_files.len(), 2);
        assert_eq!(
            std::fs::metadata(
                workspace
                    .path()
                    .join(&operation.change.files[0].relative_path)
            )
            .unwrap()
            .ino(),
            applied_inode
        );
        for file in &operation.change.files {
            assert_eq!(
                std::fs::read_to_string(workspace.path().join(&file.relative_path)).unwrap(),
                file.replacement
            );
        }
        let count = events.watermark_capability().snapshot().unwrap().tip_seq;
        assert_eq!(count, 2);
        std::fs::write(
            workspace
                .path()
                .join(&operation.change.files[0].relative_path),
            "later user edit\n",
        )
        .unwrap();
        assert_eq!(
            operation.apply(&events.append_capability()).unwrap(),
            receipt
        );
        assert_eq!(
            std::fs::read_to_string(
                workspace
                    .path()
                    .join(&operation.change.files[0].relative_path)
            )
            .unwrap(),
            "later user edit\n"
        );
        assert_eq!(
            events.watermark_capability().snapshot().unwrap().tip_seq,
            count
        );
        let records = events.replay_capability().newest_page(16).unwrap().records;
        assert_eq!(records[1].event_type, COMPLETED);
        assert_eq!(records[1].provenance.source_records[0].seq, records[0].seq);
        assert!(!records
            .iter()
            .any(|record| record.domain_id == "dependency-security"));
        assert_eq!(
            std::fs::read_dir(workspace.path()).unwrap().count(),
            2,
            "no runtime journal or staging files remain in the target"
        );
    }

    #[test]
    fn exact_source_conflicts_and_symlink_escape_refuse_before_any_change() {
        let workspace = tempfile::tempdir().unwrap();
        let external = tempfile::tempdir().unwrap();
        let events = EventAuthority::open(
            sled::open(external.path().join("events")).unwrap(),
            EventAuthorityOpenOptions::default(),
        )
        .unwrap();
        let change = change(workspace.path());
        let operation = Operation {
            binding: serde_json::json!({"invocation": "conflict"}),
            change,
            root: workspace.path(),
        };
        let second = workspace
            .path()
            .join(&operation.change.files[1].relative_path);
        std::fs::write(&second, "user edit").unwrap();
        assert!(operation.apply(&events.append_capability()).is_err());
        assert_eq!(
            std::fs::read_to_string(
                workspace
                    .path()
                    .join(&operation.change.files[0].relative_path)
            )
            .unwrap(),
            "version = 1\n"
        );
        std::fs::remove_file(&second).unwrap();
        let foreign = external.path().join("foreign");
        std::fs::write(&foreign, "version = 1\n").unwrap();
        std::os::unix::fs::symlink(&foreign, &second).unwrap();
        assert!(operation.apply(&events.append_capability()).is_err());
        assert_eq!(std::fs::read_to_string(foreign).unwrap(), "version = 1\n");
        assert_eq!(events.watermark_capability().snapshot().unwrap().tip_seq, 0);
    }

    #[test]
    fn interrupted_staging_recovers_but_preexisting_or_conflicting_staging_is_preserved() {
        let workspace = tempfile::tempdir().unwrap();
        let external = tempfile::tempdir().unwrap();
        let events = EventAuthority::open(
            sled::open(external.path()).unwrap(),
            EventAuthorityOpenOptions::default(),
        )
        .unwrap();
        let change = change(workspace.path());
        let operation = Operation {
            binding: serde_json::json!({"invocation": "staging"}),
            change,
            root: workspace.path(),
        };
        let file = &operation.change.files[0];
        let stage = workspace.path().join(format!(
            ".meld-code-{}",
            &hash(&(
                &operation.id(&events.replay_capability()).unwrap(),
                &file.relative_path
            ))
            .unwrap()[..32]
        ));
        std::fs::write(&stage, "unrelated").unwrap();
        assert!(operation.apply(&events.append_capability()).is_err());
        assert_eq!(events.watermark_capability().snapshot().unwrap().tip_seq, 0);
        assert_eq!(std::fs::read_to_string(&stage).unwrap(), "unrelated");
        std::fs::remove_file(&stage).unwrap();
        operation.prepare(&events.append_capability()).unwrap();
        std::fs::write(&stage, "conflicting content").unwrap();
        assert!(operation.apply(&events.append_capability()).is_err());
        assert_eq!(
            std::fs::read_to_string(&stage).unwrap(),
            "conflicting content"
        );
        std::fs::write(&stage, &file.replacement.as_bytes()[..4]).unwrap();
        operation.apply(&events.append_capability()).unwrap();
        assert!(!stage.exists());
    }
    #[test]
    fn pending_intent_refuses_another_workspace_at_the_same_path() {
        let workspace = tempfile::tempdir().unwrap();
        let external = tempfile::tempdir().unwrap();
        let events = EventAuthority::open(
            sled::open(external.path().join("events")).unwrap(),
            EventAuthorityOpenOptions::default(),
        )
        .unwrap();
        let change = change(workspace.path());
        let operation = Operation {
            binding: serde_json::json!({"invocation": "root-binding"}),
            change,
            root: workspace.path(),
        };
        operation.prepare(&events.append_capability()).unwrap();
        std::fs::rename(workspace.path(), external.path().join("original-workspace")).unwrap();
        std::fs::create_dir(workspace.path()).unwrap();
        for file in &operation.change.files {
            std::fs::write(workspace.path().join(&file.relative_path), "version = 1\n").unwrap();
        }
        assert!(operation
            .apply(&events.append_capability())
            .unwrap_err()
            .contains("physical resource"));
        assert_eq!(events.watermark_capability().snapshot().unwrap().tip_seq, 1);
        for file in &operation.change.files {
            assert_eq!(
                std::fs::read_to_string(workspace.path().join(&file.relative_path)).unwrap(),
                "version = 1\n"
            );
        }
    }
}
