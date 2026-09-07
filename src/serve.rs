//! Served harness substrate: the machine-readable product boundary.
//!
//! Owner: serve domain. Meld ships no visualizer: the product surface is
//! this served substrate — versioned `/v1` endpoints over loopback HTTP
//! whose JSON bodies are the existing contract types verbatim. Endpoints
//! delegate to native domain contracts, report reads and harness walks;
//! the blocking watch is the subscription long-poll over the commit watermark.
//! Read handlers mount over both live compositions and sealed session roots.
//! Named Agent request intake is enabled explicitly by the live runtime and
//! uses its existing native owner handle.
//!
//! Every consumer — a terminal client, a dashboard, the t3code preview —
//! is external and non-authoritative: a consumer that renders what this
//! substrate does not serve has a consumer defect, never an argument for
//! widening the substrate.

pub mod discovery;
pub mod listener;
pub mod routes;
pub mod sources;
