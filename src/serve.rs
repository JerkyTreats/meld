//! Served harness substrate: the machine-readable product boundary.
//!
//! Owner: serve domain. Meld ships no visualizer: the product surface is
//! this served substrate — versioned `/v1` endpoints over loopback HTTP
//! whose JSON bodies are the existing contract types verbatim. Endpoints
//! map one to one from the transport-neutral event authority contract
//! plus the report store reader and the harness walks; the blocking watch
//! is the subscription long-poll over the commit watermark. The same
//! handlers mount over a live composition and over a sealed session root,
//! so live and playback surfaces are byte-consistent by construction.
//!
//! Every consumer — a terminal client, a dashboard, the t3code preview —
//! is external and non-authoritative: a consumer that renders what this
//! substrate does not serve has a consumer defect, never an argument for
//! widening the substrate.

pub mod discovery;
pub mod listener;
pub mod routes;
pub mod sources;
