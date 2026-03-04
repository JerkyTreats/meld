//! Frame metadata domain types.

use crate::metadata::frame_key_registry::{
    is_key_visible_by_default,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::ops::{Deref, DerefMut};

/// Frame metadata contract type.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct FrameMetadata(HashMap<String, String>);

impl FrameMetadata {
    pub fn new() -> Self {
        Self(HashMap::new())
    }
}

impl From<HashMap<String, String>> for FrameMetadata {
    fn from(value: HashMap<String, String>) -> Self {
        Self(value)
    }
}

impl From<FrameMetadata> for HashMap<String, String> {
    fn from(value: FrameMetadata) -> Self {
        value.0
    }
}

impl FromIterator<(String, String)> for FrameMetadata {
    fn from_iter<T: IntoIterator<Item = (String, String)>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl Deref for FrameMetadata {
    type Target = HashMap<String, String>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for FrameMetadata {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl IntoIterator for FrameMetadata {
    type Item = (String, String);
    type IntoIter = std::collections::hash_map::IntoIter<String, String>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a FrameMetadata {
    type Item = (&'a String, &'a String);
    type IntoIter = std::collections::hash_map::Iter<'a, String, String>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<'a> IntoIterator for &'a mut FrameMetadata {
    type Item = (&'a String, &'a mut String);
    type IntoIter = std::collections::hash_map::IterMut<'a, String, String>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter_mut()
    }
}

pub type VisibleFrameMetadata = BTreeMap<String, String>;

/// Metadata projection policy for read surfaces.
pub fn project_visible_metadata(metadata: &FrameMetadata) -> VisibleFrameMetadata {
    metadata
        .iter()
        .filter(|(key, _)| is_key_visible_by_default(key))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::profile::metadata_types::AgentMetadata;
    use crate::metadata::frame_key_registry::{
        FORBIDDEN_KEY_RAW_PROMPT, KEY_AGENT_ID, KEY_DELETED, KEY_PROMPT, KEY_PROMPT_DIGEST,
    };
    use crate::store::node_metadata::NodeMetadata;
    use std::any::TypeId;

    #[test]
    fn metadata_domain_types_are_distinct() {
        assert_ne!(TypeId::of::<FrameMetadata>(), TypeId::of::<NodeMetadata>());
        assert_ne!(TypeId::of::<FrameMetadata>(), TypeId::of::<AgentMetadata>());
        assert_ne!(TypeId::of::<NodeMetadata>(), TypeId::of::<AgentMetadata>());
    }

    #[test]
    fn projection_is_filtered_and_ordered() {
        let mut metadata = FrameMetadata::new();
        metadata.insert("z_key".to_string(), "z".to_string());
        metadata.insert(KEY_AGENT_ID.to_string(), "writer".to_string());
        metadata.insert("a_key".to_string(), "a".to_string());
        metadata.insert(KEY_DELETED.to_string(), "true".to_string());
        metadata.insert(KEY_PROMPT.to_string(), "raw prompt".to_string());
        metadata.insert(FORBIDDEN_KEY_RAW_PROMPT.to_string(), "raw payload".to_string());
        metadata.insert(KEY_PROMPT_DIGEST.to_string(), "digest".to_string());

        let projected = project_visible_metadata(&metadata);
        let keys: Vec<&str> = projected.keys().map(String::as_str).collect();
        assert_eq!(keys, vec!["prompt_digest"]);
        assert!(!projected.contains_key(KEY_AGENT_ID));
        assert!(!projected.contains_key(KEY_DELETED));
        assert!(!projected.contains_key(KEY_PROMPT));
        assert!(!projected.contains_key(FORBIDDEN_KEY_RAW_PROMPT));
    }
}
