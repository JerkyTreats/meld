//! Interactive runtime harness over the three-customer information model.
//!
//! Owner: harness. The harness is a development-time observation layer
//! composed from product primitives: it boots product assemblies into
//! isolated roots through the same staged pipeline the product uses,
//! records what it injected and stepped as a durable manifest, and derives
//! navigable walks over records the runtime already persists. It owns no
//! semantic truth — every durable effect crosses a domain command or the
//! canonical append, and every observation derives from durable records.
//!
//! Delivery is chartered by
//! `design/plan/integration/runtime_harness_plan.md` over the frozen
//! register in
//! `design/plan/integration/agent_native_debugger_requirements.md`.

pub mod boot;
pub mod manifest;
