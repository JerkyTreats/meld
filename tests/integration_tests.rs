//! Integration tests entry point
//!
//! This file includes all integration test modules from the integration/ subdirectory.
//! Rust automatically compiles files in tests/ as separate test binaries, so this
//! approach allows organizing tests in subdirectories while maintaining discoverability.

#![allow(clippy::duplicate_mod)]

mod integration;
