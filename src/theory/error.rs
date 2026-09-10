use thiserror::Error;

use super::contracts::TheoryRouteId;

/// Stable structural diagnostic emitted by package attachment.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TheoryRouterDiagnostic {
    pub code: String,
    pub package_id: Option<String>,
    pub component_id: Option<String>,
    pub route: Option<TheoryRouteId>,
    pub owner_code: Option<String>,
    pub message: String,
}

impl TheoryRouterDiagnostic {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            package_id: None,
            component_id: None,
            route: None,
            owner_code: None,
            message: message.into(),
        }
    }

    pub fn package(mut self, package_id: impl Into<String>) -> Self {
        self.package_id = Some(package_id.into());
        self
    }

    pub fn component(mut self, component_id: impl Into<String>) -> Self {
        self.component_id = Some(component_id.into());
        self
    }

    pub fn route(mut self, route: TheoryRouteId) -> Self {
        self.route = Some(route);
        self
    }

    pub fn owner(mut self, owner_code: impl Into<String>) -> Self {
        self.owner_code = Some(owner_code.into());
        self
    }
}

/// Failure of structural package installation or resolution.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[error("{diagnostic_code}: {message}")]
pub struct TheoryRouterError {
    pub diagnostic_code: String,
    pub message: String,
    pub diagnostic: Box<TheoryRouterDiagnostic>,
}

impl From<TheoryRouterDiagnostic> for TheoryRouterError {
    fn from(diagnostic: TheoryRouterDiagnostic) -> Self {
        Self {
            diagnostic_code: diagnostic.code.clone(),
            message: diagnostic.message.clone(),
            diagnostic: Box::new(diagnostic),
        }
    }
}

pub(crate) fn error(code: impl Into<String>, message: impl Into<String>) -> TheoryRouterError {
    TheoryRouterDiagnostic::new(code, message).into()
}
