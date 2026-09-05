//! Physical, non-secret PDS activation identity.

use std::collections::BTreeMap;

use meld_execution::capability::CapabilityContractRevisionRef;
use serde::{Deserialize, Serialize};

use crate::error::ApiError;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PhysicalBindingRef {
    WorkspaceRef(String),
    ProviderRef(String),
    CredentialRef(String),
    EndpointRef(String),
    ExecutableRef(String),
    ConfigRef(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdapterPlacement {
    InProcess,
    SerializedLocal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct RuntimeIsolationRequirements {
    pub separate_process: bool,
    pub network_denied: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct OperationalLimits {
    pub max_in_flight: u32,
    pub step_budget: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StewardshipActivationV1 {
    pub activation_id: String,
    pub assignment_id: String,
    pub bindings: BTreeMap<String, PhysicalBindingRef>,
    #[serde(with = "selected_implementation_map")]
    pub selected_implementations: BTreeMap<CapabilityContractRevisionRef, String>,
    pub placement: AdapterPlacement,
    pub isolation_requirements: RuntimeIsolationRequirements,
    pub operational_limits: OperationalLimits,
}

mod selected_implementation_map {
    use std::collections::BTreeMap;

    use meld_execution::capability::CapabilityContractRevisionRef;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(
        value: &BTreeMap<CapabilityContractRevisionRef, String>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        value.iter().collect::<Vec<_>>().serialize(serializer)
    }

    pub fn deserialize<'de, D>(
        deserializer: D,
    ) -> Result<BTreeMap<CapabilityContractRevisionRef, String>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let entries = Vec::<(CapabilityContractRevisionRef, String)>::deserialize(deserializer)?;
        let expected = entries.len();
        let map = entries.into_iter().collect::<BTreeMap<_, _>>();
        if map.len() != expected {
            return Err(serde::de::Error::custom(
                "duplicate exact capability implementation selection",
            ));
        }
        Ok(map)
    }
}

#[derive(Serialize)]
struct ActivationIdentity<'a> {
    assignment_id: &'a str,
    bindings: &'a BTreeMap<String, PhysicalBindingRef>,
    selected_implementations: Vec<(&'a CapabilityContractRevisionRef, &'a str)>,
    placement: &'a AdapterPlacement,
    isolation_requirements: &'a RuntimeIsolationRequirements,
    operational_limits: &'a OperationalLimits,
}

impl StewardshipActivationV1 {
    pub fn new(
        assignment_id: String,
        bindings: BTreeMap<String, PhysicalBindingRef>,
        selected_implementations: BTreeMap<CapabilityContractRevisionRef, String>,
        placement: AdapterPlacement,
        isolation_requirements: RuntimeIsolationRequirements,
        operational_limits: OperationalLimits,
    ) -> Result<Self, ApiError> {
        if assignment_id.trim().is_empty()
            || bindings.keys().any(|key| key.trim().is_empty())
            || selected_implementations
                .values()
                .any(|value| value.trim().is_empty())
        {
            return Err(ApiError::ConfigError(
                "PDS activation identity fields must be non-empty".to_string(),
            ));
        }
        let selected = selected_implementations
            .iter()
            .map(|(key, value)| (key, value.as_str()))
            .collect();
        let identity = ActivationIdentity {
            assignment_id: &assignment_id,
            bindings: &bindings,
            selected_implementations: selected,
            placement: &placement,
            isolation_requirements: &isolation_requirements,
            operational_limits: &operational_limits,
        };
        let activation_id = hash(&identity)?;
        Ok(Self {
            activation_id,
            assignment_id,
            bindings,
            selected_implementations,
            placement,
            isolation_requirements,
            operational_limits,
        })
    }

    pub fn verify_identity(&self) -> Result<(), ApiError> {
        let selected = self
            .selected_implementations
            .iter()
            .map(|(key, value)| (key, value.as_str()))
            .collect();
        let identity = ActivationIdentity {
            assignment_id: &self.assignment_id,
            bindings: &self.bindings,
            selected_implementations: selected,
            placement: &self.placement,
            isolation_requirements: &self.isolation_requirements,
            operational_limits: &self.operational_limits,
        };
        if hash(&identity)? != self.activation_id {
            return Err(ApiError::ConfigError(
                "PDS activation identity mismatch".to_string(),
            ));
        }
        Ok(())
    }
}

fn hash(value: &impl Serialize) -> Result<String, ApiError> {
    serde_json::to_vec(value)
        .map(|bytes| blake3::hash(&bytes).to_hex().to_string())
        .map_err(|failure| ApiError::ConfigError(failure.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_excludes_secret_values() {
        let activation = StewardshipActivationV1::new(
            "assignment".into(),
            BTreeMap::from([(
                "credential".into(),
                PhysicalBindingRef::CredentialRef("credential-revision-7".into()),
            )]),
            BTreeMap::new(),
            AdapterPlacement::InProcess,
            RuntimeIsolationRequirements::default(),
            OperationalLimits::default(),
        )
        .unwrap();
        assert!(!serde_json::to_string(&activation)
            .unwrap()
            .contains("secret"));
        activation.verify_identity().unwrap();
    }

    #[test]
    fn binding_change_requires_a_new_activation_identity() {
        let build = |endpoint: &str| {
            StewardshipActivationV1::new(
                "assignment".into(),
                BTreeMap::from([(
                    "endpoint".into(),
                    PhysicalBindingRef::EndpointRef(endpoint.into()),
                )]),
                BTreeMap::new(),
                AdapterPlacement::SerializedLocal,
                RuntimeIsolationRequirements::default(),
                OperationalLimits::default(),
            )
            .unwrap()
        };
        assert_ne!(
            build("endpoint-r1").activation_id,
            build("endpoint-r2").activation_id
        );
    }
}
