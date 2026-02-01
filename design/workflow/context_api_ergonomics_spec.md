# Context API Ergonomics Specification

## Overview

This specification documents ergonomic improvements to the Context API that provide convenience layers for common user workflows. These improvements are **additive and non-breaking**—they build upon the existing core API without modifying existing functionality.

## Table of Contents

- [Goals + Outcomes](#goals--outcomes)
- [Current State](#current-state)
- [User Pain Points](#user-pain-points)
- [Proposed Improvements](#proposed-improvements)
- [Implementation Approach](#implementation-approach)
- [API Specifications](#api-specifications)
- [Migration Path](#migration-path)
- [Constraints & Non-Goals](#constraints--non-goals)

---

## Goals + Outcomes

### Goals
- Reduce boilerplate code for common context retrieval patterns
- Provide type-safe, ergonomic access to frame content
- Enable fluent query construction for context views
- Support common aggregation and composition workflows
- Maintain backward compatibility with existing API

### Outcomes
- Users can access frame content as text without manual UTF-8 conversion
- Query construction is more readable and less error-prone
- Common patterns (latest frame, frames by type, etc.) are one-line operations
- Content aggregation and formatting are built-in
- Existing code continues to work unchanged

---

## Current State

### Core API (Unchanged)

The existing core API provides:

```rust
// Core retrieval
pub fn get_node(&self, node_id: NodeID, view: ContextView) -> Result<NodeContext, ApiError>

// Response structure
pub struct NodeContext {
    pub node_id: NodeID,
    pub node_record: NodeRecord,
    pub frames: Vec<Frame>,
    pub frame_count: usize,
}

// Frame structure
pub struct Frame {
    pub frame_id: FrameID,
    pub basis: Basis,
    pub content: Vec<u8>,  // Raw bytes
    pub frame_type: String,
    pub metadata: HashMap<String, String>,
    pub timestamp: SystemTime,
}

// View construction
pub struct ContextView {
    pub max_frames: usize,
    pub ordering: OrderingPolicy,
    pub filters: Vec<FrameFilter>,
}
```

### Current Usage Pattern

Users must manually:
- Convert `Vec<u8>` to `String` with error handling
- Construct `ContextView` structs with verbose syntax
- Iterate and filter frames manually
- Aggregate content manually
- Handle encoding errors at each call site

---

## User Pain Points

### 1. Content Access

**User Action:** "I want the text content from all frames."

**Current Workflow:**
- Iterate through `context.frames`
- Convert each `frame.content` (Vec<u8>) to String
- Handle UTF-8 encoding errors manually
- Decide how to combine multiple frames

**Pain:** Repetitive error handling, no type hints for content format

### 2. Query Construction

**User Action:** "I want the 20 most recent analysis frames from agent-1."

**Current Workflow:**
- Manually construct `ContextView` struct
- Set `max_frames: 20`
- Set `ordering: OrderingPolicy::Recency`
- Build `Vec<FrameFilter>` with string allocations
- Remember enum variant names

**Pain:** Verbose, error-prone, requires remembering struct fields

### 3. Finding Specific Frames

**User Action:** "I need the latest analysis frame."

**Current Workflow:**
- Get all frames via `get_node()`
- Manually filter by `frame_type == "analysis"`
- Manually sort by timestamp
- Take first result

**Pain:** Common pattern requires multiple lines of boilerplate

### 4. Content Aggregation

**User Action:** "I want all context combined into one string for an LLM prompt."

**Current Workflow:**
- Iterate frames
- Convert each to text
- Join with separators
- Handle encoding errors

**Pain:** Repetitive code, error handling at each step

### 5. Metadata Access

**User Action:** "I need the agent ID and timestamp from each frame."

**Current Workflow:**
- Access `frame.metadata.get("agent_id")` (returns Option)
- Handle Option unwrapping
- Parse strings if needed
- No compile-time type safety

**Pain:** No type safety, manual Option handling, string parsing

### 6. Pagination

**User Action:** "I want frames 10-20, then 20-30."

**Current Workflow:**
- Request all frames with high `max_frames`
- Manually slice the vector
- Or make multiple requests with different `max_frames`
- No efficient cursor-based pagination

**Pain:** Inefficient, no standard pagination pattern

### 7. Content Type Detection

**User Action:** "I want to parse JSON frames."

**Current Workflow:**
- Assume content is JSON
- Try to parse each frame
- Handle errors manually
- No type hints or validation

**Pain:** No type safety, manual error handling

### 8. Common Query Patterns

**User Action:** "I want the latest context for this file."

**Current Workflow:**
- Build `ContextView` each time
- Remember correct parameters
- Repeat across codebase

**Pain:** Repetitive code, no reusable patterns

---

## Proposed Improvements

All improvements are **additive convenience layers** that use existing functionality.

### 1. Content Access Methods on NodeContext

Add methods to `NodeContext` for common content access patterns:

- `text_contents()` - Get all frame contents as UTF-8 strings (filters invalid UTF-8)
- `combined_text(separator)` - Get concatenated text with custom separator
- `content_by_type(frame_type)` - Get content slices filtered by type
- `json_frames<T>()` - Parse frames as JSON with type safety
- `latest_frame_of_type(frame_type)` - Get most recent frame of specific type
- `frames_by_agent(agent_id)` - Get all frames from specific agent

### 2. Query Builder for ContextView

Add fluent builder pattern for constructing `ContextView`:

- `ContextView::builder()` - Start builder
- `.max_frames(n)` - Set maximum frames
- `.recent()` - Order by recency
- `.by_type(type)` - Filter by frame type
- `.by_agent(agent_id)` - Filter by agent
- `.build()` - Construct ContextView

### 3. Typed Accessors on Frame

Add convenience methods to `Frame`:

- `text_content()` - Get content as String (Result)
- `json_content<T>()` - Parse content as JSON
- `agent_id()` - Get agent ID from metadata (Option)
- `metadata_value(key)` - Get typed metadata value
- `is_type(frame_type)` - Check frame type

### 4. Convenience Methods on ContextApi

Add high-level query methods:

- `latest_context(node_id)` - Get most recent context (default view)
- `context_by_type(node_id, frame_type)` - Get frames filtered by type
- `context_by_agent(node_id, agent_id)` - Get frames filtered by agent
- `combined_context_text(node_id, separator)` - Get combined text directly

### 5. Iterator Support

Add iterator methods for streaming and pagination:

- `frames_iter()` - Iterator over frames
- `text_iter()` - Iterator over text content
- `filter_by_type(frame_type)` - Filtered iterator
- Pagination support (offset/limit or cursor-based)

---

## Implementation Approach

### Non-Breaking Additions

All improvements are implemented as:

1. **New `impl` blocks** on existing types (`NodeContext`, `Frame`, `ContextView`)
2. **New methods** on `ContextApi` that compose existing `get_node()` calls
3. **Builder pattern** as a separate type that constructs existing `ContextView`
4. **No changes** to existing struct fields or method signatures

### Composition Pattern

New methods use existing functionality:

```rust
// Example: text_contents() implementation
impl NodeContext {
    pub fn text_contents(&self) -> Vec<String> {
        self.frames.iter()
            .filter_map(|f| String::from_utf8(f.content.clone()).ok())
            .collect()
    }
}

// Example: latest_context() implementation
impl ContextApi {
    pub fn latest_context(&self, node_id: NodeID) -> Result<NodeContext, ApiError> {
        let view = ContextView {
            max_frames: 1,
            ordering: OrderingPolicy::Recency,
            filters: vec![],
        };
        self.get_node(node_id, view)
    }
}
```

### Error Handling Strategy

- **Graceful degradation**: Methods like `text_contents()` filter out invalid UTF-8
- **Explicit errors**: Methods like `text_content()` return `Result` for error handling
- **Type safety**: JSON parsing methods return typed results with clear errors

---

## API Specifications

### NodeContext Extensions

#### Content Access

```rust
impl NodeContext {
    /// Get all frame contents as UTF-8 strings
    /// Filters out frames with invalid UTF-8 content
    pub fn text_contents(&self) -> Vec<String>
    
    /// Get concatenated text content with separator
    pub fn combined_text(&self, separator: &str) -> String
    
    /// Get content slices filtered by frame type
    pub fn content_by_type(&self, frame_type: &str) -> Vec<&[u8]>
    
    /// Parse frames as JSON (filters out non-JSON frames)
    pub fn json_frames<T: DeserializeOwned>(&self) -> Vec<Result<T, serde_json::Error>>
    
    /// Get most recent frame of specific type
    pub fn latest_frame_of_type(&self, frame_type: &str) -> Option<&Frame>
    
    /// Get all frames from specific agent
    pub fn frames_by_agent(&self, agent_id: &str) -> Vec<&Frame>
}
```

#### Iterator Support

```rust
impl NodeContext {
    /// Iterator over frames
    pub fn frames_iter(&self) -> impl Iterator<Item = &Frame>
    
    /// Iterator over text content (filters invalid UTF-8)
    pub fn text_iter(&self) -> impl Iterator<Item = String>
    
    /// Filtered iterator by frame type
    pub fn filter_by_type(&self, frame_type: &str) -> impl Iterator<Item = &Frame>
}
```

### Frame Extensions

```rust
impl Frame {
    /// Get content as UTF-8 string
    pub fn text_content(&self) -> Result<String, FromUtf8Error>
    
    /// Parse content as JSON
    pub fn json_content<T: DeserializeOwned>(&self) -> Result<T, serde_json::Error>
    
    /// Get agent ID from metadata
    pub fn agent_id(&self) -> Option<&str>
    
    /// Get metadata value by key
    pub fn metadata_value(&self, key: &str) -> Option<&str>
    
    /// Check if frame matches type
    pub fn is_type(&self, frame_type: &str) -> bool
}
```

### ContextView Builder

```rust
impl ContextView {
    /// Create a new builder
    pub fn builder() -> ContextViewBuilder
}

pub struct ContextViewBuilder {
    max_frames: Option<usize>,
    ordering: Option<OrderingPolicy>,
    filters: Vec<FrameFilter>,
}

impl ContextViewBuilder {
    /// Set maximum number of frames
    pub fn max_frames(mut self, n: usize) -> Self
    
    /// Order by recency (most recent first)
    pub fn recent(mut self) -> Self
    
    /// Order by frame type (lexicographic)
    pub fn by_type_ordering(mut self) -> Self
    
    /// Order by agent ID (lexicographic)
    pub fn by_agent_ordering(mut self) -> Self
    
    /// Filter by frame type
    pub fn by_type(mut self, frame_type: impl Into<String>) -> Self
    
    /// Filter by agent ID
    pub fn by_agent(mut self, agent_id: impl Into<String>) -> Self
    
    /// Build the ContextView
    pub fn build(self) -> ContextView
}
```

### ContextApi Convenience Methods

```rust
impl ContextApi {
    /// Get latest context (most recent frame)
    pub fn latest_context(&self, node_id: NodeID) -> Result<NodeContext, ApiError>
    
    /// Get context filtered by frame type
    pub fn context_by_type(
        &self,
        node_id: NodeID,
        frame_type: &str,
        max_frames: usize,
    ) -> Result<NodeContext, ApiError>
    
    /// Get context filtered by agent
    pub fn context_by_agent(
        &self,
        node_id: NodeID,
        agent_id: &str,
        max_frames: usize,
    ) -> Result<NodeContext, ApiError>
    
    /// Get combined text content directly
    pub fn combined_context_text(
        &self,
        node_id: NodeID,
        separator: &str,
        view: ContextView,
    ) -> Result<String, ApiError>
}
```

---

## Usage Examples

### Example 1: Get Text Content

**Before:**
```rust
let context = api.get_node(node_id, view)?;
let mut texts = Vec::new();
for frame in &context.frames {
    match String::from_utf8(frame.content.clone()) {
        Ok(text) => texts.push(text),
        Err(_) => continue, // Skip invalid UTF-8
    }
}
let combined = texts.join("\n\n");
```

**After:**
```rust
let context = api.get_node(node_id, view)?;
let combined = context.combined_text("\n\n");
```

### Example 2: Query Construction

**Before:**
```rust
let view = ContextView {
    max_frames: 20,
    ordering: OrderingPolicy::Recency,
    filters: vec![
        FrameFilter::ByType("analysis".to_string()),
        FrameFilter::ByAgent("agent-1".to_string()),
    ],
};
```

**After:**
```rust
let view = ContextView::builder()
    .max_frames(20)
    .recent()
    .by_type("analysis")
    .by_agent("agent-1")
    .build();
```

### Example 3: Find Latest Frame

**Before:**
```rust
let context = api.get_node(node_id, view)?;
let latest = context.frames
    .iter()
    .filter(|f| f.frame_type == "analysis")
    .max_by_key(|f| f.timestamp)
    .map(|f| f.text_content());
```

**After:**
```rust
let context = api.get_node(node_id, view)?;
let latest = context.latest_frame_of_type("analysis")
    .and_then(|f| f.text_content().ok());
```

### Example 4: High-Level Query

**Before:**
```rust
let view = ContextView {
    max_frames: 1,
    ordering: OrderingPolicy::Recency,
    filters: vec![],
};
let context = api.get_node(node_id, view)?;
```

**After:**
```rust
let context = api.latest_context(node_id)?;
```

### Example 5: JSON Parsing

**Before:**
```rust
let context = api.get_node(node_id, view)?;
let mut parsed = Vec::new();
for frame in &context.frames {
    if let Ok(data) = serde_json::from_slice::<Analysis>(&frame.content) {
        parsed.push(data);
    }
}
```

**After:**
```rust
let context = api.get_node(node_id, view)?;
let parsed: Vec<Analysis> = context.json_frames()
    .filter_map(|r| r.ok())
    .collect();
```

---

## Migration Path

### Phase 1: Add Convenience Methods (Non-Breaking)

- Add `impl` blocks to `NodeContext`, `Frame`, `ContextView`
- Add convenience methods to `ContextApi`
- All existing code continues to work
- New code can opt-in to convenience methods

### Phase 2: Documentation and Examples

- Update documentation with convenience method examples
- Add examples showing before/after patterns
- Update integration tests to demonstrate usage

### Phase 3: Optional Deprecation (Future)

- Consider deprecating verbose patterns in favor of convenience methods
- Provide migration guides
- Maintain backward compatibility

---

## Constraints & Non-Goals

### Constraints

- **No Breaking Changes**: All improvements must be additive
- **Performance**: Convenience methods should not add significant overhead
- **Composability**: Methods should compose well with existing API
- **Type Safety**: Where possible, use Rust's type system for safety

### Non-Goals

- **Not changing core data structures**: `NodeContext`, `Frame`, `ContextView` remain unchanged
- **Not modifying existing methods**: `get_node()`, `put_frame()`, etc. unchanged
- **Not adding new storage formats**: Content remains `Vec<u8>`, metadata remains `HashMap`
- **Not implementing semantic search**: Still policy-driven, deterministic selection
- **Not adding async/await**: Methods remain synchronous (can be wrapped if needed)

### Performance Considerations

- Convenience methods should be zero-cost abstractions where possible
- Text conversion happens on-demand (not cached)
- Iterators are lazy where appropriate
- No additional storage overhead

---

## Future Considerations

### Potential Extensions (Out of Scope)

- Cursor-based pagination for large frame sets
- Streaming iterators for very large contexts
- Content caching for frequently accessed frames
- Typed metadata accessors with schema validation
- Content transformation pipelines (markdown, HTML, etc.)

### Integration Points

- These convenience methods work with existing `compose()` API
- Can be used with `synthesize_branch()` results
- Compatible with frame generation queue outputs
- Works with all existing view policies

---

## Summary

This specification defines **additive, non-breaking convenience layers** that make common context retrieval patterns more ergonomic. The improvements:

1. **Reduce boilerplate** for content access and query construction
2. **Provide type safety** for common operations
3. **Enable fluent APIs** for better readability
4. **Maintain compatibility** with all existing code
5. **Compose cleanly** with existing functionality

All improvements are implemented as new methods on existing types, using the existing core API under the hood. No breaking changes are required, and users can adopt improvements incrementally.

---

[← Back to Phase 2 Spec](phase2_spec.md)

