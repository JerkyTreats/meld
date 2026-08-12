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
use crate::belief::outcome::mapping::{
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

/// Subject binding for one mapped publication outcome.
///
/// `object_kind` and `domain_id` always constrain the bound subject. The
/// `from` source decides where the subject object is read: the envelope's
/// referenced objects, or the publication payload itself. Payload sources
/// exist because canonical execution publications do not always reference
/// the evidence subject as an envelope object: the per-task publication
/// envelope carries no workspace-node object (its folder target lives in
/// the outcome payload's task events), and the aggregate envelope names the
/// selected scope and every folder under the same object kind, so only the
/// payload's own `selected_scope` declaration is unambiguous.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OutcomeSubjectBinding {
    /// Required object kind of the bound subject.
    pub object_kind: String,
    /// Optional owning-domain filter for the subject object. Required when
    /// the subject id is minted from payload text.
    #[serde(default)]
    pub domain_id: Option<String>,
    /// Where the subject object is read from. Absent in older configs,
    /// defaulting to envelope-object binding.
    #[serde(default)]
    pub from: OutcomeSubjectSource,
}

/// Source location of the subject object for one mapping.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(tag = "source", rename_all = "snake_case")]
pub enum OutcomeSubjectSource {
    /// Bind the unique envelope object matching the binding filters.
    #[default]
    EnvelopeObjects,
    /// Read a complete domain object reference from the payload and require
    /// it to satisfy the binding filters. Used for the aggregate outcome's
    /// own `selected_scope` declaration, which execution carries precisely
    /// so world-model mapping needs no caller reattachment.
    PayloadObject {
        /// JSON pointer to a `{domain_id, object_kind, object_id}` value.
        pointer: String,
    },
    /// Mint the subject from payload text as the object id, with the domain
    /// and kind supplied by the binding. Used for the per-task publication's
    /// folder target, which the payload carries only as a node id.
    PayloadObjectId {
        /// JSON pointer to the object id text.
        pointer: String,
    },
    /// Select one payload-array element by a configured field value, then
    /// mint the subject from text within that element. This supports
    /// canonical outcomes that carry input and output artifacts in one
    /// ordered array without assigning semantic meaning to array position.
    PayloadArrayElementObjectId {
        /// JSON pointer to the payload array.
        array_pointer: String,
        /// Field inspected on each array element.
        field: String,
        /// Required string value on exactly one element.
        equals: String,
        /// JSON pointer within the selected element to the object id text.
        pointer: String,
    },
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
    /// Finite scalar read from one uniquely selected payload-array element.
    DataArrayElementScalar {
        /// JSON pointer to the payload array.
        array_pointer: String,
        /// Field inspected on each array element.
        field: String,
        /// Required string value on exactly one element.
        equals: String,
        /// JSON pointer within the selected element to the scalar.
        pointer: String,
    },
    /// Text read from one uniquely selected payload-array element.
    DataArrayElementText {
        /// JSON pointer to the payload array.
        array_pointer: String,
        /// Field inspected on each array element.
        field: String,
        /// Required string value on exactly one element.
        equals: String,
        /// JSON pointer within the selected element to the text.
        pointer: String,
    },
    /// Fixed domain object reference carried by the installed mapping
    /// theory, promoted as a map value.
    ///
    /// This exists so a mapping can attach installed theory context that the
    /// matched publication does not carry: the per-task publication names
    /// its folder but not the selected-tree scope, and the requirements gate
    /// binds per-folder facts to one derived selected-tree belief, so the
    /// interpretation theory itself must declare which selected-tree
    /// question folder outputs bear on. Belief family source mappings read
    /// the value through their `field:` subject binding.
    ConstantObject {
        /// Domain that owns the referenced object.
        domain_id: String,
        /// Object kind within the owning domain.
        object_kind: String,
        /// Domain-local object identifier.
        object_id: String,
    },
}

/// Installed set of mapping configurations under one mapping identity.
///
/// A stewardship expression selects exactly one evidence mapping identity,
/// but one expression's theory interprets several canonical outcome shapes
/// — per-task publications plus completed and failed package aggregates —
/// so the installed unit is a set of match rules. The set identity, not the
/// per-rule label, enters every promoted evidence identity, keeping the
/// frozen identity contract a function of the selected mapping.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OutcomeMappingSetConfig {
    /// Installed mapping identity cited in evidence identity derivation.
    pub mapping_id: String,
    /// Match rules tried in authored order; the first whose domain, event
    /// type, and content rules all hold interprets the record.
    pub rules: Vec<OutcomeMappingConfig>,
}

/// Validated, config-driven [`OutcomeEvidenceMapping`] implementor.
pub struct ConfiguredOutcomeMapping {
    config: OutcomeMappingConfig,
}

impl ConfiguredOutcomeMapping {
    /// Validate and adopt one installed mapping configuration.
    pub fn new(config: OutcomeMappingConfig) -> Result<Self, StorageError> {
        validate_mapping_config(&config)?;
        Ok(Self { config })
    }

    /// Installed mapping identity this implementor was built from.
    pub fn mapping_id(&self) -> &str {
        &self.config.mapping_id
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
        if let Err(reason) = match_config(&self.config, envelope) {
            return OutcomeMappingDisposition::NotApplicable { reason };
        }
        promote_matched_record(&self.config, input)
    }
}

/// Validated, config-driven mapping set over one installed identity.
///
/// Rule order is theory content: matching is first-match in authored order,
/// so replaying a record through the same installed set always selects the
/// same rule and derives the same evidence.
pub struct ConfiguredOutcomeMappingSet {
    config: OutcomeMappingSetConfig,
}

impl ConfiguredOutcomeMappingSet {
    /// Validate and adopt one installed mapping set configuration.
    pub fn new(config: OutcomeMappingSetConfig) -> Result<Self, StorageError> {
        require_non_empty("mapping set id", &config.mapping_id)?;
        if config.rules.is_empty() {
            return Err(StorageError::InvalidPath(
                "mapping set requires at least one rule".to_string(),
            ));
        }
        let mut rule_ids = BTreeSet::new();
        for rule in &config.rules {
            validate_mapping_config(rule)?;
            // Rule ids are audit labels inside the set; duplicates would make
            // rejection and provenance text ambiguous.
            if !rule_ids.insert(rule.mapping_id.clone()) {
                return Err(StorageError::InvalidPath(format!(
                    "duplicate mapping set rule '{}'",
                    rule.mapping_id
                )));
            }
        }
        Ok(Self { config })
    }

    /// Installed mapping identity this implementor was built from.
    pub fn mapping_id(&self) -> &str {
        &self.config.mapping_id
    }
}

impl OutcomeEvidenceMapping for ConfiguredOutcomeMappingSet {
    fn map_outcome(&self, input: &OutcomeMappingInput) -> OutcomeMappingDisposition {
        // Same install/selection guard as the single-config implementor: the
        // identity entering evidence derivation must be the selected one.
        if input.mapping_id != self.config.mapping_id {
            return OutcomeMappingDisposition::Invalid {
                reason: format!(
                    "selected mapping '{}' does not match installed mapping '{}'",
                    input.mapping_id, self.config.mapping_id
                ),
            };
        }
        let envelope = input.record.envelope();
        let mut reasons = Vec::new();
        for rule in &self.config.rules {
            match match_config(rule, envelope) {
                // The first matching rule owns the record; unusable matched
                // content is Invalid, never a fall-through to a later rule,
                // so replay cannot reinterpret the record differently.
                Ok(()) => return promote_matched_record(rule, input),
                Err(reason) => reasons.push(format!("{}: {reason}", rule.mapping_id)),
            }
        }
        OutcomeMappingDisposition::NotApplicable {
            reason: format!("no mapping set rule matched ({})", reasons.join("; ")),
        }
    }
}

/// Validate one mapping configuration's semantic content.
fn validate_mapping_config(config: &OutcomeMappingConfig) -> Result<(), StorageError> {
    require_non_empty("mapping id", &config.mapping_id)?;
    require_non_empty("source kind", &config.source_kind)?;
    require_non_empty("match domain id", &config.match_domain_id)?;
    require_non_empty("match event type", &config.match_event_type)?;
    require_non_empty("subject object kind", &config.subject.object_kind)?;
    if let Some(domain_id) = &config.subject.domain_id {
        require_non_empty("subject domain id", domain_id)?;
    }
    match &config.subject.from {
        OutcomeSubjectSource::EnvelopeObjects => {}
        OutcomeSubjectSource::PayloadObject { pointer } => {
            require_non_empty("subject payload pointer", pointer)?;
        }
        OutcomeSubjectSource::PayloadObjectId { pointer } => {
            require_non_empty("subject payload pointer", pointer)?;
            // Payload text supplies only the object id; the binding must own
            // the other two subject coordinates completely.
            if config.subject.domain_id.is_none() {
                return Err(StorageError::InvalidPath(
                    "payload object id subject binding requires a subject domain id".to_string(),
                ));
            }
        }
        OutcomeSubjectSource::PayloadArrayElementObjectId {
            array_pointer,
            field,
            equals,
            pointer,
        } => {
            validate_array_element_selector(
                "subject payload",
                array_pointer,
                field,
                equals,
                pointer,
            )?;
            if config.subject.domain_id.is_none() {
                return Err(StorageError::InvalidPath(
                    "payload array element object id subject binding requires a subject domain id"
                        .to_string(),
                ));
            }
        }
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
            OutcomeValueSource::DataArrayElementScalar {
                array_pointer,
                field,
                equals,
                pointer,
            }
            | OutcomeValueSource::DataArrayElementText {
                array_pointer,
                field,
                equals,
                pointer,
            } => validate_array_element_selector(
                "evidence field",
                array_pointer,
                field,
                equals,
                pointer,
            )?,
            OutcomeValueSource::ConstantObject {
                domain_id,
                object_kind,
                object_id,
            } => {
                // Reuse the object-reference validation so the installed
                // constant cannot name an unconstructible subject.
                DomainObjectRef::new(domain_id.clone(), object_kind.clone(), object_id.clone())?;
            }
        }
    }
    Ok(())
}

fn validate_array_element_selector(
    label: &str,
    array_pointer: &str,
    field: &str,
    equals: &str,
    pointer: &str,
) -> Result<(), StorageError> {
    require_non_empty(&format!("{label} array pointer"), array_pointer)?;
    require_non_empty(&format!("{label} selector field"), field)?;
    require_non_empty(&format!("{label} selector value"), equals)?;
    require_non_empty(&format!("{label} value pointer"), pointer)?;
    Ok(())
}

/// Decide whether one config's domain, event type, and content rules match.
fn match_config(config: &OutcomeMappingConfig, envelope: &EventEnvelope) -> Result<(), String> {
    if envelope.domain_id != config.match_domain_id
        || envelope.event_type != config.match_event_type
    {
        return Err(format!(
            "record {}/{} does not match mapped outcome {}/{}",
            envelope.domain_id,
            envelope.event_type,
            config.match_domain_id,
            config.match_event_type
        ));
    }
    for rule in &config.match_content {
        if !content_rule_matches(rule, &envelope.data) {
            return Err("record payload does not satisfy mapping content rules".to_string());
        }
    }
    Ok(())
}

/// Promote one already-matched record under the selected mapping identity.
///
/// Anything unusable from here on is Invalid so the rejection is recorded
/// durably before the cursor may pass the record. The evidence identity is
/// always derived from the selected mapping identity — the set identity when
/// a set rule matched — keeping identity a function of what the stewardship
/// expression installed.
fn promote_matched_record(
    config: &OutcomeMappingConfig,
    input: &OutcomeMappingInput,
) -> OutcomeMappingDisposition {
    let envelope = input.record.envelope();
    let Some(record_id) = envelope.record_id.as_deref() else {
        return OutcomeMappingDisposition::Invalid {
            reason: "matched publication carries no record_id, so evidence identity \
                     would not be replay-stable"
                .to_string(),
        };
    };
    let subject = match bind_subject(config, envelope) {
        Ok(subject) => subject,
        Err(reason) => return OutcomeMappingDisposition::Invalid { reason },
    };
    let mut fields = BTreeMap::new();
    for rule in &config.evidence_fields {
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
        source_kind: config.source_kind.clone(),
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

/// Bind the evidence subject for one matched record.
fn bind_subject(
    config: &OutcomeMappingConfig,
    envelope: &EventEnvelope,
) -> Result<DomainObjectRef, String> {
    let binding = &config.subject;
    let subject = match &binding.from {
        OutcomeSubjectSource::EnvelopeObjects => {
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
            // Ambiguity is unusable content, not a guess: two candidate
            // subjects would let replay bind evidence to whichever came
            // first.
            if candidates.next().is_some() {
                return Err(format!(
                    "multiple envelope objects match subject binding kind '{}'",
                    binding.object_kind
                ));
            }
            subject.clone()
        }
        OutcomeSubjectSource::PayloadObject { pointer } => {
            let Some(value) = envelope.data.pointer(pointer) else {
                return Err(format!("payload pointer '{pointer}' has no subject object"));
            };
            let subject: DomainObjectRef = serde_json::from_value(value.clone())
                .map_err(|error| format!("payload subject at '{pointer}' is invalid: {error}"))?;
            // The binding filters are a contract on the payload declaration,
            // not a search: a drifted payload subject is unusable content.
            if subject.object_kind != binding.object_kind
                || binding
                    .domain_id
                    .as_deref()
                    .is_some_and(|domain_id| subject.domain_id != domain_id)
            {
                return Err(format!(
                    "payload subject at '{pointer}' does not satisfy the subject binding"
                ));
            }
            subject
        }
        OutcomeSubjectSource::PayloadObjectId { pointer } => {
            let Some(object_id) = envelope.data.pointer(pointer).and_then(Value::as_str) else {
                return Err(format!(
                    "payload pointer '{pointer}' is not subject id text"
                ));
            };
            // Validated at install time: this source requires a domain id.
            let domain_id = binding
                .domain_id
                .as_deref()
                .ok_or_else(|| "subject binding is missing a domain id".to_string())?;
            DomainObjectRef::new(domain_id, binding.object_kind.clone(), object_id)
                .map_err(|error| format!("payload subject id at '{pointer}': {error}"))?
        }
        OutcomeSubjectSource::PayloadArrayElementObjectId {
            array_pointer,
            field,
            equals,
            pointer,
        } => {
            let element =
                select_unique_array_element(&envelope.data, array_pointer, field, equals)?;
            let Some(object_id) = element.pointer(pointer).and_then(Value::as_str) else {
                return Err(format!(
                    "selected payload element pointer '{pointer}' is not subject id text"
                ));
            };
            let domain_id = binding
                .domain_id
                .as_deref()
                .ok_or_else(|| "subject binding is missing a domain id".to_string())?;
            DomainObjectRef::new(domain_id, binding.object_kind.clone(), object_id).map_err(
                |error| format!("selected payload element subject id at '{pointer}': {error}"),
            )?
        }
    };
    subject
        .validate()
        .map_err(|error| format!("subject object is invalid: {error}"))?;
    Ok(subject)
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
        OutcomeValueSource::DataArrayElementScalar {
            array_pointer,
            field,
            equals,
            pointer,
        } => select_unique_array_element(data, array_pointer, field, equals)?
            .pointer(pointer)
            .and_then(Value::as_f64)
            .filter(|value| value.is_finite())
            .map(EvidenceValue::Scalar)
            .ok_or_else(|| {
                format!("selected payload element pointer '{pointer}' is not a finite scalar")
            }),
        OutcomeValueSource::DataArrayElementText {
            array_pointer,
            field,
            equals,
            pointer,
        } => select_unique_array_element(data, array_pointer, field, equals)?
            .pointer(pointer)
            .and_then(Value::as_str)
            .map(|value| EvidenceValue::Text(value.to_string()))
            .ok_or_else(|| format!("selected payload element pointer '{pointer}' is not text")),
        OutcomeValueSource::ConstantObject {
            domain_id,
            object_kind,
            object_id,
        } => {
            // The key names mirror `DomainObjectRef` so the evidence
            // normalizer's `field:` subject binding can rebuild the
            // reference from the map without a translation table.
            let mut parts = BTreeMap::new();
            parts.insert("domain_id".to_string(), domain_id.clone());
            parts.insert("object_kind".to_string(), object_kind.clone());
            parts.insert("object_id".to_string(), object_id.clone());
            Ok(EvidenceValue::Map(parts))
        }
    }
}

fn select_unique_array_element<'a>(
    data: &'a Value,
    array_pointer: &str,
    field: &str,
    equals: &str,
) -> Result<&'a Value, String> {
    let elements = data
        .pointer(array_pointer)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("payload pointer '{array_pointer}' is not an array"))?;
    let mut matches = elements
        .iter()
        .filter(|element| element.get(field).and_then(Value::as_str) == Some(equals));
    let selected = matches.next().ok_or_else(|| {
        format!("payload array '{array_pointer}' has no element where '{field}' equals '{equals}'")
    })?;
    if matches.next().is_some() {
        return Err(format!(
            "payload array '{array_pointer}' has multiple elements where '{field}' equals '{equals}'"
        ));
    }
    Ok(selected)
}
