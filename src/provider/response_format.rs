//! Emit response-schema properties in their declared required-field order.
//!
//! Grammar-backed providers use property order as generation order. Keep that
//! transport choice local: changing serde_json map order globally would change
//! durable product identities throughout the runtime.

use serde::ser::{SerializeMap, SerializeSeq};
use serde::{Serialize, Serializer};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn serialize_extra_fields<S: Serializer>(
    fields: &BTreeMap<String, Value>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    let mut map = serializer.serialize_map(Some(fields.len()))?;
    for (key, value) in fields {
        if key == "response_format" {
            map.serialize_entry(key, &Schema(value))?;
        } else {
            map.serialize_entry(key, value)?;
        }
    }
    map.end()
}

struct Schema<'a>(&'a Value);

impl Serialize for Schema<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self.0 {
            Value::Object(fields) => {
                let mut map = serializer.serialize_map(Some(fields.len()))?;
                for (key, value) in fields {
                    if key == "properties" && value.is_object() {
                        map.serialize_entry(
                            key,
                            &Properties {
                                fields: value.as_object().expect("checked object"),
                                required: fields.get("required").and_then(Value::as_array),
                            },
                        )?;
                    } else {
                        map.serialize_entry(key, &Schema(value))?;
                    }
                }
                map.end()
            }
            Value::Array(values) => {
                let mut sequence = serializer.serialize_seq(Some(values.len()))?;
                for value in values {
                    sequence.serialize_element(&Schema(value))?;
                }
                sequence.end()
            }
            scalar => scalar.serialize(serializer),
        }
    }
}

struct Properties<'a> {
    fields: &'a serde_json::Map<String, Value>,
    required: Option<&'a Vec<Value>>,
}

impl Serialize for Properties<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.fields.len()))?;
        let mut emitted = BTreeSet::new();
        for key in self
            .required
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
        {
            if let Some(value) = self.fields.get(key) {
                if emitted.insert(key) {
                    map.serialize_entry(key, &Schema(value))?;
                }
            }
        }
        for (key, value) in self.fields {
            if !emitted.contains(key.as_str()) {
                map.serialize_entry(key, &Schema(value))?;
            }
        }
        map.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wire_schema_order_preserves_declared_decision_order_without_changing_json_values() {
        let format = serde_json::json!({"type":"json_schema","json_schema":{"schema":{
            "type":"object", "properties":{
                "citations":{"type":"array"}, "verdict":{"type":"string"}
            }, "required":["verdict","citations"]
        }}});
        let request = crate::provider::ChatCompletionRequest {
            model: "local".into(),
            messages: vec![],
            temperature: None,
            max_tokens: None,
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop: None,
            stream: false,
            additional_json: BTreeMap::from([("response_format".into(), format.clone())]),
        };
        let wire = serde_json::to_string(&request).unwrap();
        assert!(wire.find("\"verdict\":").unwrap() < wire.find("\"citations\":").unwrap());
        let decoded: Value = serde_json::from_str(&wire).unwrap();
        assert_eq!(decoded["response_format"], format);
        let canonical = serde_json::to_string(&format).unwrap();
        assert!(
            canonical.find("\"citations\":").unwrap() < canonical.find("\"verdict\":").unwrap()
        );
    }
}
