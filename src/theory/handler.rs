use std::sync::Arc;

use super::contracts::{
    InstalledPackageLinkView, InstalledTheoryComponentRef, OwnerRouteDiagnostic,
    OwnerValidationToken, PackageLinkView, RoutedComponentSource, TheoryRevisionRef,
    TheoryRouteContract, TheoryRouteHandler,
};

pub type OwnerValidatePort =
    Arc<dyn Fn(&str, &[u8]) -> Result<(), OwnerRouteDiagnostic> + Send + Sync>;
pub type OwnerInstallPort =
    Arc<dyn Fn(&str, &[u8], u64) -> Result<TheoryRevisionRef, OwnerRouteDiagnostic> + Send + Sync>;
pub type OwnerVerifyPort =
    Arc<dyn Fn(&TheoryRevisionRef) -> Result<(), OwnerRouteDiagnostic> + Send + Sync>;
pub type OwnerLinkPort = Arc<
    dyn Fn(
            &InstalledTheoryComponentRef,
            &InstalledPackageLinkView,
        ) -> Result<(), OwnerRouteDiagnostic>
        + Send
        + Sync,
>;

/// Thin generic adapter over narrow owner validation, command, and query ports.
pub struct PortBackedTheoryRouteHandler {
    contract: TheoryRouteContract,
    validate_port: OwnerValidatePort,
    install_port: OwnerInstallPort,
    verify_port: OwnerVerifyPort,
    link_port: OwnerLinkPort,
}

impl PortBackedTheoryRouteHandler {
    pub fn new(
        contract: TheoryRouteContract,
        validate_port: OwnerValidatePort,
        install_port: OwnerInstallPort,
        verify_port: OwnerVerifyPort,
        link_port: OwnerLinkPort,
    ) -> Self {
        Self {
            contract,
            validate_port,
            install_port,
            verify_port,
            link_port,
        }
    }
}

impl PortBackedTheoryRouteHandler {
    /// Invoke the same owner validation port used by the package router. Local
    /// serialized owners receive body bytes before a parent-issued routing token.
    pub fn validate_body(&self, owner_id: &str, bytes: &[u8]) -> Result<(), OwnerRouteDiagnostic> {
        (self.validate_port)(owner_id, bytes)
    }

    /// The semantic store remains the authority for immutable revision identity.
    pub fn install_body(
        &self,
        owner_id: &str,
        bytes: &[u8],
        seq: u64,
    ) -> Result<TheoryRevisionRef, OwnerRouteDiagnostic> {
        (self.install_port)(owner_id, bytes, seq)
    }

    pub fn verify_revision(
        &self,
        reference: &TheoryRevisionRef,
    ) -> Result<(), OwnerRouteDiagnostic> {
        (self.verify_port)(reference)
    }
}

impl TheoryRouteHandler for PortBackedTheoryRouteHandler {
    fn contract(&self) -> TheoryRouteContract {
        self.contract.clone()
    }

    fn validate(
        &self,
        source: &RoutedComponentSource,
        package: &PackageLinkView,
    ) -> Result<OwnerValidationToken, OwnerRouteDiagnostic> {
        self.validate_body(&source.owner_component_id, &source.canonical_bytes)?;
        Ok(OwnerValidationToken::new(
            &self.contract,
            package,
            source,
            source.canonical_bytes.to_vec(),
        ))
    }

    fn install(
        &self,
        source: &RoutedComponentSource,
        validation: OwnerValidationToken,
        installed_at_seq: u64,
    ) -> Result<InstalledTheoryComponentRef, OwnerRouteDiagnostic> {
        let owner_revision = self.install_body(
            &source.owner_component_id,
            validation.opaque(),
            installed_at_seq,
        )?;
        Ok(InstalledTheoryComponentRef {
            component_id: source.component_id.clone(),
            route: source.route.clone(),
            component_schema_version: source.component_schema_version,
            source_content_hash: source.source_content_hash.clone(),
            owner_revision,
        })
    }

    fn verify(&self, reference: &InstalledTheoryComponentRef) -> Result<(), OwnerRouteDiagnostic> {
        self.verify_revision(&reference.owner_revision)
    }

    fn validate_links(
        &self,
        reference: &InstalledTheoryComponentRef,
        package: &InstalledPackageLinkView,
    ) -> Result<(), OwnerRouteDiagnostic> {
        (self.link_port)(reference, package)
    }
}

pub fn no_semantic_links() -> OwnerLinkPort {
    Arc::new(|_, _| Ok(()))
}
