//! Runtime configuration loading for belief families.
//!
//! The loader validates semantic content before evidence normalization or
//! comparator execution can use it. The snapshot hash is stored with revisions
//! so replay can distinguish config changes from evidence changes.
//!
//! # Example
//!
//! ```rust
//! use meld_world_model::belief::BeliefConfigLoader;
//!
//! let bad = r#"{
//!   "family_id": "",
//!   "dimension_id": "dimension",
//!   "predicate_id": "predicate",
//!   "evidence_policy_id": "policy",
//!   "evidence_schemas": [],
//!   "source_mappings": [],
//!   "comparator": {
//!     "engine_id": "weighted_bayesian",
//!     "engine_version": "1",
//!     "factors": [],
//!     "missing_evidence_uncertainty": 0.5
//!   },
//!   "default_prior": 0.5,
//!   "planner_projection": {
//!     "confidence_field": "confidence",
//!     "threshold": 0.7,
//!     "posterior_meaning": "probability"
//!   },
//!   "config_version": "1"
//! }"#;
//!
//! assert!(BeliefConfigLoader::load_json(bad).is_err());
//! ```

use std::collections::BTreeSet;

use crate::belief::contracts::{
    require_non_empty, validate_probability, BeliefFamilyConfig, ComparatorFactorConfig,
};
use crate::error::StorageError;

const MAX_BELIEF_CONFIG_ITEMS: usize = 1024;

/// Validated configuration plus a stable content hash.
#[derive(Debug, Clone, PartialEq)]
pub struct ConfigSnapshot {
    /// Parsed family configuration.
    pub config: BeliefFamilyConfig,
    /// Stable hash of the serialized configuration.
    pub hash: String,
}

/// Loader and validator for external belief family configuration.
pub struct BeliefConfigLoader;

impl BeliefConfigLoader {
    /// Parse and snapshot a JSON family configuration.
    pub fn load_json(input: &str) -> Result<ConfigSnapshot, StorageError> {
        let config: BeliefFamilyConfig = serde_json::from_str(input).map_err(to_storage_data)?;
        Self::snapshot(config)
    }

    /// Validate and hash an already parsed family configuration.
    pub fn snapshot(config: BeliefFamilyConfig) -> Result<ConfigSnapshot, StorageError> {
        Self::validate(&config)?;
        let encoded = serde_json::to_vec(&config).map_err(to_storage_data)?;
        Ok(ConfigSnapshot {
            hash: stable_hash_hex(&encoded),
            config,
        })
    }

    /// Enforce the first-slice runtime contract.
    pub fn validate(config: &BeliefFamilyConfig) -> Result<(), StorageError> {
        require_non_empty("family id", &config.family_id)?;
        require_non_empty("dimension id", &config.dimension_id)?;
        require_non_empty("predicate id", &config.predicate_id)?;
        require_non_empty("evidence policy id", &config.evidence_policy_id)?;
        require_non_empty("config version", &config.config_version)?;
        validate_probability("default prior", config.default_prior)?;
        validate_probability("planner threshold", config.planner_projection.threshold)?;
        require_non_empty(
            "planner confidence field",
            &config.planner_projection.confidence_field,
        )?;
        require_non_empty(
            "posterior meaning",
            &config.planner_projection.posterior_meaning,
        )?;
        require_non_empty("comparator engine id", &config.comparator.engine_id)?;
        require_non_empty(
            "comparator engine version",
            &config.comparator.engine_version,
        )?;
        if config.comparator.engine_id != "weighted_bayesian" {
            return Err(StorageError::InvalidPath(format!(
                "unsupported comparator engine '{}'",
                config.comparator.engine_id
            )));
        }
        validate_probability(
            "missing evidence uncertainty",
            config.comparator.missing_evidence_uncertainty,
        )?;
        for (name, count) in [
            ("evidence schemas", config.evidence_schemas.len()),
            ("source mappings", config.source_mappings.len()),
            ("comparator factors", config.comparator.factors.len()),
        ] {
            if count > MAX_BELIEF_CONFIG_ITEMS {
                return Err(StorageError::InvalidPath(format!(
                    "belief {name} exceed the {MAX_BELIEF_CONFIG_ITEMS}-item limit"
                )));
            }
        }

        let mut schemas = BTreeSet::new();
        for schema in &config.evidence_schemas {
            require_non_empty("evidence schema id", &schema.schema_id)?;
            validate_probability("schema reliability", schema.reliability)?;
            validate_probability("schema precision", schema.precision)?;
            if !schemas.insert(schema.schema_id.clone()) {
                return Err(StorageError::InvalidPath(format!(
                    "duplicate evidence schema '{}'",
                    schema.schema_id
                )));
            }
        }
        if schemas.is_empty() {
            return Err(StorageError::InvalidPath(
                "at least one evidence schema is required".to_string(),
            ));
        }

        for mapping in &config.source_mappings {
            require_non_empty("source mapping id", &mapping.mapping_id)?;
            require_non_empty("source kind", &mapping.source_kind)?;
            require_non_empty("mapping subject", &mapping.subject_from)?;
            require_non_empty("mapping value field", &mapping.value_field)?;
            require_non_empty("mapping factor id", &mapping.factor_id)?;
            if !schemas.contains(&mapping.evidence_schema_id) {
                return Err(StorageError::InvalidPath(format!(
                    "unknown mapping schema '{}'",
                    mapping.evidence_schema_id
                )));
            }
        }
        if config.source_mappings.is_empty() {
            return Err(StorageError::InvalidPath(
                "at least one source mapping is required".to_string(),
            ));
        }

        for factor in &config.comparator.factors {
            validate_factor(factor, &schemas)?;
        }
        if config.comparator.factors.is_empty() {
            return Err(StorageError::InvalidPath(
                "at least one comparator factor is required".to_string(),
            ));
        }

        Ok(())
    }
}

fn validate_factor(
    factor: &ComparatorFactorConfig,
    schemas: &BTreeSet<String>,
) -> Result<(), StorageError> {
    require_non_empty("factor id", &factor.factor_id)?;
    if !schemas.contains(&factor.evidence_schema_id) {
        return Err(StorageError::InvalidPath(format!(
            "unknown factor schema '{}'",
            factor.evidence_schema_id
        )));
    }
    if !factor.weight.is_finite() || factor.weight < 0.0 {
        return Err(StorageError::InvalidPath(
            "factor weight must be finite and non-negative".to_string(),
        ));
    }
    Ok(())
}

pub(crate) fn stable_hash_hex(bytes: &[u8]) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

fn to_storage_data(err: serde_json::Error) -> StorageError {
    StorageError::IoError(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        err.to_string(),
    ))
}
