//! Config-driven interpretation of canonical publication outcomes.
//!
//! Owner: world model belief domain. This is the first implementor of the
//! frozen [`OutcomeEvidenceMapping`] contract. All outcome vocabulary —
//! matched domain, event type, payload shape, subject binding, and evidence
//! field values — arrives as installed mapping configuration data, never as
//! code: docs-freshness specifics exist only in configuration fixtures.
//!
//! Identity invariant: an applicable record's evidence identity is always
//! the frozen [`promoted_evidence_identity`] over the canonical publication
//! record id and the installed mapping identity. The same identity is
//! written into the promoted record's `source_id`, so downstream evidence
//! normalization derives every per-schema evidence id deterministically
//! from it and reopen replay dedupes on the frozen identity transitively.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::belief::contracts::{require_non_empty, EvidenceValue, PromotedEvidenceRecord};
use crate::belief::outcome_mapping::{
    promoted_evidence_identity, OutcomeEvidenceMapping, OutcomeMappingDisposition,
    OutcomeMappingInput,
};
use crate::error::StorageError;
use crate::events::{DomainObjectRef, EventEnvelope};

/// Installed mapping configuration for one publication-outcome shape.
///
/// The configuration is theory content: it is installed and selected by
/// identity, and its identity enters every promoted evidence identity it
/// produces.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OutcomeMappingConfig {
    /// Installed mapping identity cited in evidence identity derivation.
    pub mapping_id: String,
    /// Promoted source kind consumed by belief family source mappings.
    pub source_kind: String,
    /// Domain that owns the matched publication meaning.
    pub match_domain_id: String,
    /// Event type this mapping recognizes.
    pub match_event_type: String,
    /// Payload content rules that must all hold for a record to match.
    #[serde(default)]
    pub match_content: Vec<OutcomeContentRule>,
    /// How the evidence subject is bound from envelope objects.
    pub subject: OutcomeSubjectBinding,
    /// Evidence field extraction rules; at least one is required.
    pub evidence_fields: Vec<OutcomeFieldRule>,
}

/// Content predicate over the publication payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "rule", rename_all = "snake_case")]
pub enum OutcomeContentRule {
    /// A JSON-pointer field must equal a configured string.
    FieldEquals {
        /// JSON pointer into the envelope payload.
        pointer: String,
        /// Required string value at the pointer.
        equals: String,
    },
    /// Some element of a JSON-pointer array must carry a field equal to a
    /// configured string.
    ArrayAnyFieldEquals {
        /// JSON pointer to the payload array.
        array_pointer: String,
        /// Field inspected on each array element.
        field: String,
        /// Required string value on at least one element.
        equals: String,
    },
}

/// Subject binding over the envelope's referenced domain objects.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OutcomeSubjectBinding {
    /// Required object kind of the subject among envelope objects.
    pub object_kind: String,
    /// Optional owning-domain filter for the subject object.
    #[serde(default)]
    pub domain_id: Option<String>,
}

/// One extracted evidence field on the promoted record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OutcomeFieldRule {
    /// Evidence field name consumed by belief family value mappings.
    pub field: String,
    /// Where the field value comes from.
    pub source: OutcomeValueSource,
}

/// Source of one evidence field value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "source", rename_all = "snake_case")]
pub enum OutcomeValueSource {
    /// Fixed value carried by the installed mapping theory itself.
    Constant {
        /// Finite scalar value.
        value: f64,
    },
    /// Finite scalar read from the publication payload.
    DataScalar {
        /// JSON pointer into the envelope payload.
        pointer: String,
    },
    /// Text read from the publication payload.
    DataText {
        /// JSON pointer into the envelope payload.
        pointer: String,
    },
}

/// Validated, config-driven [`OutcomeEvidenceMapping`] implementor.
pub struct ConfiguredOutcomeMapping {
    config: OutcomeMappingConfig,
}

impl ConfiguredOutcomeMapping {
    /// Validate and adopt one installed mapping configuration.
    pub fn new(config: OutcomeMappingConfig) -> Result<Self, StorageError> {
        require_non_empty("mapping id", &config.mapping_id)?;
        require_non_empty("source kind", &config.source_kind)?;
        require_non_empty("match domain id", &config.match_domain_id)?;
        require_non_empty("match event type", &config.match_event_type)?;
        require_non_empty("subject object kind", &config.subject.object_kind)?;
        if let Some(domain_id) = &config.subject.domain_id {
            require_non_empty("subject domain id", domain_id)?;
        }
        for rule in &config.match_content {
            match rule {
                OutcomeContentRule::FieldEquals { pointer, .. } => {
                    require_non_empty("content rule pointer", pointer)?;
                }
                OutcomeContentRule::ArrayAnyFieldEquals {
                    array_pointer,
                    field,
                    ..
                } => {
                    require_non_empty("content rule array pointer", array_pointer)?;
                    require_non_empty("content rule field", field)?;
                }
            }
        }
        if config.evidence_fields.is_empty() {
            return Err(StorageError::InvalidPath(
                "at least one evidence field rule is required".to_string(),
            ));
        }
        let mut fields = BTreeSet::new();
        for rule in &config.evidence_fields {
            require_non_empty("evidence field name", &rule.field)?;
            if !fields.insert(rule.field.clone()) {
                return Err(StorageError::InvalidPath(format!(
                    "duplicate evidence field '{}'",
                    rule.field
                )));
            }
            match &rule.source {
                OutcomeValueSource::Constant { value } => {
                    if !value.is_finite() {
                        return Err(StorageError::InvalidPath(format!(
                            "constant for evidence field '{}' must be finite",
                            rule.field
                        )));
                    }
                }
                OutcomeValueSource::DataScalar { pointer }
                | OutcomeValueSource::DataText { pointer } => {
                    require_non_empty("evidence field pointer", pointer)?;
                }
            }
        }
        Ok(Self { config })
    }

    /// Installed mapping identity this implementor was built from.
    pub fn mapping_id(&self) -> &str {
        &self.config.mapping_id
    }

    fn bind_subject(&self, envelope: &EventEnvelope) -> Result<DomainObjectRef, String> {
        let binding = &self.config.subject;
        let mut candidates = envelope.objects.iter().filter(|object| {
            object.object_kind == binding.object_kind
                && binding
                    .domain_id
                    .as_deref()
                    .is_none_or(|domain_id| object.domain_id == domain_id)
        });
        let Some(subject) = candidates.next() else {
            return Err(format!(
                "no envelope object matches subject binding kind '{}'",
                binding.object_kind
            ));
        };
        // Ambiguity is unusable content, not a guess: two candidate subjects
        // would let replay bind evidence to whichever came first.
        if candidates.next().is_some() {
            return Err(format!(
                "multiple envelope objects match subject binding kind '{}'",
                binding.object_kind
            ));
        }
        subject
            .validate()
            .map_err(|error| format!("subject object is invalid: {error}"))?;
        Ok(subject.clone())
    }
}

impl OutcomeEvidenceMapping for ConfiguredOutcomeMapping {
    fn map_outcome(&self, input: &OutcomeMappingInput) -> OutcomeMappingDisposition {
        // A selection/installation mismatch is recorded durably instead of
        // silently deriving identities under the wrong mapping name.
        if input.mapping_id != self.config.mapping_id {
            return OutcomeMappingDisposition::Invalid {
                reason: format!(
                    "selected mapping '{}' does not match installed mapping '{}'",
                    input.mapping_id, self.config.mapping_id
                ),
            };
        }
        let envelope = input.record.envelope();
        if envelope.domain_id != self.config.match_domain_id
            || envelope.event_type != self.config.match_event_type
        {
            return OutcomeMappingDisposition::NotApplicable {
                reason: format!(
                    "record {}/{} does not match mapped outcome {}/{}",
                    envelope.domain_id,
                    envelope.event_type,
                    self.config.match_domain_id,
                    self.config.match_event_type
                ),
            };
        }
        for rule in &self.config.match_content {
            if !content_rule_matches(rule, &envelope.data) {
                return OutcomeMappingDisposition::NotApplicable {
                    reason: "record payload does not satisfy mapping content rules".to_string(),
                };
            }
        }

        // The record matched the mapping; anything unusable from here on is
        // Invalid so the rejection is recorded durably before the cursor
        // may pass the record.
        let Some(record_id) = envelope.record_id.as_deref() else {
            return OutcomeMappingDisposition::Invalid {
                reason: "matched publication carries no record_id, so evidence identity \
                         would not be replay-stable"
                    .to_string(),
            };
        };
        let subject = match self.bind_subject(envelope) {
            Ok(subject) => subject,
            Err(reason) => return OutcomeMappingDisposition::Invalid { reason },
        };
        let mut fields = BTreeMap::new();
        for rule in &self.config.evidence_fields {
            match extract_value(&rule.source, &envelope.data) {
                Ok(value) => {
                    fields.insert(rule.field.clone(), value);
                }
                Err(reason) => {
                    return OutcomeMappingDisposition::Invalid {
                        reason: format!("evidence field '{}': {reason}", rule.field),
                    }
                }
            }
        }

        let evidence_id = promoted_evidence_identity(record_id, &input.mapping_id);
        let record = PromotedEvidenceRecord {
            source_kind: self.config.source_kind.clone(),
            // The frozen identity doubles as the promoted source id so the
            // evidence normalizer's per-schema ids derive from it and replay
            // dedupe keys on the frozen identity.
            source_id: evidence_id.clone(),
            subject,
            source_fact_ids: vec![format!("event-spine::{}", input.record.seq)],
            graph_anchor_ids: Vec::new(),
            objects: envelope.objects.clone(),
            relations: envelope.relations.clone(),
            source_cursor_start: input.record.seq,
            source_cursor_end: input.record.seq,
            reference_time: envelope
                .occurred_at
                .clone()
                .or_else(|| Some(envelope.recorded_at.clone())),
            transaction_seq: input.record.seq,
            content_hash: envelope.content_hash.clone(),
            fields,
        };
        OutcomeMappingDisposition::Applicable {
            evidence_id,
            record: Box::new(record),
        }
    }
}

fn content_rule_matches(rule: &OutcomeContentRule, data: &Value) -> bool {
    match rule {
        OutcomeContentRule::FieldEquals { pointer, equals } => data
            .pointer(pointer)
            .and_then(Value::as_str)
            .is_some_and(|value| value == equals),
        OutcomeContentRule::ArrayAnyFieldEquals {
            array_pointer,
            field,
            equals,
        } => data
            .pointer(array_pointer)
            .and_then(Value::as_array)
            .is_some_and(|elements| {
                elements.iter().any(|element| {
                    element.get(field).and_then(Value::as_str) == Some(equals.as_str())
                })
            }),
    }
}

fn extract_value(source: &OutcomeValueSource, data: &Value) -> Result<EvidenceValue, String> {
    match source {
        OutcomeValueSource::Constant { value } => Ok(EvidenceValue::Scalar(*value)),
        OutcomeValueSource::DataScalar { pointer } => data
            .pointer(pointer)
            .and_then(Value::as_f64)
            .filter(|value| value.is_finite())
            .map(EvidenceValue::Scalar)
            .ok_or_else(|| format!("payload pointer '{pointer}' is not a finite scalar")),
        OutcomeValueSource::DataText { pointer } => data
            .pointer(pointer)
            .and_then(Value::as_str)
            .map(|value| EvidenceValue::Text(value.to_string()))
            .ok_or_else(|| format!("payload pointer '{pointer}' is not text")),
    }
}
