//! Installed selection of managed documents and source comparison boundaries.

use crate::error::ApiError;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::{Component, Path};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocsScopePolicy {
    pub operator: DocsScopeOperator,
    pub document_name: String,
    pub exclude_document_names_case_insensitive: bool,
    pub exclude_hidden_directories: bool,
    pub excluded_directory_names: BTreeSet<String>,
    pub comparison: DocsSourceComparison,
    /// Absent only in historical selections, which remain readable but cannot
    /// authorize a new capture.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capture_limits: Option<DocsCaptureLimits>,
}

/// Explicit bounds for the selected capture and its downstream evidence use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocsCaptureLimits {
    pub maximum_directories: usize,
    pub maximum_file_bytes: usize,
    pub maximum_directory_evidence_bytes: usize,
    pub maximum_child_readme_bytes: usize,
    pub maximum_revision_report_chars: usize,
}

/// Return the longest UTF-8 prefix whose encoded bytes fit the selected bound.
pub(crate) fn truncate_utf8_bytes(value: &str, maximum_bytes: usize) -> String {
    if value.len() <= maximum_bytes {
        return value.to_owned();
    }
    let mut end = maximum_bytes.min(value.len());
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    value[..end].to_owned()
}

/// Select nonbinary source files and manage one document per containing directory
/// and ancestor. Empty and document-only directories do not establish scope.
/// Bounded capture reports missing text as a coverage gap, never as satisfied work.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocsScopeOperator {
    DocumentPerSourceDirectoryV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocsSourceComparison {
    DirectorySubtreeV1,
    DirectDirectoryV1,
}

impl DocsScopePolicy {
    pub fn validate(&self) -> Result<(), ApiError> {
        self.validate_historical()?;
        let limits = self.capture_limits.as_ref().ok_or_else(|| {
            ApiError::ConfigError("Docs scope requires explicit capture limits".into())
        })?;
        if [
            limits.maximum_directories,
            limits.maximum_file_bytes,
            limits.maximum_directory_evidence_bytes,
            limits.maximum_child_readme_bytes,
            limits.maximum_revision_report_chars,
        ]
        .contains(&0)
        {
            return Err(ApiError::ConfigError(
                "Docs capture limits must be positive".into(),
            ));
        }
        Ok(())
    }

    /// Validate the shape shared with stored pre-limit selections. Historical
    /// readers use this without admitting the selection for a new capture.
    pub(crate) fn validate_historical(&self) -> Result<(), ApiError> {
        for name in std::iter::once(&self.document_name).chain(&self.excluded_directory_names) {
            let mut components = Path::new(name).components();
            if name.is_empty()
                || name.contains(['/', '\\'])
                || !matches!(components.next(), Some(Component::Normal(_)))
                || components.next().is_some()
            {
                return Err(ApiError::ConfigError(
                    "Docs scope requires plain file and directory names".into(),
                ));
            }
        }
        Ok(())
    }

    pub(crate) fn capture_limits(&self) -> Result<&DocsCaptureLimits, ApiError> {
        self.capture_limits.as_ref().ok_or_else(|| {
            ApiError::ConfigError("Docs scope requires explicit capture limits".into())
        })
    }

    pub(crate) fn document_path(&self, directory: &str) -> String {
        if directory == "." {
            self.document_name.clone()
        } else {
            format!("{directory}/{}", self.document_name)
        }
    }

    pub(crate) fn excludes_document(&self, path: &Path) -> bool {
        path.file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| {
                if self.exclude_document_names_case_insensitive {
                    name.eq_ignore_ascii_case(&self.document_name)
                } else {
                    name == self.document_name
                }
            })
    }

    pub(crate) fn includes_directory(&self, name: &str) -> bool {
        !(self.excluded_directory_names.contains(name)
            || self.exclude_hidden_directories && name.starts_with('.'))
    }

    pub(crate) fn compares(&self, document: &str, source: &str) -> bool {
        let Some(prefix) = document.strip_suffix(&self.document_name) else {
            return false;
        };
        match self.comparison {
            DocsSourceComparison::DirectorySubtreeV1 => source.starts_with(prefix),
            DocsSourceComparison::DirectDirectoryV1 => source
                .strip_prefix(prefix)
                .is_some_and(|tail| !tail.contains('/')),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy(capture_limits: Option<DocsCaptureLimits>) -> DocsScopePolicy {
        DocsScopePolicy {
            operator: DocsScopeOperator::DocumentPerSourceDirectoryV1,
            document_name: "README.md".into(),
            exclude_document_names_case_insensitive: true,
            exclude_hidden_directories: true,
            excluded_directory_names: BTreeSet::new(),
            comparison: DocsSourceComparison::DirectorySubtreeV1,
            capture_limits,
        }
    }

    fn limits() -> DocsCaptureLimits {
        DocsCaptureLimits {
            maximum_directories: 64,
            maximum_file_bytes: 8 * 1024,
            maximum_directory_evidence_bytes: 24 * 1024,
            maximum_child_readme_bytes: 3 * 1024,
            maximum_revision_report_chars: 8 * 1024,
        }
    }

    #[test]
    fn historical_scope_shape_is_exact_but_cannot_select_a_live_capture() {
        let historical = policy(None);
        assert_eq!(
            serde_json::to_string(&historical).unwrap(),
            r#"{"operator":"document_per_source_directory_v1","document_name":"README.md","exclude_document_names_case_insensitive":true,"exclude_hidden_directories":true,"excluded_directory_names":[],"comparison":"directory_subtree_v1"}"#
        );
        historical.validate_historical().unwrap();
        assert!(historical.validate().is_err());
    }

    #[test]
    fn every_selected_capture_bound_must_be_positive() {
        let selected = policy(Some(limits()));
        selected.validate().unwrap();
        for clear in 0..5 {
            let mut limits = limits();
            match clear {
                0 => limits.maximum_directories = 0,
                1 => limits.maximum_file_bytes = 0,
                2 => limits.maximum_directory_evidence_bytes = 0,
                3 => limits.maximum_child_readme_bytes = 0,
                4 => limits.maximum_revision_report_chars = 0,
                _ => unreachable!(),
            }
            assert!(policy(Some(limits)).validate().is_err());
        }
    }

    #[test]
    fn byte_truncation_never_splits_or_exceeds_multibyte_text() {
        let value = "aé🙂z";
        for maximum in 0..=value.len() {
            let prefix = truncate_utf8_bytes(value, maximum);
            assert!(prefix.len() <= maximum);
            assert!(value.starts_with(&prefix));
        }
        assert_eq!(truncate_utf8_bytes(value, 2), "a");
        assert_eq!(truncate_utf8_bytes(value, 3), "aé");
        assert_eq!(truncate_utf8_bytes(value, 6), "aé");
        assert_eq!(truncate_utf8_bytes(value, 7), "aé🙂");
        assert_eq!(truncate_utf8_bytes(value, 8), value);
    }
}
