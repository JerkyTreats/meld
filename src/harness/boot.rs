//! Boot one harness run through the product staged pipeline into an
//! isolated root, and drive it with injected time.
//!
//! Owner: harness. A run boots exactly the way the product boots — the
//! product event binding, the composed assembly scoped to the registration
//! subset, and the world-initialization command pipeline — because a
//! harness that initializes differently from the product debugs a system
//! that does not exist. The default root is temporary and owned by the
//! run; pointing a run at an existing product data root requires the
//! explicit unsafe flag (DBG-011).
//!
//! Discipline: stimuli enter only through the canonical append capability
//! (DBG-003), steps take injected time against the supervisor's bounded
//! contract (DBG-002), and the harness never pokes stores.

use std::fs;
use std::path::{Path, PathBuf};

use meld_events::error::EventAuthorityError;
use meld_events::{
    AppendDisposition, AppendMode, AppendReceipt, EventAppendCapability, EventEnvelope,
};
use meld_world_model::agent::AgentStore;
use meld_world_model::belief::BeliefFamilyRegistryStore;
use thiserror::Error;

use crate::branches::{BranchKind, ResolvedBranch};
use crate::events::binding::{resolve_product_event_authority, ProductEventBindingError};
use crate::harness::manifest::{
    HarnessBootRecord, HarnessClosingRecord, HarnessDesiredRuntimeRecord, HarnessManifest,
    HarnessManifestError, HarnessRootRecord, HarnessStageRecord, HarnessStepRecord,
    HarnessStimulusRecord, HARNESS_MANIFEST_SCHEMA_VERSION,
};
use crate::init::world::pipeline::{WorldInitContent, WorldInitError, WorldInitPipeline};
use crate::init::world::{
    StageDisposition, WorldInitRequest, WorldInitStage, WorldInitStageReport,
};
use crate::runtime::assembly::{
    ProductRuntimeAssembly, ProductRuntimeConfig, StewardshipComposition,
};
use crate::runtime::contracts::WorkBudget;
use crate::runtime::error::RuntimeAssemblyError;
use crate::runtime::registration::RegistrationSet;
use crate::runtime::storage::ProductStorageLayout;
use crate::runtime::supervisor::{
    RuntimeSupervisor, SupervisorRuntimeError, SupervisorStartCommand, SupervisorTickReport,
};

/// Default branch id bound into a temporary run's product event binding.
const DEFAULT_BRANCH_ID: &str = "harness";

/// Manifest file name inside a temporary run directory.
const MANIFEST_FILE: &str = "harness_manifest.json";

/// Failure booting, driving, or sealing a harness run.
#[derive(Debug, Error)]
pub enum HarnessError {
    /// An existing product data root was addressed without the explicit
    /// unsafe flag.
    #[error(
        "existing product data root '{0}' requires the explicit unsafe_existing_root flag; \
         the default harness boot uses a temporary root"
    )]
    ExistingRootGuard(String),
    /// Filesystem failure preparing the run root.
    #[error("harness io failure: {0}")]
    Io(#[from] std::io::Error),
    /// Product event binding resolution failed.
    #[error("product event binding failed: {0}")]
    Binding(#[from] ProductEventBindingError),
    /// Product runtime assembly failed.
    #[error("runtime assembly failed: {0}")]
    Assembly(#[from] RuntimeAssemblyError),
    /// World initialization was requested but the composed registration
    /// subset did not open the stores its stages write through.
    #[error("world init requested outside the composed store scope: {0}")]
    WorldInitScope(String),
    /// A world-initialization stage failed.
    #[error("world init failed: {0}")]
    WorldInit(#[from] WorldInitError),
    /// A domain store surface failed while binding the pipeline.
    #[error("harness storage failure: {0}")]
    Storage(String),
    /// The supervisor rejected a lifecycle command.
    #[error("supervisor failed: {0}")]
    Supervisor(#[from] SupervisorRuntimeError),
    /// The event authority rejected an append or watermark read.
    #[error("event authority failed: {0}")]
    Events(#[from] EventAuthorityError),
    /// The manifest artifact could not be persisted or read.
    #[error("manifest persistence failed: {0}")]
    Manifest(#[from] HarnessManifestError),
}

/// Root selection for one harness run.
#[derive(Debug, Clone)]
pub enum HarnessRootSelection {
    /// Fresh temporary root owned by the run; removed when the run drops
    /// unless [`HarnessRun::keep_root`] persists it.
    Temporary,
    /// Existing product data root. Requires
    /// [`HarnessBootRequest::unsafe_existing_root`].
    ExistingDataRoot {
        /// Product storage root to open.
        product_root: PathBuf,
        /// Branch data home holding the product event binding.
        branch_home: PathBuf,
        /// Compatibility database path for binding resolution.
        legacy_store_path: PathBuf,
        /// Where the manifest is written at seal.
        manifest_path: PathBuf,
    },
}

/// World-initialization stages and injected stage-0 content for one boot.
#[derive(Debug, Clone)]
pub struct HarnessWorldInit {
    /// Stage subset to run, normalized to pipeline order.
    pub request: WorldInitRequest,
    /// Injected semantic content; the pipeline resolves nothing from the
    /// environment, which is what lets isolate boots stay hermetic.
    pub content: WorldInitContent,
}

/// One harness boot command.
///
/// Not `Clone`/`Debug`: the stewardship composition carries live theory
/// bindings that expose neither.
pub struct HarnessBootRequest {
    /// Stable caller-chosen run identity, recorded on the manifest.
    pub manifest_id: String,
    /// Branch id for the product event binding; stable across reopen.
    pub branch_id: String,
    /// Where the run's durable state lives.
    pub root: HarnessRootSelection,
    /// Explicit acknowledgement required to open an existing data root.
    pub unsafe_existing_root: bool,
    /// Supervisor boot time in milliseconds; injected, never wall clock.
    pub booted_at_ms: u64,
    /// Registration subset composed into the assembly; `None` composes the
    /// full legacy classification, which no isolate should want.
    pub registration_set: Option<RegistrationSet>,
    /// Runtime ids force-enabled over the product defaults.
    pub enabled_runtime_ids: Vec<String>,
    /// Runtime ids disabled; `None` keeps the product defaults.
    pub disabled_runtime_ids: Option<Vec<String>>,
    /// Bounded work budget per actor invocation; `None` keeps the product
    /// default.
    pub default_work_budget: Option<WorkBudget>,
    /// Stewardship composition when the run debugs a stewarded workspace.
    pub stewardship: Option<StewardshipComposition>,
    /// World-initialization stages to run at boot.
    pub world_init: Option<HarnessWorldInit>,
}

impl HarnessBootRequest {
    /// Default boot: a temporary root, injected boot time, no composition.
    pub fn temporary(manifest_id: impl Into<String>, booted_at_ms: u64) -> Self {
        Self {
            manifest_id: manifest_id.into(),
            branch_id: DEFAULT_BRANCH_ID.to_string(),
            root: HarnessRootSelection::Temporary,
            unsafe_existing_root: false,
            booted_at_ms,
            registration_set: None,
            enabled_runtime_ids: Vec::new(),
            disabled_runtime_ids: None,
            default_work_budget: None,
            stewardship: None,
            world_init: None,
        }
    }
}

/// One booted harness run: the composed assembly plus its manifest.
pub struct HarnessRun {
    assembly: ProductRuntimeAssembly,
    manifest: HarnessManifest,
    manifest_path: PathBuf,
    temp_root: Option<tempfile::TempDir>,
}

/// Records accumulated by a driver, merged into the manifest at seal.
#[derive(Debug, Clone, PartialEq)]
pub struct HarnessDriveOutcome {
    /// Step schedule executed by the driver.
    pub steps: Vec<HarnessStepRecord>,
    /// Stimuli appended by the driver.
    pub stimuli: Vec<HarnessStimulusRecord>,
    /// Injected shutdown time.
    pub shutdown_at_ms: u64,
    /// Shutdown id written by the supervisor.
    pub shutdown_id: String,
}

impl HarnessRun {
    /// Boot a run through the product staged pipeline.
    ///
    /// Sequencing mirrors the product: binding resolution, scoped assembly,
    /// then the world-initialization command pipeline. The manifest is
    /// built here and persisted at [`HarnessRun::seal`].
    pub fn boot(request: HarnessBootRequest) -> Result<Self, HarnessError> {
        let (paths, temp_root, existing) = resolve_root(&request)?;
        fs::create_dir_all(&paths.product_root)?;

        let branch = ResolvedBranch {
            branch_id: request.branch_id.clone(),
            branch_kind: BranchKind::WorkspaceFs,
            canonical_locator: paths.branch_home.join("workspace"),
            data_home_path: paths.branch_home.clone(),
            manifest_path: paths.branch_home.join("branch_manifest.json"),
            ledger_path: paths.branch_home.join("branch_migration_ledger.jsonl"),
        };
        let layout = ProductStorageLayout::from_root(&paths.product_root);
        let resolved =
            resolve_product_event_authority(&branch, &layout.ledger_db, &paths.legacy_store_path)?;

        let mut config = ProductRuntimeConfig::for_product_root(paths.product_root.clone());
        config.registration_set = request.registration_set;
        config.enabled_runtime_ids = request.enabled_runtime_ids;
        if let Some(disabled) = request.disabled_runtime_ids {
            config.disabled_runtime_ids = disabled;
        }
        if let Some(budget) = request.default_work_budget {
            config.default_work_budget = budget;
        }
        let assembly =
            ProductRuntimeAssembly::load_composed(config, resolved.authority, request.stewardship)?;

        let world_init = match request.world_init {
            Some(init) => run_scoped_world_init(&assembly, &init)?,
            None => Vec::new(),
        };

        let manifest = HarnessManifest {
            schema_version: HARNESS_MANIFEST_SCHEMA_VERSION,
            manifest_id: request.manifest_id,
            branch_id: request.branch_id,
            booted_at_ms: request.booted_at_ms,
            root: HarnessRootRecord {
                product_root: paths.product_root.clone(),
                supervisor_store_path: assembly.supervisor_store().path().to_path_buf(),
                temporary: temp_root.is_some(),
                existing_data_root: existing,
            },
            ledger_identity: assembly.event_authority().ledger_identity().to_string(),
            boot: HarnessBootRecord {
                registration_ids: assembly
                    .registration_set()
                    .map(|set| {
                        set.registrations
                            .iter()
                            .map(|registration| registration.registration_id.clone())
                            .collect()
                    })
                    .unwrap_or_default(),
                desired_runtimes: assembly
                    .desired_runtime_state()
                    .iter()
                    .map(|state| HarnessDesiredRuntimeRecord {
                        runtime_id: state.runtime_id.clone(),
                        enabled: state.enabled,
                        factory_available: state.factory_available,
                    })
                    .collect(),
                world_init: world_init.iter().map(stage_record).collect(),
                default_budget_max_items: assembly.default_work_budget().max_items,
            },
            stimuli: Vec::new(),
            steps: Vec::new(),
            closing: None,
        };

        Ok(Self {
            assembly,
            manifest,
            manifest_path: paths.manifest_path,
            temp_root,
        })
    }

    /// The composed product assembly, for typed inspection through the
    /// domain query facades (DBG-004).
    pub fn assembly(&self) -> &ProductRuntimeAssembly {
        &self.assembly
    }

    /// The manifest as recorded so far; closing fields land at seal.
    pub fn manifest(&self) -> &HarnessManifest {
        &self.manifest
    }

    /// Where the manifest is written at seal.
    pub fn manifest_path(&self) -> &Path {
        &self.manifest_path
    }

    /// Start the supervisor and hand back a step driver.
    ///
    /// The driver borrows the run exclusively so one run drives one
    /// supervisor at a time; finish it and pass its outcome to
    /// [`HarnessRun::seal`] before reading the manifest artifact.
    pub fn driver(&mut self) -> Result<HarnessDriver<'_>, HarnessError> {
        let mut command = SupervisorStartCommand::new(
            format!("harness::{}", self.manifest.manifest_id),
            self.manifest.booted_at_ms,
        );
        // The same registration intersection the product foreground run
        // composes, through the registration domain contract.
        command.registration_set = self.assembly.registration_set().and_then(|set| {
            set.intersect_desired_runtime_ids(
                self.assembly
                    .desired_runtime_state()
                    .iter()
                    .map(|state| state.runtime_id.as_str()),
            )
        });
        let supervisor =
            RuntimeSupervisor::start(self.assembly.supervisor_startup_package(), command)?;
        Ok(HarnessDriver {
            supervisor,
            append: self.assembly.event_authority().append_capability(),
            budget_max_items: self.assembly.default_work_budget().max_items,
            steps: Vec::new(),
            stimuli: Vec::new(),
        })
    }

    /// Merge a drive outcome, record closing watermarks, and persist the
    /// manifest artifact.
    pub fn seal(&mut self, outcome: HarnessDriveOutcome) -> Result<(), HarnessError> {
        let watermark = self
            .assembly
            .event_authority()
            .watermark_capability()
            .snapshot()?;
        self.manifest.stimuli = outcome.stimuli;
        self.manifest.steps = outcome.steps;
        self.manifest.closing = Some(HarnessClosingRecord {
            shutdown_at_ms: outcome.shutdown_at_ms,
            shutdown_id: outcome.shutdown_id,
            committed_seq: watermark.committed_seq,
            tip_seq: watermark.tip_seq,
        });
        self.manifest.save(&self.manifest_path)?;
        Ok(())
    }

    /// Persist a temporary root past the run's lifetime.
    ///
    /// Returns the retained run directory, or `None` when the run opened
    /// an existing data root and owns nothing temporary.
    pub fn keep_root(&mut self) -> Option<PathBuf> {
        self.temp_root.take().map(tempfile::TempDir::keep)
    }
}

/// Deterministic step driver over one run's supervisor.
pub struct HarnessDriver<'a> {
    supervisor: RuntimeSupervisor<'a>,
    append: EventAppendCapability,
    budget_max_items: usize,
    steps: Vec<HarnessStepRecord>,
    stimuli: Vec<HarnessStimulusRecord>,
}

impl HarnessDriver<'_> {
    /// Append one stimulus through the canonical append capability.
    pub fn append_stimulus(
        &mut self,
        envelope: EventEnvelope,
        mode: AppendMode,
    ) -> Result<AppendReceipt, HarnessError> {
        let event_type = envelope.event_type.clone();
        let record_id = envelope.record_id.clone();
        let receipt = self.append.append_durable(envelope, mode)?;
        self.stimuli.push(HarnessStimulusRecord {
            ledger_id: receipt.ledger_id.to_string(),
            seq: receipt.seq,
            record_id,
            event_type,
            disposition: match receipt.disposition {
                AppendDisposition::Inserted => "inserted".to_string(),
                AppendDisposition::Duplicate => "duplicate".to_string(),
            },
            after_step: self.steps.len() as u64,
        });
        Ok(receipt)
    }

    /// Run one supervisor maintenance pass at an injected time.
    pub fn step(&mut self, now_ms: u64) -> Result<SupervisorTickReport, HarnessError> {
        let report = self.supervisor.tick(now_ms)?;
        self.steps.push(HarnessStepRecord {
            ordinal: self.steps.len() as u64 + 1,
            now_ms,
            budget_max_items: self.budget_max_items,
            action_ids: report
                .actions
                .iter()
                .map(|action| action.action_id.clone())
                .collect(),
        });
        Ok(report)
    }

    /// Shut the supervisor down and return the accumulated records.
    pub fn finish(mut self, shutdown_at_ms: u64) -> Result<HarnessDriveOutcome, HarnessError> {
        let shutdown = self.supervisor.request_shutdown(shutdown_at_ms)?;
        Ok(HarnessDriveOutcome {
            steps: self.steps,
            stimuli: self.stimuli,
            shutdown_at_ms,
            shutdown_id: shutdown.shutdown_id,
        })
    }
}

/// Resolved filesystem locations for one run.
struct RunPaths {
    product_root: PathBuf,
    branch_home: PathBuf,
    legacy_store_path: PathBuf,
    manifest_path: PathBuf,
}

/// Resolve the run root, enforcing the existing-root guard (DBG-011).
fn resolve_root(
    request: &HarnessBootRequest,
) -> Result<(RunPaths, Option<tempfile::TempDir>, bool), HarnessError> {
    match &request.root {
        HarnessRootSelection::Temporary => {
            let dir = tempfile::tempdir()?;
            let base = dir.path().to_path_buf();
            Ok((
                RunPaths {
                    product_root: base.join("root"),
                    branch_home: base.join("branch-home"),
                    legacy_store_path: base.join("legacy-compat"),
                    manifest_path: base.join(MANIFEST_FILE),
                },
                Some(dir),
                false,
            ))
        }
        HarnessRootSelection::ExistingDataRoot {
            product_root,
            branch_home,
            legacy_store_path,
            manifest_path,
        } => {
            if !request.unsafe_existing_root {
                return Err(HarnessError::ExistingRootGuard(
                    product_root.display().to_string(),
                ));
            }
            Ok((
                RunPaths {
                    product_root: product_root.clone(),
                    branch_home: branch_home.clone(),
                    legacy_store_path: legacy_store_path.clone(),
                    manifest_path: manifest_path.clone(),
                },
                None,
                true,
            ))
        }
    }
}

/// Run the world-initialization pipeline over the composed assembly.
///
/// The pipeline binds the same domain command surfaces the product init
/// command binds; a registration subset that did not open those stores is
/// an explicit scope error rather than a silent skip, so a manifest never
/// claims an initialization that could not have happened.
fn run_scoped_world_init(
    assembly: &ProductRuntimeAssembly,
    init: &HarnessWorldInit,
) -> Result<Vec<WorldInitStageReport>, HarnessError> {
    let stores = assembly.stores();
    let traversal = stores.traversal_store.opened().ok_or_else(|| {
        HarnessError::WorldInitScope(
            "world-model stores are not open under the composed registration subset".to_string(),
        )
    })?;
    let agent_store: &AgentStore = stores.agent_store.opened().ok_or_else(|| {
        HarnessError::WorldInitScope(
            "agent store is not open under the composed registration subset".to_string(),
        )
    })?;
    let mut registry = BeliefFamilyRegistryStore::new(traversal.db().clone())
        .map_err(|error| HarnessError::Storage(error.to_string()))?;
    let authority = assembly.event_authority();
    let append = authority.append_capability();
    let report = WorldInitPipeline::new(&mut registry, agent_store, &append)
        .run(&init.request, &init.content)?;
    Ok(report.stage_reports)
}

/// Map one stage report onto the manifest's stable vocabulary.
fn stage_record(report: &WorldInitStageReport) -> HarnessStageRecord {
    HarnessStageRecord {
        stage: match report.stage {
            WorldInitStage::InstallTheory => "install-theory",
            WorldInitStage::GenesisIdentities => "genesis-identities",
            WorldInitStage::SeedEpistemicFacts => "seed-epistemic-facts",
        }
        .to_string(),
        disposition: match report.disposition {
            StageDisposition::Applied => "applied",
            StageDisposition::Unchanged => "unchanged",
        }
        .to_string(),
        record_ids: report.record_ids.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::registration::{RegistrationKind, RuntimeRegistration};

    fn belief_assessment_subset() -> RegistrationSet {
        RegistrationSet {
            registrations: vec![RuntimeRegistration {
                registration_id: "world_model.belief_assessment".to_string(),
                runtime_id: "world_model.belief_assessment".to_string(),
                kind: RegistrationKind::ActiveActor,
                required_resources: vec![crate::runtime::assembly::RuntimeResource::WorldModel],
            }],
        }
    }

    #[test]
    fn existing_data_root_without_the_unsafe_flag_fails() {
        let dir = tempfile::tempdir().unwrap();
        let mut request = HarnessBootRequest::temporary("guarded-run", 10);
        request.root = HarnessRootSelection::ExistingDataRoot {
            product_root: dir.path().join("root"),
            branch_home: dir.path().join("branch-home"),
            legacy_store_path: dir.path().join("legacy-compat"),
            manifest_path: dir.path().join("harness_manifest.json"),
        };
        match HarnessRun::boot(request) {
            Err(HarnessError::ExistingRootGuard(_)) => {}
            Err(other) => panic!("expected the existing-root guard, got: {other}"),
            Ok(_) => panic!("boot must fail without the unsafe flag"),
        }
    }

    #[test]
    fn temporary_boot_steps_and_seals_a_manifest() {
        let mut request = HarnessBootRequest::temporary("temp-run", 100);
        request.registration_set = Some(belief_assessment_subset());
        let mut run = HarnessRun::boot(request).unwrap();

        assert!(run.manifest().root.temporary);
        assert!(!run.manifest().root.existing_data_root);
        assert!(!run.manifest().ledger_identity.is_empty());
        assert_eq!(
            run.manifest().boot.registration_ids,
            vec!["world_model.belief_assessment".to_string()]
        );

        let mut driver = run.driver().unwrap();
        driver.step(110).unwrap();
        driver.step(120).unwrap();
        let outcome = driver.finish(130).unwrap();
        run.seal(outcome).unwrap();

        let manifest = HarnessManifest::load(run.manifest_path()).unwrap();
        assert_eq!(manifest.steps.len(), 2);
        assert_eq!(manifest.steps[0].now_ms, 110);
        assert_eq!(manifest.steps[1].ordinal, 2);
        let closing = manifest
            .closing
            .expect("sealed manifest has closing record");
        assert_eq!(closing.shutdown_at_ms, 130);
        assert!(!closing.shutdown_id.is_empty());
    }

    #[test]
    fn temporary_root_lives_under_the_run_and_can_be_kept() {
        let mut request = HarnessBootRequest::temporary("kept-run", 100);
        request.registration_set = Some(belief_assessment_subset());
        let mut run = HarnessRun::boot(request).unwrap();
        let product_root = run.manifest().root.product_root.clone();
        assert!(product_root.exists());

        let kept = run.keep_root().expect("temporary run owns a root");
        drop(run);
        assert!(kept.exists());
        assert!(product_root.starts_with(&kept));
        std::fs::remove_dir_all(kept).unwrap();
    }
}
