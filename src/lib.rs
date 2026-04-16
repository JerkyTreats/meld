//! Merkle: Deterministic Filesystem State Management
//!
//! A Merkle-based filesystem state management system that provides deterministic,
//! hash-based tracking of filesystem state and associated context.

pub mod agent;
pub mod api;
pub mod capability;
pub mod cli;
pub mod concurrency;
pub mod config;
pub mod context;
pub mod control;
pub mod error;
pub mod heads;
pub mod ignore;
pub mod init;
pub mod logging;
pub mod merkle_traversal;
pub mod metadata;
pub mod prompt_context;
pub mod provider;
pub mod roots;
pub mod store;
pub mod task;
pub mod telemetry;
pub mod tree;
pub mod types;
pub mod views;
pub mod workflow;
pub mod workspace;
pub mod world_state;
