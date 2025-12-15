# Phase 2 Development Phases

## Development Phases

### Phase 2A — Agent Interaction Model

| Task | Status |
|-----|--------|
| Define agent roles | Todo |
| Agent identity model | Todo |
| Writer append rules | Todo |
| Reader access rules | Todo |
| Concurrent access safety | Todo |
| Agent authorization checks | Todo |

**Exit Criteria:**
- Agent roles clearly defined (Reader, Writer, Synthesis)
- Authorization enforced on all write operations
- Concurrent agents can operate safely
- Agent identity preserved in all frames

---

### Phase 2B — Core Context APIs

| Task | Status |
|-----|--------|
| GetNode API | Todo |
| PutFrame API | Todo |
| ContextView wiring | Todo |
| Error model | Todo |
| API determinism tests | Todo |
| Concurrent request handling | Todo |

**Exit Criteria:**
- GetNode and PutFrame APIs implemented
- APIs are deterministic (same inputs → same outputs)
- Error handling is comprehensive and deterministic
- Concurrent requests handled safely

---

### Phase 2C — Branch Synthesis

| Task | Status |
|-----|--------|
| Synthesis frame types | Todo |
| Bottom-up traversal logic | Todo |
| Basis construction rules | Todo |
| Synthesis triggers | Todo |
| Synthesis policies | Todo |
| Determinism tests | Todo |

**Exit Criteria:**
- Branch synthesis algorithm implemented
- Synthesis is deterministic (same inputs → same outputs)
- Bottom-up synthesis enforced
- Multiple synthesis policies supported

---

### Phase 2D — Incremental Regeneration

| Task | Status |
|-----|--------|
| Basis diff detection | Todo |
| Regeneration workflow | Todo |
| Atomic head updates | Todo |
| Basis index implementation | Todo |
| Regeneration tests | Todo |
| Idempotency tests | Todo |

**Exit Criteria:**
- Basis change detection working
- Regeneration only affects changed frames
- Regeneration is idempotent
- Old frames preserved (append-only)

---

### Phase 2E — Multi-Frame Composition

| Task | Status |
|-----|--------|
| Composition policies | Todo |
| Ordering strategies | Todo |
| Bounded output enforcement | Todo |
| Multi-source composition | Todo |
| Determinism tests | Todo |

**Exit Criteria:**
- Composition policies implemented
- Composition is deterministic
- Output is bounded (max frames enforced)
- Multiple composition sources supported

---

### Phase 2F — Tooling & Integrations

| Task | Status |
|-----|--------|
| CLI tooling | Todo |
| Editor integration hooks | Todo |
| CI integration | Todo |
| Internal agent adapters | Todo |
| Tool idempotency tests | Todo |

**Exit Criteria:**
- CLI commands implemented
- Tools are idempotent
- Editor hooks functional
- CI integration working
- Clear separation from core engine

---

## Phase Exit Criteria

Phase 2 is complete when:
- Agents can reliably read and write context
- Branch context is synthesized incrementally
- Regeneration is minimal and deterministic
- Workflows compose without search or mutation
- All components tested and documented
- Tooling is functional and idempotent

---

[← Back to Phase 2 Spec](phase2_spec.md)

