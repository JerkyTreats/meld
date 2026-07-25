//! Stewardship expression selection and physical binding.
//!
//! Owner: root config. This module carries the user-facing selection of a
//! stewardship expression (by identity, never by embedded theory bodies)
//! and the pure resolver that turns a validated selection plus environment
//! into one typed physical binding. Nothing here creates runtime state:
//! loading, validation, and resolution perform no writes and open no
//! stores. Internal runtime actor topology is not expressed here.

pub mod binding;
pub mod selection;
