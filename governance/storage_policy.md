# Storage Policy

Date: 2026-03-06
Status: active

## Intent

Define inviolable storage requirements for runtime state.
Preserve target workspace purity and keep storage behavior deterministic.

## Requirement 1 Workspace Purity

- Runtime state must never be persisted under the target workspace path.
- Runtime state includes node store data, frame blobs, prompt context artifacts, workflow state records, telemetry logs, and lock files.
- Default paths and fallback paths must resolve outside the target workspace path.
- Any code path that writes under `<workspace>/.meld` is prohibited.

## Requirement 2 External Storage Roots

- Storage implementations must use XDG data roots or another external data root.
- Fallback roots must also remain outside the target workspace path.

## Requirement 3 Verification And Change Control

- New storage features must include tests that assert write paths are outside the target workspace path.
- Any proposal that introduces target workspace writes must be rejected.
