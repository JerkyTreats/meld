//! Product-owned binding from one product identity to one event authority.
//!
//! The event crate owns ledger migration semantics. This root adapter owns the
//! product path, identity, cutover lock, and crash-safe binding file.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use fs2::FileExt;
use meld_events::{
    EventAuthority, EventAuthorityOpenOptions, LedgerIdentity, LegacyEventMigrationOptions,
    LegacyEventMigrationSource,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::branches::ResolvedBranch;

const BINDING_SCHEMA_VERSION: u32 = 1;
const BINDING_FILE: &str = "event_authority.json";
const LOCK_FILE: &str = "event_authority.lock";

/// Durable state of one product event-authority cutover.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProductEventBindingState {
    /// Target identity is durable and legacy history is being copied.
    Preparing,
    /// Target identity and any legacy migration have been verified.
    Active,
}

/// Frozen legacy source named by a product binding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LegacyEventSourceBinding {
    /// Canonical path of the compatibility event database.
    pub ledger_path: PathBuf,
    /// Persisted identity assigned before source rows are copied.
    pub ledger_identity: LedgerIdentity,
}

/// Durable mapping from one product to one canonical event authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProductEventBinding {
    /// Binding wire schema.
    pub schema_version: u32,
    /// Product identity. The wire field retains its original branch name for reopen compatibility.
    pub branch_id: String,
    /// Canonical path of the writable authority ledger.
    pub ledger_path: PathBuf,
    /// Durable authority identity expected at `ledger_path`.
    pub ledger_identity: LedgerIdentity,
    /// Monotonic cutover generation.
    pub generation: u64,
    /// Crash-recovery state of the binding.
    pub state: ProductEventBindingState,
    /// Frozen source, when product history required migration.
    pub source: Option<LegacyEventSourceBinding>,
}

/// Resolved product authority and the active binding that selected it.
pub struct ResolvedProductEventAuthority {
    /// One process-local writable authority shared by CLI and runtime hosts.
    pub authority: Arc<EventAuthority>,
    /// Verified active product binding.
    pub binding: ProductEventBinding,
}

/// Failures while resolving or recovering a product authority binding.
#[derive(Debug, Error)]
pub enum ProductEventBindingError {
    /// Product binding persistence failed.
    #[error("product event binding I/O failed: {0}")]
    Io(#[from] io::Error),
    /// Product binding JSON is malformed.
    #[error("product event binding JSON is invalid: {0}")]
    Json(#[from] serde_json::Error),
    /// The event authority or migration rejected persisted state.
    #[error("event authority rejected product binding: {0}")]
    Authority(#[from] meld_events::error::EventAuthorityError),
    /// Persisted binding does not name the requested product or path.
    #[error("product event binding mismatch: {0}")]
    Mismatch(String),
    /// A preparing binding cannot resume because its source disappeared.
    #[error("product event cutover cannot resume: {0}")]
    SourceUnavailable(String),
}

/// Resolve, create, or resume the authority bound to one product branch.
///
/// The caller supplies the externally validated product ledger path and the
/// compatibility database that may contain legacy event trees. An active
/// binding never falls back to that source.
pub fn resolve_product_event_authority(
    branch: &ResolvedBranch,
    target_ledger_path: &Path,
    legacy_ledger_path: &Path,
) -> Result<ResolvedProductEventAuthority, ProductEventBindingError> {
    resolve_product_event_authority_at(
        &branch.branch_id,
        &branch.data_home_path,
        target_ledger_path,
        legacy_ledger_path,
    )
}

/// Resolve the canonical authority for a product with or without a workspace branch.
/// The same durable binding and cutover protocol apply to both scopes.
pub fn resolve_product_event_authority_at(
    product_id: &str,
    binding_directory: &Path,
    target_ledger_path: &Path,
    legacy_ledger_path: &Path,
) -> Result<ResolvedProductEventAuthority, ProductEventBindingError> {
    if product_id.trim().is_empty() {
        return Err(ProductEventBindingError::Mismatch(
            "empty product identity".to_string(),
        ));
    }
    fs::create_dir_all(binding_directory)?;
    let _lock = CutoverLock::acquire(&binding_directory.join(LOCK_FILE))?;

    let binding_path = binding_directory.join(BINDING_FILE);
    let persisted_binding = read_binding(&binding_path)?;
    fs::create_dir_all(target_ledger_path)?;
    if persisted_binding.is_none() {
        // A fresh workspace still migrates an empty compatibility database so
        // it receives a cutover marker before SessionStore opens its separate
        // trees. Existing bindings never recreate a missing frozen source.
        fs::create_dir_all(legacy_ledger_path)?;
    }
    let target_path = target_ledger_path.canonicalize()?;
    let legacy_path = canonicalize_if_present(legacy_ledger_path)?;
    if legacy_path
        .as_ref()
        .is_some_and(|path| path == &target_path)
    {
        return Err(ProductEventBindingError::Mismatch(format!(
            "legacy source and target resolve to the same path {}",
            target_path.display()
        )));
    }

    match persisted_binding {
        Some(binding) => {
            resolve_existing_binding(product_id, &binding_path, target_path, legacy_path, binding)
        }
        None => create_binding(product_id, &binding_path, target_path, legacy_path),
    }
}

fn resolve_existing_binding(
    product_id: &str,
    binding_path: &Path,
    target_path: PathBuf,
    legacy_path: Option<PathBuf>,
    binding: ProductEventBinding,
) -> Result<ResolvedProductEventAuthority, ProductEventBindingError> {
    validate_binding(product_id, &target_path, &binding)?;

    match binding.state {
        ProductEventBindingState::Active => {
            validate_active_source(&binding, legacy_path.as_deref())?;
            let authority =
                open_bound_authority(&target_path, binding.ledger_identity, product_id)?;
            Ok(ResolvedProductEventAuthority { authority, binding })
        }
        ProductEventBindingState::Preparing => {
            let source_binding = binding.source.as_ref().ok_or_else(|| {
                ProductEventBindingError::SourceUnavailable(
                    "preparing binding has no legacy source".to_string(),
                )
            })?;
            let source_path = legacy_path.ok_or_else(|| {
                ProductEventBindingError::SourceUnavailable(format!(
                    "legacy source {} no longer exists",
                    source_binding.ledger_path.display()
                ))
            })?;
            if source_path != source_binding.ledger_path {
                return Err(ProductEventBindingError::Mismatch(format!(
                    "legacy source path changed from {} to {}",
                    source_binding.ledger_path.display(),
                    source_path.display()
                )));
            }
            let source = LegacyEventMigrationSource::open(&source_path)?;
            if source.ledger_identity() != source_binding.ledger_identity {
                return Err(ProductEventBindingError::Mismatch(format!(
                    "legacy source identity changed from {} to {}",
                    source_binding.ledger_identity,
                    source.ledger_identity()
                )));
            }
            let authority =
                open_bound_authority(&target_path, binding.ledger_identity, product_id)?;
            let report = source.migrate_all_into(
                &target_path,
                authority.as_ref(),
                LegacyEventMigrationOptions::new(binding.generation),
            )?;
            if !report.complete {
                return Err(ProductEventBindingError::SourceUnavailable(
                    "migration returned without a durable cutover marker".to_string(),
                ));
            }
            let mut active = binding;
            active.state = ProductEventBindingState::Active;
            write_binding(binding_path, &active)?;
            Ok(ResolvedProductEventAuthority {
                authority,
                binding: active,
            })
        }
    }
}

fn create_binding(
    product_id: &str,
    binding_path: &Path,
    target_path: PathBuf,
    legacy_path: Option<PathBuf>,
) -> Result<ResolvedProductEventAuthority, ProductEventBindingError> {
    let source = legacy_path
        .as_deref()
        .map(LegacyEventMigrationSource::open)
        .transpose()?;
    if source
        .as_ref()
        .map(LegacyEventMigrationSource::cutover_marker)
        .transpose()?
        .flatten()
        .is_some()
    {
        return Err(ProductEventBindingError::Mismatch(
            "legacy cutover marker exists but product binding is missing".to_string(),
        ));
    }
    let authority = Arc::new(EventAuthority::open(
        sled::open(&target_path).map_err(authority_persistence)?,
        EventAuthorityOpenOptions::default(),
    )?);
    authority.bind_product_identity(product_id)?;
    let source_binding = source.as_ref().map(|source| LegacyEventSourceBinding {
        ledger_path: source.canonical_path().to_path_buf(),
        ledger_identity: source.ledger_identity(),
    });
    let mut binding = ProductEventBinding {
        schema_version: BINDING_SCHEMA_VERSION,
        branch_id: product_id.to_string(),
        ledger_path: target_path.clone(),
        ledger_identity: authority.ledger_identity(),
        generation: 1,
        state: if source.is_some() {
            ProductEventBindingState::Preparing
        } else {
            ProductEventBindingState::Active
        },
        source: source_binding,
    };
    write_binding(binding_path, &binding)?;

    if let Some(source) = source {
        let report = source.migrate_all_into(
            &target_path,
            authority.as_ref(),
            LegacyEventMigrationOptions::new(binding.generation),
        )?;
        if !report.complete {
            return Err(ProductEventBindingError::SourceUnavailable(
                "migration returned without a durable cutover marker".to_string(),
            ));
        }
        binding.state = ProductEventBindingState::Active;
        write_binding(binding_path, &binding)?;
    }

    Ok(ResolvedProductEventAuthority { authority, binding })
}

fn validate_active_source(
    binding: &ProductEventBinding,
    legacy_path: Option<&Path>,
) -> Result<(), ProductEventBindingError> {
    let Some(source_binding) = binding.source.as_ref() else {
        return Ok(());
    };
    let source_path = legacy_path.ok_or_else(|| {
        ProductEventBindingError::SourceUnavailable(format!(
            "active binding source {} no longer exists",
            source_binding.ledger_path.display()
        ))
    })?;
    if source_path != source_binding.ledger_path {
        return Err(ProductEventBindingError::Mismatch(format!(
            "legacy source path changed from {} to {}",
            source_binding.ledger_path.display(),
            source_path.display()
        )));
    }
    let source = LegacyEventMigrationSource::open(source_path)?;
    if source.ledger_identity() != source_binding.ledger_identity {
        return Err(ProductEventBindingError::Mismatch(format!(
            "legacy source identity changed from {} to {}",
            source_binding.ledger_identity,
            source.ledger_identity()
        )));
    }
    let marker = source.cutover_marker()?.ok_or_else(|| {
        ProductEventBindingError::Mismatch(
            "active binding source is missing its durable cutover marker".to_string(),
        )
    })?;
    if marker.target_ledger_id != binding.ledger_identity || marker.generation != binding.generation
    {
        return Err(ProductEventBindingError::Mismatch(format!(
            "legacy cutover marker targets ledger {} generation {}, expected {} generation {}",
            marker.target_ledger_id, marker.generation, binding.ledger_identity, binding.generation
        )));
    }
    Ok(())
}

fn validate_binding(
    product_id: &str,
    target_path: &Path,
    binding: &ProductEventBinding,
) -> Result<(), ProductEventBindingError> {
    if binding.schema_version != BINDING_SCHEMA_VERSION {
        return Err(ProductEventBindingError::Mismatch(format!(
            "unsupported schema version {}",
            binding.schema_version
        )));
    }
    if binding.branch_id != product_id {
        return Err(ProductEventBindingError::Mismatch(format!(
            "binding branch {} does not match resolved branch {}",
            binding.branch_id, product_id
        )));
    }
    if binding.ledger_path != target_path {
        return Err(ProductEventBindingError::Mismatch(format!(
            "binding ledger path {} does not match configured path {}",
            binding.ledger_path.display(),
            target_path.display()
        )));
    }
    if binding.generation == 0 {
        return Err(ProductEventBindingError::Mismatch(
            "binding generation must be non-zero".to_string(),
        ));
    }
    Ok(())
}

fn open_bound_authority(
    target_path: &Path,
    ledger_identity: LedgerIdentity,
    product_identity: &str,
) -> Result<Arc<EventAuthority>, ProductEventBindingError> {
    let authority = Arc::new(EventAuthority::open_existing(
        sled::open(target_path).map_err(authority_persistence)?,
        ledger_identity,
    )?);
    authority.bind_product_identity(product_identity)?;
    Ok(authority)
}

fn read_binding(path: &Path) -> Result<Option<ProductEventBinding>, ProductEventBindingError> {
    match fs::read(path) {
        Ok(bytes) => Ok(Some(serde_json::from_slice(&bytes)?)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn write_binding(
    path: &Path,
    binding: &ProductEventBinding,
) -> Result<(), ProductEventBindingError> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "binding path has no parent"))?;
    fs::create_dir_all(parent)?;
    let temp_path = parent.join(format!(
        ".{BINDING_FILE}.{}.{}.tmp",
        std::process::id(),
        binding.generation
    ));
    let bytes = serde_json::to_vec_pretty(binding)?;
    let mut temp = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&temp_path)?;
    temp.write_all(&bytes)?;
    temp.write_all(b"\n")?;
    temp.sync_all()?;
    fs::rename(&temp_path, path)?;
    File::open(parent)?.sync_all()?;
    Ok(())
}

fn canonicalize_if_present(path: &Path) -> Result<Option<PathBuf>, io::Error> {
    if path.exists() {
        path.canonicalize().map(Some)
    } else {
        Ok(None)
    }
}

fn authority_persistence(error: sled::Error) -> meld_events::error::EventAuthorityError {
    meld_events::error::EventAuthorityError::Persistence {
        message: error.to_string(),
    }
}

struct CutoverLock {
    file: File,
}

impl CutoverLock {
    fn acquire(path: &Path) -> Result<Self, io::Error> {
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(path)?;
        file.lock_exclusive()?;
        Ok(Self { file })
    }
}

impl Drop for CutoverLock {
    fn drop(&mut self) {
        let _ = self.file.unlock();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::branches::BranchKind;
    use meld_events::events::test_support::{EventStore, EventStoreTestSupport as _}; // boundary-allow: event-test
    use meld_events::EventEnvelope;
    use serde_json::json;

    fn branch(root: &Path, branch_id: &str) -> ResolvedBranch {
        ResolvedBranch {
            branch_id: branch_id.to_string(),
            branch_kind: BranchKind::WorkspaceFs,
            canonical_locator: root.join("workspace"),
            data_home_path: root.join("branch-home"),
            manifest_path: root.join("branch-home/branch_manifest.json"),
            ledger_path: root.join("branch-home/branch_migration_ledger.jsonl"),
        }
    }

    fn write_legacy_events(path: &Path, count: usize) {
        fs::create_dir_all(path).unwrap();
        let store = EventStore::new(sled::open(path).unwrap()).unwrap(); // boundary-allow: event-test
        for index in 0..count {
            store
                .append_envelope(
                    EventEnvelope::new_domain(
                        "2026-07-12T00:00:00Z".to_string(),
                        "legacy-session",
                        "workspace_fs",
                        "workspace-a",
                        format!("workspace.legacy_{index}"),
                        None,
                        json!({ "index": index }),
                    )
                    .with_record_id(format!("legacy-{index}")),
                )
                .unwrap();
        }
        store.flush().unwrap();
    }

    #[test]
    fn empty_product_binding_is_active_and_stable_after_reopen() {
        let temp = tempfile::tempdir().unwrap();
        let branch = branch(temp.path(), "branch-a");
        let target = temp.path().join("product/ledger.sled");
        let legacy = temp.path().join("missing-legacy.sled");

        let first = resolve_product_event_authority(&branch, &target, &legacy).unwrap();
        let identity = first.authority.ledger_identity();
        assert_eq!(first.binding.state, ProductEventBindingState::Active);
        assert!(first.binding.source.is_some());
        drop(first);

        let second = resolve_product_event_authority(&branch, &target, &legacy).unwrap();
        assert_eq!(second.authority.ledger_identity(), identity);
        assert_eq!(second.binding.ledger_identity, identity);
    }

    #[test]
    fn persisted_binding_rejects_another_branch_without_fallback() {
        let temp = tempfile::tempdir().unwrap();
        let target = temp.path().join("product/ledger.sled");
        let legacy = temp.path().join("missing-legacy.sled");
        let first_branch = branch(temp.path(), "branch-a");
        let second_branch = branch(temp.path(), "branch-b");

        let resolved = resolve_product_event_authority(&first_branch, &target, &legacy).unwrap();
        drop(resolved);
        let error = resolve_product_event_authority(&second_branch, &target, &legacy)
            .err()
            .expect("mismatched branch must fail");
        assert!(error.to_string().contains("does not match resolved branch"));
    }

    #[test]
    fn separate_branch_bindings_cannot_share_one_target_ledger() {
        let temp = tempfile::tempdir().unwrap();
        let target = temp.path().join("shared-product/ledger.sled");
        let first_root = temp.path().join("first");
        let second_root = temp.path().join("second");
        let first_branch = branch(&first_root, "branch-a");
        let second_branch = branch(&second_root, "branch-b");
        let first_legacy = first_root.join("legacy.sled");
        let second_legacy = second_root.join("legacy.sled");

        let resolved =
            resolve_product_event_authority(&first_branch, &target, &first_legacy).unwrap();
        let ledger_id = resolved.authority.ledger_identity();
        drop(resolved);

        let error = resolve_product_event_authority(&second_branch, &target, &second_legacy)
            .err()
            .expect("another branch must not claim a shared target ledger");
        assert!(error.to_string().contains("bound to product branch-a"));
        assert!(
            !second_branch.data_home_path.join(BINDING_FILE).exists(),
            "a rejected branch must not persist a local binding"
        );

        let reopened = resolve_product_event_authority(&first_branch, &target, &first_legacy)
            .expect("the original product binding must remain valid");
        assert_eq!(reopened.authority.ledger_identity(), ledger_id);
    }

    #[test]
    fn active_binding_rejects_a_deleted_ledger_instead_of_reseeding_identity() {
        let temp = tempfile::tempdir().unwrap();
        let target = temp.path().join("product/ledger.sled");
        let legacy = temp.path().join("missing-legacy.sled");
        let branch = branch(temp.path(), "branch-a");

        let resolved = resolve_product_event_authority(&branch, &target, &legacy).unwrap();
        drop(resolved);
        fs::remove_dir_all(&target).unwrap();

        let error = resolve_product_event_authority(&branch, &target, &legacy)
            .err()
            .expect("deleted active ledger must fail closed");
        assert!(error.to_string().contains("no persisted ledger_identity"));
    }

    #[test]
    fn preparing_binding_resumes_a_partial_copy_and_promotes_active() {
        let temp = tempfile::tempdir().unwrap();
        let branch = branch(temp.path(), "branch-a");
        let target = temp.path().join("product/ledger.sled");
        let legacy = temp.path().join("legacy.sled");
        write_legacy_events(&legacy, 3);
        fs::create_dir_all(&branch.data_home_path).unwrap();
        fs::create_dir_all(&target).unwrap();

        let source = LegacyEventMigrationSource::open(&legacy).unwrap();
        let authority = EventAuthority::open(
            sled::open(&target).unwrap(),
            EventAuthorityOpenOptions::default(),
        )
        .unwrap();
        let binding = ProductEventBinding {
            schema_version: BINDING_SCHEMA_VERSION,
            branch_id: branch.branch_id.clone(),
            ledger_path: target.canonicalize().unwrap(),
            ledger_identity: authority.ledger_identity(),
            generation: 7,
            state: ProductEventBindingState::Preparing,
            source: Some(LegacyEventSourceBinding {
                ledger_path: legacy.canonicalize().unwrap(),
                ledger_identity: source.ledger_identity(),
            }),
        };
        write_binding(&branch.data_home_path.join(BINDING_FILE), &binding).unwrap();
        let partial = source
            .migrate_batch_into(&target, &authority, binding.generation, 1)
            .unwrap();
        assert!(!partial.complete);
        assert_eq!(partial.mapped_record_count, 1);
        drop(source);
        drop(authority);

        let resolved = resolve_product_event_authority(&branch, &target, &legacy).unwrap();
        assert_eq!(resolved.binding.state, ProductEventBindingState::Active);
        assert_eq!(resolved.binding.generation, 7);
        assert_eq!(
            resolved
                .authority
                .watermark_capability()
                .snapshot()
                .unwrap()
                .tip_seq,
            3
        );
    }

    #[test]
    fn active_binding_fails_closed_when_the_frozen_source_is_deleted() {
        let temp = tempfile::tempdir().unwrap();
        let branch = branch(temp.path(), "branch-a");
        let target = temp.path().join("product/ledger.sled");
        let legacy = temp.path().join("legacy.sled");
        write_legacy_events(&legacy, 1);

        let resolved = resolve_product_event_authority(&branch, &target, &legacy).unwrap();
        assert!(resolved.binding.source.is_some());
        drop(resolved);
        fs::remove_dir_all(&legacy).unwrap();

        let error = resolve_product_event_authority(&branch, &target, &legacy)
            .err()
            .expect("deleted frozen source must fail closed");
        assert!(matches!(
            error,
            ProductEventBindingError::SourceUnavailable(_)
        ));
        assert!(
            !legacy.exists(),
            "active reopen must not recreate the source"
        );
    }

    #[test]
    fn missing_binding_rejects_an_existing_cutover_marker() {
        let temp = tempfile::tempdir().unwrap();
        let branch = branch(temp.path(), "branch-a");
        let target = temp.path().join("product/ledger.sled");
        let legacy = temp.path().join("legacy.sled");
        write_legacy_events(&legacy, 1);

        let resolved = resolve_product_event_authority(&branch, &target, &legacy).unwrap();
        drop(resolved);
        fs::remove_file(branch.data_home_path.join(BINDING_FILE)).unwrap();

        let error = resolve_product_event_authority(&branch, &target, &legacy)
            .err()
            .expect("cutover marker without binding must fail closed");
        assert!(matches!(error, ProductEventBindingError::Mismatch(_)));
    }

    #[test]
    fn active_binding_rejects_ledger_identity_substitution() {
        let temp = tempfile::tempdir().unwrap();
        let branch = branch(temp.path(), "branch-a");
        let target = temp.path().join("product/ledger.sled");
        let legacy = temp.path().join("missing-legacy.sled");

        let resolved = resolve_product_event_authority(&branch, &target, &legacy).unwrap();
        drop(resolved);
        let binding_path = branch.data_home_path.join(BINDING_FILE);
        let mut binding = read_binding(&binding_path).unwrap().unwrap();
        binding.ledger_identity = LedgerIdentity::new();
        write_binding(&binding_path, &binding).unwrap();

        let error = resolve_product_event_authority(&branch, &target, &legacy)
            .err()
            .expect("substituted identity must fail closed");
        assert!(error.to_string().contains("cutover marker targets ledger"));
    }
}
