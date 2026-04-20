use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::branches::catalog;
use crate::branches::contracts::BranchCatalogEntry;
use crate::branches::locator;
use crate::branches::runtime::BranchRuntime;
use crate::error::{ApiError, StorageError};
use crate::events::{DomainObjectRef, EventRelation};
use crate::world_state::{
    GraphWalkSpec, TraversalDirection, TraversalFactRecord, TraversalQuery, TraversalStore,
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
pub struct FederatedObjectPresence {
    pub branch_id: String,
    pub canonical_locator: String,
    pub object: DomainObjectRef,
    pub first_seen_seq: u64,
    pub last_seen_seq: u64,
    pub current_in_branch: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FederatedTraversalFact {
    pub branch_id: String,
    pub canonical_locator: String,
    pub federated_fact_id: String,
    pub fact: TraversalFactRecord,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FederatedRelationRecord {
    pub branch_id: String,
    pub canonical_locator: String,
    pub federated_fact_id: String,
    pub fact_id: String,
    pub seq: u64,
    pub relation: EventRelation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FederatedGraphWalkResult {
    pub visited_objects: Vec<FederatedObjectPresence>,
    pub visited_facts: Vec<FederatedTraversalFact>,
    pub traversed_relations: Vec<FederatedRelationRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FederatedNeighborsOutput {
    pub metadata: FederatedReadMetadata,
    pub neighbors: Vec<FederatedObjectPresence>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FederatedWalkOutput {
    pub metadata: FederatedReadMetadata,
    pub walk: FederatedGraphWalkResult,
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
                    let last_reduced_seq =
                        store.last_reduced_seq().map_err(ApiError::StorageError)?;
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
        let mut neighbors = Vec::new();

        for entry in &selection.entries {
            match self.query_branch(entry, |query| {
                let branch_neighbors =
                    query.neighbors(object, direction, relation_types, current_only)?;
                branch_neighbors
                    .into_iter()
                    .map(|neighbor| object_presence(entry, query, neighbor))
                    .collect::<Result<Vec<_>, StorageError>>()
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
        neighbors.sort_by(|left, right| {
            left.branch_id
                .cmp(&right.branch_id)
                .then(left.object.index_key().cmp(&right.object.index_key()))
        });
        Ok(FederatedNeighborsOutput {
            metadata,
            neighbors,
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
        let mut visited_objects = Vec::new();
        let mut visited_facts = Vec::new();
        let mut traversed_relations = Vec::new();

        for entry in &selection.entries {
            match self.query_branch(entry, |query| {
                let mut branch_spec = spec.clone();
                branch_spec.include_facts = true;
                let branch_walk = query.walk(start, &branch_spec)?;
                let object_rows = branch_walk
                    .visited_objects
                    .iter()
                    .cloned()
                    .map(|object| object_presence(entry, query, object))
                    .collect::<Result<Vec<_>, StorageError>>()?;
                let fact_rows = branch_walk
                    .visited_facts
                    .iter()
                    .cloned()
                    .map(|fact| federated_fact(entry, fact))
                    .collect::<Vec<_>>();
                let relation_rows = federated_relations(
                    entry,
                    &branch_walk.visited_facts,
                    &branch_walk.traversed_relations,
                );
                Ok((object_rows, fact_rows, relation_rows))
            }) {
                Ok((branch_objects, branch_facts, branch_relations)) => {
                    metadata.readable_branch_ids.push(entry.branch_id.clone());
                    visited_objects.extend(branch_objects);
                    if spec.include_facts {
                        visited_facts.extend(branch_facts);
                    }
                    traversed_relations.extend(branch_relations);
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
        visited_objects.sort_by(|left, right| {
            left.branch_id
                .cmp(&right.branch_id)
                .then(left.object.index_key().cmp(&right.object.index_key()))
        });
        visited_facts.sort_by(|left, right| {
            left.branch_id
                .cmp(&right.branch_id)
                .then(left.fact.seq.cmp(&right.fact.seq))
                .then(left.fact.fact_id.cmp(&right.fact.fact_id))
        });

        Ok(FederatedWalkOutput {
            metadata,
            walk: FederatedGraphWalkResult {
                visited_objects,
                visited_facts,
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

fn object_presence(
    entry: &BranchCatalogEntry,
    query: &TraversalQuery<'_>,
    object: DomainObjectRef,
) -> Result<FederatedObjectPresence, StorageError> {
    let facts = query.facts_for_object(&object, 0)?;
    let first_seen_seq = facts.first().map(|fact| fact.seq).unwrap_or_default();
    let last_seen_seq = facts.last().map(|fact| fact.seq).unwrap_or_default();
    Ok(FederatedObjectPresence {
        branch_id: entry.branch_id.clone(),
        canonical_locator: entry.canonical_locator.clone(),
        object,
        first_seen_seq,
        last_seen_seq,
        current_in_branch: !facts.is_empty(),
    })
}

fn federated_fact(entry: &BranchCatalogEntry, fact: TraversalFactRecord) -> FederatedTraversalFact {
    let federated_fact_id = federated_fact_id(&entry.branch_id, &fact.fact_id);
    FederatedTraversalFact {
        branch_id: entry.branch_id.clone(),
        canonical_locator: entry.canonical_locator.clone(),
        federated_fact_id,
        fact,
    }
}

fn federated_relations(
    entry: &BranchCatalogEntry,
    facts: &[TraversalFactRecord],
    traversed_relations: &[EventRelation],
) -> Vec<FederatedRelationRecord> {
    let mut rows = Vec::new();
    for relation in traversed_relations {
        if let Some(fact) = facts
            .iter()
            .find(|fact| fact.relations.iter().any(|candidate| candidate == relation))
        {
            rows.push(FederatedRelationRecord {
                branch_id: entry.branch_id.clone(),
                canonical_locator: entry.canonical_locator.clone(),
                federated_fact_id: federated_fact_id(&entry.branch_id, &fact.fact_id),
                fact_id: fact.fact_id.clone(),
                seq: fact.seq,
                relation: relation.clone(),
            });
        }
    }
    rows
}

fn federated_fact_id(branch_id: &str, fact_id: &str) -> String {
    format!("{branch_id}::{fact_id}")
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
