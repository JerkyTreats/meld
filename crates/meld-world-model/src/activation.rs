//! Source-format-neutral product activation contracts for the world model.
//!
//! Root assembly may parse an external activation document and translate it
//! into these typed values. World-model runtimes consume only this package and
//! never receive the source path, source format, or raw document.
//!
//! W2B bootstrap owns the compatibility decoder for legacy agent records and
//! the transactional migration that writes the directive, canonical agent,
//! and migration receipt. This activation boundary only freezes those values.

mod contracts;
mod validation;

pub use contracts::{
    AgentBootstrapReceipt, AgentCurationRuleRecord, BeliefActivationReceipt, DirectiveRecord,
    LegacyDirectiveMigrationConflict, LegacyDirectiveMigrationConflictField,
    LegacyDirectiveMigrationIdentity, LegacyDirectiveMigrationReceipt, SeedAgentActivation,
    WorldModelActivationIdentity, WorldModelActivationInput,
    LEGACY_DIRECTIVE_MIGRATION_SCHEMA_VERSION,
};
pub use validation::{validate_world_model_activation, WorldModelActivationValidationError};
