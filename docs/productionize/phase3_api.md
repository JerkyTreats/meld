# Phase 3 API Specifications

## Public API Surface

All APIs are versioned via URL path (`/v1/`, `/v2/`, etc.) and require workspace identification.

### Workspace Management

#### Create Workspace
```rust
POST /v1/workspaces
{
  "name": "my-workspace",
  "description": "Optional description"
}

Response: {
  "workspace_id": "abc123...",
  "name": "my-workspace",
  "created_at": "2024-01-01T00:00:00Z"
}
```

#### Get Workspace
```rust
GET /v1/workspaces/{workspace_id}

Response: {
  "workspace_id": "abc123...",
  "name": "my-workspace",
  "root_hash": "def456...",
  "created_at": "2024-01-01T00:00:00Z"
}
```

### Node Operations

#### Get Node (Versioned)
```rust
GET /v1/workspaces/{workspace_id}/nodes/{node_id}?view={view_policy}

Headers:
  Authorization: Bearer {token}
  X-Workspace-ID: {workspace_id}  // Alternative to path

Response: {
  "node_id": "node123...",
  "node_record": { ... },
  "frames": [ ... ],
  "frame_count": 42
}
```

#### Get Nodes Batch
```rust
POST /v1/workspaces/{workspace_id}/nodes/batch
{
  "node_ids": ["node1...", "node2..."],
  "view": { ... }
}

Response: {
  "nodes": {
    "node1...": { ... },
    "node2...": { ... }
  }
}
```

### Frame Operations

#### Put Frame (Versioned)
```rust
POST /v1/workspaces/{workspace_id}/frames
{
  "node_id": "node123...",
  "frame": {
    "basis": { ... },
    "content": "...",
    "frame_type": "analysis",
    "metadata": { ... }
  }
}

Headers:
  Authorization: Bearer {token}
  X-Agent-ID: {agent_id}

Response: {
  "frame_id": "frame456...",
  "node_id": "node123...",
  "created_at": "2024-01-01T00:00:00Z"
}
```

#### Get Frames Batch
```rust
POST /v1/workspaces/{workspace_id}/frames/batch
{
  "frame_ids": ["frame1...", "frame2..."]
}

Response: {
  "frames": {
    "frame1...": { ... },
    "frame2...": { ... }
  }
}
```

### Synthesis Operations

#### Synthesize Branch (Versioned)
```rust
POST /v1/workspaces/{workspace_id}/synthesize
{
  "node_id": "node123...",
  "frame_type": "branch-summary",
  "policy": { ... }  // Optional
}

Headers:
  Authorization: Bearer {token}
  X-Agent-ID: {agent_id}

Response: {
  "frame_id": "frame789...",
  "node_id": "node123...",
  "synthesized_at": "2024-01-01T00:00:00Z"
}
```

### Export/Import Operations

#### Export Workspace
```rust
GET /v1/workspaces/{workspace_id}/export?root_hash={hash}

Response: Binary stream (tar.gz)
```

#### Import Workspace
```rust
POST /v1/workspaces/import
Content-Type: multipart/form-data

Response: {
  "workspace_id": "new123...",
  "root_hash": "def456...",
  "imported_nodes": 1000,
  "imported_frames": 5000
}
```

### Health & Observability

#### Health Check
```rust
GET /v1/health/ready
GET /v1/health/live

Response: {
  "status": "ready",
  "workspace_count": 10,
  "storage_ok": true
}
```

#### Metrics
```rust
GET /v1/metrics

Response: Prometheus format or JSON
```

---

## Error Handling

### Error Types

#### ApiError (Versioned)
```rust
#[derive(Debug, thiserror::Error, Serialize)]
#[serde(tag = "error_type")]
enum ApiError {
    #[error("Node not found: {0:?}")]
    NodeNotFound(NodeID),
    
    #[error("Frame not found: {0:?}")]
    FrameNotFound(FrameID),
    
    #[error("Workspace not found: {0:?}")]
    WorkspaceNotFound(WorkspaceID),
    
    #[error("Agent unauthorized: {0}")]
    Unauthorized(String),
    
    #[error("Access denied: {0}")]
    AccessDenied(String),
    
    #[error("Invalid request: {0}")]
    InvalidRequest(String),
    
    #[error("Schema validation failed: {0}")]
    SchemaValidationFailed(String),
    
    #[error("Version not supported: {0}")]
    VersionNotSupported(String),
    
    #[error("Storage error: {0}")]
    StorageError(String),
    
    #[error("Internal error: {0}")]
    InternalError(String),
}
```

### Error Response Format
```json
{
  "error": {
    "error_type": "NodeNotFound",
    "message": "Node not found: abc123...",
    "node_id": "abc123...",
    "code": "NODE_NOT_FOUND",
    "request_id": "req-xyz789..."
  }
}
```

### Error Handling Principles
- **Deterministic**: Same error conditions → same error responses
- **Versioned**: Error formats are versioned with API
- **Actionable**: Error messages include context and remediation steps
- **Structured**: Errors are structured JSON, not plain text
- **Correlation**: All errors include request correlation ID

### Common Error Scenarios

#### Workspace Not Found
- **Cause**: WorkspaceID doesn't exist or access denied
- **Response**: `WorkspaceNotFound` or `AccessDenied`
- **Recovery**: Verify workspace ID or request access

#### Unauthorized Access
- **Cause**: Agent lacks required permissions
- **Response**: `Unauthorized` or `AccessDenied`
- **Recovery**: Request appropriate permissions from workspace owner

#### Schema Validation Failed
- **Cause**: Request payload doesn't match API schema
- **Response**: `SchemaValidationFailed` with detailed field errors
- **Recovery**: Fix payload according to schema documentation

#### Version Not Supported
- **Cause**: Client requests unsupported API version
- **Response**: `VersionNotSupported` with supported versions
- **Recovery**: Update client to use supported version

---

[← Back to Phase 3 Spec](phase3_spec.md)

