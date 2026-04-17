use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::branches::catalog;
use crate::branches::contracts::BranchCatalogEntry;
use crate::branches::locator;
use crate::branches::runtime::BranchRuntime;
use crate::error::{ApiError, StorageError};
use crate::telemetry::DomainObjectRef;
use crate::world_state::{
    GraphWalkResult, GraphWalkSpec, TraversalDirection, TraversalFactRecord, TraversalQuery,
    TraversalStore,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BranchQueryScope {
    All,
    Active,
    BranchIds(Vec<String>),
}

impl BranchQueryScope {
    pub fn scope_name(&self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Active => "active",
            Self::BranchIds(_) => "branch",
        }
    }

    pub fn requested_branch_ids(&self) -> Vec<String> {
        match self {
            Self::BranchIds(branch_ids) => branch_ids.clone(),
            _ => Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BranchReadFailure {
    pub branch_id: String,
    pub canonical_locator: String,
    pub store_path: Option<String>,
    pub error: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FederatedReadMetadata {
    pub scope: String,
    pub requested_branch_ids: Vec<String>,
    pub matched_branch_ids: Vec<String>,
    pub readable_branch_ids: Vec<String>,
    pub skipped_branches: Vec<BranchReadFailure>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BranchGraphStatusRow {
    pub branch_id: String,
    pub canonical_locator: String,
    pub store_path: Option<String>,
    pub read_status: String,
    pub last_reduced_seq: Option<u64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BranchGraphStatusOutput {
    pub metadata: FederatedReadMetadata,
    pub branches: Vec<BranchGraphStatusRow>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FederatedNeighborsOutput {
    pub metadata: FederatedReadMetadata,
    pub neighbors: Vec<DomainObjectRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FederatedWalkOutput {
    pub metadata: FederatedReadMetadata,
    pub walk: GraphWalkResult,
}

#[derive(Debug, Clone, Default)]
pub struct BranchQueryRuntime {
    branch_runtime: BranchRuntime,
}

#[derive(Debug, Clone)]
struct BranchSelection {
    entries: Vec<BranchCatalogEntry>,
    scope: BranchQueryScope,
}

impl BranchQueryRuntime {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn graph_status(
        &self,
        scope: BranchQueryScope,
        workspace_root: Option<&Path>,
    ) -> Result<BranchGraphStatusOutput, ApiError> {
        let selection = self.select_branches(scope, workspace_root)?;
        let mut metadata = self.base_metadata(&selection);
        let mut rows = Vec::new();

        for entry in &selection.entries {
            match self.open_traversal_store(entry) {
                Ok(store) => {
                    let last_reduced_seq = store.last_reduced_seq().map_err(ApiError::StorageError)?;
                    metadata.readable_branch_ids.push(entry.branch_id.clone());
                    rows.push(BranchGraphStatusRow {
                        branch_id: entry.branch_id.clone(),
                        canonical_locator: entry.canonical_locator.clone(),
                        store_path: entry.store_path.clone(),
                        read_status: "ready".to_string(),
                        last_reduced_seq: Some(last_reduced_seq),
                        error: None,
                    });
                }
                Err(err) => {
                    let error = err.to_string();
                    metadata
                        .skipped_branches
                        .push(read_failure(entry, error.clone()));
                    rows.push(BranchGraphStatusRow {
                        branch_id: entry.branch_id.clone(),
                        canonical_locator: entry.canonical_locator.clone(),
                        store_path: entry.store_path.clone(),
                        read_status: "unreadable".to_string(),
                        last_reduced_seq: None,
                        error: Some(error),
                    });
                }
            }
        }

        rows.sort_by(|left, right| left.branch_id.cmp(&right.branch_id));
        metadata.readable_branch_ids.sort();
        Ok(BranchGraphStatusOutput {
            metadata,
            branches: rows,
        })
    }

    pub fn neighbors(
        &self,
        scope: BranchQueryScope,
        workspace_root: Option<&Path>,
        object: &DomainObjectRef,
        direction: TraversalDirection,
        relation_types: Option<&[String]>,
        current_only: bool,
    ) -> Result<FederatedNeighborsOutput, ApiError> {
        let selection = self.select_branches(scope, workspace_root)?;
        let strict_scope = selection.scope.is_strict();
        let mut metadata = self.base_metadata(&selection);
        let mut neighbors = BTreeSet::new();

        for entry in &selection.entries {
            match self.query_branch(entry, |query| {
                query.neighbors(object, direction, relation_types, current_only)
            }) {
                Ok(branch_neighbors) => {
                    metadata.readable_branch_ids.push(entry.branch_id.clone());
                    neighbors.extend(branch_neighbors);
                }
                Err(err) => {
                    if strict_scope {
                        return Err(err);
                    }
                    metadata
                        .skipped_branches
                        .push(read_failure(entry, err.to_string()));
                }
            }
        }

        if metadata.readable_branch_ids.is_empty() {
            return Err(no_readable_branches_error(&metadata));
        }

        metadata.readable_branch_ids.sort();
        Ok(FederatedNeighborsOutput {
            metadata,
            neighbors: neighbors.into_iter().collect(),
        })
    }

    pub fn walk(
        &self,
        scope: BranchQueryScope,
        workspace_root: Option<&Path>,
        start: &DomainObjectRef,
        spec: &GraphWalkSpec,
    ) -> Result<FederatedWalkOutput, ApiError> {
        let selection = self.select_branches(scope, workspace_root)?;
        let strict_scope = selection.scope.is_strict();
        let mut metadata = self.base_metadata(&selection);
        let mut visited_objects = BTreeSet::new();
        let mut visited_facts = BTreeMap::new();
        let mut traversed_relations = Vec::new();

        for entry in &selection.entries {
            match self.query_branch(entry, |query| query.walk(start, spec)) {
                Ok(branch_walk) => {
                    metadata.readable_branch_ids.push(entry.branch_id.clone());
                    visited_objects.extend(branch_walk.visited_objects);
                    for fact in branch_walk.visited_facts {
                        visited_facts.insert(fact.fact_id.clone(), fact);
                    }
                    traversed_relations.extend(branch_walk.traversed_relations);
                }
                Err(err) => {
                    if strict_scope {
                        return Err(err);
                    }
                    metadata
                        .skipped_branches
                        .push(read_failure(entry, err.to_string()));
                }
            }
        }

        if metadata.readable_branch_ids.is_empty() {
            return Err(no_readable_branches_error(&metadata));
        }

        metadata.readable_branch_ids.sort();
        let mut facts: Vec<TraversalFactRecord> = visited_facts.into_values().collect();
        facts.sort_by(|left, right| left.seq.cmp(&right.seq).then(left.fact_id.cmp(&right.fact_id)));

        Ok(FederatedWalkOutput {
            metadata,
            walk: GraphWalkResult {
                visited_objects: visited_objects.into_iter().collect(),
                visited_facts: facts,
                traversed_relations,
            },
        })
    }

    fn select_branches(
        &self,
        scope: BranchQueryScope,
        workspace_root: Option<&Path>,
    ) -> Result<BranchSelection, ApiError> {
        let catalog_path = locator::global_catalog_path()?;
        let branch_catalog = catalog::load(&catalog_path)?;
        let entries = match &scope {
            BranchQueryScope::All => branch_catalog.branches,
            BranchQueryScope::Active => {
                let workspace_root = workspace_root.ok_or_else(|| {
                    ApiError::ConfigError(
                        "Active branch scope requires a workspace root".to_string(),
                    )
                })?;
                let active_branch = self.branch_runtime.resolve_active_branch(workspace_root)?;
                branch_catalog
                    .branches
                    .into_iter()
                    .filter(|entry| entry.branch_id == active_branch.resolved().branch_id)
                    .collect()
            }
            BranchQueryScope::BranchIds(branch_ids) => {
                let requested: BTreeSet<_> = branch_ids.iter().collect();
                branch_catalog
                    .branches
                    .into_iter()
                    .filter(|entry| requested.contains(&entry.branch_id))
                    .collect()
            }
        };

        if entries.is_empty() {
            return Err(ApiError::ConfigError(format!(
                "No branches matched scope '{}'",
                scope.scope_name()
            )));
        }

        let mut entries = entries;
        entries.sort_by(|left, right| left.branch_id.cmp(&right.branch_id));
        Ok(BranchSelection { entries, scope })
    }

    fn base_metadata(&self, selection: &BranchSelection) -> FederatedReadMetadata {
        FederatedReadMetadata {
            scope: selection.scope.scope_name().to_string(),
            requested_branch_ids: selection.scope.requested_branch_ids(),
            matched_branch_ids: selection
                .entries
                .iter()
                .map(|entry| entry.branch_id.clone())
                .collect(),
            readable_branch_ids: Vec::new(),
            skipped_branches: Vec::new(),
        }
    }

    fn query_branch<T, F>(&self, entry: &BranchCatalogEntry, f: F) -> Result<T, ApiError>
    where
        F: FnOnce(&TraversalQuery<'_>) -> Result<T, StorageError>,
    {
        let store = self.open_traversal_store(entry)?;
        let query = TraversalQuery::new(store.as_ref());
        f(&query).map_err(ApiError::StorageError)
    }

    fn open_traversal_store(
        &self,
        entry: &BranchCatalogEntry,
    ) -> Result<Arc<TraversalStore>, ApiError> {
        let store_path = branch_store_path(entry);
        if !store_path.exists() {
            return Err(ApiError::ConfigError(format!(
                "Branch store path does not exist: {}",
                store_path.display()
            )));
        }
        let db = sled::open(&store_path).map_err(to_api_storage_error)?;
        TraversalStore::shared(db).map_err(ApiError::StorageError)
    }
}

impl BranchQueryScope {
    fn is_strict(&self) -> bool {
        match self {
            Self::All => false,
            Self::Active => true,
            Self::BranchIds(branch_ids) => branch_ids.len() == 1,
        }
    }
}

fn branch_store_path(entry: &BranchCatalogEntry) -> PathBuf {
    entry
        .store_path
        .as_ref()
        .map(PathBuf::from)
        .unwrap_or_else(|| locator::branch_store_path(Path::new(&entry.data_home_path)))
}

fn read_failure(entry: &BranchCatalogEntry, error: String) -> BranchReadFailure {
    BranchReadFailure {
        branch_id: entry.branch_id.clone(),
        canonical_locator: entry.canonical_locator.clone(),
        store_path: entry.store_path.clone(),
        error,
    }
}

fn no_readable_branches_error(metadata: &FederatedReadMetadata) -> ApiError {
    if let Some(first_failure) = metadata.skipped_branches.first() {
        ApiError::ConfigError(format!(
            "No readable branches in scope '{}': {}",
            metadata.scope, first_failure.error
        ))
    } else {
        ApiError::ConfigError(format!(
            "No readable branches in scope '{}'",
            metadata.scope
        ))
    }
}

fn to_api_storage_error(error: sled::Error) -> ApiError {
    ApiError::StorageError(StorageError::IoError(std::io::Error::other(format!(
        "Failed to open sled database: {}",
        error
    ))))
}
