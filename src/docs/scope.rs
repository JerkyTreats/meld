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
