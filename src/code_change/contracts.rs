use meld_events::DomainObjectRef;
use serde::{Deserialize, Serialize};
use std::path::{Component, Path};

pub const MAX_FILES: usize = 64;
pub const MAX_BYTES: usize = 16 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// UTF-8 replacement guarded by the BLAKE3 digest of the prior file bytes.
pub struct FileReplacement {
    pub relative_path: String,
    pub expected_content_hash: String,
    pub replacement: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Canonical change proposal; Execution supplies its separate mutation authority.
pub struct CodeChangeSet {
    pub change_id: String,
    pub subject: DomainObjectRef,
    pub reason_refs: Vec<DomainObjectRef>,
    pub files: Vec<FileReplacement>,
}

pub(crate) fn hash(value: &impl Serialize) -> Result<String, String> {
    serde_json::to_vec(value)
        .map(|bytes| blake3::hash(&bytes).to_hex().to_string())
        .map_err(|e| e.to_string())
}

impl CodeChangeSet {
    pub fn new(
        subject: DomainObjectRef,
        mut reason_refs: Vec<DomainObjectRef>,
        mut files: Vec<FileReplacement>,
    ) -> Result<Self, String> {
        subject.validate().map_err(|e| e.to_string())?;
        for reason in &reason_refs {
            reason.validate().map_err(|e| e.to_string())?;
        }
        reason_refs.sort_by_key(DomainObjectRef::index_key);
        reason_refs.dedup();
        files.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
        if files.is_empty() || files.len() > MAX_FILES {
            return Err("code change requires a bounded nonempty set of files".into());
        }
        let mut remaining = MAX_BYTES;
        let mut prior = None;
        for file in &files {
            let path = Path::new(&file.relative_path);
            if file.relative_path.is_empty()
                || path
                    .components()
                    .any(|part| !matches!(part, Component::Normal(_)))
                || path.to_string_lossy()
                    != path
                        .components()
                        .collect::<std::path::PathBuf>()
                        .to_string_lossy()
                || prior == Some(&file.relative_path)
            {
                return Err(
                    "code change paths must be unique, normalized workspace-relative files".into(),
                );
            }
            if file.expected_content_hash.len() != 64
                || !file
                    .expected_content_hash
                    .bytes()
                    .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
            {
                return Err("code change requires an exact prior content hash".into());
            }
            remaining = remaining
                .checked_sub(file.replacement.len())
                .ok_or("code change exceeds its byte bound")?;
            if blake3::hash(file.replacement.as_bytes()).to_hex().as_str()
                == file.expected_content_hash
            {
                return Err("code change must change the declared file content".into());
            }
            prior = Some(&file.relative_path);
        }
        let change_id = format!("code-change::{}", hash(&(&subject, &reason_refs, &files))?);
        Ok(Self {
            change_id,
            subject,
            reason_refs,
            files,
        })
    }

    pub fn validate(&self) -> Result<(), String> {
        if Self::new(
            self.subject.clone(),
            self.reason_refs.clone(),
            self.files.clone(),
        )? != *self
        {
            return Err("code change identity or canonical content differs".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Observed materialization under one durable intent, independent of downstream judgment.
pub struct CodeChangeReceipt {
    pub operation_id: String,
    pub change_id: String,
    pub subject: DomainObjectRef,
    pub materialized_files: Vec<MaterializedFile>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaterializedFile {
    pub relative_path: String,
    pub before_hash: String,
    pub after_hash: String,
}

impl CodeChangeReceipt {
    pub(crate) fn expected(operation_id: String, change: &CodeChangeSet) -> Self {
        Self {
            operation_id,
            change_id: change.change_id.clone(),
            subject: change.subject.clone(),
            materialized_files: change
                .files
                .iter()
                .map(|file| MaterializedFile {
                    relative_path: file.relative_path.clone(),
                    before_hash: file.expected_content_hash.clone(),
                    after_hash: blake3::hash(file.replacement.as_bytes())
                        .to_hex()
                        .to_string(),
                })
                .collect(),
        }
    }
}

/// Code-change-owned proof for an intact artifact returned through Execution.
/// Physical identity names the directory bound when the intent was accepted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaterializationEvidence {
    pub change: CodeChangeSet,
    pub intent: meld_events::EventRecordRef,
    pub materialization: meld_events::EventRecordRef,
    pub workspace_root: std::path::PathBuf,
    pub workspace_identity: (u64, u64),
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn change_identity_refuses_escape_aliases_duplicates_and_tampering() {
        let subject = DomainObjectRef::new("workspace_fs", "node", "repo").unwrap();
        let original = FileReplacement {
            relative_path: "src/main.rs".into(),
            expected_content_hash: blake3::hash(b"before").to_hex().to_string(),
            replacement: "after".into(),
        };
        for path in [
            "../outside",
            "/outside",
            "src/../main.rs",
            "src//main.rs",
            "src/./main.rs",
            "src/main.rs/",
        ] {
            let mut file = original.clone();
            file.relative_path = path.into();
            assert!(
                CodeChangeSet::new(subject.clone(), vec![], vec![file]).is_err(),
                "{path}"
            );
        }
        assert!(CodeChangeSet::new(
            subject.clone(),
            vec![],
            vec![original.clone(), original.clone()]
        )
        .is_err());
        let mut change = CodeChangeSet::new(subject, vec![], vec![original]).unwrap();
        change.files[0].replacement = "unreviewed replacement".into();
        assert!(change.validate().is_err());
    }
}
