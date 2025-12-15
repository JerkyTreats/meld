# Phase 3 Component Specifications

## Major Components

### 1. Public API Contracts & Versioning

#### Description
Freeze stable API surfaces and version them to prevent breaking consumers. Establish clear contracts for request/response formats, error handling, and backward compatibility guarantees.

#### Requirements
- **API versioning strategy**: URL-based versioning (e.g., `/v1/`, `/v2/`) with semantic versioning
- **Backward-compatible evolution rules**: Additive changes only; no breaking changes within major version
- **Schema versioning**: Separate versioning for NodeRecord, ContextFrame, and API payloads
- **Deterministic serialization**: Canonical JSON/CBOR/MessagePack serialization specified in contract
- **API documentation**: OpenAPI/Swagger specs with examples
- **Deprecation policy**: Clear deprecation timeline and migration guides

#### Versioning Strategy
```rust
// API version in URL path
GET /v1/nodes/{node_id}
POST /v1/frames
GET /v1/synthesize/{node_id}

// Schema version in headers or payload
{
  "api_version": "1.0",
  "schema_version": "1.2",
  "data": { ... }
}
```

#### Compatibility Rules
- **Major version (v1 → v2)**: Breaking changes allowed; old clients may not work
- **Minor version (v1.0 → v1.1)**: Additive changes only; old clients must work
- **Patch version (v1.0.0 → v1.0.1)**: Bug fixes only; no behavior changes
- **Support window**: Minimum 6 months support for previous major version

#### Test Criteria
- Old clients work against new server versions (within support window)
- Schema validation rejects invalid payloads deterministically
- Canonical serialization produces identical hashes across languages
- Version negotiation works correctly (client requests v1, server supports v1-v2)
- Deprecation warnings are clear and actionable
- Migration guides enable smooth transitions

---

### 2. Workspace Isolation & Access Control

#### Description
Support multiple workspaces safely and enforce access boundaries. Ensure complete isolation between workspaces while providing fine-grained access control within each workspace.

#### Requirements
- **Workspace-scoped namespaces**: All storage keys prefixed with workspace ID
- **Storage-level isolation**: Database/blob store enforces workspace boundaries
- **API-level isolation**: All API requests scoped to workspace (via header or path)
- **Authentication mechanisms**: Support token-based, mTLS, or identity binding (e.g., Tailscale)
- **Authorization model**: Per-workspace ACLs for read/write/synthesis operations
- **Audit logging**: All write operations logged with agent identity, timestamp, and operation details
- **Workspace metadata**: Workspace name, description, creation time, owner

#### Workspace Model
```rust
struct Workspace {
    workspace_id: WorkspaceID,  // Deterministic hash
    name: String,
    root_hash: Hash,  // Current workspace root
    created_at: Timestamp,
    owner: AgentID,
    acls: Vec<ACLEntry>,
}

struct ACLEntry {
    agent_id: AgentID,
    permissions: Vec<Permission>,  // Read, Write, Synthesize, Admin
    granted_at: Timestamp,
    granted_by: AgentID,
}
```

#### Access Control Model
- **Workspace-level**: Access to entire workspace (read/write/synthesis)
- **Node-level**: Future extension for per-node permissions
- **Frame-type-level**: Future extension for per-frame-type permissions
- **Default permissions**: Workspace owner has all permissions; others require explicit grant

#### Audit Log Format
```rust
struct AuditLogEntry {
    timestamp: Timestamp,
    workspace_id: WorkspaceID,
    agent_id: AgentID,
    operation: Operation,  // PutFrame, Synthesize, Regenerate, etc.
    node_id: Option<NodeID>,
    frame_id: Option<FrameID>,
    success: bool,
    error: Option<String>,
}
```

#### Test Criteria
- No cross-workspace reads/writes possible (storage isolation verified)
- Unauthorized writes rejected with clear error messages
- ACL changes do not mutate historical FrameIDs (ACLs not in basis)
- Audit log entries emitted for all PutFrame / Synthesize operations
- Workspace creation is idempotent (same inputs → same workspace ID)
- Workspace deletion requires admin permission and confirmation
- Concurrent access from multiple agents handled safely

---

### 3. Snapshot Export, Verification, and Replay

#### Description
Enable external systems to export a workspace snapshot and verify it independently. Support deterministic replay/rebuild from exports to enable backup, migration, and verification workflows.

#### Requirements
- **Export format**: Self-contained archive (tar, zip, or custom format) with manifest
- **Export scope**: Export by root hash (complete workspace state at that point)
- **Export contents**: NodeRecords, frame sets, frames, blobs, and metadata
- **Integrity verification**: Cryptographic signatures and checksums for all components
- **Deterministic replay**: Import/replay produces identical root hash and NodeIDs/FrameIDs
- **Partial export support**: Optional support for exporting subtrees or specific nodes
- **Compression**: Optional compression for large exports

#### Export Format Structure
```
workspace-export-{root_hash}.tar.gz
├── manifest.json          # Export metadata, root hash, timestamp
├── checksums.sha256       # Checksums for all files
├── nodes/
│   ├── {node_id_1}.json   # NodeRecord
│   └── {node_id_2}.json
├── frames/
│   ├── {frame_id_1}.json  # Frame metadata
│   ├── {frame_id_1}.blob  # Frame content
│   └── {frame_id_2}.json
├── blobs/
│   └── {blob_hash}.blob   # Large blob content
└── metadata.json          # Workspace metadata, ACLs, etc.
```

#### Verification Tool
```rust
fn verify_export(export_path: &Path) -> Result<VerificationReport, Error>;

struct VerificationReport {
    root_hash: Hash,
    node_count: usize,
    frame_count: usize,
    blob_count: usize,
    checksums_valid: bool,
    signatures_valid: bool,
    integrity_ok: bool,
}
```

#### Replay/Import Tool
```rust
fn import_export(
    export_path: &Path,
    target_workspace_id: Option<WorkspaceID>,
) -> Result<ImportReport, Error>;

struct ImportReport {
    workspace_id: WorkspaceID,
    root_hash: Hash,
    imported_nodes: usize,
    imported_frames: usize,
    verification_passed: bool,
}
```

#### Test Criteria
- Export + import yields identical root hash (deterministic replay)
- Verification catches any tampering (modified files detected)
- Replay rebuilds same NodeIDs/FrameIDs (deterministic reconstruction)
- Partial exports fail safely (or are explicitly supported with clear errors)
- Export format is self-describing (manifest contains all necessary metadata)
- Large exports (>10GB) handled efficiently (streaming, compression)
- Export can be verified without importing (standalone verification)

---

### 4. Observability & Diagnostics

#### Description
Make the system operable: trace changes, measure performance, debug determinism issues, and monitor health. Provide comprehensive observability without compromising determinism or performance.

#### Requirements
- **Metrics collection**: Request latency, storage IO, synthesis/regeneration counts, cache hit rates
- **Structured logging**: JSON logs with correlation IDs, log levels, and contextual metadata
- **Determinism diagnostics**: Hash diff tooling, basis mismatch reports, non-determinism detection
- **Health endpoints**: Ready/live endpoints that reflect true system state
- **Tracing**: Distributed tracing support for request flows (optional)
- **Performance profiling**: Optional profiling hooks for performance analysis

#### Metrics
```rust
struct Metrics {
    // API metrics
    api_request_count: Counter,
    api_request_latency: Histogram,
    api_error_count: Counter,
    
    // Storage metrics
    storage_read_count: Counter,
    storage_write_count: Counter,
    storage_latency: Histogram,
    
    // Synthesis metrics
    synthesis_count: Counter,
    synthesis_latency: Histogram,
    regeneration_count: Counter,
    
    // Cache metrics
    cache_hit_rate: Gauge,
    cache_size: Gauge,
    
    // Workspace metrics
    workspace_count: Gauge,
    node_count: Gauge,
    frame_count: Gauge,
}
```

#### Structured Logging
```rust
struct LogEntry {
    timestamp: Timestamp,
    level: LogLevel,  // Error, Warn, Info, Debug, Trace
    correlation_id: String,  // Request correlation ID
    workspace_id: Option<WorkspaceID>,
    agent_id: Option<AgentID>,
    operation: String,
    message: String,
    fields: HashMap<String, Value>,  // Additional context
}
```

#### Determinism Diagnostics
```rust
fn diagnose_hash_mismatch(
    expected_hash: Hash,
    actual_hash: Hash,
    context: &DiagnosticContext,
) -> DiagnosticReport;

struct DiagnosticReport {
    mismatch_type: MismatchType,  // NodeID, FrameID, SetRoot, etc.
    affected_nodes: Vec<NodeID>,
    basis_diffs: Vec<BasisDiff>,
    recommendations: Vec<String>,
}

struct BasisDiff {
    node_id: NodeID,
    expected_basis: Hash,
    actual_basis: Hash,
    diff_explanation: String,
}
```

#### Health Endpoints
```rust
// Readiness: System ready to accept requests
GET /health/ready
{
  "status": "ready",
  "workspace_count": 10,
  "storage_ok": true,
  "dependencies_ok": true
}

// Liveness: System is alive (minimal check)
GET /health/live
{
  "status": "alive"
}
```

#### Test Criteria
- Metrics emitted under load (all metrics populated during stress tests)
- Logs sufficient to reconstruct causal chain of a regeneration (full audit trail)
- Hash diff tooling identifies minimal changed subtree (precise diagnostics)
- Health endpoints reflect true dependency state (fail when dependencies down)
- Correlation IDs enable request tracing across components
- Non-determinism detection catches hash mismatches early
- Performance metrics do not impact request latency (<1% overhead)

---

### 5. Performance Hardening

#### Description
Improve throughput and tail latency while maintaining correctness. Add batching, caching, and concurrency controls to handle production workloads efficiently.

#### Requirements
- **Batch fetch APIs**: Retrieve multiple nodes/frames in single request
- **Internal batching**: Batch storage operations for efficiency
- **Caching layer**: LRU cache for NodeRecords and hot frames
- **Backpressure controls**: Rate limiting and queue management for synthesis workloads
- **Concurrency-safe storage**: Atomic operations, proper locking, no race conditions
- **Connection pooling**: Efficient database/backend connection management

#### Batch APIs
```rust
// Batch node retrieval
async fn get_nodes_batch(
    node_ids: Vec<NodeID>,
    view: ContextView,
) -> Result<HashMap<NodeID, NodeContext>, ApiError>;

// Batch frame retrieval
async fn get_frames_batch(
    frame_ids: Vec<FrameID>,
) -> Result<HashMap<FrameID, Frame>, ApiError>;
```

#### Caching Strategy
```rust
struct CacheConfig {
    node_record_cache_size: usize,  // LRU cache size
    frame_cache_size: usize,
    cache_ttl: Duration,
    cache_invalidation: InvalidationPolicy,
}

enum InvalidationPolicy {
    TimeBased(Duration),
    EventBased,  // Invalidate on write
    Manual,      // Explicit invalidation
}
```

#### Backpressure Controls
```rust
struct BackpressureConfig {
    max_concurrent_synthesis: usize,
    synthesis_queue_size: usize,
    rate_limit_per_agent: RateLimit,
}

struct RateLimit {
    requests_per_second: f64,
    burst_size: usize,
}
```

#### Concurrency Safety
- **Atomic head updates**: Head pointers updated atomically (compare-and-swap)
- **Frame set updates**: Merkle set updates are atomic (transactional)
- **Workspace isolation**: No cross-workspace interference (proper locking)
- **Read-write separation**: Readers don't block writers (optimistic concurrency)

#### Test Criteria
- Concurrent writes do not corrupt heads or frame sets (stress test with 100+ concurrent writers)
- Tail latency remains bounded under load (p99 < 100ms for GetNode)
- Cache does not alter correctness or hash outputs (cache misses handled correctly)
- Large workspaces remain responsive (1M+ nodes, 10M+ frames)
- Batch APIs reduce request count (10x improvement for bulk operations)
- Backpressure prevents resource exhaustion (synthesis queue bounded)
- Cache hit rate >80% for hot data (efficient caching)

---

### 6. Pluggable Backends & Portability

#### Description
Allow swapping storage and compression backends without changing semantics. Ensure cross-platform determinism and enable deployment flexibility.

#### Requirements
- **Storage interface**: Abstract interface for NodeRecord and frame storage
- **Blob store interface**: Abstract interface for large blob storage
- **Compression interface**: Abstract interface for compression/decompression
- **Backend adapters**: Implementations for SQLite, RocksDB, Badger, S3, etc.
- **Cross-platform determinism**: Same data → same hashes across OS/arch
- **Backend migration tools**: Tools to migrate between backends

#### Storage Interface
```rust
trait StorageBackend: Send + Sync {
    fn get_node_record(&self, node_id: &NodeID) -> Result<Option<NodeRecord>, StorageError>;
    fn put_node_record(&self, record: &NodeRecord) -> Result<(), StorageError>;
    fn get_frame(&self, frame_id: &FrameID) -> Result<Option<Frame>, StorageError>;
    fn put_frame(&self, frame: &Frame) -> Result<(), StorageError>;
    fn get_head(&self, node_id: &NodeID, frame_type: &str) -> Result<Option<FrameID>, StorageError>;
    fn update_head(&self, node_id: &NodeID, frame_type: &str, frame_id: &FrameID) -> Result<(), StorageError>;
    // ... more methods
}
```

#### Blob Store Interface
```rust
trait BlobStore: Send + Sync {
    fn get_blob(&self, hash: &Hash) -> Result<Option<Vec<u8>>, BlobStoreError>;
    fn put_blob(&self, hash: &Hash, data: &[u8]) -> Result<(), BlobStoreError>;
    fn exists(&self, hash: &Hash) -> Result<bool, BlobStoreError>;
}
```

#### Compression Interface
```rust
trait Compression: Send + Sync {
    fn compress(&self, data: &[u8]) -> Result<Vec<u8>, CompressionError>;
    fn decompress(&self, data: &[u8]) -> Result<Vec<u8>, CompressionError>;
    fn algorithm(&self) -> &str;  // "zstd", "gzip", "lz4", etc.
}
```

#### Backend Implementations
- **Storage**: SQLite, RocksDB, Badger, PostgreSQL, etc.
- **Blob Store**: Filesystem, S3, Azure Blob, GCS, etc.
- **Compression**: Zstd, Gzip, LZ4, Brotli, etc.

#### Cross-Platform Determinism
- **Endianness**: All serialization uses network byte order (big-endian)
- **Floating point**: No floating point in deterministic paths
- **Time zones**: All timestamps in UTC
- **Path separators**: Normalized paths (forward slashes)
- **Character encoding**: UTF-8 for all text

#### Test Criteria
- Same data with different backend yields identical hashes (determinism verified)
- Backend swap requires no API change (interface abstraction works)
- Compression choices do not affect FrameID truth bytes (compression is transparent)
- End-to-end determinism holds across OS/arch targets (Linux, macOS, Windows, ARM, x86)
- Backend migration preserves all data (no data loss)
- Performance characteristics documented per backend (latency, throughput)

---

### 7. Documentation & Developer Experience

#### Description
Make external adoption feasible with clear documentation, examples, and integration guides. Ensure developers can quickly understand and integrate the system.

#### Requirements
- **Public documentation**: Comprehensive README, API docs, architecture overview
- **Reference client**: Working client implementation in popular language (Rust, Python, Go)
- **Example workflows**: End-to-end examples for common use cases
- **Integration guides**: CI/CD integration, editor tools, agent integration
- **Non-features documentation**: Clear statement of what the system does NOT do
- **Troubleshooting guide**: Common issues and solutions

#### Documentation Structure
```
docs/
├── README.md                 # Overview, quick start
├── ARCHITECTURE.md           # System architecture
├── API.md                    # API reference
├── SCHEMAS.md                # Schema definitions
├── EXAMPLES.md                # Example workflows
├── INTEGRATION.md            # Integration guides
├── TROUBLESHOOTING.md        # Common issues
└── NON_FEATURES.md           # What we don't do
```

#### Reference Client
```rust
// Rust reference client
pub struct MerkleClient {
    base_url: String,
    workspace_id: WorkspaceID,
    auth_token: String,
}

impl MerkleClient {
    pub async fn get_node(&self, node_id: NodeID, view: ContextView) -> Result<NodeContext>;
    pub async fn put_frame(&self, node_id: NodeID, frame: Frame) -> Result<FrameID>;
    // ... more methods
}
```

#### Example Workflows
- **Basic ingestion**: Scan filesystem, create nodes, attach frames
- **Context synthesis**: Synthesize branch context from children
- **Regeneration**: Trigger regeneration after file changes
- **Export/import**: Export workspace, verify, import to new workspace
- **Multi-agent workflow**: Multiple agents reading/writing concurrently

#### Integration Guides
- **CI/CD**: GitHub Actions, GitLab CI, Jenkins integration
- **Editor tools**: VSCode extension, Vim plugin, Emacs integration
- **Agent integration**: How to integrate with AI agents, CLI tools

#### Test Criteria
- New developer can ingest + fetch node + append frame in < 30 minutes (onboarding time)
- Docs match behavior exactly (no "vibes", all examples tested)
- Examples are deterministic and reproducible (same inputs → same outputs)
- Reference client works out of the box (no configuration required for basic usage)
- Integration guides enable successful integration (tested with real tools)
- Troubleshooting guide covers common issues (based on real support requests)

---

[← Back to Phase 3 Spec](phase3_spec.md)

