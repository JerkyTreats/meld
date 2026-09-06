//! Selection of a standing rule through its declared producer.

use std::sync::Arc;

use super::{CurationAuthority, StandingCurationRuleRevision};
use crate::error::StorageError;
use crate::waiting::{StructuralWakeAddress, WaitingOnDeclaration};

pub enum CurationRuleSelection {
    Selected(Box<StandingCurationRuleRevision>),
    Waiting(WaitingOnDeclaration),
}

/// Read one exact rule, or identify the durable producer position still needed.
pub trait CurationRuleSelectionPort: Send + Sync {
    fn select(&self, authority: &CurationAuthority) -> Result<CurationRuleSelection, StorageError>;
    fn binding_refs(&self) -> Result<Vec<String>, StorageError>;
    fn template_refs(&self) -> Result<Vec<crate::belief::TheoryRevisionRef>, StorageError>;
    fn resolves_wake(&self, wake: &StructuralWakeAddress) -> Result<bool, String>;
}

#[derive(Clone)]
pub enum CurationRuleSource {
    Installed(Box<StandingCurationRuleRevision>),
    Producer(Arc<dyn CurationRuleSelectionPort>),
}

impl From<StandingCurationRuleRevision> for CurationRuleSource {
    fn from(rule: StandingCurationRuleRevision) -> Self {
        Self::Installed(Box::new(rule))
    }
}

impl CurationRuleSource {
    pub fn select(
        &self,
        authority: &CurationAuthority,
    ) -> Result<CurationRuleSelection, StorageError> {
        match self {
            Self::Installed(rule) => Ok(CurationRuleSelection::Selected(rule.clone())),
            Self::Producer(port) => port.select(authority),
        }
    }

    pub fn binding_refs(&self) -> Result<Vec<String>, StorageError> {
        match self {
            Self::Installed(rule) => {
                rule.validate()?;
                Ok(vec![format!("{}::{}", rule.rule_id, rule.content_hash)])
            }
            Self::Producer(port) => port.binding_refs(),
        }
    }
}
