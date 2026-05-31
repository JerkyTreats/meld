//! Method loading and verification for execution planning.

use crate::capability::CapabilityCatalog;
use crate::planning::contracts::{
    InvalidMethodReport, OperatorResolutionReport, OperatorResolutionStatus, PlanningDiagnostic,
    PlanningDiagnosticCode,
};
use meld_lang::{
    Composition, Condition, Effect, Method, Operator, Proposition, StepKind, Term, ValidationError,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use thiserror::Error;
use walkdir::WalkDir;

/// Verified method library plus invalid entries encountered during loading.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MethodLibrary {
    /// Entries owned by this execution contract.
    pub entries: Vec<VerifiedMethodEntry>,
    /// Invalid owned by this execution contract.
    pub invalid: Vec<InvalidMethodReport>,
}

impl MethodLibrary {
    /// Build a method library from in-memory methods.
    pub fn from_methods(methods: Vec<Method>, catalog: &CapabilityCatalog) -> Self {
        let methods = methods
            .into_iter()
            .enumerate()
            .map(|(index, method)| {
                (
                    method,
                    MethodSourceRef::InMemory {
                        label: format!("method-{index}"),
                    },
                )
            })
            .collect();
        build_library(methods, catalog)
    }

    /// Load JSON method files from a directory in deterministic path order.
    pub fn load_json_dir(
        path: impl AsRef<Path>,
        catalog: &CapabilityCatalog,
    ) -> Result<Self, MethodLibraryLoadError> {
        let mut files = Vec::new();
        for entry in WalkDir::new(path.as_ref()) {
            let entry = entry.map_err(|err| MethodLibraryLoadError::Walk(err.to_string()))?;
            if entry.file_type().is_file()
                && entry
                    .path()
                    .extension()
                    .and_then(|extension| extension.to_str())
                    == Some("json")
            {
                let canonical = entry
                    .path()
                    .canonicalize()
                    .unwrap_or_else(|_| entry.path().to_path_buf());
                files.push(canonical);
            }
        }
        files.sort();

        let mut methods = Vec::new();
        for file in files {
            let content = fs::read_to_string(&file).map_err(MethodLibraryLoadError::Io)?;
            let method =
                serde_json::from_str::<Method>(&content).map_err(MethodLibraryLoadError::Serde)?;
            methods.push((
                method,
                MethodSourceRef::File {
                    path: file.display().to_string(),
                },
            ));
        }

        Ok(build_library(methods, catalog))
    }

    /// Return verified entries in method candidate order.
    pub fn sorted_verified_entries(&self) -> Vec<&VerifiedMethodEntry> {
        let mut entries = self.entries.iter().collect::<Vec<_>>();
        entries.sort_by(|left, right| method_order(&left.method, &right.method));
        entries
    }
}

/// A method that passed reusable template verification.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VerifiedMethodEntry {
    /// Method owned by this execution contract.
    pub method: Method,
    /// Source reference owned by this execution contract.
    pub source_ref: MethodSourceRef,
    /// Verification owned by this execution contract.
    pub verification: MethodVerification,
}

/// Method source provenance used in diagnostics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MethodSourceRef {
    /// Method was supplied directly by the caller.
    InMemory {
        /// Caller supplied label used in diagnostics.
        label: String,
    },
    /// Method was loaded from a filesystem document.
    File {
        /// Source path used in diagnostics.
        path: String,
    },
}

impl MethodSourceRef {
    pub(crate) fn stable_ref(&self) -> String {
        match self {
            Self::InMemory { label } => format!("memory:{label}"),
            Self::File { path } => path.clone(),
        }
    }
}

/// Verification report attached to a reusable method.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MethodVerification {
    /// Diagnostics owned by this execution contract.
    pub diagnostics: Vec<MethodVerificationDiagnostic>,
    /// Operator resolutions owned by this execution contract.
    pub operator_resolutions: Vec<OperatorResolutionReport>,
}

/// Method verification diagnostic wrapper.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MethodVerificationDiagnostic {
    /// Diagnostic owned by this execution contract.
    pub diagnostic: PlanningDiagnostic,
}

/// Loading failure before individual method verification can run.
#[derive(Debug, Error)]
pub enum MethodLibraryLoadError {
    /// Walk variant for this execution contract.
    #[error("method directory walk failed: {0}")]
    Walk(String),
    /// Io variant for this execution contract.
    #[error("method file read failed: {0}")]
    Io(#[from] std::io::Error),
    /// Serde variant for this execution contract.
    #[error("method JSON decode failed: {0}")]
    Serde(#[from] serde_json::Error),
}

fn build_library(
    methods: Vec<(Method, MethodSourceRef)>,
    catalog: &CapabilityCatalog,
) -> MethodLibrary {
    let mut id_counts = BTreeMap::<String, usize>::new();
    for (method, _) in &methods {
        if !method.method_id.trim().is_empty() {
            *id_counts.entry(method.method_id.clone()).or_default() += 1;
        }
    }

    let mut entries = Vec::new();
    let mut invalid = Vec::new();

    for (method, source_ref) in methods {
        let method_id = (!method.method_id.trim().is_empty()).then(|| method.method_id.clone());
        if method_id.is_none() {
            invalid.push(invalid_report(
                &source_ref,
                None,
                vec![diagnostic(
                    PlanningDiagnosticCode::MethodIdMissing,
                    "method id must be non-empty",
                    None,
                )],
            ));
            continue;
        }
        if id_counts
            .get(method_id.as_ref().expect("checked present"))
            .copied()
            .unwrap_or_default()
            > 1
        {
            invalid.push(invalid_report(
                &source_ref,
                method_id.clone(),
                vec![diagnostic(
                    PlanningDiagnosticCode::MethodDuplicateId,
                    "method id appears more than once",
                    method_id.as_deref(),
                )],
            ));
            continue;
        }

        match verify_method(method, source_ref, catalog) {
            Ok(entry) => entries.push(entry),
            Err(report) => invalid.push(report),
        }
    }

    entries.sort_by(|left, right| method_order(&left.method, &right.method));
    MethodLibrary { entries, invalid }
}

fn verify_method(
    method: Method,
    source_ref: MethodSourceRef,
    catalog: &CapabilityCatalog,
) -> Result<VerifiedMethodEntry, InvalidMethodReport> {
    let mut diagnostics = Vec::new();
    let method_id = method.method_id.as_str();

    if let Some(derived) = derived_issue_in_proposition(&method.trigger) {
        diagnostics.push(diagnostic(
            PlanningDiagnosticCode::MethodTriggerDerived,
            format!("method trigger contains derived term {derived}"),
            Some(method_id),
        ));
    }

    let trigger_variables = variables_in_proposition(&method.trigger);
    for variable in variables_in_preconditions(&method.preconditions)
        .into_iter()
        .chain(variables_in_effects(&method.net_effects))
        .chain(variables_in_composition(&method.composition))
    {
        if !trigger_variables.contains(&variable) {
            diagnostics.push(diagnostic(
                PlanningDiagnosticCode::MethodVariableNotBoundByTrigger,
                format!("variable {variable} is not bound by method trigger"),
                Some(method_id),
            ));
        }
    }

    let validation = meld_lang::validate(&method.composition);
    for error in validation.errors {
        if matches!(
            &error,
            ValidationError::UnboundVariable { variable, .. }
                if trigger_variables.contains(variable)
        ) {
            continue;
        }
        diagnostics.push(diagnostic(
            PlanningDiagnosticCode::MethodTemplateInvalid,
            format!("composition template is invalid: {error:?}"),
            Some(method_id),
        ));
    }

    if !diagnostics.is_empty() {
        return Err(invalid_report(
            &source_ref,
            Some(method.method_id),
            diagnostics,
        ));
    }

    let operator_resolutions = operator_resolutions(&method.composition, catalog);
    let verification = MethodVerification {
        diagnostics: operator_resolutions
            .iter()
            .flat_map(|report| report.diagnostics.clone())
            .map(|diagnostic| MethodVerificationDiagnostic { diagnostic })
            .collect(),
        operator_resolutions,
    };

    Ok(VerifiedMethodEntry {
        method,
        source_ref,
        verification,
    })
}

pub(crate) fn operator_resolutions(
    composition: &Composition,
    catalog: &CapabilityCatalog,
) -> Vec<OperatorResolutionReport> {
    composition
        .steps
        .iter()
        .filter_map(|step| match &step.kind {
            StepKind::Op(operator) => Some(resolve_operator(operator, catalog)),
            StepKind::Goal(_) => None,
        })
        .collect()
}

fn resolve_operator(operator: &Operator, catalog: &CapabilityCatalog) -> OperatorResolutionReport {
    let mut diagnostics = Vec::new();
    if !operator.resolution.tags.is_empty() {
        diagnostics.push(
            PlanningDiagnostic::new(
                PlanningDiagnosticCode::OperatorTagsDiagnosticOnly,
                "operator resolution tags are diagnostic only for this catalog",
            )
            .with_step(operator.operator_id.clone()),
        );
    }

    let matched = if let Some(specific) = &operator.resolution.specific {
        catalog
            .get(&specific.capability_type_id, specific.capability_version)
            .filter(|contract| contract_satisfies_operator(operator, contract))
    } else {
        catalog
            .iter()
            .find(|contract| contract_satisfies_operator(operator, contract))
    };

    match matched {
        Some(contract) => {
            diagnostics.push(
                PlanningDiagnostic::new(
                    PlanningDiagnosticCode::OperatorResolved,
                    "operator resolved against capability catalog",
                )
                .with_step(operator.operator_id.clone()),
            );
            OperatorResolutionReport {
                operator_id: operator.operator_id.clone(),
                status: OperatorResolutionStatus::Resolved,
                capability_type_id: Some(contract.capability_type_id.clone()),
                capability_version: Some(contract.capability_version),
                tags: operator.resolution.tags.clone(),
                diagnostics,
            }
        }
        None => {
            diagnostics.push(
                PlanningDiagnostic::new(
                    PlanningDiagnosticCode::OperatorUnresolved,
                    "operator did not resolve against capability catalog",
                )
                .with_step(operator.operator_id.clone()),
            );
            OperatorResolutionReport {
                operator_id: operator.operator_id.clone(),
                status: OperatorResolutionStatus::Unresolved,
                capability_type_id: operator
                    .resolution
                    .specific
                    .as_ref()
                    .map(|specific| specific.capability_type_id.clone()),
                capability_version: operator
                    .resolution
                    .specific
                    .as_ref()
                    .map(|specific| specific.capability_version),
                tags: operator.resolution.tags.clone(),
                diagnostics,
            }
        }
    }
}

fn contract_satisfies_operator(
    operator: &Operator,
    contract: &crate::capability::CapabilityTypeContract,
) -> bool {
    if let Some(scope_kind) = &operator.resolution.scope_kind {
        if &contract.scope_contract.scope_kind != scope_kind {
            return false;
        }
    }

    let input_ok = operator
        .resolution
        .requires_inputs
        .iter()
        .filter(|constraint| constraint.required)
        .all(|constraint| {
            contract.input_contract.iter().any(|slot| {
                slot.accepted_artifact_type_ids
                    .iter()
                    .any(|artifact_type| artifact_type == &constraint.artifact_type_id)
            })
        });
    let output_ok = operator
        .resolution
        .requires_outputs
        .iter()
        .filter(|constraint| constraint.required)
        .all(|constraint| {
            contract
                .output_contract
                .iter()
                .any(|slot| slot.artifact_type_id == constraint.artifact_type_id)
        });
    input_ok && output_ok
}

fn invalid_report(
    source_ref: &MethodSourceRef,
    method_id: Option<String>,
    diagnostics: Vec<PlanningDiagnostic>,
) -> InvalidMethodReport {
    InvalidMethodReport {
        source_ref: Some(source_ref.stable_ref()),
        method_id,
        diagnostics,
    }
}

fn diagnostic(
    code: PlanningDiagnosticCode,
    message: impl Into<String>,
    method_id: Option<&str>,
) -> PlanningDiagnostic {
    let diagnostic = PlanningDiagnostic::new(code, message);
    match method_id {
        Some(method_id) => diagnostic.with_method(method_id.to_string()),
        None => diagnostic,
    }
}

fn method_order(left: &Method, right: &Method) -> std::cmp::Ordering {
    left.preference
        .cmp(&right.preference)
        .then_with(|| left.cost.time_ms.cmp(&right.cost.time_ms))
        .then_with(|| {
            left.cost
                .money_microdollars
                .cmp(&right.cost.money_microdollars)
        })
        .then_with(|| left.cost.provider_calls.cmp(&right.cost.provider_calls))
        .then_with(|| left.method_id.cmp(&right.method_id))
}

fn variables_in_preconditions(preconditions: &[Proposition]) -> BTreeSet<String> {
    preconditions
        .iter()
        .flat_map(variables_in_proposition)
        .collect()
}

fn variables_in_effects(effects: &[Effect]) -> BTreeSet<String> {
    effects.iter().flat_map(variables_in_effect).collect()
}

fn variables_in_composition(composition: &Composition) -> BTreeSet<String> {
    let mut variables = BTreeSet::new();
    for step in &composition.steps {
        match &step.kind {
            StepKind::Op(operator) => {
                variables.extend(variables_in_preconditions(&operator.preconditions));
                variables.extend(variables_in_effects(&operator.effects));
            }
            StepKind::Goal(proposition) => {
                variables.extend(variables_in_proposition(proposition));
            }
        }
    }
    for edge in &composition.edges {
        if let meld_lang::EdgeKind::Conditional { guard, .. } = &edge.kind {
            variables.extend(variables_in_condition(guard));
        }
    }
    variables
}

fn variables_in_effect(effect: &Effect) -> BTreeSet<String> {
    match effect {
        Effect::Assert(proposition) | Effect::Retract(proposition) => {
            variables_in_proposition(proposition)
        }
        Effect::Update {
            subject,
            dimension,
            value,
        } => [subject, dimension, value]
            .into_iter()
            .flat_map(variables_in_term)
            .collect(),
    }
}

fn variables_in_proposition(proposition: &Proposition) -> BTreeSet<String> {
    match proposition {
        Proposition::Holds {
            subject,
            dimension,
            condition,
        } => {
            let mut variables = variables_in_term(subject);
            variables.extend(variables_in_term(dimension));
            variables.extend(variables_in_condition(condition));
            variables
        }
        Proposition::Exists {
            scope,
            artifact_type,
        } => {
            let mut variables = variables_in_term(scope);
            variables.extend(variables_in_term(artifact_type));
            variables
        }
        Proposition::Accessible { scope } => variables_in_term(scope),
        Proposition::Related { src, relation, dst } => [src, relation, dst]
            .into_iter()
            .flat_map(variables_in_term)
            .collect(),
        Proposition::All(children) | Proposition::Any(children) => {
            children.iter().flat_map(variables_in_proposition).collect()
        }
        Proposition::Not(child) => variables_in_proposition(child),
    }
}

fn variables_in_condition(condition: &Condition) -> BTreeSet<String> {
    match condition {
        Condition::Above(term)
        | Condition::Below(term)
        | Condition::Equals(term)
        | Condition::Within(term)
        | Condition::Exceeds(term) => variables_in_term(term),
        Condition::In(terms) => terms.iter().flat_map(variables_in_term).collect(),
        Condition::Present | Condition::Absent => BTreeSet::new(),
    }
}

fn variables_in_term(term: &Term) -> BTreeSet<String> {
    match term {
        Term::Variable(variable) => BTreeSet::from([variable.clone()]),
        _ => BTreeSet::new(),
    }
}

fn derived_issue_in_proposition(proposition: &Proposition) -> Option<String> {
    match proposition {
        Proposition::Holds {
            subject,
            dimension,
            condition,
        } => derived_issue_in_term(subject)
            .or_else(|| derived_issue_in_term(dimension))
            .or_else(|| derived_issue_in_condition(condition)),
        Proposition::Exists {
            scope,
            artifact_type,
        } => derived_issue_in_term(scope).or_else(|| derived_issue_in_term(artifact_type)),
        Proposition::Accessible { scope } => derived_issue_in_term(scope),
        Proposition::Related { src, relation, dst } => derived_issue_in_term(src)
            .or_else(|| derived_issue_in_term(relation))
            .or_else(|| derived_issue_in_term(dst)),
        Proposition::All(children) | Proposition::Any(children) => {
            children.iter().find_map(derived_issue_in_proposition)
        }
        Proposition::Not(child) => derived_issue_in_proposition(child),
    }
}

fn derived_issue_in_condition(condition: &Condition) -> Option<String> {
    match condition {
        Condition::Above(term)
        | Condition::Below(term)
        | Condition::Equals(term)
        | Condition::Within(term)
        | Condition::Exceeds(term) => derived_issue_in_term(term),
        Condition::In(terms) => terms.iter().find_map(derived_issue_in_term),
        Condition::Present | Condition::Absent => None,
    }
}

fn derived_issue_in_term(term: &Term) -> Option<String> {
    match term {
        Term::Derived {
            source_step,
            field_path,
        } => Some(format!("{source_step}.{field_path}")),
        _ => None,
    }
}
