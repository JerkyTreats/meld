#!/usr/bin/env bash
# Domain boundary guard: fail if code violates cross-domain use rules.
# Run from repo root. Add to CI to prevent regressions.
# See design/refactor/PLAN.md Phase 10 and this directory's README.

set -e
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
VIOLATIONS=0

# CLI must not reach into context::frame::storage; use context::frame::open_storage or api instead.
if grep -rq --include='*.rs' 'context::frame::storage' src/cli/ 2>/dev/null; then
  echo "Boundary violation: src/cli/ must not use context::frame::storage (use context::frame::open_storage or api)."
  VIOLATIONS=$((VIOLATIONS + 1))
fi

# Legacy top-level composition removed; use context::query::composition.
if grep -rq --include='*.rs' 'crate::composition::' src/ 2>/dev/null; then
  echo "Boundary violation: crate::composition:: is removed; use crate::context::query::composition."
  VIOLATIONS=$((VIOLATIONS + 1))
fi

# Tooling is removed; no references in src.
if grep -rq --include='*.rs' 'crate::tooling::' src/ 2>/dev/null; then
  echo "Boundary violation: crate::tooling:: is removed; use crate::cli, crate::workspace, crate::agent."
  VIOLATIONS=$((VIOLATIONS + 1))
fi

# Production domains must consume event authority capabilities instead of
# opening canonical stores or spawning independent writers. Temporary E5
# compatibility and in-file test fixtures carry an explicit boundary marker.
RAW_EVENT_CONSTRUCTORS='EventStore::(new|shared)|EventRuntime::(new|from_store)|EventWriter::spawn'
EVENT_CONSTRUCTOR_MATCHES="$(
  rg -n "$RAW_EVENT_CONSTRUCTORS" src crates \
    --glob '*.rs' \
    --glob '!crates/meld-events/**' \
    --glob '!**/tests/**' \
    | while IFS= read -r match; do
        case "$match" in
          *'://!'*) ;;
          crates/meld-world-model/src/world_state/graph/runtime.rs:*'boundary-allow: event-compat'*) ;;
          crates/meld-world-model/src/world_state/graph/runtime.rs:*'boundary-allow: event-test'*) ;;
          src/runtime/storage.rs:*'boundary-allow: event-compat'*) ;;
          src/telemetry/sessions/service.rs:*'boundary-allow: event-compat'*) ;;
          src/events/tooling/tail.rs:*'boundary-allow: event-test'*) ;;
          *) echo "$match" ;;
        esac
      done \
    || true
)"
if [ -n "$EVENT_CONSTRUCTOR_MATCHES" ]; then
  echo "Boundary violation: production code must consume EventAuthority capabilities:"
  echo "$EVENT_CONSTRUCTOR_MATCHES"
  VIOLATIONS=$((VIOLATIONS + 1))
fi

if [ "$VIOLATIONS" -gt 0 ]; then
  echo "Total violations: $VIOLATIONS"
  exit 1
fi
echo "Domain boundary check passed."
exit 0
