//! Shared test utilities for integration tests
//!
//! Provides shared XDG isolation, Event owners and Agent configuration fixtures.

use meld::agent::{AgentRole, AgentStorage, XdgAgentStorage};
use meld::config::AgentConfig;
use meld::events::{
    EventAuthority, EventAuthorityOpenOptions, EventRecord, LedgerCursor, LedgerIdentity,
    ReplayRequest, MAX_REPLAY_LIMIT,
};
use meld::execution::projection::ExecutionProjection;
use meld::runtime::ports::{ProductEventReplayPort, ProductGraphCursorPort};
use meld::session::{SessionRuntime, SessionStore};
use meld::telemetry::ProgressRuntime;
use meld::world_state::graph::runtime::GraphRuntime;
use meld::world_state::graph::store::TraversalStore;
use meld_events::events::test_support::{EventStore, EventStoreTestSupport as _};
use std::fs;
use std::sync::{Arc, Mutex};
use tempfile::TempDir;

/// Global mutex to serialize XDG environment variable access across all tests
/// This prevents race conditions when tests run in parallel
static XDG_ENV_MUTEX: Mutex<()> = Mutex::new(());

/// Authority-backed event and session fixture for integration tests.
///
/// Tests use the same capability-only construction as production while still
/// choosing a supplied database for legacy-row characterization.
pub(crate) struct AuthorityProgressFixture {
    pub(crate) authority: EventAuthority,
    pub(crate) progress: ProgressRuntime,
    pub(crate) store: std::sync::Arc<EventStore>,
}

pub(crate) fn open_authority_progress(db: sled::Db) -> AuthorityProgressFixture {
    let authority = EventAuthority::open(db.clone(), EventAuthorityOpenOptions::default())
        .expect("open test event authority");
    let store = EventStore::shared(db.clone()).expect("open test event store");
    let session_store = SessionStore::shared(db).expect("open test session store");
    let sessions = std::sync::Arc::new(SessionRuntime::new(session_store));
    let progress = ProgressRuntime::from_capabilities(authority.append_capability(), sessions);
    AuthorityProgressFixture {
        authority,
        progress,
        store,
    }
}

impl AuthorityProgressFixture {
    pub fn append_capability(&self) -> meld_events::EventAppendCapability {
        self.authority.append_capability()
    }

    pub fn ledger_identity(&self) -> LedgerIdentity {
        self.authority.ledger_identity()
    }

    pub fn replay_all(&self) -> Vec<EventRecord> {
        let replay = self.authority.replay_capability();
        let mut records = Vec::new();
        let mut cursor = LedgerCursor {
            ledger_id: self.ledger_identity(),
            after_seq: 0,
        };
        loop {
            let page = replay
                .replay(ReplayRequest {
                    cursor,
                    limit: MAX_REPLAY_LIMIT,
                })
                .expect("replay test event page");
            let is_complete = page.records.is_empty();
            records.extend(page.records);
            cursor = page.next_cursor;
            if is_complete || cursor.after_seq >= page.coverage.tip_seq {
                return records;
            }
        }
    }

    pub fn replay_session(&self, session_id: &str) -> Vec<EventRecord> {
        self.replay_all()
            .into_iter()
            .filter(|record| record.session == session_id)
            .collect()
    }

    pub fn replay_execution_projection(&self, after_seq: u64) -> ExecutionProjection {
        ExecutionProjection::replay_from_source(
            &self.authority.replay_capability(),
            LedgerCursor {
                ledger_id: self.ledger_identity(),
                after_seq,
            },
            MAX_REPLAY_LIMIT,
        )
        .expect("replay execution projection")
    }

    pub fn graph_runtime(&self, db: sled::Db) -> std::sync::Arc<GraphRuntime> {
        let replay = Arc::new(ProductEventReplayPort::new(
            self.authority.replay_capability(),
        ));
        let cursor = Arc::new(ProductGraphCursorPort::new(
            self.authority.consumer_registry_capability(),
        ));
        let traversal = TraversalStore::shared(db).expect("open graph traversal test store");
        Arc::new(
            GraphRuntime::from_ports(replay, cursor, traversal)
                .expect("open authority-backed graph runtime"),
        )
    }
}

/// Environment variable state to restore after test
struct EnvState {
    home: Option<String>,
    xdg_config_home: Option<String>,
    xdg_data_home: Option<String>,
}

impl EnvState {
    fn capture() -> Self {
        Self {
            home: std::env::var("HOME").ok(),
            xdg_config_home: std::env::var("XDG_CONFIG_HOME").ok(),
            xdg_data_home: std::env::var("XDG_DATA_HOME").ok(),
        }
    }

    fn restore(self) {
        if let Some(orig) = self.home {
            std::env::set_var("HOME", orig);
        } else {
            std::env::remove_var("HOME");
        }

        if let Some(orig) = self.xdg_config_home {
            std::env::set_var("XDG_CONFIG_HOME", orig);
        } else {
            std::env::remove_var("XDG_CONFIG_HOME");
        }

        if let Some(orig) = self.xdg_data_home {
            std::env::set_var("XDG_DATA_HOME", orig);
        } else {
            std::env::remove_var("XDG_DATA_HOME");
        }
    }
}

/// Set up isolated XDG directories for a test with automatic cleanup
///
/// This function:
/// - Creates isolated XDG_CONFIG_HOME and an external XDG_DATA_HOME
/// - Sets HOME to ensure fallback paths work correctly
/// - Automatically restores original environment variables after the test
/// - Uses a global mutex to prevent race conditions in parallel test execution
///
/// # Example
/// ```
/// use tempfile::TempDir;
/// use crate::test_utils::with_xdg_env;
///
/// let test_dir = TempDir::new().unwrap();
/// with_xdg_env(&test_dir, || {
///     // Your test code here
///     // XDG_CONFIG_HOME and XDG_DATA_HOME are isolated for the test
/// });
/// // Environment automatically restored
/// ```
pub fn with_xdg_env<F, R>(test_dir: &TempDir, f: F) -> R
where
    F: FnOnce() -> R,
{
    with_env_lock(|| {
        let env_state = EnvState::capture();

        // Set up test directories
        let test_config_home = test_dir.path().to_path_buf();
        // Product storage must remain outside the target workspace. A
        // separate temporary root keeps this helper valid even when callers
        // use `test_dir.path()` itself as the workspace.
        let external_data_home = tempfile::tempdir().unwrap();
        let test_data_home = external_data_home.path().to_path_buf();
        let test_home = test_dir.path().join("home");

        std::fs::create_dir_all(&test_data_home).unwrap();
        std::fs::create_dir_all(&test_home).unwrap();

        // Set environment variables
        std::env::set_var("HOME", test_home.to_str().unwrap());
        std::env::set_var("XDG_CONFIG_HOME", test_config_home.to_str().unwrap());
        std::env::set_var("XDG_DATA_HOME", test_data_home.to_str().unwrap());

        // Run test
        let result = f();

        // Restore original environment
        env_state.restore();

        result
    })
}

/// Set up only XDG_DATA_HOME (for workspace isolation tests)
///
/// This is a lighter-weight version for tests that only need data directory isolation
/// but don't need config directory isolation.
pub fn with_xdg_data_home<F, R>(test_dir: &TempDir, f: F) -> R
where
    F: FnOnce() -> R,
{
    with_env_lock(|| {
        let env_state = EnvState::capture();

        let external_data_home = tempfile::tempdir().unwrap();
        let test_data_home = external_data_home.path().to_path_buf();
        let test_home = test_dir.path().join("home");

        std::fs::create_dir_all(&test_data_home).unwrap();
        std::fs::create_dir_all(&test_home).unwrap();

        std::env::set_var("HOME", test_home.to_str().unwrap());
        std::env::set_var("XDG_DATA_HOME", test_data_home.to_str().unwrap());

        let result = f();

        env_state.restore();

        result
    })
}

/// Serialize direct environment variable access for integration tests.
pub fn with_env_lock<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    let _guard = XDG_ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
    f()
}

/// Writes a docs-writer style agent config into the isolated XDG agents dir.
pub fn create_test_agent(agent_id: &str, workflow_id: Option<&str>) {
    let agents_dir = XdgAgentStorage::new().agents_dir().unwrap();
    fs::create_dir_all(&agents_dir).unwrap();
    let config_path = agents_dir.join(format!("{agent_id}.toml"));

    let mut metadata = std::collections::HashMap::new();
    metadata.insert(
        "user_prompt_file".to_string(),
        "Summarize file context".to_string(),
    );
    metadata.insert(
        "user_prompt_directory".to_string(),
        "Summarize directory context".to_string(),
    );

    let agent_config = AgentConfig {
        agent_id: agent_id.to_string(),
        role: AgentRole::Writer,
        system_prompt: Some("You are a careful docs writer.".to_string()),
        system_prompt_path: None,
        workflow_id: workflow_id.map(ToString::to_string),
        metadata: metadata.into(),
    };

    fs::write(&config_path, toml::to_string(&agent_config).unwrap()).unwrap();
}
