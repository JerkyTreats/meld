# Phase 3 Spec — Prepare for External Use

## Overview

This specification documents Phase 3 of the Merkle-based filesystem state management system. Phase 3 builds upon the Phase 1 and Phase 2 foundations to prepare the system for external consumption by teams, services, and tools. The focus is on stabilization, formalization, safety, and operational readiness while preserving all Phase 1 and Phase 2 invariants (determinism, no search, no mutation).

## Table of Contents

- [Goals + Outcomes](#goals--outcomes)
- [Dependencies & Assumptions](#dependencies--assumptions)
- [Major Components](#major-components)
- [Component Relationships](#component-relationships)
- [API Specifications](phase3_api.md)
- [Error Handling](phase3_api.md#error-handling)
- [Performance Considerations](#performance-considerations)
- [Constraints & Non-Goals](#constraints--non-goals)
- [Development Phases](phase3_phases.md)
- [Phase Exit Criteria](#phase-exit-criteria)

---

## Goals + Outcomes

### Goals
- Stabilize the system for external consumers (teams, services, tools)
- Formalize contracts, schemas, and versioning
- Add safety, isolation, and operational readiness
- Preserve the deterministic, Merkle-truth architecture (no search required)
- Enable multi-tenant usage with workspace isolation
- Provide operational observability and diagnostics

### Outcomes
- Versioned public API and schemas with backward compatibility guarantees
- Workspace isolation and access controls with audit logging
- Exportable/verifiable snapshots and deterministic replay
- Operational observability and performance hardening
- Pluggable backends (storage, compression) without behavior drift
- Production-ready system suitable for external adoption

---

## Dependencies & Assumptions

### Phase 1 Prerequisites
Phase 3 assumes the following Phase 1 components are complete and operational:

- **Filesystem Merkle Tree**: Stable NodeID generation and root hash computation
- **NodeRecord Store**: O(1) node lookup by NodeID
- **Context Frames**: Immutable, append-only frame storage with deterministic FrameIDs
- **Context Frame Merkle Set**: Deterministic frame set membership tracking
- **Frame Heads**: O(1) head resolution by (NodeID, type)
- **Context Views**: Policy-driven, bounded frame selection

### Phase 2 Prerequisites
Phase 3 assumes the following Phase 2 components are complete and operational:

- **Agent Read/Write Model**: Agent identity and role-based access
- **Context APIs**: GetNode, PutFrame, SynthesizeBranch, Regenerate
- **Branch Context Synthesis**: Deterministic bottom-up synthesis
- **Incremental Regeneration**: Basis-driven frame regeneration
- **Multi-Frame Composition**: Policy-driven frame composition
- **Tooling & Integration Layer**: CLI tools and basic integrations

### System Invariants (Preserved from Phase 1 & 2)
- **Determinism**: Same inputs → same outputs (hashes, IDs, frame sets)
- **No Search**: No semantic search, full scans, or fuzzy matching
- **No Mutation**: Append-only operations; frames and nodes are immutable
- **Hash-Based Invalidation**: Changes detected only through hash comparison
- **Bounded Context**: Context views are bounded (max frame count)

### Assumptions
- Multiple workspaces can coexist in the same deployment
- External consumers require stable API contracts
- Authentication/authorization mechanisms are available (tokens, mTLS, or identity binding)
- Storage backends can be swapped without changing semantics
- Cross-platform determinism is achievable (same data → same hashes)

---

## Major Components

Phase 3 consists of seven major components that prepare the system for external use:

1. **Public API Contracts & Versioning**: Versioned, stable API surfaces with backward compatibility
2. **Workspace Isolation & Access Control**: Multi-tenant isolation and fine-grained access control
3. **Snapshot Export, Verification, and Replay**: Export/import workflows with integrity verification
4. **Observability & Diagnostics**: Metrics, logging, and determinism diagnostics
5. **Performance Hardening**: Batching, caching, and concurrency controls
6. **Pluggable Backends & Portability**: Swappable storage and compression backends
7. **Documentation & Developer Experience**: Comprehensive docs and reference implementations

For detailed component specifications, see **[Component Specifications](phase3_components.md)**.

### Component Overview

#### Public API Contracts & Versioning
Freezes stable API surfaces and versions them to prevent breaking consumers. Establishes clear contracts for request/response formats, error handling, and backward compatibility guarantees.

#### Workspace Isolation & Access Control
Supports multiple workspaces safely and enforces access boundaries. Ensures complete isolation between workspaces while providing fine-grained access control within each workspace.

#### Snapshot Export, Verification, and Replay
Enables external systems to export a workspace snapshot and verify it independently. Supports deterministic replay/rebuild from exports to enable backup, migration, and verification workflows.

#### Observability & Diagnostics
Makes the system operable: traces changes, measures performance, debugs determinism issues, and monitors health. Provides comprehensive observability without compromising determinism or performance.

#### Performance Hardening
Improves throughput and tail latency while maintaining correctness. Adds batching, caching, and concurrency controls to handle production workloads efficiently.

#### Pluggable Backends & Portability
Allows swapping storage and compression backends without changing semantics. Ensures cross-platform determinism and enables deployment flexibility.

#### Documentation & Developer Experience
Makes external adoption feasible with clear documentation, examples, and integration guides. Ensures developers can quickly understand and integrate the system.

---

## Component Relationships

### Data Flow
```
External Request
    ↓
Public API (Versioned)
    ↓
Workspace Isolation & Access Control
    ↓
Core APIs (Phase 2)
    ↓
Storage Backend (Pluggable)
    ↓
Response with Observability
```

### Observability Flow
```
Operation
    ↓
Metrics Collection
    ↓
Structured Logging
    ↓
Diagnostics (if needed)
    ↓
Health Check Updates
```

### Export/Import Flow
```
Workspace State
    ↓
Snapshot Export
    ↓
Verification
    ↓
Import/Replay
    ↓
Verification
    ↓
Identical Workspace State
```

### Dependencies
- **Public API** depends on **Workspace Isolation** (for workspace scoping)
- **Workspace Isolation** depends on **Storage Backend** (for namespace enforcement)
- **Snapshot Export** depends on **Storage Backend** (to read all data)
- **Observability** depends on all components (to collect metrics/logs)
- **Performance Hardening** depends on **Storage Backend** (for caching/batching)
- **Documentation** depends on all components (to document behavior)

---

## Performance Considerations

### Performance Targets

#### API Operations
- **GetNode**: < 10ms p50, < 50ms p99 (with bounded view)
- **PutFrame**: < 5ms p50, < 20ms p99 (frame creation + head update)
- **GetNodesBatch**: < 50ms p50, < 200ms p99 (100 nodes)
- **SynthesizeBranch**: < 50ms p50, < 200ms p99 (directory with 100 children)
- **Export**: < 1s per GB (streaming export)

#### Storage Operations
- **NodeRecord lookup**: < 1ms (O(1) with caching)
- **Frame retrieval**: < 5ms per frame (with caching)
- **Head resolution**: < 1ms (O(1) with caching)
- **Batch operations**: 10x improvement over individual operations

#### System Capacity
- **Workspaces**: Support 1000+ workspaces per deployment
- **Nodes per workspace**: Support 10M+ nodes
- **Frames per workspace**: Support 100M+ frames
- **Concurrent requests**: Support 1000+ concurrent requests

### Optimization Strategies

#### Caching
- **NodeRecord cache**: LRU cache, 80%+ hit rate for hot data
- **Frame cache**: LRU cache for frequently accessed frames
- **Head cache**: In-memory hash table for O(1) head resolution
- **Cache invalidation**: Event-based invalidation on writes

#### Batching
- **Batch storage reads**: Read multiple records in single operation
- **Batch storage writes**: Write multiple records in transaction
- **Batch API requests**: Client can batch multiple operations

#### Connection Pooling
- **Database connections**: Pool of connections for storage backend
- **Blob store connections**: Pool of connections for blob storage
- **Connection reuse**: Reuse connections across requests

#### Backpressure
- **Synthesis queue**: Bounded queue with backpressure
- **Rate limiting**: Per-agent rate limits to prevent abuse
- **Circuit breakers**: Fail fast when dependencies are down

### Scalability Considerations
- **Horizontal scaling**: Stateless API layer enables horizontal scaling
- **Storage sharding**: Future: shard workspaces across storage nodes
- **Caching layer**: Distributed cache (Redis) for multi-instance deployments
- **Load balancing**: Stateless design enables standard load balancing

---

## Constraints & Non-Goals

### Constraints

#### Determinism Requirement
- All operations must be deterministic (preserved from Phase 1 & 2)
- No random number generation in core paths
- No time-dependent behavior (except metadata timestamps)
- No external API calls that could vary

#### No Search Constraint
- No semantic search or fuzzy matching (preserved from Phase 1 & 2)
- No full scans of frame storage
- No content-based queries (only hash-based)
- No machine learning or AI in core engine

#### Append-Only Constraint
- Frames are immutable once created (preserved from Phase 1 & 2)
- Nodes are immutable (new state = new NodeID)
- No deletion or modification of existing data
- History is preserved (can archive old data)

#### Bounded Context Constraint
- Context views have maximum frame count (preserved from Phase 1 & 2)
- No unbounded frame retrieval
- Memory usage is bounded per operation

#### Versioning Constraint
- Backward compatibility required within major version
- Breaking changes require major version bump
- Deprecation policy must be followed

### Non-Goals (Out of Scope for Phase 3)

#### Not Included
- **Semantic Search**: No content-based search or similarity matching
- **Frame Deletion**: No deletion of frames (append-only)
- **Frame Modification**: No mutation of existing frames
- **Global Queries**: No queries across entire workspace (only node-scoped)
- **Real-time Collaboration**: No live sync or conflict resolution
- **Version Control Integration**: No direct Git/SVN integration
- **Distributed Storage**: No multi-machine storage (single deployment per workspace)
- **Frame Encryption**: No encryption of frame content (storage-level encryption OK)
- **Advanced Access Control**: Basic ACLs only, no fine-grained permissions
- **GraphQL API**: REST API only (GraphQL may be future phase)

#### Future Phases
- **Phase 4+**: May add semantic search, advanced queries, distributed storage

---

## Quick Links

- **[Component Specifications](phase3_components.md)** - Detailed specifications for each component
- **[API Specifications](phase3_api.md)** - Public API surface and error handling
- **[Development Phases](phase3_phases.md)** - Task breakdown and exit criteria

---

## Phase Exit Criteria

Phase 3 is complete when:

- **Public API and schemas are versioned and stable**: API v1 is frozen with backward compatibility guarantees
- **Workspaces are isolated with enforced access controls**: Multi-tenant isolation verified, ACLs working
- **Snapshots can be exported, verified, and replayed deterministically**: Export/import produces identical state
- **System is observable and debuggable under real workloads**: Metrics, logs, and diagnostics operational
- **Performance targets are met**: All performance targets achieved under load
- **Backends are swappable without semantic drift**: Multiple backends tested, determinism verified
- **Documentation enables external adoption**: New developers can integrate in < 30 minutes
- **All components operational**: All seven major components implemented and tested

---
